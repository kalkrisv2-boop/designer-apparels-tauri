//! purchases.rs
//! ------------
//! Port of `routes/purchases.py` -- Phase 3 of the roadmap. Mirror image
//! of billing.rs: instead of a sale reducing stock, a purchase
//! *increases* it; instead of a sale return reducing stock further, a
//! purchase return *decreases* it.
//!
//! One real difference from billing.rs: on a *sale*, price is looked up
//! fresh from the database and never trusted from the caller (protects
//! the shop from a tampered bill). On a *purchase*, there is no
//! "authoritative" price to check against -- the purchase price IS the
//! data being entered, copied from the supplier's paper invoice. So the
//! entered price is trusted here (it's the shop owner's own data entry,
//! not money being taken from a customer), but every other safety check
//! (ownership, quantity, product existence) still applies, same as
//! billing.rs.
//!
//! Two separate tables, inherited from the original schema, matching
//! Python exactly -- these are NOT one table with a mode flag, unlike
//! sales:
//!     vm_purentry / vm_puritems             -- purchases (stock IN)
//!     vm_purreturnentry / vm_purreturnitem  -- purchase returns (stock OUT)
//! so this file has two mostly-parallel sets of commands, same as the
//! Python route file.
//!
//! Type gotcha (same as customers vs suppliers elsewhere in this app,
//! called out in Python's own module doc): vm_purentry/vm_puritems use
//! an INTEGER user_id column (bind `admin` directly), while
//! vm_purreturnentry/vm_purreturnitem use TEXT (bind `admin.to_string()`).
//! Mixing these up silently returns zero rows rather than erroring, so
//! it's worth double-checking any new query added to this file.
//!
//! Ledger/Daybook auto-posting on the *purchases* side mirrors
//! billing.rs's checkout exactly (paid portion -> Ledger + Daybook via
//! accounts::post_ledger_entry/post_daybook_entry inside this module's
//! own `tx`; unpaid portion -> vm_supplier.rs_balance, which INCREASES
//! since a credit purchase creates new debt -- opposite direction from a
//! Payment voucher settling existing debt, same reasoning as
//! billing::checkout, see accounts.rs's module doc).
//!
//! Asymmetry carried over faithfully from Python, NOT a bug fixed here:
//! `purchase_return_checkout` does NOT post to the Ledger/Daybook and
//! does NOT adjust the linked supplier's balance at all -- Python's
//! `routes/purchases.py::returns_checkout` only writes the return bill
//! and decreases stock. Whether that's an intentional simplification or
//! a gap in the original app wasn't something to guess at while
//! porting; flagging it here so it's visible before Reports (a future
//! phase) is built on top of these numbers.
//!
//! No schema migration needed this phase -- every column this file
//! touches already exists with the right type.

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::accounts::PartyOption;
use crate::billing::GST_RATES;
use crate::products::chrono_now;
use crate::session::{check_login, LoginGuard, SessionState};
use crate::shop_guard::check_shop_type;
use crate::platform_db::PlatformDb;

fn guard(session: &State<SessionState>) -> Result<(i64, String), String> {
    let s = session.0.lock().map_err(|e| e.to_string())?;
    match check_login(&s) {
        LoginGuard::NotLoggedIn => return Err("not_logged_in".into()),
        LoginGuard::NoShopSelected => return Err("no_shop_selected".into()),
        LoginGuard::Ok => {}
    }
    check_shop_type("purchases", &s.shop_type)?;
    let admin = s.active_shop_id.ok_or_else(|| "no_shop_selected".to_string())?;
    Ok((admin, s.username.clone()))
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[derive(Debug, Serialize)]
pub struct PurchaseProductOption {
    pub pr_productid: i64,
    pub pr_productcode: String,
    pub pr_productname: String,
    pub pr_purchaseprice: f64,
    pub pr_stock: f64,
    pub pr_unit: String,
    pub pr_hsn: String,
}

// ---------------------------------------------------------------------
// Purchases (stock IN) -- vm_purentry / vm_puritems
// ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct PurchaseRecent {
    pub pe_billid: i64,
    pub pe_billnumber: i64,
    pub pe_customername: String, // supplier name, stored under this legacy column name
    pub pe_invoice_number: String,
    pub pe_billdate: String,
    pub pe_gtotal: String,
}

