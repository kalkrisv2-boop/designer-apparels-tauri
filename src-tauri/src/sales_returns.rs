//! sales_returns.rs
//! -----------------
//! Port of `routes/sales_returns.py` -- Phase 4 of the roadmap. Mirrors
//! purchases.rs's Purchase Returns section, which is itself the mirror
//! image of billing.rs: instead of a sale reducing stock, a sale return
//! can *increase* it again -- but only when the goods are actually fit
//! to go back on the shelf.
//!
//! Two things Purchase Returns didn't need that Sales Returns does:
//!
//! 1. Restock choice, per line. A customer return might be resaleable
//!    (put it back in stock) or not (damaged, wrong size cut, whatever
//!    -- write it off). There's no purpose-built column for this flag on
//!    vm_salreturnitem, so Python repurposes `sri_unitprice` (REAL,
//!    present in the base schema, unused anywhere else) to store it:
//!    1.0 = restocked, 0.0 = not. Carried over exactly as-is -- flagging
//!    again here in case a properly-named column is ever preferred.
//!
//! 2. Ledger auto-posting and customer-balance adjustment on save.
//!    Unlike purchases.rs's `purchase_return_checkout` (which
//!    deliberately does neither, see that file's module doc), Sales
//!    Returns DOES post to the Ledger/Daybook and adjust the customer's
//!    balance -- same post_ledger_entry/post_daybook_entry/balance-
//!    update shape billing.rs already uses for sales, just pointed in
//!    the refund/credit direction instead of receipt/debt. This is a
//!    real asymmetry between the two Returns flows, confirmed against
//!    Python (not something to "fix" into consistency without being
//!    asked -- see purchases.rs's own module doc on this).
//!
//! Type gotcha (same shape as elsewhere in this app): vm_salreturnentry
//! and vm_salreturnitem use an INTEGER user_id column (bind `admin`
//! directly), unlike vm_billentry/vm_billitems which use TEXT (bind
//! `admin.to_string()`). vm_products also uses TEXT here.
//!
//! Column-naming quirk inherited from the schema (same shape as Purchase
//! Returns' `pre_customerid` actually holding the *supplier* id): here,
//! `sre_supplierid` is where the *customer* id is stored. Not renaming
//! it -- that's a schema change out of scope -- just documented inline
//! everywhere it's used, matching Python's own comment.
//!
//! Product lookups in this file (search AND checkout) are deliberately
//! NOT filtered to `pr_isactive = 0`, unlike every other product query
//! in this app -- a product discontinued since the original sale must
//! still be returnable. Matches Python exactly; don't "fix" this to be
//! consistent with products.rs's usual filter.
//!
//! No schema migration needed this phase.

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::billing::GST_RATES;
use crate::platform_db::PlatformDb;
use crate::products::chrono_now;
use crate::session::{check_login, LoginGuard, SessionState};
use crate::shop_guard::check_shop_type;

fn guard(session: &State<SessionState>) -> Result<(i64, String), String> {
    let s = session.0.lock().map_err(|e| e.to_string())?;
    match check_login(&s) {
        LoginGuard::NotLoggedIn => return Err("not_logged_in".into()),
        LoginGuard::NoShopSelected => return Err("no_shop_selected".into()),
        LoginGuard::Ok => {}
    }
    check_shop_type("sales_returns", &s.shop_type)?;
    let admin = s.active_shop_id.ok_or_else(|| "no_shop_selected".to_string())?;
    Ok((admin, s.username.clone()))
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

// ---------------------------------------------------------------------
// Init (screen load) -- vm_salreturnentry / vm_customer
// ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct SalesReturnCustomerOption {
    pub cs_customerid: i64,
    pub cs_customername: String,
    pub cs_customerphone: String,
    pub cs_tin_number: String,
    pub cs_balance: f64,
}

#[derive(Debug, Serialize)]
pub struct SalesReturnRecent {
    pub sre_billid: i64,
    pub sre_billnumber: i64,
    pub sre_customername: String,
    pub sre_rebill: String, // original sale bill number this return references, if any
    pub sre_billdate: String,
    pub sre_gtotal: String,
}

