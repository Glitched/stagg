use anyhow::Result;
use clap::{Parser, Subcommand};

use stagg::{Kettle, Preset, Units};

#[derive(Parser)]
#[command(
    name = "stagg",
    about = "Control your Fellow Stagg EKG Pro kettle from the command line"
)]
struct Cli {
    /// Kettle IP address
    #[arg(long, env = "STAGG_HOST")]
    host: String,

    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Turn the kettle on (heat to target temperature)
    On,
    /// Turn the kettle off
    Off,
    /// Get current kettle state
    State,
    /// Get temperature stats
    Temp,
    /// Set target temperature and start heating
    Set {
        /// Target temperature (in current units)
        temp: u32,
    },
    /// Heat to a preset temperature
    Brew {
        /// Preset name, or "list" to show all presets
        preset: String,
        /// Hold time in minutes
        #[arg(long)]
        hold: Option<u32>,
    },
    /// Set units to Fahrenheit
    Fahrenheit,
    /// Set units to Celsius
    Celsius,
    /// Set hold time in minutes
    Hold {
        /// Minutes (0 to disable)
        minutes: u32,
    },
    /// Buzz the kettle
    Buz {
        /// Frequency in Hz, or "sos"
        #[arg(default_value = "sos")]
        pattern: String,
        /// Duty cycle (0-8191)
        #[arg(short, long)]
        duty: Option<u32>,
        /// Duration in ms
        #[arg(short = 't', long)]
        duration: Option<u32>,
    },
    /// Get firmware info
    Firmware,
    /// Print all settings
    Settings,
    /// Send a raw CLI command to the kettle
    Raw {
        /// The command to send
        cmd: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let kettle = Kettle::new(&cli.host)?;

    match cli.command {
        Command::On => {
            kettle.heat_on()?;
            println!("Kettle on");
        }
        Command::Off => {
            kettle.heat_off()?;
            println!("Kettle off");
        }
        Command::State => {
            let state = kettle.state()?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&state)?);
            } else {
                let unit_label = match state.units {
                    Units::Fahrenheit => "F",
                    Units::Celsius => "C",
                };
                println!("Mode:    {}", state.mode);
                println!("Current: {:.1}°C", state.current_temp_c);
                println!("Target:  {}°{unit_label}", state.target_temp_f);
                println!(
                    "BLE:     {}",
                    if state.ble_connected {
                        "connected"
                    } else {
                        "disconnected"
                    }
                );
            }
        }
        Command::Temp => {
            println!("{}", kettle.raw_cmd("temp")?);
        }
        Command::Set { temp } => {
            kettle.set_temp(temp)?;
            println!("Set to {temp}° and heating");
        }
        Command::Brew { preset, hold } => {
            if preset == "list" {
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(Preset::all())?);
                } else {
                    for p in Preset::all() {
                        println!("{:<10} {:>3}°F  {}", p.name, p.temp_f, p.label);
                    }
                }
                return Ok(());
            }

            let p = Preset::by_name(&preset)?;
            kettle.brew(p, hold)?;
            match hold {
                Some(m) => println!("Brewing {} at {}°F (hold {m} min)", p.label, p.temp_f),
                None => println!("Brewing {} at {}°F", p.label, p.temp_f),
            }
        }
        Command::Fahrenheit => {
            kettle.set_units(Units::Fahrenheit)?;
            println!("Units set to °F");
        }
        Command::Celsius => {
            kettle.set_units(Units::Celsius)?;
            println!("Units set to °C");
        }
        Command::Hold { minutes } => {
            kettle.set_hold(minutes)?;
            if minutes == 0 {
                println!("Hold disabled");
            } else {
                println!("Hold set to {minutes} min");
            }
        }
        Command::Buz {
            pattern,
            duty,
            duration,
        } => {
            if pattern == "sos" {
                kettle.buzz_sos()?;
            } else {
                let freq: u32 = pattern
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Frequency must be a number or 'sos'"))?;
                kettle.buzz(freq, duty.unwrap_or(4096), duration.unwrap_or(500))?;
            }
            println!("Buzzed");
        }
        Command::Firmware => {
            let fw = kettle.firmware()?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&fw)?);
            } else {
                println!("Version:   {}", fw.version);
                println!("Boot:      {}", fw.boot_partition);
                println!("Running:   {}", fw.running_partition);
            }
        }
        Command::Settings => {
            let settings = kettle.settings()?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&settings)?);
            } else {
                let unit_label = match settings.units {
                    Units::Fahrenheit => "F",
                    Units::Celsius => "C",
                };
                println!("Target:    {}°{unit_label}", settings.target_temp_f);
                println!("Hold:      {} min", settings.hold_minutes);
                println!("Chime:     {}", settings.chime);
                println!("Clock:     {}", settings.clock_mode);
                println!("Altitude:  {} ft", settings.altitude_ft);
            }
        }
        Command::Raw { cmd } => {
            println!("{}", kettle.raw_cmd(&cmd.join(" "))?);
        }
    }

    Ok(())
}
