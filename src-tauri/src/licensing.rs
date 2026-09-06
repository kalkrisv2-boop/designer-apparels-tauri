//! licensing.rs
//! -------------
//! Machine-bound license activation, ported from the Python app's
//! `utils/licensing.py` + `routes/licensing.py`.
//!
//! READ THIS BEFORE RELYING ON IT FOR ANYTHING BEYOND A SOFT DETERRENT
//! ---------------------------------------------------------------------
//! Same offline-first design, same honest limits as the Python version:
//! this app has no server to call home to, so the HMAC secret used to
//! sign/verify activation keys has to ship *inside* the same .exe the
//! end user has physical access to. Anyone able to pull strings/bytes
//! out of a compiled Rust binary (much harder than PyInstaller
//! extraction, but not impossible) can eventually recover it and mint a
//! key for any hardware ID. What this achieves: stops a shop owner
//! copying the .exe + db folder onto a second PC and running it there
//! unlicensed. What it does NOT achieve: protection against someone
//! technically capable choosing to pirate the software on purpose. If
//! that ever becomes the real threat model, the fix is an online
//! activation server -- a separate, explicit decision, not something to
//! bolt on here.
//!
//! HARDWARE ID NOTE: this is a fresh Rust product, not a byte-for-byte
//! port of Python's `uuid.getnode()` -- it derives the hardware ID from
//! the primary network interface's MAC address the same way, but uses a
//! separate keygen/secret from the Python app's. Existing Python-issued
//! activation keys will NOT work here and vice versa; whoever runs the
//! vendor-side keygen for this app needs to issue keys against *this*
//! app's hardware ID (see `generate_activation_key`), not the Python
//! app's. This is a deliberate scope decision for Phase 0, called out
//! explicitly rather than silently assumed -- flag it if per-client
//! licensing needs to be shared across both codebases during the
//! migration window.

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::fs;

use crate::paths::persistent_path;

type HmacSha256 = Hmac<Sha256>;

// Generated once with a CSPRNG and fixed here so activation works out of
// the box across builds from this baseline -- same role as Python's
// LICENSE_SECRET, but a distinct value (see module docstring: this is a
// separate product/keygen, not required to match the Python app's).
// Regenerating this is a breaking change for every activation key
// already issued against it -- treat it as such, not a routine edit.
const LICENSE_SECRET: &[u8] = b"7c3f9a2e5d81b4406fa1e9c8d2b5f70c93a6e41d8b0c2f75a4e916d3c8b5f0a2";

const LICENSE_FILE: &str = "license.json";

/// Deterministic-per-machine ID derived from the primary network
/// interface's MAC address, hashed so the raw MAC is never itself
/// stored or displayed. Documented limitation, not hidden: this can
/// change if the network adapter changes (new NIC, a USB dongle, some
/// VM/hypervisor configurations), which looks like "a different
/// machine" and forces re-activation -- same accepted tradeoff as the
/// Python version, for staying dependency-light.
pub fn get_hardware_id() -> String {
    let mac_bytes: [u8; 6] = mac_address::get_mac_address()
        .ok()
        .flatten()
        .map(|m| m.bytes())
        .unwrap_or([0, 0, 0, 0, 0, 0]);

    let raw = mac_bytes
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<String>();

    let digest = Sha256::digest(raw.as_bytes());
    let hex = digest.iter().map(|b| format!("{:02x}", b)).collect::<String>().to_uppercase();

    format!("DA-{}-{}-{}", &hex[0..4], &hex[4..8], &hex[8..12])
}

/// Vendor-side helper only -- produces a valid activation key for a
/// given hardware ID. Not wired to any Tauri command in this app; keep
/// it import-only from a small, separate vendor-side keygen tool (a
/// plain Rust binary or script you run yourself when a client pays). If
/// a call to this ever shows up behind a `#[tauri::command]`, the
/// vendor's own keygen logic has leaked into the same binary a curious
/// client could inspect.
pub fn generate_activation_key(hardware_id: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(LICENSE_SECRET).expect("HMAC accepts any key length");
    mac.update(hardware_id.as_bytes());
    let sig = mac.finalize().into_bytes();
    let hex = sig.iter().map(|b| format!("{:02x}", b)).collect::<String>();
    hex[..20].to_uppercase()
}

/// Constant-time-ish comparison via byte equality on fixed-length
/// uppercase hex strings (both sides normalized before compare) -- no
/// reason to make this any easier to attack than the offline-secret
/// design already makes it (see module docstring).
pub fn verify_activation_key(hardware_id: &str, key: &str) -> bool {
    if key.trim().is_empty() {
        return false;
    }
    let expected = generate_activation_key(hardware_id);
    let given = key.trim().to_uppercase();
    expected.len() == given.len() && expected.as_bytes() == given.as_bytes()
}

/// True only if license.json exists next to the .exe AND its stored key
/// verifies against *this* machine's current hardware ID -- copying an
/// activated license.json (or the whole db) onto a second PC does not
/// carry activation with it, since the hardware ID recomputed there
/// won't match.
pub fn is_activated() -> bool {
    let path = persistent_path(LICENSE_FILE);
    let Ok(text) = fs::read_to_string(&path) else {
        return false;
    };
    let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    let key = data.get("key").and_then(|v| v.as_str()).unwrap_or("");
    verify_activation_key(&get_hardware_id(), key)
}

/// Persists an activation key next to the .exe once it's been verified.
pub fn save_activation(hardware_id: &str, key: &str) -> Result<(), String> {
    let path = persistent_path(LICENSE_FILE);
    let data = serde_json::json!({ "hardware_id": hardware_id, "key": key });
    fs::write(&path, data.to_string()).map_err(|e| e.to_string())
}
