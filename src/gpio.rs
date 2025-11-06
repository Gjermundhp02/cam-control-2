use anyhow::{Result, Context};
use log::{info, warn};
use rppal::gpio::{Gpio, OutputPin, InputPin, Level};
use serde::Serialize;
use std::thread;
use std::time::Duration;

// GPIO pin assignments for motors and actuator
const X_STEP_PIN: u8 = 17;
const X_DIR_PIN: u8 = 27;
const Y_STEP_PIN: u8 = 22;
const Y_DIR_PIN: u8 = 23;
const ACTUATOR_PIN: u8 = 24;

// GPIO pin assignments for limit switches
const X_LIMIT_PIN: u8 = 5;      // X-axis home/reference position (for continuous rotation)
const Y_START_LIMIT_PIN: u8 = 6; // Y-axis start limit (minimum position)
const Y_STOP_LIMIT_PIN: u8 = 13; // Y-axis stop limit (maximum position)

// Configuration constants
const STEPS_PER_DEGREE: f32 = 200.0 / 360.0; // Assuming 200 steps per revolution for typical stepper

#[derive(Debug, Clone, Serialize)]
pub struct TurretState {
    pub x_angle: f32,      // Current X-axis angle in degrees
    pub y_angle: f32,      // Current Y-axis angle in degrees
    pub x_steps: i32,      // Total steps from origin on X-axis
    pub y_steps: i32,      // Total steps from origin on Y-axis
    pub actuator_active: bool,
    pub x_limit_triggered: bool,    // X-axis limit switch state
    pub y_start_limit_triggered: bool, // Y-axis start limit switch state
    pub y_stop_limit_triggered: bool,  // Y-axis stop limit switch state
    pub x_homed: bool,     // Whether X-axis has been homed
    pub y_homed: bool,     // Whether Y-axis has been homed
}

impl Default for TurretState {
    fn default() -> Self {
        Self {
            x_angle: 0.0,
            y_angle: 0.0,
            x_steps: 0,
            y_steps: 0,
            actuator_active: false,
            x_limit_triggered: false,
            y_start_limit_triggered: false,
            y_stop_limit_triggered: false,
            x_homed: false,
            y_homed: false,
        }
    }
}

pub struct GpioController {
    x_step: OutputPin,
    x_dir: OutputPin,
    y_step: OutputPin,
    y_dir: OutputPin,
    actuator: OutputPin,
    x_limit: InputPin,
    y_start_limit: InputPin,
    y_stop_limit: InputPin,
    state: TurretState,
}

impl GpioController {
    pub fn new() -> Result<Self> {
        let gpio = Gpio::new().context("Failed to initialize GPIO")?;
        
        let mut controller = Self {
            x_step: gpio.get(X_STEP_PIN)?.into_output(),
            x_dir: gpio.get(X_DIR_PIN)?.into_output(),
            y_step: gpio.get(Y_STEP_PIN)?.into_output(),
            y_dir: gpio.get(Y_DIR_PIN)?.into_output(),
            actuator: gpio.get(ACTUATOR_PIN)?.into_output(),
            x_limit: gpio.get(X_LIMIT_PIN)?.into_input_pullup(),
            y_start_limit: gpio.get(Y_START_LIMIT_PIN)?.into_input_pullup(),
            y_stop_limit: gpio.get(Y_STOP_LIMIT_PIN)?.into_input_pullup(),
            state: TurretState::default(),
        };
        
        // Update initial limit switch states
        controller.update_limit_states();
        
        Ok(controller)
    }

    fn update_limit_states(&mut self) {
        // Limit switches are active LOW (triggered when pin reads LOW)
        self.state.x_limit_triggered = self.x_limit.read() == Level::Low;
        self.state.y_start_limit_triggered = self.y_start_limit.read() == Level::Low;
        self.state.y_stop_limit_triggered = self.y_stop_limit.read() == Level::Low;
    }

    pub fn get_state(&self) -> TurretState {
        self.state.clone()
    }

    pub fn move_x(&mut self, direction: &str, steps: u32) -> Result<()> {
        info!("Moving X-axis: direction={}, steps={}", direction, steps);
        
        // Set direction pin and calculate step delta
        let step_delta = match direction.to_lowercase().as_str() {
            "forward" | "clockwise" | "cw" => {
                self.x_dir.set_high();
                steps as i32
            }
            "backward" | "counterclockwise" | "ccw" => {
                self.x_dir.set_low();
                -(steps as i32)
            }
            _ => return Err(anyhow::anyhow!("Invalid direction: {}", direction)),
        };

        // Generate step pulses
        self.generate_steps(&mut self.x_step, steps)?;
        
        // Update state
        self.state.x_steps += step_delta;
        self.state.x_angle = self.state.x_steps as f32 / STEPS_PER_DEGREE;
        
        info!("X-axis position: {} steps, {} degrees", self.state.x_steps, self.state.x_angle);
        
        Ok(())
    }

