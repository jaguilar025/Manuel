//! Recorder: when armed, the next ParsedEvent from any watched device is captured,
//! converted into an InputTrigger, and emitted to the frontend.

use std::sync::atomic::{AtomicBool, Ordering};
use parking_lot::Mutex;
use crate::models::{HidButtonRef, InputTrigger};
use crate::services::engine::ParsedEvent;

type Emitter = Box<dyn Fn(InputTrigger) + Send + Sync>;

pub struct Recorder {
    armed: AtomicBool,
    emitter: Mutex<Option<Emitter>>,
}

impl Recorder {
    pub fn new() -> Self {
        Self { armed: AtomicBool::new(false), emitter: Mutex::new(None) }
    }
    pub fn set_emitter(&self, e: Emitter) { *self.emitter.lock() = Some(e); }
    pub fn arm(&self)  { self.armed.store(true,  Ordering::SeqCst); }
    pub fn stop(&self) { self.armed.store(false, Ordering::SeqCst); }
    pub fn is_armed(&self) -> bool { self.armed.load(Ordering::SeqCst) }

    pub fn submit(&self, ev: &ParsedEvent) {
        if !self.is_armed() { return; }
        let trigger = match ev {
            ParsedEvent::Key { combo, device, device_name, vendor_id, product_id, raw_code } => {
                if combo.contains('+') {
                    InputTrigger::KeyCombo(combo.clone())
                } else {
                    InputTrigger::HidButton(HidButtonRef {
                        device: device.clone(),
                        device_name: device_name.clone(),
                        vendor_id: *vendor_id,
                        product_id: *product_id,
                        code: *raw_code,
                    })
                }
            }
            ParsedEvent::Button { device, device_name, vendor_id, product_id, code } => {
                InputTrigger::HidButton(HidButtonRef {
                    device: device.clone(),
                    device_name: device_name.clone(),
                    vendor_id: *vendor_id,
                    product_id: *product_id,
                    code: *code,
                })
            }
        };
        self.armed.store(false, Ordering::SeqCst);
        if let Some(emit) = self.emitter.lock().as_ref() {
            emit(trigger);
        }
    }
}
