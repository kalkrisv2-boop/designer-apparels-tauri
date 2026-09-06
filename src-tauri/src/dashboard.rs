//! dashboard.rs
//! -------------
//! Ported from `routes/dashboard.py`. For shop types other than
//! designer_apparels, the Python version deliberately shows a plain
//! welcome screen instead of Products/Customers/Suppliers summary cards
//! -- those verticals don't have real modules built yet (Phase 3-6),
//! so showing retail counts would just be misleading zeros. Same
//! behaviour here: `counts` is `None` unless shop_type is
//! designer_apparels.

use rusqlite::params;
use serde::Serialize;
use tauri::State;

use crate::platform_db::PlatformDb;
use crate::session::{check_login, LoginGuard, SessionState};
use crate::shops::SHOP_TYPES;

#[derive(Serialize)]
pub struct DashboardCounts {
    pub product_count: i64,
    pub customer_count: i64,
    pub supplier_count: i64,
}

#[derive(Serialize)]
pub struct DashboardData {
    pub shop_name: String,
    pub shop_type: String,
    pub shop_type_label: String,
    pub counts: Option<DashboardCounts>,
}

#[tauri::command]
pub fn get_dashboard(db: State<PlatformDb>, session: State<SessionState>) -> Result<DashboardData, String> {
    let s = session.0.lock().map_err(|e| e.to_string())?;
    match check_login(&s) {
        LoginGuard::NotLoggedIn => return Err("not_logged_in".into()),
        LoginGuard::NoShopSelected => return Err("no_shop_selected".into()),
        LoginGuard::Ok => {}
    }

    let shop_type_label = SHOP_TYPES
        .iter()
        .find(|(k, _)| *k == s.shop_type)
        .map(|(_, label)| label.to_string())
        .unwrap_or_else(|| s.shop_type.clone());

    if s.shop_type != "designer_apparels" {
        return Ok(DashboardData {
            shop_name: s.shop_name.clone(),
            shop_type: s.shop_type.clone(),
            shop_type_label,
            counts: None,
        });
    }

    let admin = s.active_shop_id.ok_or("no_shop_selected")?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let product_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM vm_products WHERE user_id = ?1 AND pr_isactive = 0",
            params![admin.to_string()],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let customer_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM vm_customer WHERE user_id = ?1 AND cs_isactive = 0",
            params![admin],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let supplier_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM vm_supplier WHERE user_id = ?1 AND rs_isactive = 0",
            params![admin.to_string()],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(DashboardData {
        shop_name: s.shop_name.clone(),
        shop_type: s.shop_type.clone(),
        shop_type_label,
        counts: Some(DashboardCounts {
            product_count,
            customer_count,
            supplier_count,
        }),
    })
}
