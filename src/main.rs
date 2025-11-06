mod gpio;
mod rtsp;
mod websocket;

use anyhow::Result;
use log::info;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();

    info!("Starting cam-control-2 application");

    // Initialize GPIO controller
    let gpio = match gpio::GpioController::new() {
        Ok(controller) => {
            info!("GPIO controller initialized successfully");
            Arc::new(Mutex::new(controller))
        }
        Err(e) => {
            log::warn!("Failed to initialize GPIO controller: {}. Running without GPIO support.", e);
            log::warn!("Commands will be logged but not executed on hardware.");
            // Create a mock controller for development/testing
            return Err(anyhow::anyhow!("GPIO initialization failed. This application requires GPIO support."));
        }
    };

    // Start WebSocket server
    let ws_port = 8080;
    let websocket_server = websocket::WebSocketServer::new(ws_port, Arc::clone(&gpio));
    
    // Start RTSP server in a separate thread
    let rtsp_port = 8554;
    let rtsp_server = rtsp::RtspServer::new(rtsp_port);
    
    std::thread::spawn(move || {
        if let Err(e) = rtsp_server.run() {
            log::error!("RTSP server error: {}", e);
        }
    });

    // Run WebSocket server (this blocks)
    websocket_server.run().await?;

    Ok(())
}

