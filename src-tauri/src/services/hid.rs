//! Linux evdev device enumeration and reading.
//!
//! We treat anything under /dev/input/event* as a candidate. We classify a device
//! as a "keyboard-like" source if it advertises EV_KEY events. We expose both
//! vendor/product IDs (when available) and a stable path (/dev/input/eventN).

use std::path::PathBuf;
use evdev::Device;
use crate::models::DeviceInfo;

pub fn enumerate() -> Vec<DeviceInfo> {
    let mut out = Vec::new();
    let dir = match std::fs::read_dir("/dev/input") {
        Ok(d) => d,
        Err(_) => return out,
    };
    for entry in dir.flatten() {
        let path = entry.path();
        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if !name.starts_with("event") {
            continue;
        }
        match Device::open(&path) {
            Ok(dev) => {
                // Only show devices that produce key events
                let has_keys = dev.supported_keys().map(|k| k.iter().count() > 0).unwrap_or(false);
                if !has_keys { continue; }
                let id = dev.input_id();
                out.push(DeviceInfo {
                    name: dev.name().unwrap_or("(unknown)").to_string(),
                    path: path.to_string_lossy().into_owned(),
                    vendor_id: Some(id.vendor()),
                    product_id: Some(id.product()),
                    connected: true,
                });
            }
            Err(_) => {
                // Likely permission denied — surface as disconnected so user sees it
                out.push(DeviceInfo {
                    name: name.clone(),
                    path: path.to_string_lossy().into_owned(),
                    vendor_id: None,
                    product_id: None,
                    connected: false,
                });
            }
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

pub fn open(path: &str) -> anyhow::Result<Device> {
    Ok(Device::open(PathBuf::from(path))?)
}
