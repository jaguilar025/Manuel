//! Output emission. Strategy on Linux:
//!   - Wayland session → prefer `ydotool` (uinput-based, works in any compositor
//!     including GNOME Wayland, unlike wtype which needs virtual-keyboard protocol)
//!   - Fallback        → `wtype` (Sway/Hyprland)  → `xdotool` (X11)  → `enigo`
//!
//! Requires ydotoold running and /dev/uinput accessible by group `input`.

use std::process::Command;
use anyhow::{anyhow, Result};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Backend { Ydotool, Wtype, Xdotool, Enigo }

fn detect() -> Backend {
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    let on_wayland = session == "wayland" || std::env::var("WAYLAND_DISPLAY").is_ok();
    if on_wayland {
        if which("ydotool") { return Backend::Ydotool; }
        if which("wtype")   { return Backend::Wtype; }
    }
    if which("xdotool") { return Backend::Xdotool; }
    Backend::Enigo
}

fn which(bin: &str) -> bool {
    Command::new("sh").arg("-c").arg(format!("command -v {bin}"))
        .status().map(|s| s.success()).unwrap_or(false)
}

// ydotool wants YDOTOOL_SOCKET env or default path; we set it to
// $XDG_RUNTIME_DIR/.ydotool_socket which matches our ydotoold invocation.
fn ydotool_cmd() -> Command {
    let mut c = Command::new("ydotool");
    if std::env::var("YDOTOOL_SOCKET").is_err() {
        if let Ok(rt) = std::env::var("XDG_RUNTIME_DIR") {
            c.env("YDOTOOL_SOCKET", format!("{rt}/.ydotool_socket"));
        }
    }
    c
}

// ---------------- Public API ----------------

pub fn type_text(text: &str) -> Result<()> {
    match detect() {
        Backend::Ydotool => type_text_ydotool(text),
        Backend::Wtype   => run(Command::new("wtype").args(["--", text])),
        Backend::Xdotool => run(Command::new("xdotool").args(["type", "--delay", "0", "--", text])),
        Backend::Enigo   => enigo_text(text),
    }
}

/// Type text via ydotool using the active XKB layout to translate each character
/// into raw evdev keycodes (+ shift / altgr if needed). Characters that the
/// layout cannot produce in a single keystroke (e.g. an emoji on a US layout)
/// fall back to clipboard-paste for that character only.
fn type_text_ydotool(text: &str) -> Result<()> {
    use crate::services::textmap;
    textmap::ensure_built();

    let shift_code = evdev::Key::KEY_LEFTSHIFT.code();
    let altgr_code = evdev::Key::KEY_RIGHTALT.code();

    // Buffer adjacent xkb-mapped chars into one ydotool invocation per "run"
    // so the round-trip cost is paid once. When we hit an unmapped char we
    // flush the buffer, send it via xkb, then handle that char via clipboard.
    let mut buf: Vec<String> = vec!["key".into()];
    let mut buf_has_keys = false;

    let flush = |buf: &mut Vec<String>, has: &mut bool| -> Result<()> {
        if *has {
            let refs: Vec<&str> = buf.iter().map(String::as_str).collect();
            run(ydotool_cmd().args(&refs))?;
        }
        buf.clear();
        buf.push("key".into());
        *has = false;
        Ok(())
    };

    let append_kp = |buf: &mut Vec<String>, kp: textmap::KeyPress| {
        if kp.shift { buf.push(format!("{shift_code}:1")); }
        if kp.altgr { buf.push(format!("{altgr_code}:1")); }
        buf.push(format!("{}:1", kp.evdev_code));
        buf.push(format!("{}:0", kp.evdev_code));
        if kp.altgr { buf.push(format!("{altgr_code}:0")); }
        if kp.shift { buf.push(format!("{shift_code}:0")); }
    };

    for c in text.chars() {
        // 1) Direct: char is on the layout.
        if let Some(kp) = textmap::lookup(c) {
            append_kp(&mut buf, kp);
            buf_has_keys = true;
            continue;
        }
        // 2) Dead-key compose: e.g. "ñ" -> dead_tilde + n.
        if let Some(seq) = textmap::decompose(c) {
            for kp in seq { append_kp(&mut buf, kp); }
            buf_has_keys = true;
            continue;
        }
        // 3) Last resort: clipboard paste for this char only.
        flush(&mut buf, &mut buf_has_keys)?;
        if which("wl-copy") {
            paste_via_clipboard(&c.to_string())?;
        } else {
            log::warn!("char '{c}' (U+{:04X}) not reachable on this layout", c as u32);
        }
    }
    flush(&mut buf, &mut buf_has_keys)?;
    Ok(())
}

