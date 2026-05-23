use std::{process::Command, thread, time::Duration};
use anyhow::Result;
use crate::models::{MacroStep, Output};
use crate::services::keyboard;

pub fn run(output: &Output) -> Result<()> {
    match output {
        Output::Text(t)  => keyboard::type_text(t),
        Output::Key(k)   => keyboard::press_key(k),
        Output::Combo(c) => keyboard::press_combo(c),
        Output::Shell(s) => run_shell(s),
        Output::Macro(v) => run_macro(v),
    }
}

fn run_macro(value: &serde_json::Value) -> Result<()> {
    let steps: Vec<MacroStep> = match value {
        serde_json::Value::String(s) => serde_json::from_str(s)?,
        _ => serde_json::from_value(value.clone())?,
    };
    for step in steps {
        match step {
            MacroStep::Type { text }  => keyboard::type_text(&text)?,
            MacroStep::Press { key }  => keyboard::press_key(&key)?,
            MacroStep::Combo { combo }=> keyboard::press_combo(&combo)?,
            MacroStep::Delay { ms }   => thread::sleep(Duration::from_millis(ms)),
            MacroStep::Shell { cmd }  => run_shell(&cmd)?,
        }
    }
    Ok(())
}

fn run_shell(cmd: &str) -> Result<()> {
    Command::new("sh").arg("-c").arg(cmd).spawn()?;
    Ok(())
}
