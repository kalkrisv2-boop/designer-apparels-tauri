//! shops.rs
//! ---------
//! Ported from `routes/shops.py` -- the "unified multi-shop" switcher:
//! one login, one database, multiple shop profiles the owner can switch
//! between without logging out. Only meaningful when
//! `config.multi_shop_enabled` is true; commands here still exist when
//! it's false, they're just never reached because `auth::login` sets
//! the shop immediately at login instead (same as Python: the routes
//! exist, but login_required never redirects to them in single-shop
//! mode).
//!
//! `SHOP_TYPES` is a direct port of the same dict in routes/shops.py --
//! the 5 business verticals. "designer_apparels" is the only one fully
//! built right now; the others get a dashboard welcome screen until
//! their Phase (3/4/5/6) lands.

use rusqlite::params;
use serde::Serialize;
use tauri::State;

use crate::config::AppConfig;
use crate::platform_db::PlatformDb;
use crate::session::SessionState;

pub const DEFAULT_SHOP_TYPE: &str = "designer_apparels";

pub const SHOP_TYPES: &[(&str, &str)] = &[
    ("designer_apparels", "Designer Apparels (Retail)"),
    ("staff_hr", "Staff / HR"),
    ("student_management", "Student Management"),
    ("garments", "Garments"),
    ("service", "Service"),
];

#[derive(Serialize)]
pub struct ShopRow {
    pub shop_id: i64,
    pub shop_name: String,
    pub shop_type: String,
}

fn guard_multi_shop(config: &AppConfig, session: &crate::session::Session) -> Result<(), String> {
    if !session.logged_in {
        return Err("not_logged_in".into());
    }
    if !config.multi_shop_enabled {
        return Err("multi_shop_disabled".into());
    }
    Ok(())
}

#[tauri::command]
pub fn list_shops(
    db: State<PlatformDb>,
    session: State<SessionState>,
    config: State<AppConfig>,
) -> Result<Vec<ShopRow>, String> {
    let s = session.0.lock().map_err(|e| e.to_string())?;
    guard_multi_shop(&config, &s)?;

    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT sp_shopid, sp_shopname, sp_shop_type
             FROM vm_shopprofile WHERE sp_isactive = 0 ORDER BY sp_shopname",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(ShopRow {
                shop_id: r.get(0)?,
                shop_name: r.get(1)?,
                shop_type: r.get::<_, Option<String>>(2)?.unwrap_or_else(|| DEFAULT_SHOP_TYPE.into()),
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
pub fn choose_shop(
    shop_id: i64,
    db: State<PlatformDb>,
    session: State<SessionState>,
    config: State<AppConfig>,
) -> Result<(), String> {
    {
        let s = session.0.lock().map_err(|e| e.to_string())?;
        guard_multi_shop(&config, &s)?;
    }

    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let row = conn
        .query_row(
            "SELECT sp_shopid, sp_shopname, sp_acnttype, sp_shop_type
             FROM vm_shopprofile WHERE sp_shopid = ?1 AND sp_isactive = 0",
            params![shop_id],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .map_err(|_| "Shop not found.".to_string())?;

    let mut s = session.0.lock().map_err(|e| e.to_string())?;
    s.active_shop_id = Some(row.0);
    s.shop_name = row.1;
    s.acnttype = row.2;
    s.shop_type = row.3.unwrap_or_else(|| DEFAULT_SHOP_TYPE.into());
    s.shop_selected = true;
    Ok(())
}

#[tauri::command]
pub fn add_shop(
    shop_name: String,
    shop_type: String,
    db: State<PlatformDb>,
    session: State<SessionState>,
    config: State<AppConfig>,
) -> Result<(), String> {
    {
        let s = session.0.lock().map_err(|e| e.to_string())?;
        guard_multi_shop(&config, &s)?;
    }

    let shop_name = shop_name.trim().to_string();
    if shop_name.is_empty() {
        return Err("Shop name is required.".into());
    }
    let shop_type = if SHOP_TYPES.iter().any(|(k, _)| *k == shop_type) {
        shop_type
    } else {
        DEFAULT_SHOP_TYPE.to_string()
    };

    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO vm_shopprofile
            (sp_shopname, sp_username, sp_password, sp_isactive,
             sp_acnttype, sp_adddate, sp_tin, sp_cst, sp_barcode, sp_accno,
             sp_shop_type)
         VALUES (?1, '', '', 0, 1, date('now'), '', '', '', '', ?2)",
        params![shop_name, shop_type],
    )
    .map_err(|e| e.to_string())?;
    let new_shop_id = conn.last_insert_rowid();

    let mut s = session.0.lock().map_err(|e| e.to_string())?;
    s.active_shop_id = Some(new_shop_id);
    s.shop_name = shop_name;
    s.acnttype = 1;
    s.shop_type = shop_type;
    s.shop_selected = true;
    Ok(())
}

/// Target for the sidebar's "Manage / Switch Shops" link -- clears the
/// active shop (but keeps logged_in, so no re-authentication needed)
/// and sends the frontend back to the picker.
#[tauri::command]
pub fn switch_shop(session: State<SessionState>) -> Result<(), String> {
    let mut s = session.0.lock().map_err(|e| e.to_string())?;
    if !s.logged_in {
        return Err("not_logged_in".into());
    }
    s.shop_selected = false;
    Ok(())
}