fn paste_via_clipboard(text: &str) -> Result<()> {
    use std::io::Write;
    // Save current clipboard so we can restore it.
    let backup = Command::new("wl-paste").arg("--no-newline").output().ok()
        .and_then(|o| if o.status.success() { Some(o.stdout) } else { None });

    // Push text into clipboard.
    let mut child = Command::new("wl-copy")
        .stdin(std::process::Stdio::piped())
        .spawn().map_err(|e| anyhow!("wl-copy spawn failed: {e}"))?;
    child.stdin.as_mut().unwrap().write_all(text.as_bytes())?;
    let st = child.wait()?;
    if !st.success() { return Err(anyhow!("wl-copy failed: {st}")); }

    // Small delay so clipboard managers settle before paste.
    std::thread::sleep(std::time::Duration::from_millis(40));

    // Send Ctrl+V via ydotool.
    let ctrl = evdev::Key::KEY_LEFTCTRL.code();
    let v    = evdev::Key::KEY_V.code();
    run(ydotool_cmd().args([
        "key",
        &format!("{ctrl}:1"),
        &format!("{v}:1"),
        &format!("{v}:0"),
        &format!("{ctrl}:0"),
    ]))?;

    // Restore previous clipboard (best-effort, async so we don't block).
    if let Some(prev) = backup {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(150));
            if let Ok(mut c) = Command::new("wl-copy")
                .stdin(std::process::Stdio::piped()).spawn()
            {
                let _ = c.stdin.as_mut().unwrap().write_all(&prev);
                let _ = c.wait();
            }
        });
    }
    Ok(())
}

pub fn press_key(name: &str) -> Result<()> {
    match detect() {
        Backend::Ydotool => {
            let code = to_evdev_code(name)?;
            run(ydotool_cmd().args(["key", &format!("{code}:1"), &format!("{code}:0")]))
        }
        Backend::Wtype   => run(Command::new("wtype").args(["-k", &to_wtype(name)?])),
        Backend::Xdotool => run(Command::new("xdotool").args(["key", "--", &to_xdotool(name)?])),
        Backend::Enigo   => enigo_combo(name),
    }
}

pub fn press_combo(combo: &str) -> Result<()> {
    let parts: Vec<&str> = combo.split('+').map(str::trim).collect();
    match detect() {
        Backend::Ydotool => {
            // Press modifiers down, click final, release modifiers in reverse.
            let codes: Result<Vec<u16>> = parts.iter().map(|p| to_evdev_code(p)).collect();
            let codes = codes?;
            let mut args: Vec<String> = vec!["key".into()];
            for c in &codes[..codes.len() - 1] { args.push(format!("{c}:1")); }
            let last = codes.last().unwrap();
            args.push(format!("{last}:1"));
            args.push(format!("{last}:0"));
            for c in codes[..codes.len() - 1].iter().rev() { args.push(format!("{c}:0")); }
            let refs: Vec<&str> = args.iter().map(String::as_str).collect();
            run(ydotool_cmd().args(&refs))
        }
        Backend::Wtype => {
            let mut args: Vec<String> = Vec::new();
            for p in &parts[..parts.len() - 1] {
                args.push("-M".into()); args.push(to_wtype(p)?);
            }
            args.push("-k".into());
            args.push(to_wtype(parts.last().unwrap())?);
            for p in &parts[..parts.len() - 1] {
                args.push("-m".into()); args.push(to_wtype(p)?);
            }
            let refs: Vec<&str> = args.iter().map(String::as_str).collect();
            run(Command::new("wtype").args(&refs))
        }
        Backend::Xdotool => {
            let xkeys: Result<Vec<String>> = parts.iter().map(|p| to_xdotool(p)).collect();
            let joined = xkeys?.join("+");
            run(Command::new("xdotool").args(["key", "--", &joined]))
        }
        Backend::Enigo => enigo_combo(combo),
    }
}

