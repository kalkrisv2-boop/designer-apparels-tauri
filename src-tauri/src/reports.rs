//! reports.rs
//! ----------
//! Port of `routes/reports.py` -- Phase 5 of the roadmap. Read-only
//! against vm_billitems/vm_billentry/vm_customer/vm_salreturnentry/
//! vm_transaction -- this file never writes to any of them, that's
//! Billing/Accounts/Sales Returns' territory (see those modules).
//!
//! Four commands, matching the four Python routes:
//!     gst_report      -- per-rate GST collected in a date range, read
//!                         straight off vm_billitems' own bi_vatper/
//!                         bi_sgst_amt/bi_cgst_amt/bi_igst_amt columns
//!                         (not recalculated -- these are the exact
//!                         figures billing.rs wrote at sale time).
//!     gstr1_export     -- writes a GSTR-1 B2B/B2C-Large/B2C-Small zip
//!                         (4 CSVs) straight to a path the frontend
//!                         obtained via a save dialog. See the function
//!                         doc below for the Place-of-Supply honesty
//!                         limitation carried over from Python verbatim.
//!     profit_report    -- per line item, profit = bi_total (taxable,
//!                         post-discount pre-GST) minus bi_purprice *
//!                         bi_quantity (the purchase-price snapshot
//!                         billing.rs took at sale time -- a later edit
//!                         to pr_purchaseprice can't retroactively
//!                         change a historical bill's profit). A line
//!                         with a missing/zero bi_purprice is excluded
//!                         from totals and flagged separately rather
//!                         than silently treated as 100% profit -- see
//!                         the function doc.
//!     customer_report  -- balance summary for every active customer
//!                         plus a reconciliation check against bills
//!                         and sales returns (Vouchers are explicitly
//!                         out of scope for that check -- see the
//!                         function doc), with an optional single-
//!                         customer statement drill-down.
//!
//! Type gotcha (verified against schema.sql directly, not assumed from
//! any other module's list -- see the project-wide warning about this):
//!   TEXT user_id (bind admin.to_string()):    vm_billentry, vm_billitems
//!   INTEGER user_id (bind admin directly):    vm_customer, vm_salreturnentry,
//!                                              vm_transaction, vm_shopprofile
//!                                              (its PK is sp_shopid, not a
//!                                              user_id column, but it's the
//!                                              same admin/shop_id value)
//! vm_products.user_id is TEXT, but -- matching Python's own
//! reports.py exactly -- the LEFT JOIN onto vm_products in profit_report
//! does NOT filter by p.user_id at all. Not a fix-on-sight bug; flagged
//! here so it's a visible, deliberate carry-over rather than something
//! silently "corrected" during the port.
//!
//! customer_report mixes THREE different user_id types in one command
//! (vm_customer=INTEGER, vm_billentry=TEXT, vm_salreturnentry=INTEGER)
//! -- worth re-checking every bind here individually rather than
//! copy-pasting one and assuming it covers all three queries.
//!
//! No schema migration needed this phase -- every column this file
//! touches already exists with the right type.

use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::sync::OnceLock;
use tauri::State;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::platform_db::PlatformDb;
use crate::session::{check_login, LoginGuard, SessionState};
use crate::shop_guard::check_shop_type;

fn guard(session: &State<SessionState>) -> Result<(i64, String), String> {
    let s = session.0.lock().map_err(|e| e.to_string())?;
    match check_login(&s) {
        LoginGuard::NotLoggedIn => return Err("not_logged_in".into()),
        LoginGuard::NoShopSelected => return Err("no_shop_selected".into()),
        LoginGuard::Ok => {}
    }
    check_shop_type("reports", &s.shop_type)?;
    let admin = s.active_shop_id.ok_or_else(|| "no_shop_selected".to_string())?;
    Ok((admin, s.username.clone()))
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Normalizes an optional "to" date into the same `"<date> 23:59:59"`
/// shape every other date-range report in this app uses, so a bill
/// dated anywhere on the "to" day is included.
fn end_of_day(date_to: &str) -> String {
    let trimmed = date_to.trim();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed} 23:59:59")
    }
}

// ---------------------------------------------------------------------
// GST report
// ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct GstLine {
    pub be_billnumber: i64,
    pub be_billdate: String,
    pub be_customername: Option<String>,
    pub bi_total: f64,
    pub bi_vatper: f64,
    pub bi_vatamount: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct GstRateBucket {
    pub rate: f64,
    pub taxable: f64,
    pub gst: f64,
    pub sgst: f64,
    pub cgst: f64,
    pub igst: f64,
    pub count: i64,
}

#[derive(Debug, Serialize, Default)]
pub struct GstTotals {
    pub taxable: f64,
    pub gst: f64,
    pub sgst: f64,
    pub cgst: f64,
    pub igst: f64,
}

#[derive(Debug, Serialize)]
pub struct GstReportResult {
    pub date_from: String,
    pub date_to: String,
    pub rate_summary: Vec<GstRateBucket>,
    pub totals: GstTotals,
    pub lines: Vec<GstLine>,
}