    pub fn move_y(&mut self, direction: &str, steps: u32) -> Result<()> {
        info!("Moving Y-axis: direction={}, steps={}", direction, steps);
        
        // Update limit switch states before moving
        self.update_limit_states();
        
        // Set direction pin and calculate step delta
        let (going_to_start, going_to_stop) = match direction.to_lowercase().as_str() {
            "forward" | "clockwise" | "cw" => {
                self.y_dir.set_high();
                (false, true) // Moving toward stop limit
            }
            "backward" | "counterclockwise" | "ccw" => {
                self.y_dir.set_low();
                (true, false) // Moving toward start limit
            }
            _ => return Err(anyhow::anyhow!("Invalid direction: {}", direction)),
        };

        // Check if we're already at a limit before moving
        if going_to_stop && self.state.y_stop_limit_triggered {
            return Err(anyhow::anyhow!("Y-axis at stop limit. Cannot move further in this direction."));
        }
        if going_to_start && self.state.y_start_limit_triggered {
            return Err(anyhow::anyhow!("Y-axis at start limit. Cannot move further in this direction."));
        }

        // Generate step pulses with limit checking
        let steps_moved = self.generate_steps_with_y_limits(&mut self.y_step, steps, going_to_start, going_to_stop)?;
        
        // Calculate actual step delta based on direction
        let step_delta = if going_to_stop {
            steps_moved as i32
        } else {
            -(steps_moved as i32)
        };
        
        // Update state
        self.state.y_steps += step_delta;
        self.state.y_angle = self.state.y_steps as f32 / STEPS_PER_DEGREE;
        
        info!("Y-axis position: {} steps, {} degrees (moved {} steps)", 
              self.state.y_steps, self.state.y_angle, steps_moved);
        
        if steps_moved < steps {
            warn!("Y-axis movement stopped at limit switch after {} steps (requested {})", 
                  steps_moved, steps);
        }
        
        Ok(())
    }

    pub async fn trigger_actuator(&mut self, duration_ms: u64) -> Result<()> {
        info!("Triggering actuator for {} ms", duration_ms);
        
        // Activate actuator
        self.state.actuator_active = true;
        self.actuator.set_high();
        
        // Wait for specified duration
        tokio::time::sleep(Duration::from_millis(duration_ms)).await;
        
        // Deactivate actuator
        self.actuator.set_low();
        self.state.actuator_active = false;
        
        Ok(())
    }

    pub fn home_x(&mut self) -> Result<()> {
        info!("Homing X-axis...");
        
        // X-axis is continuous, so we just find the home position
        // First, move away from limit if we're already on it
        self.update_limit_states();
        if self.state.x_limit_triggered {
            info!("Already at X home position, moving away briefly");
            self.x_dir.set_high();
            self.generate_steps(&mut self.x_step, 50)?;
            thread::sleep(Duration::from_millis(100));
        }
        
        // Now move slowly toward the limit switch
        self.x_dir.set_low(); // Direction to home
        let mut steps_taken = 0;
        const MAX_HOMING_STEPS: u32 = 3600; // Max one full rotation
        
        for _ in 0..MAX_HOMING_STEPS {
            self.x_step.set_high();
            thread::sleep(Duration::from_micros(2000)); // Slower for homing
            self.x_step.set_low();
            thread::sleep(Duration::from_micros(2000));
            
            steps_taken += 1;
            
            // Check limit switch
            if self.x_limit.read() == Level::Low {
                info!("X-axis home position found after {} steps", steps_taken);
                // Reset position to zero
                self.state.x_steps = 0;
                self.state.x_angle = 0.0;
                self.state.x_homed = true;
                self.update_limit_states();
                return Ok(());
            }
        }
        
        Err(anyhow::anyhow!("X-axis homing failed: limit switch not found after {} steps", MAX_HOMING_STEPS))
    }

    pub fn home_y(&mut self) -> Result<()> {
        info!("Homing Y-axis to start position...");
        
        // Move to start limit switch
        self.y_dir.set_low(); // Direction to start limit
        let mut steps_taken = 0;
        const MAX_HOMING_STEPS: u32 = 2000;
        
        self.update_limit_states();
        
        // If already at start limit, we're done
        if self.state.y_start_limit_triggered {
            info!("Already at Y-axis start position");
            self.state.y_steps = 0;
            self.state.y_angle = 0.0;
            self.state.y_homed = true;
            return Ok(());
        }
        
        for _ in 0..MAX_HOMING_STEPS {
            self.y_step.set_high();
            thread::sleep(Duration::from_micros(2000)); // Slower for homing
            self.y_step.set_low();
            thread::sleep(Duration::from_micros(2000));
            
            steps_taken += 1;
            
            // Check start limit switch
            if self.y_start_limit.read() == Level::Low {
                info!("Y-axis start position found after {} steps", steps_taken);
                // Reset position to zero at start limit
                self.state.y_steps = 0;
                self.state.y_angle = 0.0;
                self.state.y_homed = true;
                self.update_limit_states();
                return Ok(());
            }
        }
        
        Err(anyhow::anyhow!("Y-axis homing failed: start limit switch not found after {} steps", MAX_HOMING_STEPS))
    }

