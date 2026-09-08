//! customers.rs
//! -------------
//! Direct port of `routes/customers.py` onto `vm_customer`.
//!
//! Unlike products/suppliers, `vm_customer.user_id` is INTEGER in the
//! ported schema (see schema.sql / dashboard.rs's customer_count query)
//! -- so every query here binds `admin` directly, no `.to_string()`.
//! Getting this wrong is exactly the kind of TEXT/INTEGER mismatch bug
//! the original Python file's own docstring warned about.

use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::platform_db::PlatformDb;
use crate::products::chrono_now;
use crate::session::{check_login, LoginGuard, SessionState};
use crate::shop_guard::check_shop_type;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Customer {
    pub cs_customerid: i64,
    pub cs_customername: String,
    pub cs_customerphone: String,
    pub cs_address: String,
    pub cs_email: String,
    pub cs_tin_number: String,
    pub cs_balance: f64,
}

#[derive(Debug, Deserialize)]
pub struct CustomerInput {
    pub customername: String,
    pub phone: String,
    pub address: String,
    pub email: String,
    pub tin: String,
    pub balance: f64,
}

fn guard(session: &State<SessionState>) -> Result<i64, String> {
    let s = session.0.lock().map_err(|e| e.to_string())?;
    match check_login(&s) {
        LoginGuard::NotLoggedIn => return Err("not_logged_in".into()),
        LoginGuard::NoShopSelected => return Err("no_shop_selected".into()),
        LoginGuard::Ok => {}
    }
    check_shop_type("customers", &s.shop_type)?;
    s.active_shop_id.ok_or_else(|| "no_shop_selected".to_string())
}

#[tauri::command]
pub fn list_customers(db: State<PlatformDb>, session: State<SessionState>) -> Result<Vec<Customer>, String> {
    let admin = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT cs_customerid, cs_customername, cs_customerphone, cs_address,
                    cs_email, cs_tin_number, cs_balance
             FROM vm_customer WHERE user_id = ?1 AND cs_isactive = 0
             ORDER BY cs_customername ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![admin], |row| {
            Ok(Customer {
                cs_customerid: row.get(0)?,
                cs_customername: row.get(1)?,
                cs_customerphone: row.get(2).unwrap_or_default(),
                cs_address: row.get(3).unwrap_or_default(),
                cs_email: row.get(4).unwrap_or_default(),
                cs_tin_number: row.get(5).unwrap_or_default(),
                cs_balance: row.get(6).unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
pub fn add_customer(db: State<PlatformDb>, session: State<SessionState>, input: CustomerInput) -> Result<i64, String> {
    let admin = guard(&session)?;
    if input.customername.trim().is_empty() {
        return Err("Customer name is required.".into());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let now = chrono_now();
    conn.execute(
        "INSERT INTO vm_customer
            (user_id, cs_customername, cs_customerphone, cs_address,
             cs_email, cs_tin_number, cs_balance, cs_isactive,
             cs_updateddate, cs_updatedby, cs_acntid, cs_statecode)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9, '', '')",
        params![
            admin,
            input.customername.trim(),
            input.phone.trim(),
            input.address.trim(),
            input.email.trim(),
            input.tin.trim(),
            input.balance,
            now,
            "", // cs_updatedby -- filled from session username where available
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn update_customer(
    db: State<PlatformDb>,
    session: State<SessionState>,
    customer_id: i64,
    input: CustomerInput,
) -> Result<(), String> {
    let admin = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let now = chrono_now();
    conn.execute(
        "UPDATE vm_customer
         SET cs_customername = ?1, cs_customerphone = ?2, cs_address = ?3,
             cs_email = ?4, cs_tin_number = ?5, cs_balance = ?6,
             cs_updateddate = ?7
         WHERE cs_customerid = ?8 AND user_id = ?9",
        params![
            input.customername.trim(),
            input.phone.trim(),
            input.address.trim(),
            input.email.trim(),
            input.tin.trim(),
            input.balance,
            now,
            customer_id,
            admin,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_customer(db: State<PlatformDb>, session: State<SessionState>, customer_id: i64) -> Result<(), String> {
    let admin = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE vm_customer SET cs_isactive = 1 WHERE cs_customerid = ?1 AND user_id = ?2",
        params![customer_id, admin],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
