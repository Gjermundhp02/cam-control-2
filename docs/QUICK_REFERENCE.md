# Quick Reference

## Starting the Server

```bash
sudo ./target/release/cam-control-2
```

## Server Endpoints

- **WebSocket**: `ws://<device-ip>:8080`
- **RTSP Stream**: `rtsp://<device-ip>:8554/stream`

## Common Commands

### Control Commands (Require Control)

```json
// Acquire control
{"command": "AcquireControl"}

// Home axes
{"command": "HomeX"}
{"command": "HomeY"}

// Move turret
{"command": "MoveX", "direction": "forward", "steps": 100}
{"command": "MoveY", "direction": "backward", "steps": 50}

// Trigger actuator
{"command": "TriggerActuator", "duration_ms": 1000}

// Release control
{"command": "ReleaseControl"}
```

### Read-Only Commands (No Control Required)

```json
// Get turret state
{"command": "GetState"}
```

## Direction Values

- `"forward"` or `"clockwise"` or `"cw"`
- `"backward"` or `"counterclockwise"` or `"ccw"`

## GPIO Pin Quick Reference

### Motors
- X-axis: STEP=GPIO17, DIR=GPIO27
- Y-axis: STEP=GPIO22, DIR=GPIO23
- Actuator: GPIO24

### Limit Switches
- X-axis Home: GPIO5
- Y-axis Start: GPIO6
- Y-axis Stop: GPIO13

## Example Usage

### Python
```bash
# Control mode
python3 examples/client.py 192.168.1.100

# Monitor mode
python3 examples/client.py 192.168.1.100 monitor
```

### View RTSP Stream
```bash
# Using VLC
vlc rtsp://192.168.1.100:8554/stream

# Using ffplay
ffplay rtsp://192.168.1.100:8554/stream

# Using gst-launch
gst-launch-1.0 playbin uri=rtsp://192.168.1.100:8554/stream
```

## State Response Example

```json
{
  "status": "success",
  "state": {
    "x_angle": 45.0,
    "y_angle": 30.0,
    "x_steps": 250,
    "y_steps": 166,
    "actuator_active": false,
    "x_limit_triggered": false,
    "y_start_limit_triggered": false,
    "y_stop_limit_triggered": false,
    "x_homed": true,
    "y_homed": true
  }
}
```

## Troubleshooting Quick Fixes

### Can't build - missing GStreamer libraries
```bash
sudo apt-get install libgstreamer1.0-dev libgstreamer-rtsp-server-1.0-dev
```

### Can't access GPIO
```bash
# Run with sudo or add user to gpio group
sudo usermod -a -G gpio $USER
# Then logout and login again
```

### Motors not moving
1. Check power supply to motor drivers
2. Verify common GND between Pi and drivers
3. Test with small step counts first

### Limit switches not working
1. Check wiring (should read HIGH when open)
2. Verify GPIO pin numbers
3. Test with multimeter for continuity
