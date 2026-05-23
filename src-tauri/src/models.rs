use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub mappings: Vec<Mapping>,
    #[serde(default)]
    pub settings: Settings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub start_on_boot: bool,
    #[serde(default)]
    pub start_minimized: bool,
    #[serde(default = "default_true")]
    pub run_in_tray: bool,
    #[serde(default = "default_true")]
    pub enable_notifications: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            start_on_boot: false,
            start_minimized: false,
            run_in_tray: true,
            enable_notifications: true,
        }
    }
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapping {
    pub id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub description: String,
    pub input: InputTrigger,
    pub output: Output,
}

/// What kind of input fires the mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum InputTrigger {
    /// A keyboard combination like "Ctrl+Alt+P" or a single key like "F13".
    KeyCombo(String),
    /// An evdev button on a specific device, e.g. {"device":"/dev/input/event8","code":288}.
    HidButton(HidButtonRef),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HidButtonRef {
    /// Last-known evdev path. NOT stable across reboots/replugs — kept only as
    /// a hint/legacy field; matching prefers vendor_id+product_id.
    #[serde(default)]
    pub device: String,
    pub code: u16,
    #[serde(default)]
    pub device_name: String,
    #[serde(default)]
    pub vendor_id: Option<u16>,
    #[serde(default)]
    pub product_id: Option<u16>,
}

/// What action to perform when triggered.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum Output {
    Text(String),
    Key(String),
    Combo(String),
    /// Macro is a JSON-encoded array of MacroStep stored as a String for easy UI editing,
    /// or a real Vec when programmatically constructed.
    Macro(serde_json::Value),
    Shell(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum MacroStep {
    Type { text: String },
    Press { key: String },
    Combo { combo: String },
    Delay { ms: u64 },
    Shell { cmd: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub path: String,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub connected: bool,
}