#[derive(Debug, Serialize)]
pub struct SalesReturnsInit {
    pub next_bill_number: i64,
    pub gst_rates: Vec<f64>,
    pub customers: Vec<SalesReturnCustomerOption>,
    pub recent: Vec<SalesReturnRecent>,
}

#[tauri::command]
pub fn sales_returns_init(db: State<PlatformDb>, session: State<SessionState>) -> Result<SalesReturnsInit, String> {
    let (admin, _) = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    // vm_salreturnentry.user_id is INTEGER -- bind admin directly.
    let last_bill_number: Option<i64> = conn
        .query_row(
            "SELECT sre_billnumber FROM vm_salreturnentry WHERE user_id = ?1 ORDER BY sre_billid DESC LIMIT 1",
            params![admin],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    // vm_customer.user_id is INTEGER too.
    let mut stmt = conn
        .prepare(
            "SELECT cs_customerid, cs_customername, cs_customerphone, cs_tin_number, cs_balance
             FROM vm_customer WHERE user_id = ?1 AND cs_isactive = 0 ORDER BY cs_customername",
        )
        .map_err(|e| e.to_string())?;
    let customers = stmt
        .query_map(params![admin], |row| {
            Ok(SalesReturnCustomerOption {
                cs_customerid: row.get(0)?,
                cs_customername: row.get(1)?,
                cs_customerphone: row.get(2).unwrap_or_default(),
                cs_tin_number: row.get(3).unwrap_or_default(),
                cs_balance: row.get(4).unwrap_or(0.0),
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut stmt = conn
        .prepare(
            "SELECT sre_billid, sre_billnumber, sre_customername, sre_rebill,
                    sre_billdate, sre_gtotal
             FROM vm_salreturnentry WHERE user_id = ?1 AND sre_isactive = 0
             ORDER BY sre_billid DESC LIMIT 20",
        )
        .map_err(|e| e.to_string())?;
    let recent = stmt
        .query_map(params![admin], |row| {
            Ok(SalesReturnRecent {
                sre_billid: row.get(0)?,
                sre_billnumber: row.get(1)?,
                sre_customername: row.get(2)?,
                sre_rebill: row.get(3).unwrap_or_default(),
                sre_billdate: row.get(4).unwrap_or_default(),
                sre_gtotal: row.get(5).unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(SalesReturnsInit {
        next_bill_number: last_bill_number.unwrap_or(0) + 1,
        gst_rates: GST_RATES.to_vec(),
        customers,
        recent,
    })
}

// ---------------------------------------------------------------------
// Product search -- vm_products (NOT filtered to pr_isactive = 0, see module doc)
// ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct SalesReturnProductOption {
    pub pr_productid: i64,
    pub pr_productcode: String,
    pub pr_productname: String,
    pub pr_saleprice: f64,
    pub pr_stock: f64,
    pub pr_unit: String,
    pub pr_size: String,
}

/// NOT filtered to in-stock items, unlike Purchase Returns -- a sale
/// return often comes back at a stock level of 0 (the last one just got
/// sold), and that's exactly when a resaleable return is most useful to
/// restock. Also NOT filtered to pr_isactive = 0, see module doc.
#[tauri::command]
pub fn sales_returns_search_products(db: State<PlatformDb>, session: State<SessionState>, q: String) -> Result<Vec<SalesReturnProductOption>, String> {
    let (admin, _) = guard(&session)?;
    if q.trim().is_empty() {
        return Ok(vec![]);
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let like = format!("%{}%", q.trim());
    let mut stmt = conn
        .prepare(
            "SELECT pr_productid, pr_productcode, pr_productname, pr_saleprice,
                    pr_stock, pr_unit, pr_size
             FROM vm_products
             WHERE user_id = ?1 AND (pr_productcode LIKE ?2 OR pr_productname LIKE ?2)
             ORDER BY pr_productname ASC LIMIT 15",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![admin.to_string(), like], |row| {
            Ok(SalesReturnProductOption {
                pr_productid: row.get(0)?,
                pr_productcode: row.get(1).unwrap_or_default(),
                pr_productname: row.get(2)?,
                pr_saleprice: row.get(3).unwrap_or(0.0),
                pr_stock: row.get(4).unwrap_or(0.0),
                pr_unit: row.get(5).unwrap_or_default(),
                pr_size: row.get(6).unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ---------------------------------------------------------------------
// Lookup an original sale bill -- vm_billentry / vm_billitems
// ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct BillLookupItem {
    pub bi_productid: i64,
    pub bi_price: f64,
    pub bi_quantity: f64,
    pub pr_productcode: String,
    pub pr_productname: String,
    pub pr_size: String,
    pub pr_unit: String,
}

#[derive(Debug, Serialize)]
pub struct BillLookupResult {
    pub bill_number: i64,
    pub customer_id: Option<i64>,
    pub customer_name: String,
    pub customer_mobile: String,
    pub items: Vec<BillLookupItem>,
}

/// Given an original sale's Bill No., pull its line items so the cashier
/// can pick exactly what's being returned at exactly the price it was
/// sold for, instead of re-typing everything from a paper receipt.
/// Purely a convenience lookup -- the return can still be built from a
/// manual product search if there's no traceable original bill (e.g. a
/// walk-in return without a receipt).
#[tauri::command]
pub fn sales_returns_lookup_bill(db: State<PlatformDb>, session: State<SessionState>, bill_number: String) -> Result<BillLookupResult, String> {
    let (admin, _) = guard(&session)?;
    let bill_number = bill_number.trim();
    if bill_number.is_empty() {
        return Err("Enter a bill number.".into());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    // vm_billentry.user_id is TEXT.
    let bill = conn
        .query_row(
            "SELECT be_billid, be_billnumber, be_customerid, be_customername, be_customermobile
             FROM vm_billentry WHERE be_billnumber = ?1 AND user_id = ?2 AND be_isactive = 0",
            params![bill_number, admin.to_string()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("No sale bill #{bill_number} found."))?;
    let (bill_id, bill_number_i, be_customerid, customer_name, customer_mobile) = bill;

    let mut stmt = conn
        .prepare(
            "SELECT bi.bi_productid, bi.bi_price, bi.bi_quantity,
                    p.pr_productcode, p.pr_productname, p.pr_size, p.pr_unit
             FROM vm_billitems bi
             JOIN vm_products p ON p.pr_productid = bi.bi_productid
             WHERE bi.bi_billid = ?1 AND bi.bi_isactive = 0",
        )
        .map_err(|e| e.to_string())?;
    let items = stmt
        .query_map(params![bill_id], |row| {
            Ok(BillLookupItem {
                bi_productid: row.get(0)?,
                bi_price: row.get(1)?,
                bi_quantity: row.get(2)?,
                pr_productcode: row.get(3).unwrap_or_default(),
                pr_productname: row.get(4)?,
                pr_size: row.get(5).unwrap_or_default(),
                pr_unit: row.get(6).unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(BillLookupResult {
        bill_number: bill_number_i,
        customer_id: if be_customerid > 0 { Some(be_customerid) } else { None },
        customer_name,
        customer_mobile,
        items,
    })
}

// ---------------------------------------------------------------------
// Checkout -- vm_salreturnentry / vm_salreturnitem
// ---------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SalesReturnCartItemInput {
    pub product_id: i64,
    pub qty: f64,
    pub price: f64,
    pub gst_pct: f64,
    #[serde(default = "default_restock")]
    pub restock: bool,
}
fn default_restock() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct SalesReturnCheckoutInput {
    pub items: Vec<SalesReturnCartItemInput>,
    #[serde(default = "default_gst_type")]
    pub gst_type: String,
    #[serde(default = "default_customer_name")]
    pub customer_name: String,
    #[serde(default)]
    pub customer_mobile: String,
    #[serde(default)]
    pub customer_gstin: String,
    pub customer_id: Option<i64>,
    #[serde(default)]
    pub original_bill_number: String,
    #[serde(default = "default_pay_method")]
    pub pay_method: String,
    #[serde(default)]
    pub overall_discount: f64,
    /// Cash/bank actually refunded to the customer right now -- same
    /// field name as Billing/Purchases for consistency, just flowing in
    /// the opposite direction (money OUT, not in).
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
fn default_customer_name() -> String {
    "Walk-in Customer".to_string()
}

#[derive(Debug, Serialize)]
pub struct SalesReturnCheckoutResult {
    pub bill_id: i64,
    pub bill_number: i64,
    pub subtotal: f64,
    pub total_gst: f64,
    pub grand_total: f64,
    pub credit_amount: f64,
}

struct SalReturnLineCalc {
    product_id: i64,
    qty: f64,
    price: f64,
    gst_pct: f64,
    gst_amt: f64,
    line_total: f64,
    restock: bool,
    cgst_pct: Option<f64>,
    cgst_amt: Option<f64>,
    sgst_pct: Option<f64>,
    sgst_amt: Option<f64>,
    igst_pct: Option<f64>,
    igst_amt: Option<f64>,
}

/// Python's `original_bill_number.isdigit()` check -- non-negative
/// ASCII-digit strings only (empty string is also not "digit").
fn is_digit_string(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

#[tauri::command]
pub fn sales_returns_checkout(db: State<PlatformDb>, session: State<SessionState>, input: SalesReturnCheckoutInput) -> Result<SalesReturnCheckoutResult, String> {
    let (admin, username) = guard(&session)?;

    if input.items.is_empty() {
        return Err("No items in this return.".into());
    }
    let customer_name = if input.customer_name.trim().is_empty() { "Walk-in Customer".to_string() } else { input.customer_name.trim().to_string() };
    let gst_type = if input.gst_type == "inter" { "inter" } else { "intra" };
    let original_bill_number = input.original_bill_number.trim().to_string();

    // vm_salreturnentry/item + vm_customer all use INTEGER user_id here;
    // vm_products uses TEXT (see module doc).
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let mut line_calcs: Vec<SalReturnLineCalc> = Vec::with_capacity(input.items.len());
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

        // Not filtered to pr_isactive = 0 -- a product discontinued since
        // the original sale must still be returnable (see module doc).
        let exists: Option<i64> = tx
            .query_row(
                "SELECT pr_productid FROM vm_products WHERE pr_productid = ?1 AND user_id = ?2",
                params![raw.product_id, admin.to_string()],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if exists.is_none() {
            return Err(format!("Product #{} not found.", raw.product_id));
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

        line_calcs.push(SalReturnLineCalc {
            product_id: raw.product_id,
            qty: raw.qty,
            price: raw.price,
            gst_pct: raw.gst_pct,
            gst_amt,
            line_total,
            restock: raw.restock,
            cgst_pct,
            cgst_amt,
            sgst_pct,
            sgst_amt,
            igst_pct,
            igst_amt,
        });
    }

    let grand_total = round2(subtotal + total_gst - input.overall_discount);
    // The portion NOT refunded in cash/bank right now becomes a credit
    // note against the customer's account (cs_balance goes down -- they
    // now owe the shop less, or the shop owes them, if it goes
    // negative). Mirror of Billing's "balance" concept, opposite sign.
    let refund_amount = input.paid_amount;
    let credit_amount = round2(grand_total - refund_amount);

    if refund_amount > grand_total + 0.001 {
        return Err("Refund amount can't exceed the return's total.".into());
    }
    if credit_amount > 0.001 && input.customer_id.is_none() {
        return Err(
            "This return isn't fully refunded in cash/bank right now. Please select a customer from the list (not just typed text) so the credit can be tracked against their account.".into(),
        );
    }

    // vm_salreturnentry.user_id is INTEGER -- bind admin directly.
    let last_bill_number: Option<i64> = tx
        .query_row(
            "SELECT sre_billnumber FROM vm_salreturnentry WHERE user_id = ?1 ORDER BY sre_billid DESC LIMIT 1",
            params![admin],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let bill_number = last_bill_number.unwrap_or(0) + 1;
    let now = chrono_now();

    // sre_supplierid: column name is a schema-inherited misnomer -- this
    // is where the *customer* id goes (see module doc).
    let customer_id_int = input.customer_id.unwrap_or(0);

    tx.execute(
        "INSERT INTO vm_salreturnentry
            (user_id, sre_billnumber, sre_customername, sre_customeraddress,
             sre_customermobile, sre_customer_tin_num, sre_billdate,
             sre_total, sre_gtotal, sre_oldbal, sre_paidamount,
             sre_paymethod, sre_note, sre_updateddate, sre_updatedby,
             sre_isactive, sre_discount, sre_mode, sre_paydate,
             sre_unitprice, sre_balance, sre_supplierid,
             sre_vehicle_number, sre_rebill, sre_statecode,
             sre_debitid, sre_creditid)
         VALUES (?1, ?2, ?3, '', ?4, ?5, ?6, ?7, ?8, '0', ?9, ?10, ?11, ?12, ?13, 0, ?14,
                 'salesreturn', ?15, 0, ?16, ?17, '', ?18, ?19, 0, 0)",
        params![
            admin,
            bill_number,
            customer_name,
            input.customer_mobile.trim(),
            input.customer_gstin.trim(),
            now,
            round2(subtotal),
            format!("{}", grand_total),
            refund_amount,
            input.pay_method.trim(),
            input.note.trim(),
            now,
            username,
            input.overall_discount,
            now,
            round2(credit_amount),
            customer_id_int,
            original_bill_number,
            gst_type,
        ],
    )
    .map_err(|e| e.to_string())?;
    let bill_id = tx.last_insert_rowid();

    let return_bill_ref: i64 = if is_digit_string(&original_bill_number) { original_bill_number.parse().unwrap_or(0) } else { 0 };

    for lc in &line_calcs {
        tx.execute(
            "INSERT INTO vm_salreturnitem
                (user_id, sri_billid, sri_returnbillid, sri_productid,
                 sri_price, sri_quantity, sri_total, sri_updatedon,
                 sri_isactive, sri_vatamount, sri_vatper, sri_unitprice,
                 sri_sgst, sri_sgstamt, sri_cgst, sri_cgstamt,
                 sri_igst, sri_igstamt, sri_billdate)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                admin,
                bill_id,
                return_bill_ref,
                lc.product_id,
                lc.price,
                lc.qty,
                round2(lc.line_total),
                now,
                round2(lc.gst_amt),
                lc.gst_pct,
                // sri_unitprice repurposed as the restock flag -- see
                // module doc. 1.0 = goods went back on the shelf, 0.0 =
                // written off (damaged/unsellable).
                if lc.restock { 1.0 } else { 0.0 },
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

        if lc.restock {
            tx.execute(
                "UPDATE vm_products SET pr_stock = pr_stock + ?1 WHERE pr_productid = ?2 AND user_id = ?3",
                params![lc.qty, lc.product_id, admin.to_string()],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    // Auto-post to Ledger/Daybook and adjust the customer's balance,
    // mirroring Billing's sale-time posting but flowing the other way:
    //
    //   refund_amount (cash/bank actually paid back out now) -> a
    //   Ledger + Daybook entry, same shape as a Payment voucher.
    //
    //   credit_amount (grand_total - refund_amount, i.e. what's being
    //   credited to their account instead of paid back in cash) ->
    //   DECREASES the linked customer's balance -- the opposite
    //   direction from a credit sale (which increases it). If the
    //   customer had no outstanding debt, this can take their balance
    //   negative, meaning the shop now owes them (a store credit) --
    //   that's correct, not a bug.
    let bill_date = &now[..10];
    if refund_amount > 0.001 {
        let ledger_mode = if input.pay_method.trim() == "Cash" { "Cash" } else { "Bank" };
        let particulars = format!("Sales Return #{bill_number} -- {customer_name}");
        let customer_id_str = input.customer_id.map(|c| c.to_string()).unwrap_or_default();
        crate::accounts::post_ledger_entry(
            &tx,
            admin,
            &particulars,
            -refund_amount,
            bill_date,
            "expense",
            ledger_mode,
            &customer_id_str,
            &customer_name,
        )?;
        crate::accounts::post_daybook_entry(&tx, admin, bill_date, "Sales Return", ledger_mode, refund_amount, &particulars)?;
    }

    if credit_amount > 0.001 {
        if let Some(cid) = input.customer_id {
            tx.execute(
                "UPDATE vm_customer SET cs_balance = cs_balance - ?1 WHERE cs_customerid = ?2 AND user_id = ?3",
                params![credit_amount, cid, admin],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;

    Ok(SalesReturnCheckoutResult {
        bill_id,
        bill_number,
        subtotal: round2(subtotal),
        total_gst: round2(total_gst),
        grand_total,
        credit_amount,
    })
}
