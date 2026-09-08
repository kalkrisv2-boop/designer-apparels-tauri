//! products.rs
//! ------------
//! Direct port of `routes/products.py` onto the platform schema
//! (`vm_products`, `PlatformDb`), following the same command shape as
//! `dashboard.rs`: check_login -> check_shop_type -> query, scoped by
//! `session.active_shop_id`.
//!
//! `user_id` on `vm_products` is TEXT in the ported schema (same
//! convention as the Python app), so every query here uses
//! `admin.to_string()` -- matching dashboard.rs's product/supplier
//! counts, not its (INTEGER) customer count.
//!
//! Soft delete via `pr_isactive` (0 = active, 1 = deleted), same
//! convention as the rest of this schema. Low-stock threshold (5) is
//! carried over unchanged from the Python version so behaviour matches
//! what the shop is already used to.
//!
//! `pr_cupsize` and `pr_description` (added via schema.rs migrations --
//! neither exists in the original Python schema) are a Rust-side-only
//! extension: this shop's products are identified by Model + Cupsize +
//! Size as three separate structured fields (e.g. "Sajna" + "A" + "32"
//! is a different product row from "Sajna" + "A" + "34" or "Sajna" +
//! "B" + "32"), not by a single compound size string. `pr_size` used to
//! hold a compound value like "A/32/12" -- it now holds only the
//! band-size portion ("32/12"); the leading cup letter moved to its own
//! `pr_cupsize` column so it can be a constrained select (A-E) in the
//! UI instead of free text buried inside pr_size.

use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::platform_db::PlatformDb;
use crate::session::{check_login, LoginGuard, SessionState};
use crate::shop_guard::check_shop_type;

pub const LOW_STOCK_THRESHOLD: f64 = 5.0;

const PRODUCT_COLUMNS: &str = "pr_productid, pr_productcode, pr_productname, pr_hsn,
    pr_purchaseprice, pr_saleprice, pr_wholesale, pr_stock,
    pr_unit, pr_model, pr_cupsize, pr_size, pr_description,
    pr_type, pr_pecentage, pr_barcode";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Product {
    pub pr_productid: i64,
    pub pr_productcode: String,
    pub pr_productname: String,
    pub pr_hsn: String,
    pub pr_purchaseprice: f64,
    pub pr_saleprice: f64,
    pub pr_wholesale: String,
    pub pr_stock: f64,
    pub pr_unit: String,
    pub pr_model: String,
    pub pr_cupsize: String,
    pub pr_size: String,
    pub pr_description: String,
    pub pr_type: String,
    pub pr_pecentage: String,
    pub pr_barcode: String,
    pub stock_status: String, // "ok" | "low" | "out" -- derived, not stored
    pub stock_value: f64,     // derived: pr_purchaseprice * pr_stock
}

fn product_from_row(row: &rusqlite::Row) -> rusqlite::Result<Product> {
    let stock: f64 = row.get(7)?;
    let purchaseprice: f64 = row.get(4)?;
    Ok(Product {
        pr_productid: row.get(0)?,
        pr_productcode: row.get(1)?,
        pr_productname: row.get(2)?,
        pr_hsn: row.get(3)?,
        pr_purchaseprice: purchaseprice,
        pr_saleprice: row.get(5)?,
        pr_wholesale: row.get(6)?,
        pr_stock: stock,
        pr_unit: row.get(8)?,
        pr_model: row.get(9)?,
        pr_cupsize: row.get(10).unwrap_or_default(),
        pr_size: row.get(11).unwrap_or_default(),
        pr_description: row.get(12).unwrap_or_default(),
        pr_type: row.get(13).unwrap_or_default(),
        pr_pecentage: row.get(14).unwrap_or_default(),
        pr_barcode: row.get(15).unwrap_or_default(),
        stock_status: stock_status(stock).to_string(),
        stock_value: (purchaseprice * stock * 100.0).round() / 100.0,
    })
}

#[derive(Debug, Deserialize)]
pub struct ProductInput {
    pub productcode: String,
    pub productname: String,
    pub hsn: String,
    pub purchaseprice: f64,
    pub saleprice: f64,
    pub wholesale: String,
    pub stock: f64,
    pub unit: String,
    pub model: String,
    #[serde(default)]
    pub cupsize: String,
    pub size: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub percentage: String,
    pub barcode: String,
}

#[derive(Debug, Serialize)]
pub struct ProductsSummary {
    pub products: Vec<Product>,
    pub total_count: i64,
    pub low_stock_count: i64,
    pub out_of_stock_count: i64,
    pub total_stock_value: f64,
    pub low_stock_threshold: f64,
}

fn stock_status(stock: f64) -> &'static str {
    if stock <= 0.0 {
        "out"
    } else if stock < LOW_STOCK_THRESHOLD {
        "low"
    } else {
        "ok"
    }
}

