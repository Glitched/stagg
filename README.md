# stagg

CLI and Rust library for controlling the [Fellow Stagg EKG Pro](https://fellowproducts.com/products/stagg-ekg-pro-studio-electric-pour-over-kettle) kettle over WiFi.

The EKG Pro runs an ESP32 with an unauthenticated HTTP CLI interface on port 80. This project wraps that interface into a proper CLI and reusable library.

## Setup

### Connecting your kettle to WiFi

1. Install the [Fellow EKG Updater](https://apps.apple.com) app (not the "Fellow" app)
2. Enable connectivity in your kettle's settings menu
3. Use the app to provision your WiFi credentials over BLE

### Finding your kettle's IP

```sh
nmap -p 80 --open 192.168.0.0/24
```

Or check your router's DHCP client list for an Espressif device. Once found, set up a DHCP reservation so the IP stays fixed.

### Verify connectivity

```sh
curl "http://<KETTLE_IP>/cli?cmd=state"
```

## Install

```sh
cargo install --git https://github.com/Glitched/stagg
```

Set `STAGG_HOST` to avoid passing `--host` every time:

```sh
export STAGG_HOST=192.168.0.130
```

## Usage

```sh
# Power
stagg on                        # Heat to target temperature
stagg off                       # Turn off

# Temperature
stagg set 205                   # Set target temp and start heating
stagg state                     # Current state (mode, temps, BLE)
stagg temp                      # Temperature statistics

# Brew presets
stagg brew coffee               # Pour over (205°F)
stagg brew green                # Green tea (175°F)
stagg brew black                # Black tea (212°F)
stagg brew oolong               # Oolong (195°F)
stagg brew french               # French press (200°F)
stagg brew list                 # Show all presets
stagg brew coffee --hold 30     # Brew with 30 min hold

# Settings
stagg hold 30                   # Set hold time (minutes)
stagg fahrenheit                # Switch to °F
stagg celsius                   # Switch to °C
stagg settings                  # Print all settings
stagg firmware                  # Firmware info

# Fun
stagg buz                       # SOS buzzer
stagg buz 1000 -d 4096 -t 300  # Custom frequency

# Raw access
stagg raw help                  # Send any command to the kettle CLI

# JSON output (for scripting)
stagg state --json
stagg settings --json
stagg brew list --json
```

## As a library

```rust
use stagg::{Kettle, Preset};

let kettle = Kettle::new("192.168.0.130")?;

// Get state
let state = kettle.state()?;
println!("Current temp: {:.1}°C", state.current_temp_c);

// Brew a preset
let preset = Preset::by_name("coffee")?;
kettle.brew(preset, Some(30))?;

// Direct control
kettle.set_temp(200)?;
kettle.heat_off()?;
kettle.buzz_sos()?;
```

## How it works

The Fellow Stagg EKG Pro runs an ESP32-WROVER-E that exposes an HTTP endpoint at `http://<IP>/cli?cmd=<command>`. This was likely a development/debug interface that shipped in production firmware.

Key discoveries:
- `setstate S_Heat` / `setstate S_Off` control the kettle through its proper state machine (safer than raw GPIO `heaton`/`heatoff`)
- `setsetting settempr <value>` sets the target temperature, followed by `refresh` to update the display
- The kettle handles heating logic internally — setting a target below the current water temp just holds without firing the element

## Prior art

- [tomtastic/stagg-ekg-pro](https://github.com/tomtastic/stagg-ekg-pro) — Python WiFi control
- [tlyakhov/fellow-stagg-ekg-plus](https://github.com/tlyakhov/fellow-stagg-ekg-plus) — BLE reverse engineering (EKG+)
- [fabiankirchen/staggassistant](https://github.com/fabiankirchen/staggassistant) — Home Assistant integration

## License

MIT
