//! invoices.rs
//! ------------
//! Direct port of `routes/invoices.py`'s `print_invoice()` /
//! `find_by_number()` onto the platform schema. Read-only against
//! `vm_billentry`/`vm_billitems` -- this module never writes to
//! either, matching the Python original's own boundary (that's
//! billing.rs's job).
//!
//! Design note: Python's `invoice_print.html` has no PDF generation at
//! all -- it's a plain HTML page with a `window.print()` button
//! (browser-native print), NOT reportlab/pdf_utils.py. So this module
//! deliberately returns structured data for a React view to render in
//! the same shape as invoice_print.html, with "Print" wired to the
//! webview's native print (Tauri supports this directly) rather than
//! hand-drawing the invoice in `printpdf`. That's a faithful port of
//! what Python actually does for invoices specifically -- pdf_utils.py
//! was written for Estimation/Report exports, not this screen.
//! Revisit only if a real need for a headless (no-print-dialog) PDF
//! file shows up later.
//!
//! Column/query notes carried over from the Python docstring, still
//! true on this schema:
//!   - vm_shopprofile has no user_id column -- looked up by
//!     `sp_shopid = ?` against the active shop id directly.
//!   - Buyer GSTIN as it appeared on this specific bill lives in
//!     `be_customer_tin_num` (billing.rs's checkout snapshot) -- more
//!     accurate to print than joining out to the customer's current
//!     master-record value, which could have changed since.
//!   - Buyer *state*, by contrast, has no bill-time snapshot column --
//!     only vm_customer's live cs_statecode. The React view should
//!     label it "current on file", matching Python's template note.
//!   - Size-ratio grouping keys off `bi_style_snapshot` (copied from
//!     pr_model at sale time), never a live join back to vm_products --
//!     a style renamed after the sale must not reshuffle an
//!     already-printed bill's grouping.

use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use tauri::State;

use crate::platform_db::PlatformDb;
use crate::session::{check_login, LoginGuard, SessionState};
use crate::shop_guard::check_shop_type;
use crate::words::amount_to_words;

fn guard(session: &State<SessionState>) -> Result<i64, String> {
    let s = session.0.lock().map_err(|e| e.to_string())?;
    match check_login(&s) {
        LoginGuard::NotLoggedIn => return Err("not_logged_in".into()),
        LoginGuard::NoShopSelected => return Err("no_shop_selected".into()),
        LoginGuard::Ok => {}
    }
    check_shop_type("invoices", &s.shop_type)?;
    s.active_shop_id.ok_or_else(|| "no_shop_selected".to_string())
}

#[derive(Debug, Serialize, Default)]
pub struct BillEntry {
    pub be_billid: i64,
    pub be_billnumber: i64,
    pub be_customername: String,
    pub be_customermobile: String,
    pub be_customer_tin_num: String,
    pub be_customer_pan: String,
    pub be_billdate: String,
    pub be_total: f64,
    pub be_gtotal: String,
    pub be_paidamount: f64,
    pub be_balance: f64,
    pub be_discount: f64,
    pub be_totvat: f64,
    pub be_customerid: i64,
    pub be_statecode: String,
    pub be_billmode: String,
    pub be_transport_name: String,
    pub be_lr_number: String,
    pub be_lr_date: String,
    pub be_parcels: String,
    pub be_salesman: String,
    pub be_booking: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct BillItem {
    pub bi_billitemid: i64,
    pub bi_productid: i64,
    pub bi_price: f64,
    pub bi_quantity: f64,
    pub bi_taxamount: f64,
    pub bi_discount: String,
    pub bi_total: f64,
    pub bi_sgst: Option<f64>,
    pub bi_sgst_amt: Option<f64>,
    pub bi_cgst: Option<f64>,
    pub bi_cgst_amt: Option<f64>,
    pub bi_igst: Option<f64>,
    pub bi_igst_amt: Option<f64>,
    pub bi_style_snapshot: String,
    pub pr_productname: String,
    pub pr_hsn: String,
    pub pr_cupsize: String,
    pub pr_size: String,
    pub pr_model: String,
    pub pr_unit: String,
}

#[derive(Debug, Serialize, Default)]
pub struct ShopHeader {
    pub sp_shopname: String,
    pub sp_tagline: String,
    pub sp_shopaddress: String,
    pub sp_phone: String,
    pub sp_mobile: String,
    pub sp_tin: String,
    pub sp_bank: String,
    pub sp_branch: String,
    pub sp_accno: String,
    pub sp_ifsc: String,
    pub sp_terms: String,
}

#[derive(Debug, Serialize)]
pub struct CustomerHeader {
    pub cs_address: String,
    pub cs_statecode: String,
}

#[derive(Debug, Serialize)]
pub struct StyleGroup {
    pub style: String,
    pub lines: Vec<BillItem>,
    pub total_qty: f64,
    pub total_amount: f64,
}

#[derive(Debug, Serialize)]
pub struct InvoiceData {
    pub bill: BillEntry,
    pub items: Vec<BillItem>,
    pub style_groups: Option<Vec<StyleGroup>>,
    pub shop: ShopHeader,
    pub customer: Option<CustomerHeader>,
    pub is_interstate: bool,
    pub is_wholesale: bool,
    pub grand_total_words: String,
}

#[tauri::command]
pub fn find_bill_by_number(db: State<PlatformDb>, session: State<SessionState>, bill_number: String) -> Result<i64, String> {
    let admin = guard(&session)?;
    let bill_number: i64 = bill_number
        .trim()
        .parse()
        .map_err(|_| "Enter a valid bill number.".to_string())?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT be_billid FROM vm_billentry WHERE be_billnumber = ?1 AND user_id = ?2 AND be_isactive = 0",
        params![bill_number, admin.to_string()],
        |r| r.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("No bill #{} found.", bill_number))
}

