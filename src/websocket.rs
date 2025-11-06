use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use log::{info, error, warn};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_tungstenite::{accept_async, tungstenite::Message};

use crate::gpio::{GpioController, TurretState};

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "command")]
pub enum Command {
    AcquireControl,
    ReleaseControl,
    GetState,
    HomeX,
    HomeY,
    MoveX { direction: String, steps: u32 },
    MoveY { direction: String, steps: u32 },
    TriggerActuator { duration_ms: u64 },
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Response {
    Standard {
        status: String,
        message: String,
    },
    State {
        status: String,
        state: TurretState,
    },
}

struct ControlLock {
    controller: Option<SocketAddr>,
}

impl ControlLock {
    fn new() -> Self {
        Self { controller: None }
    }

    fn acquire(&mut self, addr: SocketAddr) -> bool {
        match self.controller {
            None => {
                self.controller = Some(addr);
                true
            }
            Some(current) if current == addr => true,
            Some(_) => false,
        }
    }

    fn release(&mut self, addr: SocketAddr) -> bool {
        match self.controller {
            Some(current) if current == addr => {
                self.controller = None;
                true
            }
            _ => false,
        }
    }

    fn has_control(&self, addr: SocketAddr) -> bool {
        match self.controller {
            Some(current) => current == addr,
            None => false,
        }
    }

    fn get_controller(&self) -> Option<SocketAddr> {
        self.controller
    }
}

pub struct WebSocketServer {
    port: u16,
    gpio: Arc<Mutex<GpioController>>,
    control_lock: Arc<Mutex<ControlLock>>,
}

impl WebSocketServer {
    pub fn new(port: u16, gpio: Arc<Mutex<GpioController>>) -> Self {
        Self {
            port,
            gpio,
            control_lock: Arc::new(Mutex::new(ControlLock::new())),
        }
    }

    pub async fn run(&self) -> Result<()> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        info!("WebSocket server listening on: {}", addr);

        while let Ok((stream, addr)) = listener.accept().await {
            info!("New WebSocket connection from: {}", addr);
            let gpio = Arc::clone(&self.gpio);
            let control_lock = Arc::clone(&self.control_lock);
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, addr, gpio, control_lock).await {
                    error!("Error handling connection: {}", e);
                }
            });
        }

        Ok(())
    }
}

async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    gpio: Arc<Mutex<GpioController>>,
    control_lock: Arc<Mutex<ControlLock>>,
) -> Result<()> {
    let ws_stream = accept_async(stream).await?;
    let (mut write, mut read) = ws_stream.split();

    info!("WebSocket connection established with {}", addr);

    // Send initial status
    let initial_status = {
        let lock = control_lock.lock().await;
        if let Some(controller) = lock.get_controller() {
            Response::Standard {
                status: "info".to_string(),
                message: format!("Control currently held by {}. Use AcquireControl to request control.", controller),
            }
        } else {
            Response::Standard {
                status: "info".to_string(),
                message: "No active controller. Use AcquireControl to take control.".to_string(),
            }
        }
    };
    let initial_msg = serde_json::to_string(&initial_status)?;
    write.send(Message::Text(initial_msg)).await?;

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                info!("Received message from {}: {}", addr, text);
                
                let response = match serde_json::from_str::<Command>(&text) {
                    Ok(command) => {
                        match execute_command(command, addr, &gpio, &control_lock).await {
                            Ok(response) => response,
                            Err(e) => Response::Standard {
                                status: "error".to_string(),
                                message: format!("{}", e),
                            },
                        }
                    }
                    Err(e) => Response::Standard {
                        status: "error".to_string(),
                        message: format!("Invalid command format: {}", e),
                    },
                };

                let response_text = serde_json::to_string(&response)?;
                write.send(Message::Text(response_text)).await?;
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket connection closed by {}", addr);
                // Release control if this client had it
                let mut lock = control_lock.lock().await;
                if lock.release(addr) {
                    info!("Released control from disconnected client {}", addr);
                }
                break;
            }
            Ok(Message::Ping(data)) => {
                write.send(Message::Pong(data)).await?;
            }
            Ok(_) => {
                warn!("Received unsupported message type from {}", addr);
            }
            Err(e) => {
                error!("Error reading message from {}: {}", addr, e);
                break;
            }
        }
    }

    // Clean up: release control on disconnect
    let mut lock = control_lock.lock().await;
    if lock.release(addr) {
        info!("Released control from disconnected client {}", addr);
    }

    Ok(())
}

