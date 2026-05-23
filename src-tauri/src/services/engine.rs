//! Engine — owns one watcher thread per active input device, matches incoming
//! events against the configured mappings, and dispatches outputs.
//!
//! Architecture:
//!   - apply(&Config) rebuilds the routing table and respawns watchers.
//!   - Each watcher thread holds an `evdev::Device` in non-blocking grab-less mode
//!     and pushes parsed events into a single mpsc channel.
//!   - A dispatcher thread consumes that channel and either:
//!       (a) executes the matched output via macro_runner, or
//!       (b) forwards the trigger to the Recorder if it is armed.
//!
//! Notes:
//!   - We DO NOT grab the device exclusively, so other apps still receive the event.
//!     If you want to "swallow" the input (e.g. a real ñ key emitting only output),
//!     call `device.grab()` in the watcher — at the cost of breaking the device for
//!     other consumers while Manuel runs.

use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use evdev::{EventType, InputEventKind, Key};
use parking_lot::Mutex;

use crate::models::{Config, DeviceInfo, InputTrigger, Mapping};
use crate::services::{hid, macro_runner, recorder::Recorder};

type DeviceEmitter = Box<dyn Fn(Vec<DeviceInfo>) + Send + Sync>;

pub struct Engine {
    inner: Mutex<Inner>,
    devices_emitter: Mutex<Option<DeviceEmitter>>,
    recorder: Mutex<Option<Arc<Recorder>>>,
}

