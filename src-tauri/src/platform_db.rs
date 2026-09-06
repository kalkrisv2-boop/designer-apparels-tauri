//! platform_db.rs
//! ----------------
//! Connection handling for the ported 35-table ERP schema (schema.rs),
//! kept separate from `db.rs` (the existing standalone invoicing tool's
//! `invoices.db`) for Phase 0.
//!
//! Why two databases for now, on purpose: the existing invoicing app
//! already has real, working billing/PDF/GST logic against its own
//! `invoices.db` schema (see db.rs / models.rs). The roadmap (§2) is
//! explicit that reconciling that already-built logic with the ported
//! ERP schema's `vm_billentry`/`vm_billitems` tables is Phase 1 work,
//! not Phase 0 -- Phase 0 is foundations (config, full schema port,
//! licensing, login, multi-shop switcher, dashboard shell, the
//! shop_type guard). Standing up the platform schema alongside the
//! existing invoicing db, rather than replacing it out from under
//! already-working code, keeps that reconciliation an explicit Phase 1
//! decision instead of an accidental Phase 0 side effect.
//!
//! The platform db's filename comes from AppConfig (`db_filename`,
//! default "designer.db" -- see config.rs), resolved via
//! `paths::persistent_path`, so it lives next to the .exe alongside
//! config.json/license.json, matching the Python app's "one folder per
//! client" distribution model.

use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

use crate::config::AppConfig;
use crate::paths::persistent_path;
use crate::schema;

/// Tauri-managed wrapper, same pattern as the existing invoicing app's
/// `Db(pub Mutex<Connection>)` in db.rs.
pub struct PlatformDb(pub Mutex<Connection>);

pub fn open(cfg: &AppConfig) -> Result<Connection> {
    let db_path = persistent_path(&cfg.db_filename);
    let is_new = !Path::new(&db_path).exists();

    let conn = Connection::open(&db_path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    schema::init_full_schema(&conn)?;

    if is_new {
        seed_default_admin(&conn, cfg)?;
    }

    Ok(conn)
}

/// Seeds one working login so the app isn't a locked box on first run --
/// direct port of Python's `_create_default_admin`. Same default
/// credentials (admin / admin123); same "shop profile row IS the login
/// account" model as the Python version.
fn seed_default_admin(conn: &Connection, cfg: &AppConfig) -> Result<()> {
    let hash = bcrypt::hash("admin123", bcrypt::DEFAULT_COST)?;
    conn.execute(
        "INSERT INTO vm_shopprofile
            (sp_shopname, sp_username, sp_password, sp_isactive,
             sp_acnttype, sp_adddate, sp_tin, sp_cst, sp_barcode, sp_accno)
         VALUES (?1, 'admin', ?2, 0, 1, date('now'), '', '', '', '')",
        rusqlite::params![cfg.shop_display_name, hash],
    )?;
    Ok(())
}
