//! licensing_commands.rs
//! -----------------------
//! Thin Tauri-command layer over licensing.rs, ported from
//! `routes/licensing.py`'s `activate` / `activate_submit`. There's no
//! `before_request` gate here the way Flask had one -- Tauri commands
//! aren't intercepted by shared middleware -- so the frontend calls
//! `check_license` on startup and shows the activation screen itself if
//! `activated` is false, before rendering anything else. See
//! main.rs's setup() for the equivalent "don't even let an unlicensed
//! install look around" gate: it's enforced by the frontend routing on
//! `activated`, not by refusing to register the other commands (Tauri
//! has no per-command enable/disable), so this is a slightly different
//! shape than Flask's before_request but the same end-user behaviour.

use serde::Serialize;

use crate::licensing;

#[derive(Serialize)]
pub struct LicenseStatus {
    pub activated: bool,
    pub hardware_id: String,
}

#[tauri::command]
pub fn check_license() -> LicenseStatus {
    LicenseStatus {
        activated: licensing::is_activated(),
        hardware_id: licensing::get_hardware_id(),
    }
}

#[tauri::command]
pub fn activate_license(key: String) -> Result<(), String> {
    let hw_id = licensing::get_hardware_id();
    if !licensing::verify_activation_key(&hw_id, &key) {
        return Err(
            "That activation key doesn't match this machine's Hardware ID. \
             Double-check it was typed exactly as given, with no extra spaces."
                .into(),
        );
    }
    licensing::save_activation(&hw_id, key.trim())
}
