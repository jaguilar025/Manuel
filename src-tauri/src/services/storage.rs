use std::{fs, path::PathBuf};
use anyhow::{Context, Result};
use crate::models::Config;

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("manuel")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn load_or_default() -> Config {
    let mut cfg = load().unwrap_or_else(|e| {
        log::warn!("could not load config ({e}); using defaults");
        Config::default()
    });
    if migrate_in_place(&mut cfg) {
        if let Err(e) = save(&cfg) {
            log::warn!("could not persist migrated config: {e}");
        } else {
            log::info!("config migrated: backfilled vendor/product IDs for HID mappings");
        }
    }
    cfg
}

/// Backfill `vendor_id`/`product_id` on `HidButton` mappings that only have a
/// device path. We look up the current device at that path and copy the IDs.
/// Returns true if any mapping was modified.
fn migrate_in_place(cfg: &mut Config) -> bool {
    use crate::models::InputTrigger;
    let mut dirty = false;
    for m in &mut cfg.mappings {
        if let InputTrigger::HidButton(ref mut b) = m.input {
            let needs = b.vendor_id.is_none() || b.product_id.is_none()
                || b.vendor_id == Some(0) || b.product_id == Some(0);
            if !needs || b.device.is_empty() { continue; }
            if let Ok(dev) = evdev::Device::open(&b.device) {
                let id = dev.input_id();
                b.vendor_id  = Some(id.vendor());
                b.product_id = Some(id.product());
                if b.device_name.is_empty() {
                    b.device_name = dev.name().unwrap_or("").to_string();
                }
                dirty = true;
            }
        }
    }
    dirty
}

pub fn load() -> Result<Config> {
    let p = config_path();
    if !p.exists() {
        return Ok(Config::default());
    }
    let raw = fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?;
    let cfg: Config = serde_json::from_str(&raw).context("parsing config.json")?;
    Ok(cfg)
}

pub fn save(cfg: &Config) -> Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let p = config_path();
    let tmp = p.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(cfg)?;
    fs::write(&tmp, json)?;
    fs::rename(&tmp, &p)?;
    Ok(())
}
