//! billing.rs
//! -----------
//! Port of `routes/billing.py`'s checkout logic onto the platform
//! schema (`vm_billentry`/`vm_billitems`/`vm_products`/`vm_customer`).
//!
//! Deliberately does NOT generate a PDF or render anything -- that's
//! invoices.rs's job (see its module doc), matching the confirmed
//! Phase 1 decision to keep "create a sale" and "print/reprint a bill"
//! as separate concerns, the same split Python already had between
//! billing.py and invoices.py.
//!
//! Three Phase 1 decisions baked in here (confirmed before writing this
//! file, see the roadmap doc / area notes):
//!   1. GST type (intra/inter) is auto-derived from the buyer's state
//!      code vs the shop's own state code (`vm_shopprofile.sp_stcode`),
//!      but the cashier can override it per bill via `gst_type_override`
//!      -- unlike Python, which trusted an explicit cashier choice with
//!      no derivation at all.
//!   2. Per-line `gst_mode` ("exclusive" | "inclusive") is kept as an
//!      option, ported from the existing (single-tenant) invoicing
//!      app's model -- Python's billing.py has no such concept and is
//!      always exclusive, so this is new behaviour for the ERP side,
//!      not a straight port.
//!   3. This module only writes the bill; nothing here calls a PDF
//!      generator.
//!
//! NOT yet wired here: Ledger/Daybook auto-posting
//! (`post_ledger_entry`/`post_daybook_entry` in Python's billing.py).
//! That's Phase 2 (Accounts: Vouchers/Ledger/Daybook) per the roadmap
//! -- those tables and their posting functions don't exist on the Rust
//! side yet. Customer balance IS updated here (vm_customer.cs_balance,
//! same table customers.rs already owns), since that's plain Phase 1
//! scope, not an Accounts-module concern. A bill's credit portion is
//! therefore tracked on the customer record correctly even before
//! Phase 2 exists; only the Ledger/Daybook mirror of that entry is
//! deferred.

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::platform_db::PlatformDb;
use crate::products::chrono_now;
use crate::session::{check_login, LoginGuard, SessionState};
use crate::shop_guard::check_shop_type;

pub const GST_RATES: [f64; 5] = [0.0, 5.0, 12.0, 18.0, 28.0];

fn guard(session: &State<SessionState>) -> Result<(i64, String), String> {
    let s = session.0.lock().map_err(|e| e.to_string())?;
    match check_login(&s) {
        LoginGuard::NotLoggedIn => return Err("not_logged_in".into()),
        LoginGuard::NoShopSelected => return Err("no_shop_selected".into()),
        LoginGuard::Ok => {}
    }
    check_shop_type("billing", &s.shop_type)?;
    let admin = s.active_shop_id.ok_or_else(|| "no_shop_selected".to_string())?;
    Ok((admin, s.username.clone()))
}

#[derive(Debug, Serialize)]
pub struct BillingInit {
    pub next_bill_number: i64,
    pub gst_rates: Vec<f64>,
    pub shop_state_code: String,
    pub customers: Vec<CustomerOption>,
}

#[derive(Debug, Serialize)]
pub struct CustomerOption {
    pub cs_customerid: i64,
    pub cs_customername: String,
    pub cs_customerphone: String,
    pub cs_tin_number: String,
    pub cs_statecode: String,
}

