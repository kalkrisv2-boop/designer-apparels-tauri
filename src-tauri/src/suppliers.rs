//! suppliers.rs
//! -------------
//! Direct port of `routes/suppliers.py` onto `vm_supplier`.
//!
//! `vm_supplier.user_id` is TEXT (same convention as vm_products) --
//! every query binds `admin.to_string()`, matching products.rs, not
//! customers.rs.

use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::platform_db::PlatformDb;
use crate::session::{check_login, LoginGuard, SessionState};
use crate::shop_guard::check_shop_type;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Supplier {
    pub rs_supplierid: i64,
    pub rs_company_name: String,
    pub rs_name: String,
    pub rs_phone: String,
    pub rs_mobile: String,
    pub rs_address: String,
    pub rs_email: String,
    pub rs_balance: String,
    pub rs_tinnum: String,
}

#[derive(Debug, Deserialize)]
pub struct SupplierInput {
    pub company_name: String,
    pub contact_name: String,
    pub phone: String,
    pub mobile: String,
    pub address: String,
    pub email: String,
    pub balance: String,
    pub tin: String,
}

fn guard(session: &State<SessionState>) -> Result<i64, String> {
    let s = session.0.lock().map_err(|e| e.to_string())?;
    match check_login(&s) {
        LoginGuard::NotLoggedIn => return Err("not_logged_in".into()),
        LoginGuard::NoShopSelected => return Err("no_shop_selected".into()),
        LoginGuard::Ok => {}
    }
    check_shop_type("suppliers", &s.shop_type)?;
    s.active_shop_id.ok_or_else(|| "no_shop_selected".to_string())
}

#[tauri::command]
pub fn list_suppliers(db: State<PlatformDb>, session: State<SessionState>) -> Result<Vec<Supplier>, String> {
    let admin = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT rs_supplierid, rs_company_name, rs_name, rs_phone, rs_mobile,
                    rs_address, rs_email, rs_balance, rs_tinnum
             FROM vm_supplier WHERE user_id = ?1 AND rs_isactive = 0
             ORDER BY rs_company_name ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![admin.to_string()], |row| {
            Ok(Supplier {
                rs_supplierid: row.get(0)?,
                rs_company_name: row.get(1)?,
                rs_name: row.get(2).unwrap_or_default(),
                rs_phone: row.get(3).unwrap_or_default(),
                rs_mobile: row.get(4).unwrap_or_default(),
                rs_address: row.get(5).unwrap_or_default(),
                rs_email: row.get(6).unwrap_or_default(),
                rs_balance: row.get(7).unwrap_or_default(),
                rs_tinnum: row.get(8).unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
pub fn add_supplier(db: State<PlatformDb>, session: State<SessionState>, input: SupplierInput) -> Result<i64, String> {
    let admin = guard(&session)?;
    if input.company_name.trim().is_empty() {
        return Err("Company name is required.".into());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let balance = if input.balance.trim().is_empty() { "0".to_string() } else { input.balance.trim().to_string() };
    conn.execute(
        "INSERT INTO vm_supplier
            (rs_company_name, rs_name, rs_phone, rs_mobile, rs_address,
             rs_email, rs_balance, rs_isactive, rs_tinnum, user_id,
             rs_acntid, rs_statecode)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9, '', '')",
        params![
            input.company_name.trim(),
            input.contact_name.trim(),
            input.phone.trim(),
            input.mobile.trim(),
            input.address.trim(),
            input.email.trim(),
            balance,
            input.tin.trim(),
            admin.to_string(),
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn update_supplier(
    db: State<PlatformDb>,
    session: State<SessionState>,
    supplier_id: i64,
    input: SupplierInput,
) -> Result<(), String> {
    let admin = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let balance = if input.balance.trim().is_empty() { "0".to_string() } else { input.balance.trim().to_string() };
    conn.execute(
        "UPDATE vm_supplier
         SET rs_company_name = ?1, rs_name = ?2, rs_phone = ?3, rs_mobile = ?4,
             rs_address = ?5, rs_email = ?6, rs_balance = ?7, rs_tinnum = ?8
         WHERE rs_supplierid = ?9 AND user_id = ?10",
        params![
            input.company_name.trim(),
            input.contact_name.trim(),
            input.phone.trim(),
            input.mobile.trim(),
            input.address.trim(),
            input.email.trim(),
            balance,
            input.tin.trim(),
            supplier_id,
            admin.to_string(),
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_supplier(db: State<PlatformDb>, session: State<SessionState>, supplier_id: i64) -> Result<(), String> {
    let admin = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE vm_supplier SET rs_isactive = 1 WHERE rs_supplierid = ?1 AND user_id = ?2",
        params![supplier_id, admin.to_string()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