#[tauri::command]
pub fn gst_report(
    db: State<PlatformDb>,
    session: State<SessionState>,
    date_from: Option<String>,
    date_to: Option<String>,
) -> Result<GstReportResult, String> {
    let (admin, _) = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let from = date_from.unwrap_or_default();
    let to = end_of_day(&date_to.unwrap_or_default());

    // vm_billitems.user_id is TEXT -- bind admin.to_string() (see module doc).
    let admin_str = admin.to_string();

    let mut stmt = conn
        .prepare(
            "SELECT bi.bi_vatper, bi.bi_total, bi.bi_vatamount, bi.bi_sgst_amt, \
                    bi.bi_cgst_amt, bi.bi_igst_amt, \
                    be.be_billnumber, be.be_billdate, be.be_customername \
             FROM vm_billitems bi \
             JOIN vm_billentry be ON be.be_billid = bi.bi_billid \
             WHERE bi.user_id = ?1 AND bi.bi_isactive = 0 AND be.be_isactive = 0 \
               AND (?2 = '' OR be.be_billdate >= ?2) \
               AND (?3 = '' OR be.be_billdate <= ?3) \
             ORDER BY be.be_billdate ASC, be.be_billid ASC",
        )
        .map_err(|e| e.to_string())?;

    struct RawRow {
        rate: f64,
        taxable: f64,
        gst: f64,
        sgst: f64,
        cgst: f64,
        igst: f64,
        billnumber: i64,
        billdate: String,
        customername: Option<String>,
    }

    let rows: Vec<RawRow> = stmt
        .query_map(params![admin_str, from.trim(), to], |r| {
            Ok(RawRow {
                rate: r.get::<_, Option<f64>>(0)?.unwrap_or(0.0),
                taxable: r.get::<_, Option<f64>>(1)?.unwrap_or(0.0),
                gst: r.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                sgst: r.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
                cgst: r.get::<_, Option<f64>>(4)?.unwrap_or(0.0),
                igst: r.get::<_, Option<f64>>(5)?.unwrap_or(0.0),
                billnumber: r.get(6)?,
                billdate: r.get(7)?,
                customername: r.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Group by GST rate -- one bucket per slab (0/5/12/18/28), same
    // figures billing.rs itself computed and stored per line, not
    // recalculated here. A plain Vec + linear find is used instead of a
    // HashMap since f64 isn't hashable and there are only 5 possible
    // slabs (billing.rs already compares bi_vatper against GST_RATES
    // with direct f64 equality elsewhere -- same accepted idiom here).
    let mut rate_summary: Vec<GstRateBucket> = Vec::new();
    let mut totals = GstTotals::default();
    let mut lines: Vec<GstLine> = Vec::with_capacity(rows.len());

    for r in rows {
        totals.taxable += r.taxable;
        totals.gst += r.gst;
        totals.sgst += r.sgst;
        totals.cgst += r.cgst;
        totals.igst += r.igst;

        match rate_summary.iter_mut().find(|b| b.rate == r.rate) {
            Some(bucket) => {
                bucket.taxable += r.taxable;
                bucket.gst += r.gst;
                bucket.sgst += r.sgst;
                bucket.cgst += r.cgst;
                bucket.igst += r.igst;
                bucket.count += 1;
            }
            None => rate_summary.push(GstRateBucket {
                rate: r.rate,
                taxable: r.taxable,
                gst: r.gst,
                sgst: r.sgst,
                cgst: r.cgst,
                igst: r.igst,
                count: 1,
            }),
        }

        lines.push(GstLine {
            be_billnumber: r.billnumber,
            be_billdate: r.billdate,
            be_customername: r.customername,
            bi_total: r.taxable,
            bi_vatper: r.rate,
            bi_vatamount: r.gst,
        });
    }

    rate_summary.sort_by(|a, b| a.rate.partial_cmp(&b.rate).unwrap());

    Ok(GstReportResult {
        date_from: from,
        date_to: date_to_display(&to),
        rate_summary,
        totals,
        lines,
    })
}

/// Undoes the `" 23:59:59"` suffix `end_of_day` adds, so the value
/// echoed back to the date-picker is the plain date the user entered.
fn date_to_display(to: &str) -> String {
    to.strip_suffix(" 23:59:59").unwrap_or(to).to_string()
}

// ---------------------------------------------------------------------
// GSTR-1 B2B / B2C-Large / B2C-Small export
// ---------------------------------------------------------------------
// A real government-filing export, shaped to match the official GSTR-1
// Returns Offline Tool's per-section CSV import (B2B / B2C-Large /
// B2C-Small are separate sheets/CSVs there too). Column names/order for
// B2B verified against an external GSTR-1 upload-format reference, same
// as Python: GSTIN/UIN of Recipient, Receiver Name, Invoice Number,
// Invoice date, Invoice Value, Place Of Supply, Reverse Charge,
// Applicable % of Tax Rate, Invoice Type, E-Commerce GSTIN, Rate,
// Taxable Value, Cess Amount -- one row per (invoice, GST rate)
// combination.
//
// B2B = buyer has a GSTIN on this bill (be_customer_tin_num non-empty).
// B2C-Large = no GSTIN, inter-state, invoice value > Rs 2.5 lakh,
//   invoice-wise like B2B. B2C-Small = everything else without a
//   GSTIN, consolidated by (Place of Supply, Rate) only.
//
// HONEST LIMITATION, carried over from Python rather than guessed
// around: this schema has no bill-time "buyer state" column anywhere.
// For an INTRA-state bill, Place of Supply is unambiguous (the shop's
// own state). For an INTER-state bill, it's only derivable when the
// bill is linked to a customer record with cs_statecode filled in (and
// even then it's that customer's *current* state on file, not
// necessarily their state at sale time). A walk-in/unlinked interstate
// bill, or a linked customer with a blank cs_statecode, has NO reliable
// Place of Supply anywhere in this database -- those rows go to a
// separate needs-review CSV instead of being silently guessed at or
// dropped from the export.

fn gst_state_codes() -> &'static HashMap<&'static str, &'static str> {
    static MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    MAP.get_or_init(|| {
        HashMap::from([
            ("01", "Jammu & Kashmir"), ("02", "Himachal Pradesh"), ("03", "Punjab"),
            ("04", "Chandigarh"), ("05", "Uttarakhand"), ("06", "Haryana"), ("07", "Delhi"),
            ("08", "Rajasthan"), ("09", "Uttar Pradesh"), ("10", "Bihar"), ("11", "Sikkim"),
            ("12", "Arunachal Pradesh"), ("13", "Nagaland"), ("14", "Manipur"),
            ("15", "Mizoram"), ("16", "Tripura"), ("17", "Meghalaya"), ("18", "Assam"),
            ("19", "West Bengal"), ("20", "Jharkhand"), ("21", "Odisha"),
            ("22", "Chhattisgarh"), ("23", "Madhya Pradesh"), ("24", "Gujarat"),
            ("25", "Daman & Diu (merged into 26, legacy)"),
            ("26", "Dadra & Nagar Haveli and Daman & Diu"), ("27", "Maharashtra"),
            ("28", "Andhra Pradesh (pre-2014 legacy code)"), ("29", "Karnataka"),
            ("30", "Goa"), ("31", "Lakshadweep"), ("32", "Kerala"), ("33", "Tamil Nadu"),
            ("34", "Puducherry"), ("35", "Andaman & Nicobar Islands"), ("36", "Telangana"),
            ("37", "Andhra Pradesh"), ("38", "Ladakh"),
        ])
    })
}

const B2C_LARGE_THRESHOLD: f64 = 250000.0; // Rs 2.5 lakh -- the real GSTR-1 B2B/B2C-Large/B2C-Small split point

#[derive(Debug, Serialize)]
pub struct Gstr1ExportSummary {
    pub b2b_count: usize,
    pub b2c_large_count: usize,
    pub b2c_small_count: usize,
    pub needs_review_count: usize,
}

fn csv_field(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn csv_row(fields: &[String]) -> String {
    let mut line = fields.iter().map(|f| csv_field(f)).collect::<Vec<_>>().join(",");
    line.push_str("\r\n");
    line
}

fn write_csv_to_zip(
    zw: &mut ZipWriter<Cursor<Vec<u8>>>,
    filename: &str,
    header: &[&str],
    rows: &[Vec<String>],
) -> Result<(), String> {
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zw.start_file(filename, options).map_err(|e| e.to_string())?;
    // An empty section still gets a (header-less) file, not silently
    // omitted -- matches Python's _write_csv_to_zip exactly.
    if !rows.is_empty() {
        zw.write_all(csv_row(&header.iter().map(|s| s.to_string()).collect::<Vec<_>>()).as_bytes())
            .map_err(|e| e.to_string())?;
        for row in rows {
            zw.write_all(csv_row(row).as_bytes()).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

struct Gstr1Row {
    bill_id: i64,
    billnumber: i64,
    billdate: String,
    gtotal: String,
    customername: Option<String>,
    tin: Option<String>,
    statecode: Option<String>,
    buyer_statecode: Option<String>,
}

#[tauri::command]
pub fn gstr1_export(
    db: State<PlatformDb>,
    session: State<SessionState>,
    date_from: Option<String>,
    date_to: Option<String>,
    save_path: String,
) -> Result<Gstr1ExportSummary, String> {
    let (admin, _) = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let from = date_from.unwrap_or_default();
    let to = end_of_day(&date_to.unwrap_or_default());
    let admin_str = admin.to_string();

    // vm_shopprofile has no user_id column -- sp_shopid IS the shop's
    // own id, same value as `admin` (INTEGER, bind directly). Matches
    // billing.rs's own shop_state_code lookup exactly (no sp_isactive
    // filter there either -- if this shop_id can log in, it's active).
    let shop_stcode: String = conn
        .query_row(
            "SELECT sp_stcode FROM vm_shopprofile WHERE sp_shopid = ?1",
            params![admin],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|s| pad2(s.trim()))
        .unwrap_or_default();

    // c.user_id = be.user_id compares vm_customer's INTEGER user_id
    // against vm_billentry's TEXT user_id directly in SQL (not a bound
    // parameter) -- SQLite applies numeric affinity across the
    // comparison automatically, same as Python's raw query relied on.
    // Not something to "fix" into a cast; carried over verbatim.
    let mut stmt = conn
        .prepare(
            "SELECT bi.bi_vatper, bi.bi_total, be.be_billid, be.be_billnumber, \
                    be.be_billdate, be.be_gtotal, be.be_customername, \
                    be.be_customer_tin_num, be.be_statecode, \
                    c.cs_statecode AS buyer_statecode \
             FROM vm_billitems bi \
             JOIN vm_billentry be ON be.be_billid = bi.bi_billid \
             LEFT JOIN vm_customer c ON c.cs_customerid = be.be_customerid AND c.user_id = be.user_id \
             WHERE bi.user_id = ?1 AND bi.bi_isactive = 0 AND be.be_isactive = 0 \
               AND (?2 = '' OR be.be_billdate >= ?2) \
               AND (?3 = '' OR be.be_billdate <= ?3) \
             ORDER BY be.be_billdate ASC, be.be_billid ASC, bi.bi_vatper ASC",
        )
        .map_err(|e| e.to_string())?;

    // First pass: bill-level metadata (first row wins, all rows for the
    // same bill share it) + per-bill (rate -> taxable) line grouping,
    // keyed only by bill_id (i64, hashable) to avoid using f64 as part
    // of a hash key.
    let mut bill_meta: HashMap<i64, Gstr1Row> = HashMap::new();
    let mut bill_rate_taxable: HashMap<i64, Vec<(f64, f64)>> = HashMap::new();

    let mapped = stmt
        .query_map(params![admin_str, from.trim(), to], |r| {
            let rate: f64 = r.get::<_, Option<f64>>(0)?.unwrap_or(0.0);
            let taxable: f64 = r.get::<_, Option<f64>>(1)?.unwrap_or(0.0);
            let bill_id: i64 = r.get(2)?;
            Ok((
                bill_id,
                rate,
                taxable,
                Gstr1Row {
                    bill_id,
                    billnumber: r.get(3)?,
                    billdate: r.get(4)?,
                    gtotal: r.get::<_, Option<String>>(5)?.unwrap_or_default(),
                    customername: r.get(6)?,
                    tin: r.get(7)?,
                    statecode: r.get(8)?,
                    buyer_statecode: r.get(9)?,
                },
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in mapped {
        let (bill_id, rate, taxable, meta) = row.map_err(|e| e.to_string())?;
        bill_meta.entry(bill_id).or_insert(meta);
        bill_rate_taxable.entry(bill_id).or_default().push((rate, taxable));
    }

    let mut b2b_rows: Vec<Vec<String>> = Vec::new();
    let mut b2c_large_rows: Vec<Vec<String>> = Vec::new();
    let mut needs_review_rows: Vec<Vec<String>> = Vec::new();
    // (place_of_supply_code, rate) -> taxable total. Linear Vec, not a
    // HashMap, since rate is f64 (see gst_report's rate_summary for the
    // same reasoning).
    let mut b2c_small: Vec<(String, f64, f64)> = Vec::new();

    let state_codes = gst_state_codes();

    for (bill_id, meta) in bill_meta.iter() {
        let has_gstin = meta.tin.as_deref().unwrap_or("").trim().len() > 0;
        let is_interstate = meta.statecode.as_deref() == Some("inter");
        let invoice_value: f64 = meta.gtotal.trim().parse().unwrap_or(0.0);

        let pos_code = if is_interstate {
            meta.buyer_statecode
                .as_deref()
                .map(|s| pad2(s.trim()))
                .unwrap_or_default()
        } else {
            shop_stcode.clone()
        };
        let pos_name = state_codes.get(pos_code.as_str()).copied().unwrap_or("");

        // Sum this bill's line items per rate -- small per-bill Vec
        // (usually 1-5 lines), linear grouping is fine.
        let mut rates_for_bill: Vec<(f64, f64)> = Vec::new();
        if let Some(lines) = bill_rate_taxable.get(bill_id) {
            for &(rate, taxable) in lines {
                match rates_for_bill.iter_mut().find(|(r, _)| *r == rate) {
                    Some((_, t)) => *t += taxable,
                    None => rates_for_bill.push((rate, taxable)),
                }
            }
        }

        if pos_code.is_empty() {
            needs_review_rows.push(vec![
                meta.billnumber.to_string(),
                meta.billdate.clone(),
                format!("{:.2}", invoice_value),
                meta.customername.clone().unwrap_or_else(|| "Walk-in".to_string()),
                meta.tin.clone().unwrap_or_default(),
                "Inter-state bill with no linked customer state on file -- Place of Supply unknown".to_string(),
            ]);
            continue;
        }

        let pos_display = if !pos_name.is_empty() { format!("{pos_code}-{pos_name}") } else { pos_code.clone() };

        if has_gstin {
            for &(rate, taxable) in &rates_for_bill {
                b2b_rows.push(vec![
                    meta.tin.clone().unwrap_or_default(),
                    meta.customername.clone().unwrap_or_default(),
                    meta.billnumber.to_string(),
                    meta.billdate.clone(),
                    format!("{:.2}", round2(invoice_value)),
                    pos_display.clone(),
                    "N".to_string(), // Reverse Charge -- not tracked anywhere in this schema
                    "".to_string(),  // Applicable % of Tax Rate
                    "Regular".to_string(), // no SEZ/deemed-export distinction available
                    "".to_string(),  // E-Commerce GSTIN
                    format_rate(rate),
                    format!("{:.2}", round2(taxable)),
                    "0".to_string(), // Cess Amount
                ]);
            }
        } else if is_interstate && invoice_value > B2C_LARGE_THRESHOLD {
            for &(rate, taxable) in &rates_for_bill {
                b2c_large_rows.push(vec![
                    meta.billnumber.to_string(),
                    meta.billdate.clone(),
                    format!("{:.2}", round2(invoice_value)),
                    pos_display.clone(),
                    format_rate(rate),
                    format!("{:.2}", round2(taxable)),
                    "0".to_string(), // Cess Amount
                    "".to_string(),  // E-Commerce GSTIN
                ]);
            }
        } else {
            for &(rate, taxable) in &rates_for_bill {
                match b2c_small.iter_mut().find(|(c, r, _)| c == &pos_code && *r == rate) {
                    Some((_, _, t)) => *t += taxable,
                    None => b2c_small.push((pos_code.clone(), rate, taxable)),
                }
            }
        }
    }

    // Sort for deterministic output (Python relies on dict iteration +
    // sorted() on the b2c_small keys; b2b/b2c_large followed bill_meta's
    // dict order there too, which in CPython 3.7+ is insertion order --
    // we replicate by sorting all three CSVs by invoice number here
    // since Rust's HashMap iteration order isn't insertion-ordered).
    b2b_rows.sort_by(|a, b| a[2].parse::<i64>().unwrap_or(0).cmp(&b[2].parse::<i64>().unwrap_or(0)));
    b2c_large_rows.sort_by(|a, b| a[0].parse::<i64>().unwrap_or(0).cmp(&b[0].parse::<i64>().unwrap_or(0)));
    needs_review_rows.sort_by(|a, b| a[0].parse::<i64>().unwrap_or(0).cmp(&b[0].parse::<i64>().unwrap_or(0)));

    b2c_small.sort_by(|a, b| (a.0.clone(), a.1.to_bits()).cmp(&(b.0.clone(), b.1.to_bits())));
    let b2c_small_rows: Vec<Vec<String>> = b2c_small
        .iter()
        .map(|(code, rate, taxable)| {
            let is_inter = code != &shop_stcode;
            let pos_name = state_codes.get(code.as_str()).copied().unwrap_or("");
            let pos_display = if !pos_name.is_empty() { format!("{code}-{pos_name}") } else { code.clone() };
            vec![
                if is_inter { "INTER".to_string() } else { "INTRA".to_string() },
                pos_display,
                format_rate(*rate),
                format!("{:.2}", round2(*taxable)),
                "0".to_string(),
            ]
        })
        .collect();

    let summary = Gstr1ExportSummary {
        b2b_count: b2b_rows.len(),
        b2c_large_count: b2c_large_rows.len(),
        b2c_small_count: b2c_small_rows.len(),
        needs_review_count: needs_review_rows.len(),
    };

    let buf = Cursor::new(Vec::new());
    let mut zw = ZipWriter::new(buf);

    write_csv_to_zip(
        &mut zw,
        "b2b.csv",
        &["GSTIN/UIN of Recipient", "Receiver Name", "Invoice Number", "Invoice date",
          "Invoice Value", "Place Of Supply", "Reverse Charge", "Applicable % of Tax Rate",
          "Invoice Type", "E-Commerce GSTIN", "Rate", "Taxable Value", "Cess Amount"],
        &b2b_rows,
    )?;
    write_csv_to_zip(
        &mut zw,
        "b2c_large.csv",
        &["Invoice Number", "Invoice date", "Invoice Value", "Place Of Supply", "Rate",
          "Taxable Value", "Cess Amount", "E-Commerce GSTIN"],
        &b2c_large_rows,
    )?;
    write_csv_to_zip(
        &mut zw,
        "b2c_small.csv",
        &["Type", "Place Of Supply", "Rate", "Taxable Value", "Cess Amount"],
        &b2c_small_rows,
    )?;
    write_csv_to_zip(
        &mut zw,
        "needs_review_place_of_supply_unknown.csv",
        &["Invoice Number", "Invoice date", "Invoice Value", "Customer", "GSTIN on bill", "Reason"],
        &needs_review_rows,
    )?;

    let cursor = zw.finish().map_err(|e| e.to_string())?;
    std::fs::write(&save_path, cursor.into_inner()).map_err(|e| e.to_string())?;

    Ok(summary)
}

fn pad2(s: &str) -> String {
    if s.len() >= 2 { s.to_string() } else { format!("{s:0>2}") }
}

/// GST_RATES are always whole-number percentages in this schema (0, 5,
/// 12, 18, 28) -- format without a trailing ".0" to match what a human
/// filing GSTR-1 expects to see in the Rate column.
fn format_rate(rate: f64) -> String {
    if rate.fract() == 0.0 { format!("{}", rate as i64) } else { format!("{rate}") }
}

// ---------------------------------------------------------------------
// Profit report
// ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ProfitLine {
    pub bill_number: i64,
    pub bill_date: String,
    pub customer_name: Option<String>,
    pub product_code: String,
    pub product_name: String,
    pub qty: f64,
    pub revenue: f64,
    pub cost: f64,
    pub profit: f64,
}

#[derive(Debug, Serialize)]
pub struct FlaggedLine {
    pub bill_number: i64,
    pub bill_date: String,
    pub customer_name: Option<String>,
    pub product_code: String,
    pub product_name: String,
    pub qty: f64,
    pub revenue: f64,
    pub reason: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProductProfitSummary {
    pub product_code: String,
    pub product_name: String,
    pub qty: f64,
    pub revenue: f64,
    pub cost: f64,
    pub profit: f64,
}

#[derive(Debug, Serialize)]
pub struct ProfitReportResult {
    pub date_from: String,
    pub date_to: String,
    pub lines: Vec<ProfitLine>,
    pub flagged_lines: Vec<FlaggedLine>,
    pub flagged_revenue: f64,
    pub product_summary: Vec<ProductProfitSummary>,
    pub total_revenue: f64,
    pub total_cost: f64,
    pub total_profit: f64,
}

#[tauri::command]
pub fn profit_report(
    db: State<PlatformDb>,
    session: State<SessionState>,
    date_from: Option<String>,
    date_to: Option<String>,
) -> Result<ProfitReportResult, String> {
    let (admin, _) = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let from = date_from.unwrap_or_default();
    let to = end_of_day(&date_to.unwrap_or_default());
    let admin_str = admin.to_string();

    // LEFT JOIN vm_products deliberately does NOT filter by p.user_id --
    // matches Python's reports.py exactly (see module doc).
    let mut stmt = conn
        .prepare(
            "SELECT bi.bi_productid, bi.bi_quantity, bi.bi_total, bi.bi_purprice, \
                    be.be_billnumber, be.be_billdate, be.be_customername, \
                    p.pr_productcode, p.pr_productname \
             FROM vm_billitems bi \
             JOIN vm_billentry be ON be.be_billid = bi.bi_billid \
             LEFT JOIN vm_products p ON p.pr_productid = bi.bi_productid \
             WHERE bi.user_id = ?1 AND bi.bi_isactive = 0 AND be.be_isactive = 0 \
               AND (?2 = '' OR be.be_billdate >= ?2) \
               AND (?3 = '' OR be.be_billdate <= ?3) \
             ORDER BY be.be_billdate ASC, be.be_billid ASC",
        )
        .map_err(|e| e.to_string())?;

    struct RawRow {
        product_id: i64,
        qty: f64,
        revenue: f64,
        raw_purprice: Option<f64>,
        billnumber: i64,
        billdate: String,
        customername: Option<String>,
        product_code: Option<String>,
        product_name: Option<String>,
    }

    let rows: Vec<RawRow> = stmt
        .query_map(params![admin_str, from.trim(), to], |r| {
            Ok(RawRow {
                product_id: r.get::<_, Option<i64>>(0)?.unwrap_or(0),
                qty: r.get::<_, Option<f64>>(1)?.unwrap_or(0.0),
                revenue: r.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                raw_purprice: r.get(3)?,
                billnumber: r.get(4)?,
                billdate: r.get(5)?,
                customername: r.get(6)?,
                product_code: r.get(7)?,
                product_name: r.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut lines: Vec<ProfitLine> = Vec::new();
    let mut flagged_lines: Vec<FlaggedLine> = Vec::new();
    let mut by_product: HashMap<i64, ProductProfitSummary> = HashMap::new();
    let mut total_revenue = 0.0_f64;
    let mut total_cost = 0.0_f64;
    let mut flagged_revenue = 0.0_f64;

    for r in rows {
        let product_name_display = r
            .product_name
            .clone()
            .unwrap_or_else(|| format!("(deleted product #{})", r.product_id));
        let product_code_display = r.product_code.clone().unwrap_or_default();

        // A missing (NULL) or explicit-zero cost basis is NOT the same
        // as "this sale genuinely cost nothing" -- billing.rs should
        // always snapshot a real pr_purchaseprice at sale time, so
        // either case here means the underlying data is suspect, not
        // that profit is really 100% of revenue. Excluded from totals
        // rather than counted at cost=0; surfaced separately instead.
        let is_flagged = r.raw_purprice.is_none() || r.raw_purprice == Some(0.0);
        if is_flagged {
            flagged_revenue += r.revenue;
            flagged_lines.push(FlaggedLine {
                bill_number: r.billnumber,
                bill_date: r.billdate,
                customer_name: r.customername,
                product_code: product_code_display,
                product_name: product_name_display,
                qty: r.qty,
                revenue: r.revenue,
                reason: if r.raw_purprice.is_none() { "No cost recorded" } else { "Cost recorded as zero" }
                    .to_string(),
            });
            continue;
        }

        // bi_purprice is the purchase-price-per-unit snapshot taken at
        // sale time (see billing.rs checkout) -- using it instead of a
        // live join to vm_products.pr_purchaseprice is deliberate: a
        // later cost-price edit shouldn't rewrite a historical bill's
        // profit.
        let cost = r.raw_purprice.unwrap() * r.qty;
        let profit = r.revenue - cost;

        total_revenue += r.revenue;
        total_cost += cost;

        let bucket = by_product.entry(r.product_id).or_insert(ProductProfitSummary {
            product_code: product_code_display.clone(),
            product_name: product_name_display.clone(),
            qty: 0.0,
            revenue: 0.0,
            cost: 0.0,
            profit: 0.0,
        });
        bucket.qty += r.qty;
        bucket.revenue += r.revenue;
        bucket.cost += cost;
        bucket.profit += profit;

        lines.push(ProfitLine {
            bill_number: r.billnumber,
            bill_date: r.billdate,
            customer_name: r.customername,
            product_code: product_code_display,
            product_name: product_name_display,
            qty: r.qty,
            revenue: r.revenue,
            cost,
            profit,
        });
    }

    let mut product_summary: Vec<ProductProfitSummary> = by_product.into_values().collect();
    product_summary.sort_by(|a, b| b.profit.partial_cmp(&a.profit).unwrap());

    Ok(ProfitReportResult {
        date_from: from,
        date_to: date_to_display(&to),
        lines,
        flagged_lines,
        flagged_revenue,
        product_summary,
        total_revenue,
        total_cost,
        total_profit: total_revenue - total_cost,
    })
}

// ---------------------------------------------------------------------
// Customer report (balance summary + optional single-customer statement)
// ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct CustomerBalanceRow {
    pub cs_customerid: i64,
    pub cs_customername: Option<String>,
    pub cs_customerphone: Option<String>,
    pub cs_balance: f64,
    pub expected_balance: f64,
    pub reconciled: bool,
}

#[derive(Debug, Serialize)]
pub struct BillRow {
    pub be_billid: i64,
    pub be_billnumber: i64,
    pub be_billdate: String,
    pub be_gtotal: String,
    pub be_paidamount: f64,
    pub be_balance: f64,
}

#[derive(Debug, Serialize)]
pub struct SalesReturnRow {
    pub sre_billid: i64,
    pub sre_billnumber: i64,
    pub sre_rebill: Option<String>,
    pub sre_billdate: String,
    pub sre_gtotal: String,
    pub sre_paidamount: f64,
    pub sre_balance: f64,
}

#[derive(Debug, Serialize)]
pub struct CustomerLedgerRow {
    pub tr_id: i64,
    pub tr_date: String,
    pub tr_particulars: String,
    pub tr_transactionamount: f64,
    pub tr_closingbalance: f64,
    pub pay_type: String,
}

#[derive(Debug, Serialize)]
pub struct CustomerReportResult {
    pub date_from: String,
    pub date_to: String,
    pub customers: Vec<CustomerBalanceRow>,
    pub total_outstanding: f64,
    pub mismatch_count: i64,
    pub selected_customer: Option<CustomerBalanceRow>,
    pub bills: Vec<BillRow>,
    pub returns: Vec<SalesReturnRow>,
    pub ledger_entries: Vec<CustomerLedgerRow>,
}

#[tauri::command]
pub fn customer_report(
    db: State<PlatformDb>,
    session: State<SessionState>,
    date_from: Option<String>,
    date_to: Option<String>,
    customer_id: Option<i64>,
) -> Result<CustomerReportResult, String> {
    let (admin, _) = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let from = date_from.unwrap_or_default();
    let to = end_of_day(&date_to.unwrap_or_default());

    // vm_customer.user_id is INTEGER -- bind admin directly.
    let mut stmt = conn
        .prepare(
            "SELECT cs_customerid, cs_customername, cs_customerphone, cs_balance \
             FROM vm_customer WHERE user_id = ?1 AND cs_isactive = 0 ORDER BY cs_customername",
        )
        .map_err(|e| e.to_string())?;
    let mut customers: Vec<CustomerBalanceRow> = stmt
        .query_map(params![admin], |r| {
            Ok(CustomerBalanceRow {
                cs_customerid: r.get(0)?,
                cs_customername: r.get(1)?,
                cs_customerphone: r.get(2)?,
                cs_balance: r.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
                expected_balance: 0.0,
                reconciled: true,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Reconciliation check (read-only -- flags a mismatch, never
    // corrects cs_balance; that write path belongs to billing.rs/
    // sales_returns.rs, not this report).
    //   expected_balance = unpaid bills - unresolved sales-return credit
    // KNOWN SCOPE LIMIT, same as Python: this does NOT account for
    // standalone Vouchers (accounts.rs) adjusting a customer's balance
    // directly -- a customer whose balance was adjusted by a direct
    // voucher will show as a "mismatch" here even though nothing is
    // actually wrong. That's a false positive this check can produce
    // by design, not a bug in the check itself.

    // vm_billentry.user_id is TEXT -- bind admin.to_string() (different
    // from the vm_customer query just above -- see module doc).
    let admin_str = admin.to_string();
    let mut unpaid_stmt = conn
        .prepare(
            "SELECT be_customerid, COALESCE(SUM(be_balance), 0) AS total \
             FROM vm_billentry WHERE user_id = ?1 AND be_isactive = 0 AND be_customerid > 0 \
             GROUP BY be_customerid",
        )
        .map_err(|e| e.to_string())?;
    let unpaid_by_customer: HashMap<i64, f64> = unpaid_stmt
        .query_map(params![admin_str], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // vm_salreturnentry.user_id is INTEGER -- bind admin directly.
    // sre_supplierid is a misnamed column that actually holds the
    // customer id (same shape as Purchase Returns' pre_customerid
    // holding the supplier id) -- not a typo here, matches
    // sales_returns.rs's own module doc.
    let mut credit_stmt = conn
        .prepare(
            "SELECT sre_supplierid, COALESCE(SUM(sre_balance), 0) AS total \
             FROM vm_salreturnentry WHERE user_id = ?1 AND sre_isactive = 0 AND sre_supplierid > 0 \
             GROUP BY sre_supplierid",
        )
        .map_err(|e| e.to_string())?;
    let credit_by_customer: HashMap<i64, f64> = credit_stmt
        .query_map(params![admin], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    for c in customers.iter_mut() {
        let expected = round2(
            unpaid_by_customer.get(&c.cs_customerid).copied().unwrap_or(0.0)
                - credit_by_customer.get(&c.cs_customerid).copied().unwrap_or(0.0),
        );
        let actual = round2(c.cs_balance);
        c.expected_balance = expected;
        c.reconciled = (expected - actual).abs() < 0.01;
    }

    let total_outstanding: f64 = customers.iter().map(|c| c.cs_balance).sum();
    let mismatch_count = customers.iter().filter(|c| !c.reconciled).count() as i64;

    let mut selected_customer: Option<CustomerBalanceRow> = None;
    let mut bills: Vec<BillRow> = Vec::new();
    let mut returns: Vec<SalesReturnRow> = Vec::new();
    let mut ledger_entries: Vec<CustomerLedgerRow> = Vec::new();

    if let Some(cid) = customer_id {
        let found: Option<CustomerBalanceRow> = conn
            .query_row(
                "SELECT cs_customerid, cs_customername, cs_customerphone, cs_balance \
                 FROM vm_customer WHERE cs_customerid = ?1 AND user_id = ?2",
                params![cid, admin],
                |r| {
                    Ok(CustomerBalanceRow {
                        cs_customerid: r.get(0)?,
                        cs_customername: r.get(1)?,
                        cs_customerphone: r.get(2)?,
                        cs_balance: r.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
                        expected_balance: 0.0,
                        reconciled: true,
                    })
                },
            )
            .optional()
            .map_err(|e| e.to_string())?;

        if let Some(mut sc) = found {
            // vm_billentry.user_id TEXT -- bind admin.to_string().
            let mut bill_stmt = conn
                .prepare(
                    "SELECT be_billid, be_billnumber, be_billdate, be_gtotal, be_paidamount, be_balance \
                     FROM vm_billentry \
                     WHERE user_id = ?1 AND be_customerid = ?2 AND be_isactive = 0 \
                       AND (?3 = '' OR be_billdate >= ?3) \
                       AND (?4 = '' OR be_billdate <= ?4) \
                     ORDER BY be_billdate ASC, be_billid ASC",
                )
                .map_err(|e| e.to_string())?;
            bills = bill_stmt
                .query_map(params![admin_str, cid, from.trim(), to], |r| {
                    Ok(BillRow {
                        be_billid: r.get(0)?,
                        be_billnumber: r.get(1)?,
                        be_billdate: r.get(2)?,
                        be_gtotal: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                        be_paidamount: r.get::<_, Option<f64>>(4)?.unwrap_or(0.0),
                        be_balance: r.get::<_, Option<f64>>(5)?.unwrap_or(0.0),
                    })
                })
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();

            // vm_salreturnentry.user_id INTEGER -- bind admin directly.
            let mut return_stmt = conn
                .prepare(
                    "SELECT sre_billid, sre_billnumber, sre_rebill, sre_billdate, \
                            sre_gtotal, sre_paidamount, sre_balance \
                     FROM vm_salreturnentry \
                     WHERE user_id = ?1 AND sre_supplierid = ?2 AND sre_isactive = 0 \
                       AND (?3 = '' OR sre_billdate >= ?3) \
                       AND (?4 = '' OR sre_billdate <= ?4) \
                     ORDER BY sre_billdate ASC, sre_billid ASC",
                )
                .map_err(|e| e.to_string())?;
            returns = return_stmt
                .query_map(params![admin, cid, from.trim(), to], |r| {
                    Ok(SalesReturnRow {
                        sre_billid: r.get(0)?,
                        sre_billnumber: r.get(1)?,
                        sre_rebill: r.get(2)?,
                        sre_billdate: r.get(3)?,
                        sre_gtotal: r.get::<_, Option<String>>(4)?.unwrap_or_default(),
                        sre_paidamount: r.get::<_, Option<f64>>(5)?.unwrap_or(0.0),
                        sre_balance: r.get::<_, Option<f64>>(6)?.unwrap_or(0.0),
                    })
                })
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();

            // vm_transaction.user_id is INTEGER, but cus_id is TEXT even
            // though vm_customer's own PK is INTEGER -- verified
            // directly against schema.sql, same mixed-type gotcha as
            // elsewhere in this schema.
            let mut ledger_stmt = conn
                .prepare(
                    "SELECT tr_id, tr_date, tr_particulars, tr_transactionamount, \
                            tr_closingbalance, pay_type \
                     FROM vm_transaction \
                     WHERE user_id = ?1 AND cus_id = ?2 AND tr_isactive = 0 \
                       AND (?3 = '' OR tr_date >= ?3) \
                       AND (?4 = '' OR tr_date <= ?4) \
                     ORDER BY tr_id ASC",
                )
                .map_err(|e| e.to_string())?;
            ledger_entries = ledger_stmt
                .query_map(params![admin, cid.to_string(), from.trim(), to], |r| {
                    Ok(CustomerLedgerRow {
                        tr_id: r.get(0)?,
                        tr_date: r.get(1)?,
                        tr_particulars: r.get(2)?,
                        tr_transactionamount: r.get(3)?,
                        tr_closingbalance: r.get(4)?,
                        pay_type: r.get(5)?,
                    })
                })
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();

            let expected = round2(
                unpaid_by_customer.get(&cid).copied().unwrap_or(0.0)
                    - credit_by_customer.get(&cid).copied().unwrap_or(0.0),
            );
            sc.expected_balance = expected;
            sc.reconciled = (expected - round2(sc.cs_balance)).abs() < 0.01;
            selected_customer = Some(sc);
        }
    }

    Ok(CustomerReportResult {
        date_from: from,
        date_to: date_to_display(&to),
        customers,
        total_outstanding,
        mismatch_count,
        selected_customer,
        bills,
        returns,
        ledger_entries,
    })
}
