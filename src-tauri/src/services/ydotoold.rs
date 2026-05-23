//! Spawns `ydotoold` as a child of Manuel, and shuts it down when Manuel exits.
//!
//! Requires: /dev/uinput writable by current user (udev rule + `input` group).
//! If permission is missing, the spawn fails and we log a clear message; the
//! app keeps working except for Text/Key/Combo outputs.

use std::{path::PathBuf, process::{Child, Command, Stdio}, sync::Mutex};
use once_cell::sync::Lazy;

static CHILD: Lazy<Mutex<Option<Child>>> = Lazy::new(|| Mutex::new(None));

fn socket_path() -> PathBuf {
    let dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/run/user/{}", unsafe { libc::getuid() }));
    PathBuf::from(dir).join(".ydotool_socket")
}

fn already_running() -> bool {
    Command::new("pgrep").arg("-x").arg("ydotoold")
        .status().map(|s| s.success()).unwrap_or(false)
}

pub fn start() {
    if already_running() {
        log::info!("ydotoold already running — not spawning a new one");
        // Make sure clients use the existing socket.
        std::env::set_var("YDOTOOL_SOCKET", socket_path());
        return;
    }

    let socket = socket_path();
    log::info!("spawning ydotoold (socket={})", socket.display());

    let child = Command::new("ydotoold")
        .arg(format!("--socket-path={}", socket.display()))
        .arg("--socket-perm=0600")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn();

    match child {
        Ok(c) => {
            std::env::set_var("YDOTOOL_SOCKET", &socket);
            *CHILD.lock().unwrap() = Some(c);
            // Give the daemon ~150 ms to create the socket before any client uses it.
            std::thread::sleep(std::time::Duration::from_millis(150));
            log::info!("ydotoold spawned");
        }
        Err(e) => {
            log::warn!(
                "could not spawn ydotoold ({e}). \
                 Text/Key/Combo outputs will fail until the daemon is running. \
                 Check that /dev/uinput is writable (udev rule + 'input' group)."
            );
        }
    }
}

pub fn stop() {
    if let Some(mut c) = CHILD.lock().unwrap().take() {
        log::info!("stopping ydotoold (pid={})", c.id());
        let _ = c.kill();
        let _ = c.wait();
    }
}
