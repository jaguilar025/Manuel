//! XKB-based character → keypress lookup with dead-key fallback.
//!
//! Strategy (mirrors what antimicrox / X compose does internally):
//!   1. Build a direct map: char → (evdev_code, shift, altgr) for every char
//!      that the active layout can produce in a single keystroke.
//!   2. For characters NOT directly available (e.g. "ñ" on US layout), try a
//!      dead-key decomposition: NFD("ñ") = "n" + combining tilde (U+0303).
//!      If the layout has a key bound to `dead_tilde`, we press dead_tilde
//!      THEN "n" and the kernel/IM composes "ñ" — same as if you typed it.

use std::{collections::HashMap, process::Command};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use unicode_normalization::UnicodeNormalization;
use xkbcommon::xkb;

#[derive(Debug, Clone, Copy)]
pub struct KeyPress {
    pub evdev_code: u16,
    pub shift: bool,
    pub altgr: bool,
}

#[derive(Default)]
struct Maps {
    /// Direct: char → keypress that produces it in one stroke.
    chars: HashMap<char, KeyPress>,
    /// Dead keys: combining char (e.g. U+0303 tilde) → keypress that
    /// produces the corresponding dead_key keysym on the active layout.
    deads: HashMap<char, KeyPress>,
}

static MAPS: Lazy<RwLock<Maps>> = Lazy::new(|| RwLock::new(Maps::default()));
static BUILT: Lazy<RwLock<bool>> = Lazy::new(|| RwLock::new(false));

pub fn ensure_built() {
    if *BUILT.read() { return; }
    let mut built = BUILT.write();
    if *built { return; }
    *MAPS.write() = build_maps();
    *built = true;
}

/// Single-stroke lookup for a character.
pub fn lookup(c: char) -> Option<KeyPress> {
    ensure_built();
    MAPS.read().chars.get(&c).copied()
}

/// Try to decompose a character into a sequence of keypresses using
/// dead keys available on the active layout.
/// E.g. "ñ" → [dead_tilde, n], "é" → [dead_acute, e].
/// Returns None if the layout can't compose it.
pub fn decompose(c: char) -> Option<Vec<KeyPress>> {
    ensure_built();
    let maps = MAPS.read();

    // NFD: "ñ" -> ['n', '\u{0303}']
    let parts: Vec<char> = c.nfd().collect();
    if parts.len() < 2 { return None; }

    let base = parts[0];
    let base_kp = *maps.chars.get(&base)?;

    let mut out: Vec<KeyPress> = Vec::with_capacity(parts.len());
    // Press dead keys first (in order), then the base char.
    for combining in &parts[1..] {
        let dead = *maps.deads.get(combining)?;
        out.push(dead);
    }
    out.push(base_kp);
    Some(out)
}

#[allow(dead_code)]
pub fn rebuild() {
    *MAPS.write() = build_maps();
    *BUILT.write() = true;
}

// ---------------- internals ----------------

fn build_maps() -> Maps {
    let (layout, variant, options) = detect_layout();
    log::info!("xkb layout: layout='{layout}' variant='{variant}' options='{options}'");

    let ctx = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let keymap = match xkb::Keymap::new_from_names(
        &ctx, "", "", &layout, &variant, Some(options),
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    ) {
        Some(k) => k,
        None => {
            log::warn!("could not build xkb keymap; text emission will fall back to clipboard");
            return Maps::default();
        }
    };

    let shift_idx = keymap.mod_get_index(xkb::MOD_NAME_SHIFT);
    let altgr_idx = {
        let i = keymap.mod_get_index("Mod5");
        if i != xkb::MOD_INVALID { i } else { keymap.mod_get_index("ISO_Level3_Shift") }
    };
    let shift_mask = if shift_idx != xkb::MOD_INVALID { 1u32 << shift_idx } else { 0 };
    let altgr_mask = if altgr_idx != xkb::MOD_INVALID { 1u32 << altgr_idx } else { 0 };

    let combos: &[(u32, bool, bool)] = &[
        (0,                       false, false),
        (shift_mask,              true,  false),
        (altgr_mask,              false, true),
        (shift_mask | altgr_mask, true,  true),
    ];

    let mut chars: HashMap<char, KeyPress> = HashMap::new();
    let mut deads: HashMap<char, KeyPress> = HashMap::new();
    let min = keymap.min_keycode().raw();
    let max = keymap.max_keycode().raw();

    for kc_raw in min..=max {
        let kc = xkb::Keycode::new(kc_raw);
        for &(mask, shift, altgr) in combos {
            let mut state = xkb::State::new(&keymap);
            state.update_mask(mask, 0, 0, 0, 0, 0);
            let evdev_code = kc_raw.saturating_sub(8) as u16;
            let kp = KeyPress { evdev_code, shift, altgr };

            // 1) Direct char from utf8 (works for printable keys).
            let s = state.key_get_utf8(kc);
            if !s.is_empty() {
                let mut it = s.chars();
                if let Some(c) = it.next() {
                    if it.next().is_none() && !c.is_control() {
                        chars.entry(c).and_modify(|prev| {
                            if priority(kp) < priority(*prev) { *prev = kp; }
                        }).or_insert(kp);
                        continue;
                    }
                }
            }

            // 2) Dead key (key produces no utf8 on its own — check raw keysym).
            let sym = state.key_get_one_sym(kc).raw();
            if let Some(combining) = dead_keysym_to_combining(sym) {
                deads.entry(combining).and_modify(|prev| {
                    if priority(kp) < priority(*prev) { *prev = kp; }
                }).or_insert(kp);
            }
        }
    }

    log::info!("xkb maps built: {} direct chars, {} dead keys", chars.len(), deads.len());
    if !deads.is_empty() {
        let names: Vec<String> = deads.keys()
            .map(|c| format!("U+{:04X}", *c as u32))
            .collect();
        log::info!("dead keys available: [{}]", names.join(", "));
    }
    Maps { chars, deads }
}