#[derive(Debug, Serialize)]
pub struct PurchasesInit {
    pub next_bill_number: i64,
    pub gst_rates: Vec<f64>,
    pub suppliers: Vec<PartyOption>,
    pub recent: Vec<PurchaseRecent>,
}

#[tauri::command]
pub fn purchases_init(db: State<PlatformDb>, session: State<SessionState>) -> Result<PurchasesInit, String> {
    let (admin, _) = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    // vm_purentry.user_id is INTEGER -- bind admin directly (see module doc).
    let last_bill_number: Option<i64> = conn
        .query_row(
            "SELECT pe_billnumber FROM vm_purentry WHERE user_id = ?1 ORDER BY pe_billid DESC LIMIT 1",
            params![admin],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT rs_supplierid, rs_company_name FROM vm_supplier
             WHERE user_id = ?1 AND rs_isactive = 0 ORDER BY rs_company_name",
        )
        .map_err(|e| e.to_string())?;
    let suppliers = stmt
        .query_map(params![admin.to_string()], |row| {
            Ok(PartyOption { id: row.get(0)?, name: row.get(1)? })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut stmt = conn
        .prepare(
            "SELECT pe_billid, pe_billnumber, pe_customername, pe_invoice_number,
                    pe_billdate, pe_gtotal
             FROM vm_purentry WHERE user_id = ?1 AND pe_isactive = 0
             ORDER BY pe_billid DESC LIMIT 20",
        )
        .map_err(|e| e.to_string())?;
    let recent = stmt
        .query_map(params![admin], |row| {
            Ok(PurchaseRecent {
                pe_billid: row.get(0)?,
                pe_billnumber: row.get(1)?,
                pe_customername: row.get(2)?,
                pe_invoice_number: row.get(3).unwrap_or_default(),
                pe_billdate: row.get(4).unwrap_or_default(),
                pe_gtotal: row.get(5).unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(PurchasesInit {
        next_bill_number: last_bill_number.unwrap_or(0) + 1,
        gst_rates: GST_RATES.to_vec(),
        suppliers,
        recent,
    })
}

/// Same idea as billing's/products' product search, but deliberately
/// NOT filtered to in-stock items -- restocking an out-of-stock product
/// is exactly what this screen is for. Kept as its own command (not a
/// call into products::search_products) since the column set and the
/// missing stock filter both differ from that command's contract.
#[tauri::command]
pub fn purchases_search_products(db: State<PlatformDb>, session: State<SessionState>, q: String) -> Result<Vec<PurchaseProductOption>, String> {
    let (admin, _) = guard(&session)?;
    if q.trim().is_empty() {
        return Ok(vec![]);
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let like = format!("%{}%", q.trim());
    let mut stmt = conn
        .prepare(
            "SELECT pr_productid, pr_productcode, pr_productname, pr_purchaseprice,
                    pr_stock, pr_unit, pr_hsn
             FROM vm_products
             WHERE user_id = ?1 AND pr_isactive = 0
               AND (pr_productcode LIKE ?2 OR pr_productname LIKE ?2)
             ORDER BY pr_productname ASC LIMIT 15",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![admin.to_string(), like], |row| {
            Ok(PurchaseProductOption {
                pr_productid: row.get(0)?,
                pr_productcode: row.get(1).unwrap_or_default(),
                pr_productname: row.get(2)?,
                pr_purchaseprice: row.get(3).unwrap_or(0.0),
                pr_stock: row.get(4).unwrap_or(0.0),
                pr_unit: row.get(5).unwrap_or_default(),
                pr_hsn: row.get(6).unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[derive(Debug, Deserialize)]
pub struct PurchaseCartItemInput {
    pub product_id: i64,
    pub qty: f64,
    /// Trusted as entered -- see module doc on why purchases differ from sales here.
    pub price: f64,
    pub gst_pct: f64,
    #[serde(default)]
    pub hsn: String,
}

#[derive(Debug, Deserialize)]
pub struct PurchaseCheckoutInput {
    pub items: Vec<PurchaseCartItemInput>,
    /// "intra" | "inter" -- unlike billing.rs, purchases.py has no
    /// auto-derivation from a state code; the cashier's choice is
    /// trusted directly, matching Python exactly.
    #[serde(default = "default_gst_type")]
    pub gst_type: String,
    pub supplier_id: Option<i64>,
    #[serde(default)]
    pub supplier_name: String,
    #[serde(default)]
    pub invoice_number: String,
    #[serde(default)]
    pub invoice_date: String,
    #[serde(default)]
    pub vehicle_number: String,
    #[serde(default = "default_pay_method")]
    pub pay_method: String,
    #[serde(default)]
    pub overall_discount: f64,
    #[serde(default)]
    pub paid_amount: f64,
    #[serde(default)]
    pub note: String,
}
fn default_gst_type() -> String {
    "intra".to_string()
}
fn default_pay_method() -> String {
    "Cash".to_string()
}

#[derive(Debug, Serialize)]
pub struct PurchaseCheckoutResult {
    pub bill_id: i64,
    pub bill_number: i64,
    pub subtotal: f64,
    pub total_gst: f64,
    pub grand_total: f64,
    pub balance: f64,
}

struct PurLineCalc {
    product_id: i64,
    qty: f64,
    price: f64,
    gst_pct: f64,
    gst_amt: f64,
    taxable: f64,
    line_total: f64,
    hsn: String,
    cgst_pct: Option<f64>,
    cgst_amt: Option<f64>,
    sgst_pct: Option<f64>,
    sgst_amt: Option<f64>,
    igst_pct: Option<f64>,
    igst_amt: Option<f64>,
}

#[tauri::command]
pub fn purchases_checkout(db: State<PlatformDb>, session: State<SessionState>, input: PurchaseCheckoutInput) -> Result<PurchaseCheckoutResult, String> {
    let (admin, username) = guard(&session)?;

    if input.items.is_empty() {
        return Err("No items in this invoice.".into());
    }
    let supplier_name = input.supplier_name.trim().to_string();
    if supplier_name.is_empty() {
        return Err("Supplier is required.".into());
    }
    let gst_type = if input.gst_type == "inter" { "inter" } else { "intra" };

    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let mut line_calcs: Vec<PurLineCalc> = Vec::with_capacity(input.items.len());
    let mut subtotal = 0.0_f64;
    let mut total_gst = 0.0_f64;

    for raw in &input.items {
        if raw.qty <= 0.0 {
            return Err("Quantity must be greater than zero.".into());
        }
        if raw.price < 0.0 {
            return Err("Price cannot be negative.".into());
        }
        if !GST_RATES.contains(&raw.gst_pct) {
            return Err("Invalid GST rate.".into());
        }

        // vm_products.user_id is TEXT -- bind admin.to_string() (see module doc).
        let product = tx
            .query_row(
                "SELECT pr_productname, pr_hsn FROM vm_products
                 WHERE pr_productid = ?1 AND user_id = ?2 AND pr_isactive = 0",
                params![raw.product_id, admin.to_string()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Product #{} not found.", raw.product_id))?;
        let (_product_name, product_hsn) = product;

        let taxable = raw.price * raw.qty;
        let gst_amt = taxable * (raw.gst_pct / 100.0);
        let line_total = taxable + gst_amt;

        let (cgst_pct, cgst_amt, sgst_pct, sgst_amt, igst_pct, igst_amt) = if gst_type == "inter" {
            (None, None, None, None, Some(raw.gst_pct), Some(round2(gst_amt)))
        } else {
            (
                Some(raw.gst_pct / 2.0),
                Some(round2(gst_amt / 2.0)),
                Some(raw.gst_pct / 2.0),
                Some(round2(gst_amt / 2.0)),
                None,
                None,
            )
        };

        subtotal += taxable;
        total_gst += gst_amt;

        let hsn = if raw.hsn.trim().is_empty() { product_hsn.unwrap_or_default() } else { raw.hsn.trim().to_string() };

        line_calcs.push(PurLineCalc {
            product_id: raw.product_id,
            qty: raw.qty,
            price: raw.price,
            gst_pct: raw.gst_pct,
            gst_amt,
            taxable,
            line_total,
            hsn,
            cgst_pct,
            cgst_amt,
            sgst_pct,
            sgst_amt,
            igst_pct,
            igst_amt,
        });
    }

    let grand_total = round2(subtotal + total_gst - input.overall_discount);
    let balance = round2(grand_total - input.paid_amount);

    // Same reasoning as billing.rs: an unpaid balance has to be tracked
    // against a real supplier record, not just a free-text name.
    if balance > 0.001 && input.supplier_id.is_none() {
        return Err(
            "This invoice isn't fully paid. Please select a supplier from the list (not just typed text) so the remaining balance can be tracked on their account.".into(),
        );
    }

    // vm_purentry.user_id is INTEGER -- bind admin directly.
    let last_bill_number: Option<i64> = tx
        .query_row(
            "SELECT pe_billnumber FROM vm_purentry WHERE user_id = ?1 ORDER BY pe_billid DESC LIMIT 1",
            params![admin],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let bill_number = last_bill_number.unwrap_or(0) + 1;
    let now = chrono_now();

    let supplier_id_str = input.supplier_id.map(|s| s.to_string()).unwrap_or_default();

    tx.execute(
        "INSERT INTO vm_purentry
            (user_id, pe_billnumber, pe_customername, pe_billdate,
             pe_total, pe_gtotal, pe_oldbal, pe_paidamount, pe_paymethod,
             pe_note, pe_updateddate, pe_updatedby, pe_isactive,
             pe_discount, pe_mode, pe_paydate, pe_unitprice, pe_balance,
             pe_supplierid, pe_vehicle_number, pe_invoice_number,
             pe_invoice_date, pe_statecode, pe_debitid, pe_creditid,
             pe_purmode)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, '0', ?7, ?8, ?9, ?10, ?11, 0, ?12, 'purchase',
                 ?13, 0, ?14, ?15, ?16, ?17, ?18, ?19, '', 0, 0)",
        params![
            admin,
            bill_number,
            supplier_name,
            now,
            round2(subtotal),
            format!("{}", grand_total),
            input.paid_amount,
            input.pay_method.trim(),
            input.note.trim(),
            now,
            username,
            input.overall_discount,
            now,
            balance,
            supplier_id_str,
            input.vehicle_number.trim(),
            input.invoice_number.trim(),
            if input.invoice_date.trim().is_empty() { &now[..10] } else { input.invoice_date.trim() },
            gst_type,
        ],
    )
    .map_err(|e| e.to_string())?;
    let bill_id = tx.last_insert_rowid();

    for lc in &line_calcs {
        tx.execute(
            "INSERT INTO vm_puritems
                (user_id, pi_billid, pi_productid, pi_price, pi_quantity,
                 pi_total, pi_updatedon, pi_isactive, pi_vatamount,
                 pi_vatper, pi_unitprice, pi_sgst, pi_sgstamt, pi_cgst,
                 pi_cgstamt, pi_igst, pi_igstamt, pi_discount,
                 pi_taxamount, pi_prrate, pi_hsn, pi_billdate)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9, 0, ?10, ?11, ?12, ?13, ?14, ?15, '0', ?16, '', ?17, ?18)",
            params![
                admin,
                bill_id,
                lc.product_id,
                lc.price,
                lc.qty,
                round2(lc.taxable),
                now,
                round2(lc.gst_amt),
                lc.gst_pct,
                lc.sgst_pct,
                lc.sgst_amt,
                lc.cgst_pct,
                lc.cgst_amt,
                lc.igst_pct,
                lc.igst_amt,
                format!("{}", round2(lc.line_total)),
                lc.hsn,
                now,
            ],
        )
        .map_err(|e| e.to_string())?;

        // Restock: increase quantity, and record this invoice's price as
        // the product's new cost price / HSN going forward -- same
        // behaviour as Python (a purchase is the natural moment to
        // update what you're currently paying for a product). No stock
        // guard needed here (unlike a sale) -- stock only ever goes up.
        tx.execute(
            "UPDATE vm_products SET pr_stock = pr_stock + ?1, pr_purchaseprice = ?2, pr_hsn = ?3
             WHERE pr_productid = ?4 AND user_id = ?5",
            params![lc.qty, lc.price, lc.hsn, lc.product_id, admin.to_string()],
        )
        .map_err(|e| e.to_string())?;
    }

    // Auto-post to the Ledger/Daybook based on the payment split -- same
    // pattern as billing::checkout (see that file's doc for the fuller
    // explanation of why the balance direction is the opposite of a
    // manual Voucher).
    let bill_date = &now[..10];
    if input.paid_amount > 0.001 {
        let ledger_mode = if input.pay_method.trim() == "Cash" { "Cash" } else { "Bank" };
        let particulars = format!("Purchase Bill #{bill_number} -- {supplier_name}");
        crate::accounts::post_ledger_entry(
            &tx,
            admin,
            &particulars,
            -input.paid_amount,
            bill_date,
            "expense",
            ledger_mode,
            &supplier_id_str,
            &supplier_name,
        )?;
        crate::accounts::post_daybook_entry(&tx, admin, bill_date, "Purchase", ledger_mode, input.paid_amount, &particulars)?;
    }

    if balance > 0.001 {
        if let Some(sid) = input.supplier_id {
            tx.execute(
                "UPDATE vm_supplier SET rs_balance = CAST(rs_balance AS REAL) + ?1 WHERE rs_supplierid = ?2 AND user_id = ?3",
                params![balance, sid, admin.to_string()],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;

    Ok(PurchaseCheckoutResult {
        bill_id,
        bill_number,
        subtotal: round2(subtotal),
        total_gst: round2(total_gst),
        grand_total,
        balance,
    })
}

// ---------------------------------------------------------------------
// Purchase Returns (stock OUT, back to supplier) -- vm_purreturnentry / vm_purreturnitem
// ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct PurchaseReturnRecent {
    pub pre_billid: i64,
    pub pre_billnumber: i64,
    pub pre_customername: String, // supplier name, same legacy column naming as vm_purentry
    pub pre_invoice_number: String,
    pub pre_billdate: String,
    pub pre_gtotal: f64,
}

#[derive(Debug, Serialize)]
pub struct PurchaseReturnsInit {
    pub next_bill_number: i64,
    pub suppliers: Vec<PartyOption>,
    pub recent: Vec<PurchaseReturnRecent>,
}

#[tauri::command]
pub fn purchase_returns_init(db: State<PlatformDb>, session: State<SessionState>) -> Result<PurchaseReturnsInit, String> {
    let (admin, _) = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let admin_s = admin.to_string();

    // vm_purreturnentry.user_id is TEXT -- bind admin.to_string() (see module doc).
    let last_bill_number: Option<i64> = conn
        .query_row(
            "SELECT pre_billnumber FROM vm_purreturnentry WHERE user_id = ?1 ORDER BY pre_billid DESC LIMIT 1",
            params![admin_s],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT rs_supplierid, rs_company_name FROM vm_supplier
             WHERE user_id = ?1 AND rs_isactive = 0 ORDER BY rs_company_name",
        )
        .map_err(|e| e.to_string())?;
    let suppliers = stmt
        .query_map(params![admin_s], |row| Ok(PartyOption { id: row.get(0)?, name: row.get(1)? }))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut stmt = conn
        .prepare(
            "SELECT pre_billid, pre_billnumber, pre_customername, pre_invoice_number,
                    pre_billdate, pre_gtotal
             FROM vm_purreturnentry WHERE user_id = ?1 AND pre_isactive = 0
             ORDER BY pre_billid DESC LIMIT 20",
        )
        .map_err(|e| e.to_string())?;
    let recent = stmt
        .query_map(params![admin_s], |row| {
            Ok(PurchaseReturnRecent {
                pre_billid: row.get(0)?,
                pre_billnumber: row.get(1)?,
                pre_customername: row.get(2)?,
                pre_invoice_number: row.get(3).unwrap_or_default(),
                pre_billdate: row.get(4).unwrap_or_default(),
                pre_gtotal: row.get(5).unwrap_or(0.0),
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(PurchaseReturnsInit { next_bill_number: last_bill_number.unwrap_or(0) + 1, suppliers, recent })
}

/// Filtered to items currently in stock -- you can only return what you
/// actually have. Same column set as purchases_search_products.
#[tauri::command]
pub fn purchases_search_products_for_return(db: State<PlatformDb>, session: State<SessionState>, q: String) -> Result<Vec<PurchaseProductOption>, String> {
    let (admin, _) = guard(&session)?;
    if q.trim().is_empty() {
        return Ok(vec![]);
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let like = format!("%{}%", q.trim());
    let mut stmt = conn
        .prepare(
            "SELECT pr_productid, pr_productcode, pr_productname, pr_purchaseprice,
                    pr_stock, pr_unit, pr_hsn
             FROM vm_products
             WHERE user_id = ?1 AND pr_isactive = 0 AND pr_stock > 0
               AND (pr_productcode LIKE ?2 OR pr_productname LIKE ?2)
             ORDER BY pr_productname ASC LIMIT 15",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![admin.to_string(), like], |row| {
            Ok(PurchaseProductOption {
                pr_productid: row.get(0)?,
                pr_productcode: row.get(1).unwrap_or_default(),
                pr_productname: row.get(2)?,
                pr_purchaseprice: row.get(3).unwrap_or(0.0),
                pr_stock: row.get(4).unwrap_or(0.0),
                pr_unit: row.get(5).unwrap_or_default(),
                pr_hsn: row.get(6).unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[derive(Debug, Deserialize)]
pub struct PurchaseReturnCartItemInput {
    pub product_id: i64,
    pub qty: f64,
    pub price: f64,
    pub gst_pct: f64,
}

#[derive(Debug, Deserialize)]
pub struct PurchaseReturnCheckoutInput {
    pub items: Vec<PurchaseReturnCartItemInput>,
    #[serde(default = "default_gst_type")]
    pub gst_type: String,
    #[serde(default)]
    pub supplier_name: String,
    #[serde(default)]
    pub supplier_id: i64,
    #[serde(default)]
    pub invoice_number: String,
    #[serde(default)]
    pub invoice_date: String,
    #[serde(default)]
    pub vehicle_number: String,
    #[serde(default = "default_pay_method")]
    pub pay_method: String,
    #[serde(default)]
    pub overall_discount: f64,
    #[serde(default)]
    pub paid_amount: f64,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub original_bill_number: String,
}

struct PurReturnLineCalc {
    product_id: i64,
    product_name: String,
    qty: f64,
    price: f64,
    gst_pct: f64,
    gst_amt: f64,
    line_total: f64,
    cgst_pct: Option<f64>,
    cgst_amt: Option<f64>,
    sgst_pct: Option<f64>,
    sgst_amt: Option<f64>,
    igst_pct: Option<f64>,
    igst_amt: Option<f64>,
}

#[tauri::command]
pub fn purchase_returns_checkout(db: State<PlatformDb>, session: State<SessionState>, input: PurchaseReturnCheckoutInput) -> Result<PurchaseCheckoutResult, String> {
    let (admin, username) = guard(&session)?;

    if input.items.is_empty() {
        return Err("No items in this return.".into());
    }
    let supplier_name = input.supplier_name.trim().to_string();
    if supplier_name.is_empty() {
        return Err("Supplier is required.".into());
    }
    let gst_type = if input.gst_type == "inter" { "inter" } else { "intra" };

    // vm_purreturnentry/item + vm_products all use TEXT user_id on this
    // side (see module doc) -- one string used throughout, matching
    // Python's `user_id = str(session["admin"])`.
    let user_id = admin.to_string();

    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let mut line_calcs: Vec<PurReturnLineCalc> = Vec::with_capacity(input.items.len());
    let mut subtotal = 0.0_f64;
    let mut total_gst = 0.0_f64;

    for raw in &input.items {
        if raw.qty <= 0.0 {
            return Err("Quantity must be greater than zero.".into());
        }
        if !GST_RATES.contains(&raw.gst_pct) {
            return Err("Invalid GST rate.".into());
        }

        let product = tx
            .query_row(
                "SELECT pr_productname, pr_stock FROM vm_products
                 WHERE pr_productid = ?1 AND user_id = ?2 AND pr_isactive = 0",
                params![raw.product_id, user_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Product #{} not found.", raw.product_id))?;
        let (product_name, stock) = product;

        if stock < raw.qty {
            return Err(format!("Cannot return {} of '{}' -- only {} in stock.", raw.qty, product_name, stock));
        }

        let taxable = raw.price * raw.qty;
        let gst_amt = taxable * (raw.gst_pct / 100.0);
        let line_total = taxable + gst_amt;

        let (cgst_pct, cgst_amt, sgst_pct, sgst_amt, igst_pct, igst_amt) = if gst_type == "inter" {
            (None, None, None, None, Some(raw.gst_pct), Some(round2(gst_amt)))
        } else {
            (
                Some(raw.gst_pct / 2.0),
                Some(round2(gst_amt / 2.0)),
                Some(raw.gst_pct / 2.0),
                Some(round2(gst_amt / 2.0)),
                None,
                None,
            )
        };

        subtotal += taxable;
        total_gst += gst_amt;

        line_calcs.push(PurReturnLineCalc {
            product_id: raw.product_id,
            product_name,
            qty: raw.qty,
            price: raw.price,
            gst_pct: raw.gst_pct,
            gst_amt,
            line_total,
            cgst_pct,
            cgst_amt,
            sgst_pct,
            sgst_amt,
            igst_pct,
            igst_amt,
        });
    }

    let grand_total = round2(subtotal + total_gst - input.overall_discount);
    let balance = round2(grand_total - input.paid_amount);

    let last_bill_number: Option<i64> = tx
        .query_row(
            "SELECT pre_billnumber FROM vm_purreturnentry WHERE user_id = ?1 ORDER BY pre_billid DESC LIMIT 1",
            params![user_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let bill_number = last_bill_number.unwrap_or(0) + 1;
    let now = chrono_now();

    tx.execute(
        "INSERT INTO vm_purreturnentry
            (user_id, pre_billnumber, pre_customername, pre_customermobile,
             pre_customer_tin_num, pre_vehicle_number, pre_billdate,
             pre_total, pre_gtotal, pre_paidamount, pre_paymethod,
             pre_note, pre_updateddate, pre_updatedby, pre_isactive,
             pre_discount, pre_mode, pre_paydate, pre_balance,
             pre_customerid, pre_coolie, pre_oldbal, pre_totvat,
             pre_invoice_number, pre_invoice_date, pre_rebill,
             pre_statecode, pre_debitid)
         VALUES (?1, ?2, ?3, '', '', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0, ?13,
                 'purchasereturn', ?14, ?15, ?16, '', '0', ?17, ?18, ?19, ?20, ?21, '')",
        params![
            user_id,
            bill_number,
            supplier_name,
            input.vehicle_number.trim(),
            now,
            round2(subtotal),
            grand_total,
            input.paid_amount,
            input.pay_method.trim(),
            input.note.trim(),
            now,
            username,
            input.overall_discount,
            now,
            balance as i64,
            input.supplier_id,
            round2(total_gst),
            input.invoice_number.trim(),
            if input.invoice_date.trim().is_empty() { &now[..10] } else { input.invoice_date.trim() },
            input.original_bill_number.trim(),
            gst_type,
        ],
    )
    .map_err(|e| e.to_string())?;
    let bill_id = tx.last_insert_rowid();

    for lc in &line_calcs {
        tx.execute(
            "INSERT INTO vm_purreturnitem
                (user_id, pri_billid, pri_returnbillid, pri_productid,
                 pri_price, pri_quantity, pri_total, pri_updatedon,
                 pri_isactive, pri_vatamount, pri_vatper, pri_coolie,
                 pri_sgst, pri_sgstamt, pri_cgst, pr_cgstamt,
                 pri_igst, pri_igstamt, pri_billdate)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9, ?10, 0, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                user_id,
                bill_id,
                if input.original_bill_number.trim().is_empty() { "0" } else { input.original_bill_number.trim() },
                lc.product_id,
                format!("{}", lc.price),
                lc.qty,
                round2(lc.line_total),
                now,
                round2(lc.gst_amt),
                lc.gst_pct,
                lc.sgst_pct,
                lc.sgst_amt,
                lc.cgst_pct,
                lc.cgst_amt,
                lc.igst_pct,
                lc.igst_amt,
                now,
            ],
        )
        .map_err(|e| e.to_string())?;

        // Stock leaves the shop and goes back to the supplier. The
        // pr_stock >= qty guard is a second safety net, same pattern as
        // billing's sale-time stock deduction and purchases_checkout's
        // mirror-image restock.
        let updated = tx
            .execute(
                "UPDATE vm_products SET pr_stock = pr_stock - ?1
                 WHERE pr_productid = ?2 AND user_id = ?3 AND pr_stock >= ?1",
                params![lc.qty, lc.product_id, user_id],
            )
            .map_err(|e| e.to_string())?;
        if updated == 0 {
            return Err(format!("Stock for '{}' changed before this return finished -- please review and try again.", lc.product_name));
        }
    }

    // NOTE: no Ledger/Daybook posting and no supplier balance adjustment
    // here -- matches Python's returns_checkout exactly (see module doc).

    tx.commit().map_err(|e| e.to_string())?;

    Ok(PurchaseCheckoutResult {
        bill_id,
        bill_number,
        subtotal: round2(subtotal),
        total_gst: round2(total_gst),
        grand_total,
        balance,
    })
}
