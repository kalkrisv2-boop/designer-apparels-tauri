//! shop_guard.rs
//! --------------
//! Direct port of `app.py`'s `BLUEPRINT_SHOP_TYPES` dict + the
//! `restrict_by_shop_type` before_request hook.
//!
//! Python centralized this as one dict checked before every request, so
//! adding a new vertical blueprint automatically protects it the moment
//! its name is added -- no route file needs its own check. Tauri has no
//! shared middleware layer across commands, so the equivalent here is:
//! every vertical command (Phase 1 onward) calls `check_shop_type` as
//! its first line, passing its own command group name. A command group
//! left out of `COMMAND_SHOP_TYPES` entirely (dashboard, auth, shops,
//! settings) is unrestricted -- shared across every shop_type, exactly
//! matching the Python version's "not listed = no restriction" rule.
//!
//! Nothing in Phase 0 calls this yet (there are no vertical commands
//! until Phase 1+), but the map is filled in now so each future
//! vertical's commands can add one line here instead of re-deriving the
//! pattern.

use std::collections::HashMap;
use std::sync::OnceLock;

fn command_shop_types() -> &'static HashMap<&'static str, &'static [&'static str]> {
    static MAP: OnceLock<HashMap<&'static str, &'static [&'static str]>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m: HashMap<&'static str, &'static [&'static str]> = HashMap::new();
        m.insert("products", &["designer_apparels"]);
        m.insert("billing", &["designer_apparels"]);
        m.insert("customers", &["designer_apparels"]);
        m.insert("suppliers", &["designer_apparels"]);
        m.insert("purchases", &["designer_apparels"]);
        m.insert("accounts", &["designer_apparels"]);
        m.insert("barcode", &["designer_apparels"]);
        m.insert("estimation", &["designer_apparels"]);
        m.insert("sales_returns", &["designer_apparels"]);
        m.insert("reports", &["designer_apparels"]);
        m.insert("invoices", &["designer_apparels"]);
        m.insert("staff_hr", &["staff_hr"]);
        m.insert("student_management", &["student_management"]);
        m.insert("garments", &["garments"]);
        m.insert("service", &["service"]);
        // "settings" deliberately NOT listed -- Business Profile is
        // needed by every vertical, not just Designer Apparels. Same
        // reasoning as dashboard/auth/shops being left out.
        m
    })
}

/// Returns Ok(()) if `current_shop_type` is allowed to use
/// `command_group`, Err(reason) otherwise. A command_group not present
/// in the map at all is always allowed (unrestricted, shared blueprint).
pub fn check_shop_type(command_group: &str, current_shop_type: &str) -> Result<(), String> {
    match command_shop_types().get(command_group) {
        None => Ok(()),
        Some(allowed) if allowed.contains(&current_shop_type) => Ok(()),
        Some(_) => Err(format!(
            "'{command_group}' is not available for shop type '{current_shop_type}'"
        )),
    }
}