fn priority(k: KeyPress) -> u8 {
    (k.shift as u8) + ((k.altgr as u8) * 2)
}

/// Map an XKB `dead_*` keysym (constants 0xFE5n area) to the Unicode
/// combining mark it composes with.
fn dead_keysym_to_combining(sym: u32) -> Option<char> {
    Some(match sym {
        0xfe50 => '\u{0300}', // dead_grave        -> combining grave
        0xfe51 => '\u{0301}', // dead_acute        -> combining acute
        0xfe52 => '\u{0302}', // dead_circumflex   -> combining circumflex
        0xfe53 => '\u{0303}', // dead_tilde        -> combining tilde
        0xfe54 => '\u{0304}', // dead_macron       -> combining macron
        0xfe55 => '\u{0306}', // dead_breve        -> combining breve
        0xfe56 => '\u{0307}', // dead_abovedot     -> combining dot above
        0xfe57 => '\u{0308}', // dead_diaeresis    -> combining diaeresis
        0xfe58 => '\u{030A}', // dead_abovering    -> combining ring above
        0xfe59 => '\u{030B}', // dead_doubleacute  -> combining double acute
        0xfe5a => '\u{030C}', // dead_caron        -> combining caron
        0xfe5b => '\u{0327}', // dead_cedilla      -> combining cedilla
        0xfe5c => '\u{0328}', // dead_ogonek       -> combining ogonek
        0xfe5d => '\u{0345}', // dead_iota         -> combining iota
        0xfe5e => '\u{3099}', // dead_voiced_sound -> Japanese voiced
        0xfe5f => '\u{309A}', // dead_semivoiced_sound
        0xfe60 => '\u{0323}', // dead_belowdot     -> combining dot below
        0xfe61 => '\u{0309}', // dead_hook         -> combining hook above
        0xfe62 => '\u{031B}', // dead_horn         -> combining horn
        0xfe63 => '\u{0322}', // dead_stroke (custom variants)
        _ => return None,
    })
}

fn detect_layout() -> (String, String, String) {
    let on_wayland = std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland")
        || std::env::var("WAYLAND_DISPLAY").is_ok();

    // On GNOME/Wayland, setxkbmap reports stale/incomplete info (only sees the
    // XWayland subset), so prefer gsettings there. On X11, setxkbmap is truth.
    if on_wayland {
        if let Some(r) = from_gsettings() { return r; }
        if let Some(r) = from_setxkbmap() { return r; }
    } else {
        if let Some(r) = from_setxkbmap() { return r; }
        if let Some(r) = from_gsettings() { return r; }
    }

    if let Ok(v) = std::env::var("XKB_DEFAULT_LAYOUT") {
        return (
            v,
            std::env::var("XKB_DEFAULT_VARIANT").unwrap_or_default(),
            std::env::var("XKB_DEFAULT_OPTIONS").unwrap_or_default(),
        );
    }
    ("us".into(), String::new(), String::new())
}

fn from_setxkbmap() -> Option<(String, String, String)> {
    let out = Command::new("setxkbmap").arg("-query").output().ok()?;
    if !out.status.success() { return None; }
    let (mut layout, mut variant, mut options) = (String::new(), String::new(), String::new());
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if let Some(r) = line.strip_prefix("layout:")  { layout  = r.trim().into(); }
        else if let Some(r) = line.strip_prefix("variant:") { variant = r.trim().into(); }
        else if let Some(r) = line.strip_prefix("options:") { options = r.trim().into(); }
    }
    if layout.is_empty() { None } else { Some((layout, variant, options)) }
}

fn from_gsettings() -> Option<(String, String, String)> {
    // gsettings output: [('ibus', 'mozc-jp'), ('xkb', 'us+intl'), ('xkb', 'es')]
    // We want the first 'xkb' entry, skipping ibus/other engines.
    let out = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.input-sources", "sources"])
        .output().ok()?;
    if !out.status.success() { return None; }
    let s = String::from_utf8_lossy(&out.stdout);
    // Naive scan for "'xkb', '...'"; ignores tuples not starting with 'xkb'.
    let mut cursor = 0usize;
    while let Some(rel) = s[cursor..].find("'xkb', '") {
        let abs = cursor + rel + "'xkb', '".len();
        if let Some(end_rel) = s[abs..].find('\'') {
            let spec = &s[abs..abs + end_rel];
            let (layout, variant) = match spec.split_once('+') {
                Some((l, v)) => (l.to_string(), v.to_string()),
                None => (spec.to_string(), String::new()),
            };
            // Also try to pick up options if user has any (org.gnome.desktop.input-sources xkb-options)
            let options = gsettings_xkb_options().unwrap_or_default();
            return Some((layout, variant, options));
        }
        cursor = abs;
    }
    None
}

fn gsettings_xkb_options() -> Option<String> {
    let out = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.input-sources", "xkb-options"])
        .output().ok()?;
    if !out.status.success() { return None; }
    // Output like: ['terminate:ctrl_alt_bksp', 'caps:ctrl_modifier']
    let s = String::from_utf8_lossy(&out.stdout);
    let mut opts = Vec::new();
    let mut cursor = 0usize;
    while let Some(rel) = s[cursor..].find('\'') {
        let abs = cursor + rel + 1;
        if let Some(end_rel) = s[abs..].find('\'') {
            opts.push(s[abs..abs + end_rel].to_string());
            cursor = abs + end_rel + 1;
        } else { break; }
    }
    if opts.is_empty() { None } else { Some(opts.join(",")) }
}