// ---------------- Helpers ----------------

fn run(cmd: &mut Command) -> Result<()> {
    let out = cmd.output().map_err(|e| anyhow!("spawn failed: {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(anyhow!("{:?} exited with {}: {}", cmd.get_program(), out.status, stderr.trim()));
    }
    Ok(())
}

/// Map a key name to a Linux evdev key code (used by ydotool).
fn to_evdev_code(name: &str) -> Result<u16> {
    use evdev::Key as K;
    let n = name.trim();
    let lower = n.to_lowercase();
    let k: K = match lower.as_str() {
        "ctrl" | "control"       => K::KEY_LEFTCTRL,
        "shift"                  => K::KEY_LEFTSHIFT,
        "alt"                    => K::KEY_LEFTALT,
        "super" | "meta" | "win" => K::KEY_LEFTMETA,
        "enter" | "return"       => K::KEY_ENTER,
        "tab"                    => K::KEY_TAB,
        "escape" | "esc"         => K::KEY_ESC,
        "space"                  => K::KEY_SPACE,
        "backspace"              => K::KEY_BACKSPACE,
        "delete"                 => K::KEY_DELETE,
        "up"                     => K::KEY_UP,
        "down"                   => K::KEY_DOWN,
        "left"                   => K::KEY_LEFT,
        "right"                  => K::KEY_RIGHT,
        "home"                   => K::KEY_HOME,
        "end"                    => K::KEY_END,
        "pageup"                 => K::KEY_PAGEUP,
        "pagedown"               => K::KEY_PAGEDOWN,
        s if s.starts_with('f') && s[1..].parse::<u32>().is_ok() => f_evdev(s[1..].parse()?)?,
        // letters a-z
        s if s.len() == 1 && s.chars().next().unwrap().is_ascii_alphabetic() => {
            letter_evdev(s.chars().next().unwrap())
        }
        // digits 0-9
        s if s.len() == 1 && s.chars().next().unwrap().is_ascii_digit() => {
            digit_evdev(s.chars().next().unwrap())
        }
        _ => return Err(anyhow!("unknown key for ydotool: {n}")),
    };
    Ok(k.code())
}

fn f_evdev(n: u32) -> Result<evdev::Key> {
    use evdev::Key as K;
    Ok(match n {
        1 => K::KEY_F1,   2 => K::KEY_F2,   3 => K::KEY_F3,   4 => K::KEY_F4,
        5 => K::KEY_F5,   6 => K::KEY_F6,   7 => K::KEY_F7,   8 => K::KEY_F8,
        9 => K::KEY_F9,  10 => K::KEY_F10, 11 => K::KEY_F11, 12 => K::KEY_F12,
        13 => K::KEY_F13, 14 => K::KEY_F14, 15 => K::KEY_F15, 16 => K::KEY_F16,
        17 => K::KEY_F17, 18 => K::KEY_F18, 19 => K::KEY_F19, 20 => K::KEY_F20,
        21 => K::KEY_F21, 22 => K::KEY_F22, 23 => K::KEY_F23, 24 => K::KEY_F24,
        _ => return Err(anyhow!("F{n} not supported")),
    })
}

fn letter_evdev(c: char) -> evdev::Key {
    use evdev::Key as K;
    match c.to_ascii_lowercase() {
        'a' => K::KEY_A, 'b' => K::KEY_B, 'c' => K::KEY_C, 'd' => K::KEY_D,
        'e' => K::KEY_E, 'f' => K::KEY_F, 'g' => K::KEY_G, 'h' => K::KEY_H,
        'i' => K::KEY_I, 'j' => K::KEY_J, 'k' => K::KEY_K, 'l' => K::KEY_L,
        'm' => K::KEY_M, 'n' => K::KEY_N, 'o' => K::KEY_O, 'p' => K::KEY_P,
        'q' => K::KEY_Q, 'r' => K::KEY_R, 's' => K::KEY_S, 't' => K::KEY_T,
        'u' => K::KEY_U, 'v' => K::KEY_V, 'w' => K::KEY_W, 'x' => K::KEY_X,
        'y' => K::KEY_Y, 'z' => K::KEY_Z, _ => K::KEY_RESERVED,
    }
}

fn digit_evdev(c: char) -> evdev::Key {
    use evdev::Key as K;
    match c {
        '0' => K::KEY_0, '1' => K::KEY_1, '2' => K::KEY_2, '3' => K::KEY_3,
        '4' => K::KEY_4, '5' => K::KEY_5, '6' => K::KEY_6, '7' => K::KEY_7,
        '8' => K::KEY_8, '9' => K::KEY_9, _ => K::KEY_RESERVED,
    }
}

// ---- wtype/xdotool keysym tables (kept for fallback) ----

fn to_wtype(name: &str) -> Result<String> {
    let lower = name.trim().to_lowercase();
    Ok(match lower.as_str() {
        "ctrl" | "control"       => "ctrl".into(),
        "shift"                  => "shift".into(),
        "alt"                    => "alt".into(),
        "super" | "meta" | "win" => "logo".into(),
        "enter" | "return"       => "Return".into(),
        "tab" => "Tab".into(), "escape" | "esc" => "Escape".into(),
        "space" => "space".into(), "backspace" => "BackSpace".into(),
        "delete" => "Delete".into(), "up" => "Up".into(), "down" => "Down".into(),
        "left" => "Left".into(), "right" => "Right".into(),
        "home" => "Home".into(), "end" => "End".into(),
        "pageup" => "Prior".into(), "pagedown" => "Next".into(),
        s if s.starts_with('f') && s[1..].parse::<u32>().is_ok() => format!("F{}", &s[1..]),
        s if s.chars().count() == 1 => s.into(),
        _ => return Err(anyhow!("unknown key: {}", name)),
    })
}

fn to_xdotool(name: &str) -> Result<String> {
    let lower = name.trim().to_lowercase();
    Ok(match lower.as_str() {
        "ctrl" | "control"       => "ctrl".into(),
        "shift" => "shift".into(), "alt" => "alt".into(),
        "super" | "meta" | "win" => "super".into(),
        "enter" | "return"       => "Return".into(),
        "tab" => "Tab".into(), "escape" | "esc" => "Escape".into(),
        "space" => "space".into(), "backspace" => "BackSpace".into(),
        "delete" => "Delete".into(), "up" => "Up".into(), "down" => "Down".into(),
        "left" => "Left".into(), "right" => "Right".into(),
        "home" => "Home".into(), "end" => "End".into(),
        "pageup" => "Page_Up".into(), "pagedown" => "Page_Down".into(),
        s if s.starts_with('f') && s[1..].parse::<u32>().is_ok() => format!("F{}", &s[1..]),
        s if s.chars().count() == 1 => s.into(),
        _ => return Err(anyhow!("unknown key: {}", name)),
    })
}

// ---- enigo fallback ----

fn enigo_text(text: &str) -> Result<()> {
    let mut e = Enigo::new(&Settings::default())?;
    e.text(text)?;
    Ok(())
}

fn enigo_combo(combo: &str) -> Result<()> {
    let mut e = Enigo::new(&Settings::default())?;
    let parts: Vec<&str> = combo.split('+').map(str::trim).collect();
    let mut held: Vec<Key> = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        let k = parse_enigo_key(part)?;
        if i == parts.len() - 1 { e.key(k, Direction::Click)?; }
        else { e.key(k, Direction::Press)?; held.push(k); }
    }
    for k in held.into_iter().rev() { e.key(k, Direction::Release)?; }
    Ok(())
}

fn parse_enigo_key(name: &str) -> Result<Key> {
    let lower = name.trim().to_lowercase();
    Ok(match lower.as_str() {
        "ctrl" | "control" => Key::Control,
        "shift" => Key::Shift, "alt" => Key::Alt,
        "super" | "meta" | "win" => Key::Meta,
        "enter" | "return" => Key::Return,
        "tab" => Key::Tab, "escape" | "esc" => Key::Escape,
        "space" => Key::Space, "backspace" => Key::Backspace,
        "delete" => Key::Delete,
        "up" => Key::UpArrow, "down" => Key::DownArrow,
        "left" => Key::LeftArrow, "right" => Key::RightArrow,
        "home" => Key::Home, "end" => Key::End,
        "pageup" => Key::PageUp, "pagedown" => Key::PageDown,
        s if s.chars().count() == 1 => Key::Unicode(s.chars().next().unwrap()),
        _ => return Err(anyhow!("unknown key: {}", name)),
    })
}
