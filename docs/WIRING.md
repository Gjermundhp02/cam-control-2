# Hardware Wiring Guide

This document describes how to wire the hardware components to the Raspberry Pi GPIO pins.

## GPIO Pin Mapping

### Raspberry Pi GPIO Pinout Reference

```
    3V3  (1) (2)  5V
  GPIO2  (3) (4)  5V
  GPIO3  (5) (6)  GND
  GPIO4  (7) (8)  GPIO14
    GND  (9) (10) GPIO15
 GPIO17 (11) (12) GPIO18
 GPIO27 (13) (14) GND
 GPIO22 (15) (16) GPIO23
    3V3 (17) (18) GPIO24
 GPIO10 (19) (20) GND
  GPIO9 (21) (22) GPIO25
 GPIO11 (23) (24) GPIO8
    GND (25) (26) GPIO7
  GPIO0 (27) (28) GPIO1
  GPIO5 (29) (30) GND
  GPIO6 (31) (32) GPIO12
 GPIO13 (33) (34) GND
 GPIO19 (35) (36) GPIO16
 GPIO26 (37) (38) GPIO20
    GND (39) (40) GPIO21
```

## Component Connections

### X-Axis Stepper Motor Driver

| Component Pin | Raspberry Pi Pin | GPIO Number | Description |
|--------------|------------------|-------------|-------------|
| STEP         | Pin 11           | GPIO 17     | Step pulse signal |
| DIR          | Pin 13           | GPIO 27     | Direction signal |
| GND          | Pin 9 or 14      | GND         | Ground |
| VCC          | External PSU     | -           | Motor power (12V/24V) |

### Y-Axis Stepper Motor Driver

| Component Pin | Raspberry Pi Pin | GPIO Number | Description |
|--------------|------------------|-------------|-------------|
| STEP         | Pin 15           | GPIO 22     | Step pulse signal |
| DIR          | Pin 16           | GPIO 23     | Direction signal |
| GND          | Pin 20 or 25     | GND         | Ground |
| VCC          | External PSU     | -           | Motor power (12V/24V) |

### Actuator (Relay or Direct Control)

| Component Pin | Raspberry Pi Pin | GPIO Number | Description |
|--------------|------------------|-------------|-------------|
| Signal       | Pin 18           | GPIO 24     | Actuator control signal |
| GND          | Pin 34           | GND         | Ground |
| VCC          | External PSU     | -           | Actuator power (if needed) |

**Note:** If using a relay, the GPIO pin should connect to the relay control input. If driving the actuator directly, ensure it's compatible with 3.3V logic or use a level shifter/transistor.

### Limit Switches

All limit switches are wired as normally-open (NO) switches with internal pull-up resistors enabled in software.

#### X-Axis Home Limit Switch

| Switch Terminal | Connection       | GPIO Number | Description |
|----------------|------------------|-------------|-------------|
| NO (Common)    | Pin 29           | GPIO 5      | Switch input |
| NC             | Pin 30 (GND)     | GND         | Ground when closed |

#### Y-Axis Start Limit Switch

| Switch Terminal | Connection       | GPIO Number | Description |
|----------------|------------------|-------------|-------------|
| NO (Common)    | Pin 31           | GPIO 6      | Switch input |
| NC             | Pin 34 (GND)     | GND         | Ground when closed |

#### Y-Axis Stop Limit Switch

| Switch Terminal | Connection       | GPIO Number | Description |
|----------------|------------------|-------------|-------------|
| NO (Common)    | Pin 33           | GPIO 13     | Switch input |
| NC             | Pin 39 (GND)     | GND         | Ground when closed |

## Wiring Notes

### Important Safety Considerations

1. **Never connect motor power to Raspberry Pi pins** - Use an external power supply for stepper motors
2. **Common ground** - Ensure the Raspberry Pi GND is connected to the motor driver GND
3. **Logic level** - Most stepper motor drivers accept 3.3V logic from Raspberry Pi
4. **Current limits** - GPIO pins can source/sink max 16mA. Use drivers for motors and relays for high-current loads
5. **ESD protection** - Use anti-static precautions when handling the Raspberry Pi

### Stepper Motor Drivers

Common stepper motor drivers (A4988, DRV8825, TMC2208, etc.) typically require:
- **STEP**: Pulse signal for each step
- **DIR**: Direction (HIGH/LOW)
- **EN**: Enable (optional, can tie to GND to always enable)
- **MS1, MS2, MS3**: Microstepping configuration (optional)
- **VMOT**: Motor power supply (12V-24V typical)
- **GND**: Ground (common with Raspberry Pi GND)

### Limit Switch Wiring

Limit switches are configured with internal pull-up resistors and are active-LOW:
- **Switch open**: Pin reads HIGH (3.3V from pull-up)
- **Switch closed**: Pin reads LOW (connected to GND)

This configuration is safer as a wire break will be detected as "not triggered" rather than potentially causing unwanted movement.

## Example Wiring Diagram (Text-based)

```
Raspberry Pi                 Stepper Drivers                 Motors
┌─────────┐                 ┌──────────┐                   ┌────────┐
│ GPIO 17 ├────────────────►│ X-STEP   │                   │        │
│ GPIO 27 ├────────────────►│ X-DIR    │                   │ X-Axis │
│         │                 │ X-ENABLE │                   │ Stepper│
│ GPIO 22 ├────────────────►│ Y-STEP   │                   │        │
│ GPIO 23 ├────────────────►│ Y-DIR    │                   │ Y-Axis │
│         │                 │ Y-ENABLE │                   │ Stepper│
│         │                 │          ├──────────────────►│        │
│         │                 │   GND    │                   └────────┘
│   GND   ├────────┬────────┤          │
└─────────┘        │        └──────────┘
                   │
                   │        ┌──────────┐
                   │        │ Limit    │
                   │        │ Switches │
                   └────────┤ (3x)     │
                            │ Common   │
  GPIO 5, 6, 13 ◄───────────┤ Signal   │
                            └──────────┘

External PSU (12-24V)
┌─────────┐
│  +V     ├──────────► Stepper Driver VMOT
│  GND    ├──────────► Common GND (with Pi)
└─────────┘
```

## Testing Connections

Before running the full application, you can test individual components:

### Test Limit Switches
```bash
# Monitor GPIO input
gpio -g mode 5 in
gpio -g read 5    # Should be 1 when switch is open, 0 when closed
```

### Test Motor Drivers
Use the example client to:
1. Acquire control
2. Move each axis slowly with small step counts
3. Verify motors turn in correct directions
4. Test limit switches stop movement appropriately

## Troubleshooting

### Motors Not Moving
- Check power supply connections
- Verify GND is common between Pi and drivers
- Check driver enable pins are set correctly
- Measure voltage on STEP pin while commanding movement

### Limit Switches Not Working
- Check switch wiring (should read HIGH when open)
- Test continuity with multimeter
- Verify GPIO pin numbers match configuration
- Check for loose connections

### Wrong Direction
- Swap motor coil pairs on driver outputs, OR
- Flip the DIR pin logic by changing direction in commands

## Power Requirements

- **Raspberry Pi**: 5V @ 2.5A minimum (3A recommended)
- **Stepper Motors**: Typically 12V-24V @ 1-2A per motor (check motor specs)
- **Camera**: USB powered from Pi or separate 5V supply
- **Total**: Recommend separate power supplies for Pi and motors

Do not power motors from the Raspberry Pi GPIO pins!
