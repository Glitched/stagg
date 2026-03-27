use std::collections::HashMap;
use std::time::Duration;

use reqwest::blocking::Client;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to reach kettle at {host} - is it on and connected to WiFi?")]
    Unreachable { host: String },
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Unknown brew preset: {preset}. Use Preset::all() for available presets")]
    UnknownPreset { preset: String },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Serialize)]
pub struct Preset {
    pub name: &'static str,
    pub label: &'static str,
    pub temp_f: u32,
}

const PRESETS: &[Preset] = &[
    Preset {
        name: "coffee",
        label: "pour over coffee",
        temp_f: 205,
    },
    Preset {
        name: "pourover",
        label: "pour over coffee",
        temp_f: 205,
    },
    Preset {
        name: "french",
        label: "French press",
        temp_f: 200,
    },
    Preset {
        name: "black",
        label: "black tea",
        temp_f: 212,
    },
    Preset {
        name: "green",
        label: "green tea",
        temp_f: 175,
    },
    Preset {
        name: "oolong",
        label: "oolong tea",
        temp_f: 195,
    },
    Preset {
        name: "white",
        label: "white tea",
        temp_f: 185,
    },
    Preset {
        name: "herbal",
        label: "herbal tea",
        temp_f: 212,
    },
    Preset {
        name: "matcha",
        label: "matcha",
        temp_f: 175,
    },
    Preset {
        name: "boil",
        label: "boil",
        temp_f: 212,
    },
];

