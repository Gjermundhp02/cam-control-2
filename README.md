# cam-control-2

A Rust application for controlling a camera turret system with RTSP streaming and WebSocket command interface.

## Features

- **RTSP GStreamer Server**: Streams camera feed that multiple devices can view simultaneously
- **WebSocket Command Server**: Accepts commands for controlling the turret
- **Turret Control**: Controls two stepper motors for X and Y axis movement
- **Actuator Control**: Triggers an actuator via GPIO
- **Multi-Client Support**: 
  - Multiple devices can connect to the RTSP stream
  - Only one device can control the turret at a time
  - All devices can read the turret state (angles, actuator status)

## System Requirements

### Hardware
- Raspberry Pi (or compatible GPIO-enabled device)
- Two stepper motors (X and Y axis)
- One actuator
- Three limit switches:
  - X-axis home position switch (for continuous rotation reference)
  - Y-axis start limit switch (minimum position)
  - Y-axis stop limit switch (maximum position)
- Camera (USB, CSI, or compatible with v4l2)

### Software
- Rust 1.70 or later
- GStreamer 1.0 and development libraries
- System libraries:
  - libglib2.0-dev
  - libgobject-2.0-dev
  - libgstreamer1.0-dev
  - libgstreamer-plugins-base1.0-dev
  - libgstreamer-rtsp-server-1.0-dev

## Installation

### Install System Dependencies (Debian/Ubuntu/Raspberry Pi OS)

```bash
sudo apt-get update
sudo apt-get install -y \
    libglib2.0-dev \
    libgobject-2.0-dev \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    libgstreamer-rtsp-server-1.0-dev \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly \
    gstreamer1.0-tools
```

### Build the Application

```bash
cargo build --release
```

## Configuration

### GPIO Pin Assignments

The default GPIO pin assignments are defined in `src/gpio.rs`:

**Motor Control:**
- **X-axis Step Pin**: GPIO 17
- **X-axis Direction Pin**: GPIO 27
- **Y-axis Step Pin**: GPIO 22
- **Y-axis Direction Pin**: GPIO 23
- **Actuator Pin**: GPIO 24

**Limit Switches:**
- **X-axis Home Limit**: GPIO 5 (for continuous rotation reference)
- **Y-axis Start Limit**: GPIO 6 (minimum position)
- **Y-axis Stop Limit**: GPIO 13 (maximum position)

Limit switches are configured with internal pull-up resistors and are active LOW (triggered when connected to ground).

To change these, edit the constants in `src/gpio.rs`:

```rust
// Motor pins
const X_STEP_PIN: u8 = 17;
const X_DIR_PIN: u8 = 27;
const Y_STEP_PIN: u8 = 22;
const Y_DIR_PIN: u8 = 23;
const ACTUATOR_PIN: u8 = 24;

// Limit switch pins
const X_LIMIT_PIN: u8 = 5;
const Y_START_LIMIT_PIN: u8 = 6;
const Y_STOP_LIMIT_PIN: u8 = 13;
```

### Camera Configuration

The default camera pipeline in `src/rtsp.rs` uses `/dev/video0`. To use a different camera:

- For Raspberry Pi Camera Module, change `v4l2src` to `rpicamsrc`
- For different video devices, update `device=/dev/video0` to your device path
- For testing without a camera, use `videotestsrc` instead of `v4l2src`

## Usage

### Running the Application

```bash
sudo ./target/release/cam-control-2
```

Note: `sudo` is required for GPIO access.

### WebSocket Server

The WebSocket server listens on port **8080** by default.

### RTSP Stream

The RTSP stream is available at: `rtsp://<device-ip>:8554/stream`

You can view the stream with:

```bash
# Using gst-launch-1.0
gst-launch-1.0 playbin uri=rtsp://<device-ip>:8554/stream

# Using VLC
vlc rtsp://<device-ip>:8554/stream

# Using ffplay
ffplay rtsp://<device-ip>:8554/stream
```

## WebSocket Commands

All commands are sent as JSON messages to the WebSocket server.

### Control Commands (Require Control Lock)

#### Acquire Control
```json
{"command": "AcquireControl"}
```

#### Release Control
```json
{"command": "ReleaseControl"}
```

#### Home X-Axis
```json
{"command": "HomeX"}
```

Moves the X-axis to find the home position using the limit switch. Once found, the position is reset to 0 degrees/steps.

#### Home Y-Axis
```json
{"command": "HomeY"}
```

Moves the Y-axis to the start limit switch. Once found, the position is reset to 0 degrees/steps.

#### Move X-Axis
```json
{
  "command": "MoveX",
  "direction": "forward",
  "steps": 100
}
```

Directions: `forward`, `backward`, `clockwise` (cw), `counterclockwise` (ccw)

#### Move Y-Axis
```json
{
  "command": "MoveY",
  "direction": "backward",
  "steps": 50
}
```

Directions: `forward`, `backward`, `clockwise` (cw), `counterclockwise` (ccw)

**Note:** Y-axis movement automatically stops if a limit switch is triggered, preventing damage to the mechanism.

#### Trigger Actuator
```json
{
  "command": "TriggerActuator",
  "duration_ms": 1000
}
```

### Read-Only Commands (No Control Required)

#### Get Turret State
```json
{"command": "GetState"}
```

