//! Ubuntu autostart via XDG .desktop file at ~/.config/autostart/manuel.desktop.

use std::{fs, path::PathBuf};
use anyhow::{Context, Result};

fn desktop_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("autostart")
        .join("manuel.desktop")
}

fn exec_path() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(String::from))
        .unwrap_or_else(|| "manuel".to_string())
}

pub fn is_enabled() -> bool {
    desktop_path().exists()
}

pub fn enable() -> Result<()> {
    let path = desktop_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("creating autostart dir")?;
    }
    let content = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Manuel\n\
         Comment=Input mapper\n\
         Exec={exec}\n\
         Icon=manuel\n\
         Terminal=false\n\
         Categories=Utility;\n\
         X-GNOME-Autostart-enabled=true\n",
        exec = exec_path(),
    );
    fs::write(&path, content).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

pub fn disable() -> Result<()> {
    let p = desktop_path();
    if p.exists() {
        fs::remove_file(&p).with_context(|| format!("removing {}", p.display()))?;
    }
    Ok(())
}