#[tauri::command]
pub fn billing_init(db: State<PlatformDb>, session: State<SessionState>) -> Result<BillingInit, String> {
    let (admin, _) = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let last_bill_number: Option<i64> = conn
        .query_row(
            "SELECT be_billnumber FROM vm_billentry WHERE user_id = ?1 ORDER BY be_billid DESC LIMIT 1",
            params![admin.to_string()],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    let shop_state_code: String = conn
        .query_row(
            "SELECT sp_stcode FROM vm_shopprofile WHERE sp_shopid = ?1",
            params![admin],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten()
        .unwrap_or_default();

    let mut stmt = conn
        .prepare(
            "SELECT cs_customerid, cs_customername, cs_customerphone, cs_tin_number, cs_statecode
             FROM vm_customer WHERE user_id = ?1 AND cs_isactive = 0 ORDER BY cs_customername",
        )
        .map_err(|e| e.to_string())?;
    let customers = stmt
        .query_map(params![admin], |row| {
            Ok(CustomerOption {
                cs_customerid: row.get(0)?,
                cs_customername: row.get(1)?,
                cs_customerphone: row.get(2).unwrap_or_default(),
                cs_tin_number: row.get(3).unwrap_or_default(),
                cs_statecode: row.get(4).unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(BillingInit {
        next_bill_number: last_bill_number.unwrap_or(0) + 1,
        gst_rates: GST_RATES.to_vec(),
        shop_state_code,
        customers,
    })
}

#[derive(Debug, Deserialize)]
pub struct CartItemInput {
    pub product_id: i64,
    pub qty: f64,
    pub discount_pct: f64,
    pub gst_pct: f64,
    #[serde(default = "default_gst_mode")]
    pub gst_mode: String, // "exclusive" | "inclusive" -- decision #2
}
fn default_gst_mode() -> String {
    "exclusive".to_string()
}

#[derive(Debug, Deserialize)]
pub struct CheckoutInput {
    pub items: Vec<CartItemInput>,
    /// Buyer's state code, used to auto-derive gst_type (decision #1).
    #[serde(default)]
    pub customer_state_code: String,
    /// Cashier override -- "intra" | "inter" | absent (use auto-derived).
    #[serde(default)]
    pub gst_type_override: Option<String>,
    #[serde(default)]
    pub customer_name: String,
    #[serde(default)]
    pub customer_mobile: String,
    pub customer_id: Option<i64>,
    #[serde(default = "default_pay_method")]
    pub pay_method: String,
    #[serde(default)]
    pub overall_discount: f64,
    #[serde(default)]
    pub paid_amount: f64,
    #[serde(default)]
    pub note: String,
    #[serde(default = "default_bill_mode")]
    pub bill_mode: String, // "retail" | "wholesale"
    #[serde(default)]
    pub transport_name: String,
    #[serde(default)]
    pub lr_number: String,
    #[serde(default)]
    pub lr_date: String,
    #[serde(default)]
    pub parcels: String,
    #[serde(default)]
    pub salesman: String,
    #[serde(default)]
    pub booking: String,
    #[serde(default)]
    pub customer_pan: String,
    #[serde(default)]
    pub customer_gstin: String,
}
fn default_pay_method() -> String {
    "Cash".to_string()
}
fn default_bill_mode() -> String {
    "retail".to_string()
}

#[derive(Debug, Serialize)]
pub struct CheckoutResult {
    pub bill_id: i64,
    pub bill_number: i64,
    pub subtotal: f64,
    pub total_gst: f64,
    pub grand_total: f64,
    pub balance: f64,
    pub gst_type: String,
}

struct LineCalc {
    product_id: i64,
    product_name: String,
    purchase_price: f64,
    qty: f64,
    price: f64,
    discount_pct: f64,
    gst_pct: f64,
    gst_amt: f64,
    taxable: f64,
    pre_net: f64,
    line_total: f64,
    cgst_pct: Option<f64>,
    cgst_amt: Option<f64>,
    sgst_pct: Option<f64>,
    sgst_amt: Option<f64>,
    igst_pct: Option<f64>,
    igst_amt: Option<f64>,
    style_snapshot: String,
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[tauri::command]
pub fn checkout(db: State<PlatformDb>, session: State<SessionState>, input: CheckoutInput) -> Result<CheckoutResult, String> {
    let (admin, username) = guard(&session)?;

    if input.items.is_empty() {
        return Err("Cart is empty.".into());
    }
    for it in &input.items {
        if it.qty <= 0.0 {
            return Err("Quantity must be greater than zero.".into());
        }
        if !GST_RATES.contains(&it.gst_pct) {
            return Err("Invalid GST rate.".into());
        }
        if it.gst_mode != "exclusive" && it.gst_mode != "inclusive" {
            return Err("Invalid GST mode.".into());
        }
    }

    let bill_mode = if input.bill_mode == "wholesale" { "wholesale" } else { "retail" };
    let is_wholesale = bill_mode == "wholesale";

    let mut conn = db.0.lock().map_err(|e| e.to_string())?;

    // --- Decision #1: auto-derive gst_type from shop vs buyer state,
    // cashier override wins when present and valid.
    let shop_state_code: String = conn
        .query_row(
            "SELECT sp_stcode FROM vm_shopprofile WHERE sp_shopid = ?1",
            params![admin],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten()
        .unwrap_or_default();

    // Unknown buyer state (walk-in, or a customer with no state code on
    // file) defaults to "intra" -- matches Python's original default
    // value exactly, and is the safer assumption for a shop whose sales
    // are mostly local. Only flips to "inter" when we actually have a
    // buyer state code AND it differs from the shop's.
    let auto_gst_type = if input.customer_state_code.trim().is_empty() {
        "intra"
    } else if input.customer_state_code.trim() == shop_state_code.trim() {
        "intra"
    } else {
        "inter"
    };
    let gst_type = match input.gst_type_override.as_deref() {
        Some("intra") => "intra",
        Some("inter") => "inter",
        _ => auto_gst_type,
    };

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let mut line_calcs: Vec<LineCalc> = Vec::with_capacity(input.items.len());
    let mut subtotal = 0.0_f64;
    let mut total_gst = 0.0_f64;

    for raw in &input.items {
        let product = tx
            .query_row(
                "SELECT pr_productname, pr_purchaseprice, pr_saleprice, pr_wholesale, pr_stock, pr_model
                 FROM vm_products WHERE pr_productid = ?1 AND user_id = ?2 AND pr_isactive = 0",
                params![raw.product_id, admin.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, f64>(1)?,
                        row.get::<_, f64>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, f64>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Product #{} not found.", raw.product_id))?;

        let (name, purchase_price, saleprice, wholesale_raw, stock, model) = product;

        if stock < raw.qty {
            return Err(format!(
                "Not enough stock for '{}' (have {}, need {}).",
                name, stock, raw.qty
            ));
        }

        let price = if is_wholesale {
            let w: f64 = wholesale_raw.trim().parse().unwrap_or(0.0);
            if w <= 0.0 {
                return Err(format!(
                    "'{}' has no wholesale price set -- add one on the Products screen before selling it in Wholesale mode.",
                    name
                ));
            }
            w
        } else {
            saleprice
        };

        let pre_net = price * raw.qty;
        let discount_amt = pre_net * (raw.discount_pct / 100.0);
        let taxable = pre_net - discount_amt;

        // Decision #2: honor per-line inclusive/exclusive.
        let (taxable, gst_amt) = if raw.gst_mode == "inclusive" {
            let t = taxable / (1.0 + raw.gst_pct / 100.0);
            (t, taxable - t)
        } else {
            (taxable, taxable * (raw.gst_pct / 100.0))
        };
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

        line_calcs.push(LineCalc {
            product_id: raw.product_id,
            product_name: name,
            purchase_price,
            qty: raw.qty,
            price,
            discount_pct: raw.discount_pct,
            gst_pct: raw.gst_pct,
            gst_amt,
            taxable,
            pre_net,
            line_total,
            cgst_pct,
            cgst_amt,
            sgst_pct,
            sgst_amt,
            igst_pct,
            igst_amt,
            style_snapshot: model,
        });
    }

    let grand_total = round2(subtotal + total_gst - input.overall_discount);
    let balance = round2(grand_total - input.paid_amount);

    if balance > 0.001 && input.customer_id.is_none() {
        return Err(
            "This bill isn't fully paid. Please select a customer so the remaining balance can be tracked on their account."
                .into(),
        );
    }

    let last_bill_number: Option<i64> = tx
        .query_row(
            "SELECT be_billnumber FROM vm_billentry WHERE user_id = ?1 ORDER BY be_billid DESC LIMIT 1",
            params![admin.to_string()],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let bill_number = last_bill_number.unwrap_or(0) + 1;

    let now = chrono_now();

    tx.execute(
        "INSERT INTO vm_billentry
            (user_id, be_billnumber, be_customername, be_customermobile,
             be_customer_tin_num, be_vehicle_number, be_billdate,
             be_total, be_gtotal, be_paidamount, be_paymethod, be_note,
             be_updateddate, be_updatedby, be_isactive, be_discount,
             be_mode, be_paydate, be_balance, be_customerid, be_coolie,
             be_oldbal, be_totvat, be_debitid, be_creditid,
             be_statecode, be_mod, be_billmode, be_transport_name,
             be_lr_number, be_lr_date, be_parcels, be_salesman,
             be_booking, be_customer_pan)
         VALUES (?1, ?2, ?3, ?4, ?5, '', ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 0, ?14,
                 'sales', ?15, ?16, ?17, '', '0', ?18, '', '', ?19, 0, ?20, ?21,
                 ?22, ?23, ?24, ?25, ?26, ?27)",
        params![
            admin.to_string(),
            bill_number,
            if input.customer_name.trim().is_empty() { "Walk-in Customer" } else { input.customer_name.trim() },
            input.customer_mobile.trim(),
            input.customer_gstin.trim(),
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
            balance as i64,
            input.customer_id.unwrap_or(0),
            round2(total_gst),
            gst_type,
            bill_mode,
            input.transport_name.trim(),
            input.lr_number.trim(),
            input.lr_date.trim(),
            input.parcels.trim(),
            input.salesman.trim(),
            input.booking.trim(),
            input.customer_pan.trim(),
        ],
    )
    .map_err(|e| e.to_string())?;
    let bill_id = tx.last_insert_rowid();

    for lc in &line_calcs {
        tx.execute(
            "INSERT INTO vm_billitems
                (user_id, bi_billid, bi_productid, bi_price, bi_quantity,
                 bi_taxamount, bi_discount, bi_total, bi_updatedon,
                 bi_isactive, bi_vatamount, bi_vatper, be_coolie,
                 bi_sgst, bi_sgst_amt, bi_cgst, bi_cgst_amt,
                 bi_igst, bi_igst_amt, bi_billdate, bi_purprice,
                 bi_disc, bi_prenet, bi_style_snapshot)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, ?10, ?11, '', ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
            params![
                admin.to_string(),
                bill_id,
                lc.product_id,
                lc.price,
                lc.qty,
                round2(lc.line_total),
                format!("{}", lc.discount_pct),
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
                now,
                lc.purchase_price,
                lc.discount_pct,
                lc.pre_net,
                lc.style_snapshot,
            ],
        )
        .map_err(|e| e.to_string())?;

        let updated = tx
            .execute(
                "UPDATE vm_products SET pr_stock = pr_stock - ?1
                 WHERE pr_productid = ?2 AND user_id = ?3 AND pr_stock >= ?1",
                params![lc.qty, lc.product_id, admin.to_string()],
            )
            .map_err(|e| e.to_string())?;
        if updated == 0 {
            return Err(format!(
                "Stock for '{}' changed before this sale finished -- please review the cart and try again.",
                lc.product_name
            ));
        }
    }

    // Customer balance update (Phase 1 scope). Ledger/Daybook mirror of
    // this sale is Phase 2 (Accounts) -- see module doc.
    if balance > 0.001 {
        if let Some(cid) = input.customer_id {
            tx.execute(
                "UPDATE vm_customer SET cs_balance = cs_balance + ?1 WHERE cs_customerid = ?2 AND user_id = ?3",
                params![balance, cid, admin],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;

    Ok(CheckoutResult {
        bill_id,
        bill_number,
        subtotal: round2(subtotal),
        total_gst: round2(total_gst),
        grand_total,
        balance,
        gst_type: gst_type.to_string(),
    })
}