fn guard(session: &State<SessionState>) -> Result<i64, String> {
    let s = session.0.lock().map_err(|e| e.to_string())?;
    match check_login(&s) {
        LoginGuard::NotLoggedIn => return Err("not_logged_in".into()),
        LoginGuard::NoShopSelected => return Err("no_shop_selected".into()),
        LoginGuard::Ok => {}
    }
    check_shop_type("products", &s.shop_type)?;
    s.active_shop_id.ok_or_else(|| "no_shop_selected".to_string())
}

#[tauri::command]
pub fn list_products(db: State<PlatformDb>, session: State<SessionState>) -> Result<ProductsSummary, String> {
    let admin = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let sql = format!(
        "SELECT {PRODUCT_COLUMNS} FROM vm_products
         WHERE user_id = ?1 AND pr_isactive = 0
         ORDER BY pr_model ASC, pr_cupsize ASC, pr_size ASC"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![admin.to_string()], product_from_row)
        .map_err(|e| e.to_string())?;

    let products: Vec<Product> = rows.filter_map(|r| r.ok()).collect();
    let total_count = products.len() as i64;
    let low_stock_count = products.iter().filter(|p| p.stock_status == "low").count() as i64;
    let out_of_stock_count = products.iter().filter(|p| p.stock_status == "out").count() as i64;
    let total_stock_value = (products.iter().map(|p| p.stock_value).sum::<f64>() * 100.0).round() / 100.0;

    Ok(ProductsSummary {
        products,
        total_count,
        low_stock_count,
        out_of_stock_count,
        total_stock_value,
        low_stock_threshold: LOW_STOCK_THRESHOLD,
    })
}

#[tauri::command]
pub fn add_product(db: State<PlatformDb>, session: State<SessionState>, input: ProductInput) -> Result<i64, String> {
    let admin = guard(&session)?;
    if input.productcode.trim().is_empty() || input.productname.trim().is_empty() {
        return Err("Product code and name are required.".into());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let now = chrono_now();
    conn.execute(
        "INSERT INTO vm_products
            (pr_productcode, pr_productname, pr_hsn, pr_purchaseprice,
             pr_saleprice, pr_wholesale, pr_stock, pr_unit, pr_model,
             pr_cupsize, pr_size, pr_description, pr_type, pr_pecentage,
             pr_isactive, pr_updateddate, user_id, pr_retail, pr_barcode)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, 0, ?15, ?16, '', ?17)",
        params![
            input.productcode.trim(),
            input.productname.trim(),
            input.hsn.trim(),
            input.purchaseprice,
            input.saleprice,
            input.wholesale.trim(),
            input.stock,
            input.unit.trim(),
            input.model.trim(),
            input.cupsize.trim(),
            input.size.trim(),
            input.description.trim(),
            input.type_.trim(),
            input.percentage.trim(),
            now,
            admin.to_string(),
            input.barcode.trim(),
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn update_product(
    db: State<PlatformDb>,
    session: State<SessionState>,
    product_id: i64,
    input: ProductInput,
) -> Result<(), String> {
    let admin = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let now = chrono_now();
    conn.execute(
        "UPDATE vm_products
         SET pr_productcode = ?1, pr_productname = ?2, pr_hsn = ?3,
             pr_purchaseprice = ?4, pr_saleprice = ?5, pr_wholesale = ?6,
             pr_stock = ?7, pr_unit = ?8, pr_model = ?9, pr_cupsize = ?10,
             pr_size = ?11, pr_description = ?12, pr_type = ?13,
             pr_pecentage = ?14, pr_updateddate = ?15, pr_barcode = ?16
         WHERE pr_productid = ?17 AND user_id = ?18",
        params![
            input.productcode.trim(),
            input.productname.trim(),
            input.hsn.trim(),
            input.purchaseprice,
            input.saleprice,
            input.wholesale.trim(),
            input.stock,
            input.unit.trim(),
            input.model.trim(),
            input.cupsize.trim(),
            input.size.trim(),
            input.description.trim(),
            input.type_.trim(),
            input.percentage.trim(),
            now,
            input.barcode.trim(),
            product_id,
            admin.to_string(),
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_product(db: State<PlatformDb>, session: State<SessionState>, product_id: i64) -> Result<(), String> {
    let admin = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE vm_products SET pr_isactive = 1 WHERE pr_productid = ?1 AND user_id = ?2",
        params![product_id, admin.to_string()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Live product search for the billing POS search box -- active,
/// in-stock only, matches the Python `/billing/search-products` shape
/// (kept here rather than in billing.rs since it's a read against
/// vm_products, same home as the rest of this module's queries).
#[tauri::command]
pub fn search_products(db: State<PlatformDb>, session: State<SessionState>, q: String) -> Result<Vec<Product>, String> {
    let admin = guard(&session)?;
    if q.trim().is_empty() {
        return Ok(vec![]);
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let like = format!("%{}%", q.trim());
    let sql = format!(
        "SELECT {PRODUCT_COLUMNS} FROM vm_products
         WHERE user_id = ?1 AND pr_isactive = 0 AND pr_stock > 0
           AND (pr_productcode LIKE ?2 OR pr_productname LIKE ?2 OR pr_model LIKE ?2)
         ORDER BY pr_productname ASC LIMIT 15"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![admin.to_string(), like], product_from_row)
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Wholesale helper: every active, in-stock size variant sharing one
/// product's `pr_model` -- lets the cashier add a whole style's
/// size-ratio to the cart in one action. Direct port of
/// `/billing/style-variants`. Ordered by cupsize then size so, e.g.,
/// "Sajna" variants list as A32, A34, ..., B32, B34, ... rather than an
/// arbitrary size-only order that mixes cup sizes together.
#[tauri::command]
pub fn style_variants(db: State<PlatformDb>, session: State<SessionState>, model: String) -> Result<Vec<Product>, String> {
    let admin = guard(&session)?;
    if model.trim().is_empty() {
        return Ok(vec![]);
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let sql = format!(
        "SELECT {PRODUCT_COLUMNS} FROM vm_products
         WHERE user_id = ?1 AND pr_isactive = 0 AND pr_stock > 0 AND pr_model = ?2
         ORDER BY pr_cupsize ASC, pr_size ASC"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![admin.to_string(), model.trim()], product_from_row)
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// M1: quick-add a bare-minimum product mid-sale without leaving
/// Billing. Deliberately minimal (name, sale price, HSN, unit only) --
/// direct port of `routes/billing.py`'s `quick_add_product()`.
/// `pr_purchaseprice` is stored as 0 (caught later by profit-report
/// auditing, not a silent overstatement) and `pr_stock` is seeded at 1
/// (the exact amount "one unit being sold right now" needs, without
/// fabricating inventory) -- both intentional, not gaps. Model/Cupsize/
/// Size/Description are all left blank here -- quick-add is for a true
/// one-off, not a proper catalog entry; edit it on the Products screen
/// afterward if it turns out to be a recurring item.
#[derive(Debug, Deserialize)]
pub struct QuickAddInput {
    pub productname: String,
    pub hsn: String,
    pub unit: String,
    pub saleprice: f64,
}

#[tauri::command]
pub fn quick_add_product(db: State<PlatformDb>, session: State<SessionState>, input: QuickAddInput) -> Result<Product, String> {
    let admin = guard(&session)?;
    let name = input.productname.trim();
    if name.is_empty() {
        return Err("Product name is required.".into());
    }
    if input.saleprice <= 0.0 {
        return Err("Retail price must be greater than zero.".into());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let now = chrono_now();
    conn.execute(
        "INSERT INTO vm_products
            (pr_productcode, pr_productname, pr_hsn, pr_purchaseprice,
             pr_saleprice, pr_wholesale, pr_stock, pr_unit, pr_model,
             pr_cupsize, pr_size, pr_description, pr_type, pr_pecentage,
             pr_isactive, pr_updateddate, user_id, pr_retail, pr_barcode)
         VALUES ('', ?1, ?2, 0, ?3, '', 1, ?4, '', '', '', '', 0, '', 0, ?5, ?6, '', '')",
        params![name, input.hsn.trim(), input.saleprice, input.unit.trim(), now, admin.to_string()],
    )
    .map_err(|e| e.to_string())?;
    let product_id = conn.last_insert_rowid();
    // No product code in this minimal form -- backfill a short,
    // obviously-quick-add code keyed on the new row's own id, matching
    // Python's "QA-{id}" convention so it stays unique and still reads
    // sensibly everywhere "code — name" is displayed.
    let product_code = format!("QA-{}", product_id);
    conn.execute(
        "UPDATE vm_products SET pr_productcode = ?1 WHERE pr_productid = ?2 AND user_id = ?3",
        params![product_code, product_id, admin.to_string()],
    )
    .map_err(|e| e.to_string())?;

    Ok(Product {
        pr_productid: product_id,
        pr_productcode: product_code,
        pr_productname: name.to_string(),
        pr_hsn: input.hsn.trim().to_string(),
        pr_purchaseprice: 0.0,
        pr_saleprice: input.saleprice,
        pr_wholesale: String::new(),
        pr_stock: 1.0,
        pr_unit: input.unit.trim().to_string(),
        pr_model: String::new(),
        pr_cupsize: String::new(),
        pr_size: String::new(),
        pr_description: String::new(),
        pr_type: String::new(),
        pr_pecentage: String::new(),
        pr_barcode: String::new(),
        stock_status: "ok".to_string(),
        stock_value: 0.0,
    })
}

/// Matches Python's `datetime.now().strftime("%Y-%m-%d %H:%M:%S")` used
/// throughout `routes/*.py` for every `*_updateddate` audit column.
pub fn chrono_now() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}