    fn generate_steps(&mut self, step_pin: &mut OutputPin, steps: u32) -> Result<()> {
        const STEP_DELAY_US: u64 = 1000; // 1ms delay between steps
        
        for _ in 0..steps {
            step_pin.set_high();
            thread::sleep(Duration::from_micros(STEP_DELAY_US));
            step_pin.set_low();
            thread::sleep(Duration::from_micros(STEP_DELAY_US));
        }
        
        Ok(())
    }

    fn generate_steps_with_y_limits(
        &mut self,
        step_pin: &mut OutputPin,
        steps: u32,
        going_to_start: bool,
        going_to_stop: bool,
    ) -> Result<u32> {
        const STEP_DELAY_US: u64 = 1000; // 1ms delay between steps
        let mut steps_moved = 0;
        
        for _ in 0..steps {
            step_pin.set_high();
            thread::sleep(Duration::from_micros(STEP_DELAY_US));
            step_pin.set_low();
            thread::sleep(Duration::from_micros(STEP_DELAY_US));
            
            steps_moved += 1;
            
            // Check limit switches after each step
            if going_to_stop && self.y_stop_limit.read() == Level::Low {
                self.update_limit_states();
                info!("Y-axis stop limit reached");
                break;
            }
            if going_to_start && self.y_start_limit.read() == Level::Low {
                self.update_limit_states();
                info!("Y-axis start limit reached");
                break;
            }
        }
        
        // Update limit states after movement
        self.update_limit_states();
        
        Ok(steps_moved)
    }
}

// Mock implementation for testing on non-Raspberry Pi hardware
pub struct MockGpioController {
    state: TurretState,
    max_y_steps: i32, // Simulated range for Y-axis
}

impl MockGpioController {
    pub fn new() -> Self {
        warn!("Using mock GPIO controller");
        Self {
            state: TurretState::default(),
            max_y_steps: 1000, // Simulated max Y range
        }
    }

    pub fn get_state(&self) -> TurretState {
        self.state.clone()
    }

    pub fn move_x(&mut self, direction: &str, steps: u32) -> Result<()> {
        info!("[MOCK] Moving X-axis: direction={}, steps={}", direction, steps);
        
        let step_delta = match direction.to_lowercase().as_str() {
            "forward" | "clockwise" | "cw" => steps as i32,
            "backward" | "counterclockwise" | "ccw" => -(steps as i32),
            _ => return Err(anyhow::anyhow!("Invalid direction: {}", direction)),
        };
        
        self.state.x_steps += step_delta;
        self.state.x_angle = self.state.x_steps as f32 / STEPS_PER_DEGREE;
        
        Ok(())
    }

    pub fn move_y(&mut self, direction: &str, steps: u32) -> Result<()> {
        info!("[MOCK] Moving Y-axis: direction={}, steps={}", direction, steps);
        
        // Simulate limit switches
        let going_forward = matches!(direction.to_lowercase().as_str(), "forward" | "clockwise" | "cw");
        
        if going_forward && self.state.y_steps >= self.max_y_steps {
            self.state.y_stop_limit_triggered = true;
            return Err(anyhow::anyhow!("Y-axis at stop limit. Cannot move further in this direction."));
        }
        
        if !going_forward && self.state.y_steps <= 0 {
            self.state.y_start_limit_triggered = true;
            return Err(anyhow::anyhow!("Y-axis at start limit. Cannot move further in this direction."));
        }
        
        let step_delta = match direction.to_lowercase().as_str() {
            "forward" | "clockwise" | "cw" => steps as i32,
            "backward" | "counterclockwise" | "ccw" => -(steps as i32),
            _ => return Err(anyhow::anyhow!("Invalid direction: {}", direction)),
        };
        
        self.state.y_steps += step_delta;
        self.state.y_steps = self.state.y_steps.clamp(0, self.max_y_steps);
        self.state.y_angle = self.state.y_steps as f32 / STEPS_PER_DEGREE;
        
        // Update simulated limit states
        self.state.y_start_limit_triggered = self.state.y_steps == 0;
        self.state.y_stop_limit_triggered = self.state.y_steps >= self.max_y_steps;
        
        Ok(())
    }

    pub async fn trigger_actuator(&mut self, duration_ms: u64) -> Result<()> {
        info!("[MOCK] Triggering actuator for {} ms", duration_ms);
        self.state.actuator_active = true;
        tokio::time::sleep(Duration::from_millis(duration_ms)).await;
        self.state.actuator_active = false;
        Ok(())
    }

    pub fn home_x(&mut self) -> Result<()> {
        info!("[MOCK] Homing X-axis");
        self.state.x_steps = 0;
        self.state.x_angle = 0.0;
        self.state.x_homed = true;
        self.state.x_limit_triggered = true;
        Ok(())
    }

    pub fn home_y(&mut self) -> Result<()> {
        info!("[MOCK] Homing Y-axis");
        self.state.y_steps = 0;
        self.state.y_angle = 0.0;
        self.state.y_homed = true;
        self.state.y_start_limit_triggered = true;
        self.state.y_stop_limit_triggered = false;
        Ok(())
    }
}
