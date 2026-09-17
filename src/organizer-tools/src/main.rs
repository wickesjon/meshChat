//! Private pipe protocol for console.py. Never send secret material in argv.
use meshchat_organizer::{self as tool, Invalid};
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::{self, IsTerminal, Read, Write};
use zeroize::{Zeroize, Zeroizing};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    operation: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    expiry: u32,
    #[serde(default)]
    event_end: u32,
    #[serde(default)]
    before: u32,
    #[serde(default)]
    after: u32,
    #[serde(default)]
    vault: String,
    #[serde(default)]
    unlock: String,
}
impl Drop for Request {
    fn drop(&mut self) {
        self.unlock.zeroize();
    }
}
fn dispatch(r: &Request, now: u32) -> Result<Value, Invalid> {
    match r.operation.as_str() {
        "check-root" => {
            tool::check_root(&r.name, r.expiry, r.event_end, now)?;
            Ok(json!({"ok":true}))
        }
        "create" => {
            let root = tool::create_root(&r.name, r.expiry, r.event_end, now)?;
            Ok(json!({"ok":true,"vault":root.vault,"unlock":&*root.unlock,"event":root.event}))
        }
        "inspect" | "check-staff" | "issue" => {
            let root = tool::OpenRoot::open(&r.vault, &r.unlock, now)?;
            let event = root.event()?;
            if r.operation == "inspect" {
                return Ok(json!({"ok":true,"event":event,"name":root.name,"expiry":root.expiry}));
            }
            root.check_staff(&r.label, r.before, r.after, now)?;
            if r.operation == "check-staff" {
                return Ok(json!({"ok":true,"name":root.name,"expiry":root.expiry}));
            }
            let staff = root.issue(&r.label, r.before, r.after, now)?;
            Ok(
                json!({"ok":true,"matrix":tool::matrix(&staff)?,"event":event,"label":r.label,"before":r.before,"after":r.after}),
            )
        }
        _ => Err(Invalid),
    }
}
fn clear(value: &mut Value) {
    match value {
        Value::String(s) => s.zeroize(),
        Value::Array(a) => a.iter_mut().for_each(clear),
        Value::Object(o) => o.values_mut().for_each(clear),
        _ => {}
    }
}
fn run() -> Result<(), Invalid> {
    if std::env::args().skip(1).collect::<Vec<_>>() != ["--private-pipe"]
        || io::stdin().is_terminal()
        || io::stdout().is_terminal()
    {
        return Err(Invalid);
    }
    let mut input = Zeroizing::new(String::new());
    io::stdin()
        .take(8193)
        .read_to_string(&mut input)
        .map_err(|_| Invalid)?;
    if input.len() > 8192 {
        return Err(Invalid);
    }
    let request: Request = serde_json::from_str(&input).map_err(|_| Invalid)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Invalid)?
        .as_secs()
        .try_into()
        .map_err(|_| Invalid)?;
    let mut response = dispatch(&request, now)?;
    let encoded = Zeroizing::new(serde_json::to_vec(&response).map_err(|_| Invalid)?);
    clear(&mut response);
    io::stdout().write_all(&encoded).map_err(|_| Invalid)
}
fn main() {
    // Errors deliberately exclude parser input, key material and provisioning QR.
    if run().is_err() {
        eprintln!("Offline operation refused. Check inputs, validity and unlock code.");
        std::process::exit(1);
    }
}
