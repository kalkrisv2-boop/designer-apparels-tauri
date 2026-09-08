//! schema.rs
//! ----------
//! Full DB schema port (Phase 0, roadmap §3).
//!
//! `../schema.sql` is a byte-for-byte copy of the Python app's
//! `schema.sql` -- table and column names preserved exactly (see roadmap
//! §3 Phase 0: "preserving table/column names where practical so any
//! future data-import tooling from the Python version stays simple").
//! It was already valid SQLite DDL in the Python app (that app used
//! SQLite too, just via Python's `sqlite3` instead of `rusqlite`), so
//! porting it here is a straight embed-and-execute, not a rewrite.
//!
//! `SCHEMA_MIGRATIONS` is a direct port of `database.py`'s
//! `SCHEMA_MIGRATIONS` list: additive (table, column, type+default)
//! entries for columns added to a table after that table already shipped
//! to existing installs. `CREATE TABLE IF NOT EXISTS` (in schema.sql)
//! only helps brand-new tables -- it does nothing for a table that
//! already exists on someone's machine but is missing a column a newer
//! build needs. This list runs on every startup; it's a no-op (checked
//! via `PRAGMA table_info`) for any column already present, so it's
//! always safe to leave old entries here and just append new ones.

use anyhow::Result;
use rusqlite::Connection;

const SCHEMA_SQL: &str = include_str!("../schema.sql");

/// (table, column, SQLite type + default) -- kept in the exact order
/// ported from database.py so the migration history stays legible.
const SCHEMA_MIGRATIONS: &[(&str, &str, &str)] = &[
    ("vm_products", "pr_barcode", "TEXT DEFAULT ''"),
    ("vm_shopprofile", "sp_shop_type", "TEXT DEFAULT 'designer_apparels'"),
    ("vm_billentry", "be_billmode", "TEXT DEFAULT 'retail'"),
    ("vm_billentry", "be_transport_name", "TEXT DEFAULT ''"),
    ("vm_billentry", "be_lr_number", "TEXT DEFAULT ''"),
    ("vm_billentry", "be_lr_date", "TEXT DEFAULT ''"),
    ("vm_billentry", "be_parcels", "TEXT DEFAULT ''"),
    ("vm_billentry", "be_salesman", "TEXT DEFAULT ''"),
    ("vm_billentry", "be_booking", "TEXT DEFAULT ''"),
    ("vm_billentry", "be_customer_pan", "TEXT DEFAULT ''"),
    ("vm_customer", "cs_gstin", "TEXT DEFAULT ''"),
    ("vm_customer", "cs_statecode", "TEXT DEFAULT ''"),
    ("vm_billitems", "bi_style_snapshot", "TEXT DEFAULT ''"),
    ("vm_staffattendanceentry", "user_id", "TEXT DEFAULT ''"),
    ("vm_staffattendanceitems", "user_id", "TEXT DEFAULT ''"),
    ("vm_attendanceentry", "user_id", "TEXT DEFAULT ''"),
    ("vm_attendanceitems", "user_id", "TEXT DEFAULT ''"),
    ("vm_shopprofile", "sp_tagline", "TEXT DEFAULT ''"),
    ("vm_shopprofile", "sp_terms", "TEXT DEFAULT ''"),
    ("vm_products", "pr_cupsize", "TEXT DEFAULT ''"),
    ("vm_products", "pr_description", "TEXT DEFAULT ''"),
    // Phase 2 (Accounts): per-shop chart of accounts. Python's
    // administrator_account_name was shared across all installs (each
    // shop had its own separate app install, so no column was needed);
    // confirmed decision for the multi-shop platform is each shop gets
    // its own list, so this mirrors vm_transaction.user_id (INTEGER),
    // not the table's existing-but-unused `acnt_branch` TEXT column.
    ("administrator_account_name", "user_id", "INTEGER NOT NULL DEFAULT 0"),
    // Append new (table, column, type+default) tuples here as later
    // phases add columns to tables that already shipped -- never edit
    // or remove an existing entry, that would break upgrades for shops
    // already running an earlier build.
];

fn run_migrations(conn: &Connection) -> Result<()> {
    for (table, column, coltype) in SCHEMA_MIGRATIONS {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info(`{table}`)"))?;
        let existing: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .filter_map(|r| r.ok())
            .collect();

        if !existing.iter().any(|c| c == column) {
            conn.execute(
                &format!("ALTER TABLE `{table}` ADD COLUMN `{column}` {coltype}"),
                [],
            )?;
        }
    }
    Ok(())
}

/// Creates all 35 tables if they don't exist yet, then applies any
/// pending additive migrations. Safe to call on every startup.
pub fn init_full_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(SCHEMA_SQL)?;
    run_migrations(conn)?;
    Ok(())
}