Response:
```json
{
  "status": "success",
  "state": {
    "x_angle": 45.0,
    "y_angle": -30.0,
    "x_steps": 250,
    "y_steps": -166,
    "actuator_active": false,
    "x_limit_triggered": false,
    "y_start_limit_triggered": false,
    "y_stop_limit_triggered": false,
    "x_homed": true,
    "y_homed": true
  }
}
```

## Example Client

### Python WebSocket Client

```python
import asyncio
import websockets
import json

async def control_turret():
    uri = "ws://192.168.1.100:8080"
    
    async with websockets.connect(uri) as websocket:
        # Acquire control
        await websocket.send(json.dumps({"command": "AcquireControl"}))
        response = await websocket.recv()
        print(f"Acquire: {response}")
        
        # Get current state
        await websocket.send(json.dumps({"command": "GetState"}))
        response = await websocket.recv()
        print(f"State: {response}")
        
        # Home X-axis
        await websocket.send(json.dumps({"command": "HomeX"}))
        response = await websocket.recv()
        print(f"Home X: {response}")
        
        # Home Y-axis
        await websocket.send(json.dumps({"command": "HomeY"}))
        response = await websocket.recv()
        print(f"Home Y: {response}")
        
        # Move X-axis
        await websocket.send(json.dumps({
            "command": "MoveX",
            "direction": "forward",
            "steps": 100
        }))
        response = await websocket.recv()
        print(f"Move X: {response}")
        
        # Move Y-axis
        await websocket.send(json.dumps({
            "command": "MoveY",
            "direction": "backward",
            "steps": 50
        }))
        response = await websocket.recv()
        print(f"Move Y: {response}")
        
        # Trigger actuator
        await websocket.send(json.dumps({
            "command": "TriggerActuator",
            "duration_ms": 500
        }))
        response = await websocket.recv()
        print(f"Actuator: {response}")
        
        # Release control
        await websocket.send(json.dumps({"command": "ReleaseControl"}))
        response = await websocket.recv()
        print(f"Release: {response}")

asyncio.run(control_turret())
```

### JavaScript WebSocket Client

```javascript
const ws = new WebSocket('ws://192.168.1.100:8080');

ws.onopen = () => {
    console.log('Connected to turret controller');
    
    // Acquire control
    ws.send(JSON.stringify({command: 'AcquireControl'}));
};

ws.onmessage = (event) => {
    const response = JSON.parse(event.data);
    console.log('Response:', response);
    
    if (response.status === 'success') {
        // Get state
        ws.send(JSON.stringify({command: 'GetState'}));
    }
};

// Move turret
function moveX(direction, steps) {
    ws.send(JSON.stringify({
        command: 'MoveX',
        direction: direction,
        steps: steps
    }));
}

function moveY(direction, steps) {
    ws.send(JSON.stringify({
        command: 'MoveY',
        direction: direction,
        steps: steps
    }));
}

function triggerActuator(durationMs) {
    ws.send(JSON.stringify({
        command: 'TriggerActuator',
        duration_ms: durationMs
    }));
}
```

## Architecture

### Module Structure

- **`main.rs`**: Entry point, initializes GPIO and starts servers
- **`gpio.rs`**: GPIO controller for stepper motors and actuator with state tracking
- **`rtsp.rs`**: RTSP GStreamer server for camera streaming
- **`websocket.rs`**: WebSocket server for command handling and control locking

### Control Flow

1. Application starts and initializes GPIO controller
2. RTSP server starts in a separate thread for camera streaming
3. WebSocket server starts and accepts connections
4. First client to send `AcquireControl` gets exclusive control
5. Other clients can still read state via `GetState`
6. All clients can view RTSP stream simultaneously
7. When controlling client disconnects, control is automatically released

## State Tracking

The turret maintains state including:

- **X-axis angle**: Current angle in degrees
- **Y-axis angle**: Current angle in degrees  
- **X-axis steps**: Total steps from origin
- **Y-axis steps**: Total steps from origin
- **Actuator active**: Boolean indicating if actuator is currently triggered
- **X limit triggered**: Boolean indicating if X-axis limit switch is triggered
- **Y start limit triggered**: Boolean indicating if Y-axis start limit is triggered
- **Y stop limit triggered**: Boolean indicating if Y-axis stop limit is triggered
- **X homed**: Boolean indicating if X-axis has been homed
- **Y homed**: Boolean indicating if Y-axis has been homed

State is updated in real-time as commands are executed and can be queried by any connected client.

## Limit Switches

### X-Axis Limit Switch
The X-axis has a single limit switch used for homing/reference position since it can rotate continuously. Use the `HomeX` command to find this reference position and reset the angle to 0.

### Y-Axis Limit Switches
The Y-axis has two limit switches:
- **Start Limit**: Defines the minimum position (position 0)
- **Stop Limit**: Defines the maximum position

The Y-axis movement will automatically stop if it encounters a limit switch, preventing mechanical damage. Use the `HomeY` command to move to the start position and reset the angle to 0.

## Troubleshooting

### GPIO Access Denied
Run with `sudo` or add your user to the `gpio` group:
```bash
sudo usermod -a -G gpio $USER
```

### Camera Not Found
Check available video devices:
```bash
ls -l /dev/video*
v4l2-ctl --list-devices
```

### RTSP Stream Not Accessible
Check that port 8554 is not blocked by firewall:
```bash
sudo ufw allow 8554/tcp
```

### GStreamer Errors
Install missing GStreamer plugins:
```bash
sudo apt-get install gstreamer1.0-plugins-*
```

## License

MIT License - see LICENSE file for details
