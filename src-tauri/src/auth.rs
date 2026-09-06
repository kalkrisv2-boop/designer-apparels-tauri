//! auth.rs
//! --------
//! Ported from `routes/auth.py`. The Python version's own docstring
//! flags that the *original PHP* it replaced concatenated raw form
//! input into SQL and compared plaintext passwords -- Python already
//! fixed both (parameterized queries + werkzeug password hashing). This
//! Rust version keeps both fixes: `rusqlite` params are always bound,
//! never string-formatted, and passwords are checked with `bcrypt`
//! (see platform_db.rs's seed step for how the default admin's hash is
//! created).

use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use tauri::State;

use crate::config::AppConfig;
use crate::platform_db::PlatformDb;
use crate::session::SessionState;

#[derive(Serialize)]
pub struct LoginResult {
    /// Where the frontend should navigate next: "pick-shop" in
    /// multi-shop mode (mirrors Python redirecting to shops.select_shop
    /// instead of setting session["admin"] immediately), or "dashboard"
    /// in single-shop mode.
    pub next: &'static str,
}

#[tauri::command]
pub fn login(
    username: String,
    password: String,
    db: State<PlatformDb>,
    session: State<SessionState>,
    config: State<AppConfig>,
) -> Result<LoginResult, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let row = conn
        .query_row(
            "SELECT sp_shopid, sp_shopname, sp_password, sp_acnttype, sp_shop_type
             FROM vm_shopprofile WHERE sp_username = ?1 AND sp_isactive = 0",
            params![username.trim()],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;

    let Some((shop_id, shop_name, password_hash, acnttype, shop_type)) = row else {
        return Err("Invalid username or password.".into());
    };

    let ok = bcrypt::verify(&password, &password_hash).unwrap_or(false);
    if !ok {
        return Err("Invalid username or password.".into());
    }

    let mut s = session.0.lock().map_err(|e| e.to_string())?;
    s.clear();
    s.logged_in = true;
    s.username = username;
    s.auth_shop_id = Some(shop_id);

    if config.multi_shop_enabled {
        // Don't set active_shop_id/shop_selected yet -- that's what
        // marks a shop as chosen (see shops.rs). Send the frontend to
        // the picker instead of the dashboard, same as Python
        // redirecting to shops.select_shop.
        Ok(LoginResult { next: "pick-shop" })
    } else {
        // Single-shop mode: the login itself IS the shop, no picker
        // step, exactly as before multi-shop mode existed.
        s.active_shop_id = Some(shop_id);
        s.shop_name = shop_name;
        s.acnttype = acnttype;
        s.shop_type = shop_type.unwrap_or_else(|| "designer_apparels".into());
        s.shop_selected = true;
        Ok(LoginResult { next: "dashboard" })
    }
}

#[tauri::command]
pub fn logout(session: State<SessionState>) -> Result<(), String> {
    let mut s = session.0.lock().map_err(|e| e.to_string())?;
    s.clear();
    Ok(())
}

#[tauri::command]
pub fn get_session(session: State<SessionState>) -> Result<crate::session::Session, String> {
    let s = session.0.lock().map_err(|e| e.to_string())?;
    Ok(s.clone())
}