async fn execute_command(
    command: Command,
    addr: SocketAddr,
    gpio: &Arc<Mutex<GpioController>>,
    control_lock: &Arc<Mutex<ControlLock>>,
) -> Result<Response> {
    match command {
        Command::GetState => {
            // GetState doesn't require control - read-only operation
            let gpio_ctrl = gpio.lock().await;
            let state = gpio_ctrl.get_state();
            Ok(Response::State {
                status: "success".to_string(),
                state,
            })
        }
        Command::AcquireControl => {
            let mut lock = control_lock.lock().await;
            if lock.acquire(addr) {
                info!("Control acquired by {}", addr);
                Ok(Response::Standard {
                    status: "success".to_string(),
                    message: format!("Control acquired by {}", addr),
                })
            } else {
                let current_controller = lock.get_controller().unwrap();
                Err(anyhow::anyhow!(
                    "Control already held by {}. Please wait or ask them to release control.",
                    current_controller
                ))
            }
        }
        Command::ReleaseControl => {
            let mut lock = control_lock.lock().await;
            if lock.release(addr) {
                info!("Control released by {}", addr);
                Ok(Response::Standard {
                    status: "success".to_string(),
                    message: format!("Control released by {}", addr),
                })
            } else {
                Err(anyhow::anyhow!("You don't have control to release"))
            }
        }
        Command::HomeX => {
            let lock = control_lock.lock().await;
            if !lock.has_control(addr) {
                return Err(anyhow::anyhow!(
                    "You don't have control. Use AcquireControl command first."
                ));
            }
            drop(lock);

            let mut gpio_ctrl = gpio.lock().await;
            gpio_ctrl.home_x()?;
            Ok(Response::Standard {
                status: "success".to_string(),
                message: "X-axis homed successfully".to_string(),
            })
        }
        Command::HomeY => {
            let lock = control_lock.lock().await;
            if !lock.has_control(addr) {
                return Err(anyhow::anyhow!(
                    "You don't have control. Use AcquireControl command first."
                ));
            }
            drop(lock);

            let mut gpio_ctrl = gpio.lock().await;
            gpio_ctrl.home_y()?;
            Ok(Response::Standard {
                status: "success".to_string(),
                message: "Y-axis homed successfully".to_string(),
            })
        }
        Command::MoveX { direction, steps } => {
            let lock = control_lock.lock().await;
            if !lock.has_control(addr) {
                return Err(anyhow::anyhow!(
                    "You don't have control. Use AcquireControl command first."
                ));
            }
            drop(lock); // Release the lock before GPIO operations

            let mut gpio_ctrl = gpio.lock().await;
            gpio_ctrl.move_x(&direction, steps)?;
            Ok(Response::Standard {
                status: "success".to_string(),
                message: format!("Moved X-axis {} steps in {} direction", steps, direction),
            })
        }
        Command::MoveY { direction, steps } => {
            let lock = control_lock.lock().await;
            if !lock.has_control(addr) {
                return Err(anyhow::anyhow!(
                    "You don't have control. Use AcquireControl command first."
                ));
            }
            drop(lock); // Release the lock before GPIO operations

            let mut gpio_ctrl = gpio.lock().await;
            gpio_ctrl.move_y(&direction, steps)?;
            Ok(Response::Standard {
                status: "success".to_string(),
                message: format!("Moved Y-axis {} steps in {} direction", steps, direction),
            })
        }
        Command::TriggerActuator { duration_ms } => {
            let lock = control_lock.lock().await;
            if !lock.has_control(addr) {
                return Err(anyhow::anyhow!(
                    "You don't have control. Use AcquireControl command first."
                ));
            }
            drop(lock); // Release the lock before GPIO operations

            let mut gpio_ctrl = gpio.lock().await;
            gpio_ctrl.trigger_actuator(duration_ms).await?;
            Ok(Response::Standard {
                status: "success".to_string(),
                message: format!("Triggered actuator for {} ms", duration_ms),
            })
        }
    }
}