struct Inner {
    mappings: Vec<Mapping>,
    /// Per-watcher stop flag. Setting to true asks the thread to exit at its next poll.
    stop_flags: Vec<Arc<AtomicBool>>,
    /// Currently held modifier keys (across all watched keyboards).
    held: Arc<Mutex<HashSet<Key>>>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                mappings: Vec::new(),
                stop_flags: Vec::new(),
                held: Arc::new(Mutex::new(HashSet::new())),
            }),
            devices_emitter: Mutex::new(None),
            recorder: Mutex::new(None),
        }
    }

    pub fn set_devices_emitter(&self, emit: DeviceEmitter) {
        *self.devices_emitter.lock() = Some(emit);
    }

    pub fn attach_recorder(&self, r: Arc<Recorder>) {
        *self.recorder.lock() = Some(r);
    }

    pub fn apply(self: &Arc<Self>, cfg: &Config) {
        self.apply_inner(cfg);
    }

    fn apply_inner(self: &Arc<Self>, cfg: &Config) {
        let mut inner = self.inner.lock();
        // Stop existing watchers
        for f in &inner.stop_flags { f.store(true, Ordering::SeqCst); }
        inner.stop_flags.clear();
        inner.mappings = cfg.mappings.clone();

        // Spawn one watcher per available device
        let devices = hid::enumerate();
        if let Some(emit) = self.devices_emitter.lock().as_ref() {
            emit(devices.clone());
        }

        for dev in devices.into_iter().filter(|d| d.connected) {
            let stop = Arc::new(AtomicBool::new(false));
            inner.stop_flags.push(stop.clone());
            let held = inner.held.clone();
            let engine = Arc::clone(self);
            let dev_path = dev.path.clone();
            let dev_name = dev.name.clone();
            let vid = dev.vendor_id;
            let pid = dev.product_id;
            thread::spawn(move || {
                if let Err(e) = watch_device(engine, dev_path, dev_name, vid, pid, held, stop) {
                    log::warn!("watcher exited: {e:?}");
                }
            });
        }
    }

    fn handle_event(self: &Arc<Self>, ev: ParsedEvent) {
        // Recorder takes precedence
        if let Some(r) = self.recorder.lock().clone() {
            if r.is_armed() {
                r.submit(&ev);
                return;
            }
        }
        let mappings = self.inner.lock().mappings.clone();
        for m in mappings.iter().filter(|m| m.enabled) {
            if matches(&m.input, &ev) {
                if let Err(e) = macro_runner::run(&m.output) {
                    log::warn!("output failed: {e:?}");
                }
                break;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ParsedEvent {
    /// A key on a recognized layout — useful for combos.
    Key {
        combo: String,
        device: String,
        device_name: String,
        vendor_id: Option<u16>,
        product_id: Option<u16>,
        raw_code: u16,
    },
    /// A button without a printable mapping (HID button typical).
    #[allow(dead_code)]
    Button {
        device: String,
        device_name: String,
        vendor_id: Option<u16>,
        product_id: Option<u16>,
        code: u16,
    },
}

fn matches(trigger: &InputTrigger, ev: &ParsedEvent) -> bool {
    match (trigger, ev) {
        (InputTrigger::KeyCombo(combo), ParsedEvent::Key { combo: c, .. }) => {
            normalize_combo(combo) == normalize_combo(c)
        }
        (InputTrigger::HidButton(b),
         ParsedEvent::Button { device, vendor_id, product_id, code, .. }) => {
            same_device(b, *vendor_id, *product_id, device) && b.code == *code
        }
        (InputTrigger::HidButton(b),
         ParsedEvent::Key { device, vendor_id, product_id, raw_code, .. }) => {
            same_device(b, *vendor_id, *product_id, device) && b.code == *raw_code
        }
        _ => false,
    }
}

/// A mapping matches a device if its vendor+product IDs match (preferred),
/// or — as a legacy fallback — if the stored evdev path matches.
fn same_device(
    b: &crate::models::HidButtonRef,
    vendor_id: Option<u16>,
    product_id: Option<u16>,
    path: &str,
) -> bool {
    match (b.vendor_id, b.product_id, vendor_id, product_id) {
        (Some(bv), Some(bp), Some(v), Some(p)) if bv != 0 || bp != 0 => bv == v && bp == p,
        _ => b.device == path, // legacy mappings without vid/pid
    }
}

fn normalize_combo(s: &str) -> String {
    let mut parts: Vec<String> = s
        .split('+')
        .map(|p| p.trim().to_lowercase())
        .collect();
    parts.sort();
    parts.join("+")
}

fn watch_device(
    engine: Arc<Engine>,
    path: String,
    name: String,
    vendor_id: Option<u16>,
    product_id: Option<u16>,
    held: Arc<Mutex<HashSet<Key>>>,
    stop: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let mut device = hid::open(&path)?;
    // Non-blocking so we can honor the stop flag.
    set_nonblocking(&device);

    loop {
        if stop.load(Ordering::SeqCst) { break; }
        match device.fetch_events() {
            Ok(events) => {
                for ev in events {
                    if ev.event_type() != EventType::KEY { continue; }
                    if let InputEventKind::Key(k) = ev.kind() {
                        let pressed = ev.value() == 1; // 1=press, 0=release, 2=repeat
                        let released = ev.value() == 0;
                        if is_modifier(k) {
                            let mut g = held.lock();
                            if pressed { g.insert(k); } else if released { g.remove(&k); }
                            continue;
                        }
                        if !pressed { continue; }
                        let combo = build_combo(&held.lock(), k);
                        let parsed = ParsedEvent::Key {
                            combo,
                            device: path.clone(),
                            device_name: name.clone(),
                            vendor_id,
                            product_id,
                            raw_code: k.code(),
                        };
                        engine.handle_event(parsed);
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(_) => break,
        }
    }
    Ok(())
}

fn set_nonblocking(device: &evdev::Device) {
    use std::os::unix::io::AsRawFd;
    let fd = device.as_raw_fd();
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags >= 0 {
            libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }
    }
}

fn is_modifier(k: Key) -> bool {
    matches!(
        k,
        Key::KEY_LEFTCTRL | Key::KEY_RIGHTCTRL
        | Key::KEY_LEFTSHIFT | Key::KEY_RIGHTSHIFT
        | Key::KEY_LEFTALT | Key::KEY_RIGHTALT
        | Key::KEY_LEFTMETA | Key::KEY_RIGHTMETA
    )
}

fn build_combo(held: &HashSet<Key>, k: Key) -> String {
    let mut parts: Vec<String> = Vec::new();
    if held.contains(&Key::KEY_LEFTCTRL)  || held.contains(&Key::KEY_RIGHTCTRL)  { parts.push("Ctrl".into()); }
    if held.contains(&Key::KEY_LEFTSHIFT) || held.contains(&Key::KEY_RIGHTSHIFT) { parts.push("Shift".into()); }
    if held.contains(&Key::KEY_LEFTALT)   || held.contains(&Key::KEY_RIGHTALT)   { parts.push("Alt".into()); }
    if held.contains(&Key::KEY_LEFTMETA)  || held.contains(&Key::KEY_RIGHTMETA)  { parts.push("Super".into()); }
    parts.push(key_to_string(k));
    parts.join("+")
}

fn key_to_string(k: Key) -> String {
    // Minimal mapping — extend as needed. Falls back to raw code.
    let s = format!("{:?}", k); // e.g. "KEY_A", "KEY_F13"
    s.strip_prefix("KEY_").map(str::to_string).unwrap_or(s)
}

