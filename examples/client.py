#!/usr/bin/env python3
"""
Example WebSocket client for controlling the cam-control-2 turret system.

This script demonstrates how to:
1. Connect to the WebSocket server
2. Acquire control of the turret
3. Home the axes using limit switches
4. Move the turret
5. Read the turret state
6. Trigger the actuator
7. Release control

Requirements:
    pip install websockets

Usage:
    python3 examples/client.py <turret-ip-address>
"""

import asyncio
import websockets
import json
import sys

async def control_turret(host):
    """Main control function for the turret."""
    uri = f"ws://{host}:8080"
    
    try:
        async with websockets.connect(uri) as websocket:
            print(f"Connected to turret at {uri}")
            
            # Receive initial status message
            initial_msg = await websocket.recv()
            print(f"Initial status: {initial_msg}\n")
            
            # Step 1: Acquire control
            print("Step 1: Acquiring control...")
            await websocket.send(json.dumps({"command": "AcquireControl"}))
            response = json.loads(await websocket.recv())
            print(f"Response: {response}")
            
            if response["status"] != "success":
                print("Failed to acquire control. Exiting.")
                return
            
            print()
            
            # Step 2: Get current state
            print("Step 2: Getting current state...")
            await websocket.send(json.dumps({"command": "GetState"}))
            response = json.loads(await websocket.recv())
            print(f"Current state: {json.dumps(response['state'], indent=2)}")
            print()
            
            # Step 3: Home X-axis
            print("Step 3: Homing X-axis...")
            await websocket.send(json.dumps({"command": "HomeX"}))
            response = json.loads(await websocket.recv())
            print(f"Response: {response}")
            print()
            
            # Step 4: Home Y-axis
            print("Step 4: Homing Y-axis...")
            await websocket.send(json.dumps({"command": "HomeY"}))
            response = json.loads(await websocket.recv())
            print(f"Response: {response}")
            print()
            
            # Step 5: Move X-axis
            print("Step 5: Moving X-axis 100 steps forward...")
            await websocket.send(json.dumps({
                "command": "MoveX",
                "direction": "forward",
                "steps": 100
            }))
            response = json.loads(await websocket.recv())
            print(f"Response: {response}")
            print()
            
            # Step 6: Move Y-axis
            print("Step 6: Moving Y-axis 50 steps forward...")
            await websocket.send(json.dumps({
                "command": "MoveY",
                "direction": "forward",
                "steps": 50
            }))
            response = json.loads(await websocket.recv())
            print(f"Response: {response}")
            print()
            
            # Step 7: Get updated state
            print("Step 7: Getting updated state...")
            await websocket.send(json.dumps({"command": "GetState"}))
            response = json.loads(await websocket.recv())
            print(f"Updated state: {json.dumps(response['state'], indent=2)}")
            print()
            
            # Step 8: Trigger actuator
            print("Step 8: Triggering actuator for 500ms...")
            await websocket.send(json.dumps({
                "command": "TriggerActuator",
                "duration_ms": 500
            }))
            response = json.loads(await websocket.recv())
            print(f"Response: {response}")
            print()
            
            # Wait a moment
            await asyncio.sleep(1)
            
            # Step 9: Move back to home
            print("Step 9: Returning to home position...")
            await websocket.send(json.dumps({"command": "HomeY"}))
            response = json.loads(await websocket.recv())
            print(f"Y-axis: {response}")
            
            await websocket.send(json.dumps({"command": "HomeX"}))
            response = json.loads(await websocket.recv())
            print(f"X-axis: {response}")
            print()
            
            # Step 10: Release control
            print("Step 10: Releasing control...")
            await websocket.send(json.dumps({"command": "ReleaseControl"}))
            response = json.loads(await websocket.recv())
            print(f"Response: {response}")
            print()
            
            print("Demo completed successfully!")
            
    except websockets.exceptions.WebSocketException as e:
        print(f"WebSocket error: {e}")
    except Exception as e:
        print(f"Error: {e}")

async def monitor_state(host, interval=1.0):
    """Monitor turret state without acquiring control."""
    uri = f"ws://{host}:8080"
    
    try:
        async with websockets.connect(uri) as websocket:
            print(f"Monitoring turret state at {uri} (Ctrl+C to exit)\n")
            
            # Receive initial status
            initial_msg = await websocket.recv()
            print(f"{initial_msg}\n")
            
            while True:
                await websocket.send(json.dumps({"command": "GetState"}))
                response = json.loads(await websocket.recv())
                
                if response["status"] == "success":
                    state = response["state"]
                    print(f"\r[{state['x_angle']:.1f}°, {state['y_angle']:.1f}°] "
                          f"X-homed:{state['x_homed']} Y-homed:{state['y_homed']} "
                          f"Actuator:{state['actuator_active']} "
                          f"Limits: X:{state['x_limit_triggered']} "
                          f"Y-start:{state['y_start_limit_triggered']} "
                          f"Y-stop:{state['y_stop_limit_triggered']}", 
                          end='', flush=True)
                
                await asyncio.sleep(interval)
                
    except KeyboardInterrupt:
        print("\n\nMonitoring stopped.")
    except Exception as e:
        print(f"\nError: {e}")

def main():
    """Main entry point."""
    if len(sys.argv) < 2:
        print("Usage:")
        print(f"  Control mode:  {sys.argv[0]} <turret-ip-address>")
        print(f"  Monitor mode:  {sys.argv[0]} <turret-ip-address> monitor")
        sys.exit(1)
    
    host = sys.argv[1]
    mode = sys.argv[2] if len(sys.argv) > 2 else "control"
    
    if mode == "monitor":
        asyncio.run(monitor_state(host))
    else:
        asyncio.run(control_turret(host))

if __name__ == "__main__":
    main()