impl Preset {
    pub fn all() -> &'static [Preset] {
        PRESETS
    }

    pub fn by_name(name: &str) -> Result<&'static Preset> {
        PRESETS
            .iter()
            .find(|p| p.name == name.to_lowercase())
            .ok_or_else(|| Error::UnknownPreset {
                preset: name.to_string(),
            })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct State {
    pub mode: String,
    pub current_temp_c: f64,
    pub target_temp_f: u32,
    pub units: Units,
    pub ble_connected: bool,
    pub raw: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Units {
    Fahrenheit,
    Celsius,
}

#[derive(Debug, Clone, Serialize)]
pub struct Settings {
    pub target_temp_f: u32,
    pub hold_minutes: u32,
    pub chime: u32,
    pub units: Units,
    pub clock_mode: String,
    pub altitude_ft: u32,
    pub raw: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FirmwareInfo {
    pub version: String,
    pub boot_partition: String,
    pub running_partition: String,
    pub raw: String,
}

pub struct Kettle {
    client: Client,
    host: String,
    base_url: String,
}

impl Kettle {
    pub fn new(host: &str) -> Result<Self> {
        let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
        Ok(Self {
            client,
            host: host.to_string(),
            base_url: format!("http://{host}/cli"),
        })
    }

    pub fn raw_cmd(&self, command: &str) -> Result<String> {
        let resp = self
            .client
            .get(&self.base_url)
            .query(&[("cmd", command)])
            .send()
            .map_err(|_| Error::Unreachable {
                host: self.host.clone(),
            })?
            .text()?;

        let output = resp
            .split("</form>")
            .nth(1)
            .unwrap_or(&resp)
            .trim()
            .to_string();

        Ok(output)
    }

    pub fn state(&self) -> Result<State> {
        let raw_output = self.raw_cmd("state")?;
        let mut raw = HashMap::new();

        for line in raw_output.lines() {
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                if !key.starts_with("I (") {
                    raw.insert(key.to_string(), value.to_string());
                }
            }
        }

        let mode = raw.get("mode").cloned().unwrap_or_default();
        let current_temp_c = raw
            .get("tempr")
            .and_then(|v| v.trim_end_matches(" C").parse::<f64>().ok())
            .unwrap_or(0.0);
        let target_temp_f = raw
            .get("temps")
            .and_then(|v| v.trim_end_matches(" F").parse::<u32>().ok())
            .unwrap_or(0);
        let units = match raw.get("units").map(|v| v.as_str()) {
            Some("0") => Units::Fahrenheit,
            _ => Units::Celsius,
        };
        let ble_connected = raw.get("ble conn").map(|v| v == "1").unwrap_or(false);

        Ok(State {
            mode,
            current_temp_c,
            target_temp_f,
            units,
            ble_connected,
            raw,
        })
    }

    pub fn settings(&self) -> Result<Settings> {
        let raw_output = self.raw_cmd("prtsettings")?;
        let mut raw = HashMap::new();

        for line in raw_output.lines() {
            if let Some(rest) = line.strip_prefix("st: ")
                && let Some((key, value)) = rest.split_once('=')
            {
                raw.insert(key.trim().to_string(), value.trim().to_string());
            }
        }

        let parse_u32 = |key: &str| -> u32 {
            raw.get(key)
                .and_then(|v| v.split_whitespace().next())
                .and_then(|v| v.parse().ok())
                .unwrap_or(0)
        };

        Ok(Settings {
            target_temp_f: parse_u32("settempr"),
            hold_minutes: parse_u32("hold"),
            chime: parse_u32("chime"),
            units: match raw.get("units").map(|v| v.as_str()) {
                Some("0") => Units::Fahrenheit,
                _ => Units::Celsius,
            },
            clock_mode: match raw.get("clockmode").map(|v| v.as_str()) {
                Some("0") => "off".to_string(),
                Some("1") => "digital".to_string(),
                Some("2") => "analog".to_string(),
                other => other.unwrap_or("unknown").to_string(),
            },
            altitude_ft: parse_u32("altitude"),
            raw,
        })
    }

    pub fn firmware(&self) -> Result<FirmwareInfo> {
        let raw_output = self.raw_cmd("fwinfo")?;
        let mut version = String::new();
        let mut boot_partition = String::new();
        let mut running_partition = String::new();

        for line in raw_output.lines() {
            if let Some(v) = line.strip_prefix("I (")
                && let Some(rest) = v.split(") OTA: ").nth(1)
            {
                if let Some(ver) = rest.strip_prefix("Current version: ") {
                    version = ver.trim().to_string();
                } else if let Some(bp) = rest.strip_prefix("Boot partition: ") {
                    boot_partition = bp.trim().to_string();
                } else if let Some(rp) = rest.strip_prefix("Running partition: ") {
                    running_partition = rp.trim().to_string();
                }
            }
        }

        Ok(FirmwareInfo {
            version,
            boot_partition,
            running_partition,
            raw: raw_output,
        })
    }

    pub fn heat_on(&self) -> Result<String> {
        self.raw_cmd("setstate S_Heat")
    }

    pub fn heat_off(&self) -> Result<String> {
        self.raw_cmd("setstate S_Off")
    }

    pub fn set_temp(&self, temp_f: u32) -> Result<()> {
        self.heat_on()?;
        self.raw_cmd(&format!("setsetting settempr {temp_f}"))?;
        self.raw_cmd("refresh")?;
        Ok(())
    }

    pub fn brew(&self, preset: &Preset, hold_minutes: Option<u32>) -> Result<()> {
        self.heat_on()?;
        self.raw_cmd(&format!("setsetting settempr {}", preset.temp_f))?;
        if let Some(hold) = hold_minutes {
            self.raw_cmd(&format!("setsetting hold {hold}"))?;
        }
        self.raw_cmd("refresh")?;
        Ok(())
    }

    pub fn set_units(&self, units: Units) -> Result<String> {
        match units {
            Units::Fahrenheit => self.raw_cmd("setunitsf"),
            Units::Celsius => self.raw_cmd("setunitsc"),
        }
    }

    pub fn set_hold(&self, minutes: u32) -> Result<String> {
        self.raw_cmd(&format!("setsetting hold {minutes}"))
    }

    pub fn buzz(&self, freq_hz: u32, duty: u32, duration_ms: u32) -> Result<String> {
        self.raw_cmd(&format!("buz {freq_hz} {duty} {duration_ms}"))
    }

    pub fn buzz_sos(&self) -> Result<String> {
        self.raw_cmd("buz sos")
    }
}