#[tauri::command]
pub fn get_invoice_data(db: State<PlatformDb>, session: State<SessionState>, bill_id: i64) -> Result<InvoiceData, String> {
    let admin = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let bill = conn
        .query_row(
            "SELECT be_billid, be_billnumber, be_customername, be_customermobile,
                    be_customer_tin_num, be_customer_pan, be_billdate, be_total,
                    be_gtotal, be_paidamount, be_balance, be_discount, be_totvat,
                    be_customerid, be_statecode, be_billmode, be_transport_name,
                    be_lr_number, be_lr_date, be_parcels, be_salesman, be_booking
             FROM vm_billentry WHERE be_billid = ?1 AND user_id = ?2 AND be_isactive = 0",
            params![bill_id, admin.to_string()],
            |row| {
                Ok(BillEntry {
                    be_billid: row.get(0)?,
                    be_billnumber: row.get(1)?,
                    be_customername: row.get(2)?,
                    be_customermobile: row.get(3).unwrap_or_default(),
                    be_customer_tin_num: row.get(4).unwrap_or_default(),
                    be_customer_pan: row.get(5).unwrap_or_default(),
                    be_billdate: row.get(6)?,
                    be_total: row.get(7).unwrap_or_default(),
                    be_gtotal: row.get(8).unwrap_or_default(),
                    be_paidamount: row.get(9).unwrap_or_default(),
                    be_balance: row.get(10).unwrap_or_default(),
                    be_discount: row.get(11).unwrap_or_default(),
                    be_totvat: row.get(12).unwrap_or_default(),
                    be_customerid: row.get(13).unwrap_or_default(),
                    be_statecode: row.get(14).unwrap_or_default(),
                    be_billmode: row.get(15).unwrap_or_default(),
                    be_transport_name: row.get(16).unwrap_or_default(),
                    be_lr_number: row.get(17).unwrap_or_default(),
                    be_lr_date: row.get(18).unwrap_or_default(),
                    be_parcels: row.get(19).unwrap_or_default(),
                    be_salesman: row.get(20).unwrap_or_default(),
                    be_booking: row.get(21).unwrap_or_default(),
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Bill not found.".to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT bi.bi_billitemid, bi.bi_productid, bi.bi_price, bi.bi_quantity,
                    bi.bi_taxamount, bi.bi_discount, bi.bi_total,
                    bi.bi_sgst, bi.bi_sgst_amt, bi.bi_cgst, bi.bi_cgst_amt,
                    bi.bi_igst, bi.bi_igst_amt, bi.bi_style_snapshot,
                    p.pr_productname, p.pr_hsn, p.pr_cupsize, p.pr_size, p.pr_model, p.pr_unit
             FROM vm_billitems bi
             LEFT JOIN vm_products p ON p.pr_productid = bi.bi_productid
             WHERE bi.bi_billid = ?1 AND bi.user_id = ?2 AND bi.bi_isactive = 0
             ORDER BY bi.bi_billitemid ASC",
        )
        .map_err(|e| e.to_string())?;
    let items: Vec<BillItem> = stmt
        .query_map(params![bill_id, admin.to_string()], |row| {
            Ok(BillItem {
                bi_billitemid: row.get(0)?,
                bi_productid: row.get(1)?,
                bi_price: row.get(2).unwrap_or_default(),
                bi_quantity: row.get(3).unwrap_or_default(),
                bi_taxamount: row.get(4).unwrap_or_default(),
                bi_discount: row.get(5).unwrap_or_default(),
                bi_total: row.get(6).unwrap_or_default(),
                bi_sgst: row.get(7).unwrap_or_default(),
                bi_sgst_amt: row.get(8).unwrap_or_default(),
                bi_cgst: row.get(9).unwrap_or_default(),
                bi_cgst_amt: row.get(10).unwrap_or_default(),
                bi_igst: row.get(11).unwrap_or_default(),
                bi_igst_amt: row.get(12).unwrap_or_default(),
                bi_style_snapshot: row.get(13).unwrap_or_default(),
                pr_productname: row.get(14).unwrap_or_default(),
                pr_hsn: row.get(15).unwrap_or_default(),
                pr_cupsize: row.get(16).unwrap_or_default(),
                pr_size: row.get(17).unwrap_or_default(),
                pr_model: row.get(18).unwrap_or_default(),
                pr_unit: row.get(19).unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let shop = conn
        .query_row(
            "SELECT sp_shopname, sp_tagline, sp_shopaddress, sp_phone, sp_mobile,
                    sp_tin, sp_bank, sp_branch, sp_accno, sp_ifsc, sp_terms
             FROM vm_shopprofile WHERE sp_shopid = ?1",
            params![admin],
            |row| {
                Ok(ShopHeader {
                    sp_shopname: row.get(0).unwrap_or_default(),
                    sp_tagline: row.get(1).unwrap_or_default(),
                    sp_shopaddress: row.get(2).unwrap_or_default(),
                    sp_phone: row.get(3).unwrap_or_default(),
                    sp_mobile: row.get(4).unwrap_or_default(),
                    sp_tin: row.get(5).unwrap_or_default(),
                    sp_bank: row.get(6).unwrap_or_default(),
                    sp_branch: row.get(7).unwrap_or_default(),
                    sp_accno: row.get(8).unwrap_or_default(),
                    sp_ifsc: row.get(9).unwrap_or_default(),
                    sp_terms: row.get(10).unwrap_or_default(),
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    let customer = if bill.be_customerid > 0 {
        conn.query_row(
            "SELECT cs_address, cs_statecode FROM vm_customer WHERE cs_customerid = ?1 AND user_id = ?2",
            params![bill.be_customerid, admin],
            |row| {
                Ok(CustomerHeader {
                    cs_address: row.get(0).unwrap_or_default(),
                    cs_statecode: row.get(1).unwrap_or_default(),
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())?
    } else {
        None
    };

    let is_interstate = bill.be_statecode == "inter";
    let is_wholesale = bill.be_billmode == "wholesale";
    let grand_total_words = amount_to_words(bill.be_gtotal.parse().unwrap_or(0.0));

    let style_groups = if is_wholesale { Some(group_by_style(&items)) } else { None };

    Ok(InvoiceData {
        bill,
        items,
        style_groups,
        shop,
        customer,
        is_interstate,
        is_wholesale,
        grand_total_words,
    })
}

/// Direct port of Python's `_group_by_style`. Preserves first-seen
/// order of styles so groupings match checkout order, not id/alpha
/// order. Items with a blank `bi_style_snapshot` each get their own
/// single-item group (keyed by billitem id) rather than being lumped
/// together under one shared blank bucket.
fn group_by_style(items: &[BillItem]) -> Vec<StyleGroup> {
    let mut order: Vec<String> = Vec::new();
    let mut groups: std::collections::HashMap<String, (String, Vec<BillItem>)> = std::collections::HashMap::new();

    for it in items {
        let style = it.bi_style_snapshot.clone();
        let key = if style.is_empty() {
            format!("__ungrouped_{}", it.bi_billitemid)
        } else {
            style.clone()
        };
        if !groups.contains_key(&key) {
            order.push(key.clone());
            groups.insert(key.clone(), (style.clone(), Vec::new()));
        }
        groups.get_mut(&key).unwrap().1.push(it.clone());
    }

    order
        .into_iter()
        .map(|key| {
            let (style, lines) = groups.remove(&key).unwrap();
            let total_qty: f64 = lines.iter().map(|l| l.bi_quantity).sum();
            let total_amount: f64 = lines.iter().map(|l| l.bi_taxamount).sum();
            StyleGroup { style, lines, total_qty, total_amount }
        })
        .collect()
}
