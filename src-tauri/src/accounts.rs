//! accounts.rs
//! -----------
//! Port of `routes/accounts.py`'s Vouchers/Ledger/Daybook trio -- Phase
//! 2 of the roadmap. Built around three tables:
//!
//!     vm_transaction              -- a running-balance cash book (the Ledger)
//!     administrator_daybook       -- paired debit/credit journal entries (the Daybook)
//!     administrator_account_name  -- a small chart of accounts (Cash,
//!                                    Bank, Sales, Rent, ...) used as the
//!                                    "other side" of a voucher when it
//!                                    isn't a customer or supplier
//!
//! How a Voucher ties everything together: entering ONE voucher (a
//! receipt or a payment) writes THREE things in a single transaction:
//!   1. One row in vm_transaction -- moves the running cash/bank balance
//!      up (receipt) or down (payment).
//!   2. One row in administrator_daybook -- records which account was
//!      debited and which was credited.
//!   3. If a customer or supplier was linked, their balance is adjusted.
//!
//! Balance convention (confirmed decision, carried over exactly from
//! Python): linking a voucher to a customer/supplier always REDUCES the
//! magnitude of what they owe -- a receipt from a customer reduces
//! cs_balance, a payment to a supplier reduces rs_balance. This is the
//! OPPOSITE direction from billing::checkout's credit-sale handling,
//! where an unpaid balance INCREASES cs_balance -- both are correct for
//! what they represent (settling a debt vs creating one).
//!
//! Vouchers are add-only (no edit/delete) once saved, same as Python.
//!
//! Confirmed Phase 2 decision differing from Python: `administrator_
//! account_name` (the chart of accounts) is now scoped per shop via a
//! `user_id` column added in schema.rs's SCHEMA_MIGRATIONS -- Python
//! shared one global list because each shop had its own separate app
//! install; a single multi-shop login can't rely on that anymore. The
//! table's existing-but-unused `acnt_branch` TEXT column was left alone
//! rather than repurposed, to match the `user_id INTEGER` convention
//! every other per-shop table already uses (vm_transaction included).
//! `administrator_daybook` did NOT need a migration -- its `ad_branchid`
//! TEXT column already does real per-shop scoping in Python (holds
//! `str(user_id)`), just under a table-specific name instead of
//! `user_id`.
//!
//! `post_ledger_entry`/`post_daybook_entry` are `pub` so billing.rs's
//! `checkout()` can call them directly (auto-posting a sale's cash
//! portion, matching Python's billing.py) -- both take `&Connection`,
//! which accepts a `&rusqlite::Transaction` too via Deref coercion, so
//! checkout's existing transaction covers the ledger/daybook writes
//! atomically along with the bill itself.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

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
    check_shop_type("accounts", &s.shop_type)?;
    let admin = s.active_shop_id.ok_or_else(|| "no_shop_selected".to_string())?;
    Ok((admin, s.username.clone()))
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

const DEFAULT_ACCOUNTS: [(&str, &str, &str); 11] = [
    ("Cash", "bs", "asset"),
    ("Bank", "bs", "asset"),
    ("Sales", "pl", "credit"),
    ("Purchase", "pl", "debit"),
    ("Rent", "pl", "debit"),
    ("Salary", "pl", "debit"),
    ("Electricity", "pl", "debit"),
    ("Capital", "bs", "liability"),
    ("Drawings", "bs", "asset"),
    ("Other Income", "pl", "credit"),
    ("Other Expense", "pl", "debit"),
];

/// Seeds this shop's chart of accounts on first use, so the voucher
/// form isn't a blank dropdown. Safe to call every request -- a no-op
/// once accounts already exist for this shop.
fn ensure_default_accounts(conn: &Connection, user_id: i64) -> Result<(), String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM administrator_account_name WHERE user_id = ?1",
            params![user_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if count == 0 {
        for (name, head, group) in DEFAULT_ACCOUNTS {
            conn.execute(
                "INSERT INTO administrator_account_name
                    (acc_name, acc_head, group_head, act_group_head,
                     opening_balance, closing_balance, user_id)
                 VALUES (?1, ?2, ?3, ?3, 0, 0, ?4)",
                params![name, head, group, user_id],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Insert one row into vm_transaction (the Ledger), chaining off the
/// last known closing balance for this shop. Returns the new closing
/// balance. The single place that touches the running cash/bank
/// balance -- used by add_voucher below AND by billing::checkout when a
/// sale is completed with cash actually changing hands.
///
/// `signed_amount`: positive for money in, negative for money out.
/// `mode`: "Cash" or "Bank" -- stored as tr_mode (0/1) and pay_type.
pub fn post_ledger_entry(
    conn: &Connection,
    user_id: i64,
    particulars: &str,
    signed_amount: f64,
    date: &str,
    txn_type: &str,
    mode: &str,
    party_id: &str,
    party_name: &str,
) -> Result<f64, String> {
    let opening_balance: f64 = conn
        .query_row(
            "SELECT tr_closingbalance FROM vm_transaction WHERE user_id = ?1 ORDER BY tr_id DESC LIMIT 1",
            params![user_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or(0.0);
    let closing_balance = round2(opening_balance + signed_amount);
    let now = chrono_now();
    let full_date = format!("{} {}", date, &now[11..]);

    conn.execute(
        "INSERT INTO vm_transaction
            (tr_billid, tr_particulars, tr_openingbalance, tr_transactionamount,
             tr_closingbalance, tr_date, tr_transactiontype, tr_isactive,
             tr_updateddate, user_id, tr_mode, pay_type, cus_id, tr_name)
         VALUES (0, ?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            particulars,
            opening_balance,
            signed_amount,
            closing_balance,
            full_date,
            txn_type,
            now,
            user_id,
            if mode == "Bank" { 1 } else { 0 },
            mode,
            party_id,
            party_name,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(closing_balance)
}

/// Insert one paired debit/credit row into administrator_daybook (the
/// Daybook). Same "single place" reasoning as post_ledger_entry.
pub fn post_daybook_entry(
    conn: &Connection,
    user_id: i64,
    date: &str,
    debit: &str,
    credit: &str,
    amount: f64,
    description: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO administrator_daybook
            (ad_branchid, dayBookDate, debit, credit, dayBookContra,
             dayBookAmount, description, status, backup, billid)
         VALUES (?1, ?2, ?3, ?4, 'Y', ?5, ?6, '', '', '')",
        params![user_id.to_string(), date, debit, credit, amount, description],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------
// Vouchers (the entry screen -- Receipt / Payment)
// ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct AccountOption {
    pub refid: i64,
    pub acc_name: String,
}

#[derive(Debug, Serialize)]
pub struct PartyOption {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct DaybookEntry {
    pub refid: i64,
    pub day_book_date: String,
    pub debit: String,
    pub credit: String,
    pub day_book_amount: f64,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct VouchersInit {
    pub accounts: Vec<AccountOption>,
    pub customers: Vec<PartyOption>,
    pub suppliers: Vec<PartyOption>,
    pub recent: Vec<DaybookEntry>,
    pub today: String,
}

#[tauri::command]
pub fn accounts_init(db: State<PlatformDb>, session: State<SessionState>) -> Result<VouchersInit, String> {
    let (admin, _username) = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    ensure_default_accounts(&conn, admin)?;

    let mut acc_stmt = conn
        .prepare("SELECT refid, acc_name FROM administrator_account_name WHERE user_id = ?1 ORDER BY acc_name")
        .map_err(|e| e.to_string())?;
    let accounts: Vec<AccountOption> = acc_stmt
        .query_map(params![admin], |r| {
            Ok(AccountOption { refid: r.get(0)?, acc_name: r.get(1)? })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut cust_stmt = conn
        .prepare(
            "SELECT cs_customerid, cs_customername FROM vm_customer
             WHERE user_id = ?1 AND cs_isactive = 0 ORDER BY cs_customername",
        )
        .map_err(|e| e.to_string())?;
    let customers: Vec<PartyOption> = cust_stmt
        .query_map(params![admin], |r| Ok(PartyOption { id: r.get(0)?, name: r.get(1)? }))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut sup_stmt = conn
        .prepare(
            "SELECT rs_supplierid, rs_company_name FROM vm_supplier
             WHERE user_id = ?1 AND rs_isactive = 0 ORDER BY rs_company_name",
        )
        .map_err(|e| e.to_string())?;
    let suppliers: Vec<PartyOption> = sup_stmt
        .query_map(params![admin.to_string()], |r| Ok(PartyOption { id: r.get(0)?, name: r.get(1)? }))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut recent_stmt = conn
        .prepare(
            "SELECT refid, dayBookDate, debit, credit, dayBookAmount, description
             FROM administrator_daybook WHERE ad_branchid = ?1 ORDER BY refid DESC LIMIT 25",
        )
        .map_err(|e| e.to_string())?;
    let recent: Vec<DaybookEntry> = recent_stmt
        .query_map(params![admin.to_string()], |r| {
            Ok(DaybookEntry {
                refid: r.get(0)?,
                day_book_date: r.get(1)?,
                debit: r.get(2)?,
                credit: r.get(3)?,
                day_book_amount: r.get(4)?,
                description: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let now = chrono_now();
    let today = now[..10].to_string();

    Ok(VouchersInit { accounts, customers, suppliers, recent, today })
}

#[derive(Debug, Deserialize)]
pub struct VoucherInput {
    pub voucher_type: String, // "receipt" | "payment"
    #[serde(default = "default_mode")]
    pub mode: String, // "Cash" | "Bank"
    pub amount: f64,
    #[serde(default)]
    pub date: String,
    #[serde(default = "default_party_type")]
    pub party_type: String, // "none" | "customer" | "supplier"
    pub party_id: Option<i64>,
    #[serde(default)]
    pub account_name: String,
    #[serde(default)]
    pub description: String,
}
fn default_mode() -> String {
    "Cash".to_string()
}
fn default_party_type() -> String {
    "none".to_string()
}

#[derive(Debug, Serialize)]
pub struct VoucherResult {
    pub closing_balance: f64,
}

#[tauri::command]
pub fn add_voucher(
    db: State<PlatformDb>,
    session: State<SessionState>,
    input: VoucherInput,
) -> Result<VoucherResult, String> {
    let (admin, _username) = guard(&session)?;

    if input.voucher_type != "receipt" && input.voucher_type != "payment" {
        return Err("Invalid voucher type.".into());
    }
    if input.amount <= 0.0 {
        return Err("Amount must be greater than zero.".into());
    }
    let date = if input.date.trim().is_empty() {
        let now = chrono_now();
        now[..10].to_string()
    } else {
        input.date.trim().to_string()
    };

    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // --- Figure out who the "other side" of the entry is ---
    let mut party_name = input.account_name.trim().to_string();
    if input.party_type == "customer" {
        if let Some(pid) = input.party_id {
            let name: Option<String> = tx
                .query_row(
                    "SELECT cs_customername FROM vm_customer WHERE cs_customerid = ?1 AND user_id = ?2",
                    params![pid, admin],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            party_name = name.ok_or_else(|| "Selected customer not found.".to_string())?;
        }
    } else if input.party_type == "supplier" {
        if let Some(pid) = input.party_id {
            let name: Option<String> = tx
                .query_row(
                    "SELECT rs_company_name FROM vm_supplier WHERE rs_supplierid = ?1 AND user_id = ?2",
                    params![pid, admin.to_string()],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            party_name = name.ok_or_else(|| "Selected supplier not found.".to_string())?;
        }
    }

    if party_name.is_empty() {
        return Err("Please select a customer/supplier or choose an account.".into());
    }

    // --- 1. Ledger: running cash/bank balance (vm_transaction) ---
    let signed_amount = if input.voucher_type == "receipt" { input.amount } else { -input.amount };
    let mut particulars = format!(
        "{} {}",
        if input.voucher_type == "receipt" { "Received from" } else { "Paid to" },
        party_name
    );
    if !input.description.trim().is_empty() {
        particulars.push_str(" -- ");
        particulars.push_str(input.description.trim());
    }

    let party_id_str = input.party_id.map(|p| p.to_string()).unwrap_or_default();
    let closing_balance = post_ledger_entry(
        &tx,
        admin,
        &particulars,
        signed_amount,
        &date,
        if input.voucher_type == "receipt" { "income" } else { "expense" },
        &input.mode,
        &party_id_str,
        &party_name,
    )?;

    // --- 2. Daybook: paired debit/credit journal entry ---
    let (debit_acct, credit_acct) = if input.voucher_type == "receipt" {
        (input.mode.clone(), party_name.clone())
    } else {
        (party_name.clone(), input.mode.clone())
    };
    let description = if input.description.trim().is_empty() {
        particulars.clone()
    } else {
        input.description.trim().to_string()
    };
    post_daybook_entry(&tx, admin, &date, &debit_acct, &credit_acct, input.amount, &description)?;

    // --- 3. Adjust the linked party's balance, if any (settling a debt
    // -- always reduces the magnitude of what they owe, see module doc) ---
    if input.party_type == "customer" {
        if let Some(pid) = input.party_id {
            tx.execute(
                "UPDATE vm_customer SET cs_balance = cs_balance - ?1 WHERE cs_customerid = ?2 AND user_id = ?3",
                params![input.amount, pid, admin],
            )
            .map_err(|e| e.to_string())?;
        }
    } else if input.party_type == "supplier" {
        if let Some(pid) = input.party_id {
            tx.execute(
                "UPDATE vm_supplier SET rs_balance = CAST(rs_balance AS REAL) - ?1 WHERE rs_supplierid = ?2 AND user_id = ?3",
                params![input.amount, pid, admin.to_string()],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;

    Ok(VoucherResult { closing_balance })
}

// ---------------------------------------------------------------------
// Ledger (running cash/bank balance -- vm_transaction)
// ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct LedgerEntry {
    pub tr_id: i64,
    pub tr_particulars: String,
    pub tr_openingbalance: f64,
    pub tr_transactionamount: f64,
    pub tr_closingbalance: f64,
    pub tr_date: String,
    pub tr_transactiontype: String,
    pub tr_mode: i64,
    pub pay_type: String,
    pub cus_id: String,
    pub tr_name: String,
}

#[derive(Debug, Serialize)]
pub struct LedgerData {
    pub entries: Vec<LedgerEntry>, // newest first, matching Python's display order
    pub current_balance: f64,
    pub total_in: f64,
    pub total_out: f64,
}

#[tauri::command]
pub fn list_ledger(
    db: State<PlatformDb>,
    session: State<SessionState>,
    date_from: Option<String>,
    date_to: Option<String>,
) -> Result<LedgerData, String> {
    let (admin, _username) = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let from = date_from.unwrap_or_default();
    let to_raw = date_to.unwrap_or_default();
    let to = if to_raw.trim().is_empty() {
        String::new()
    } else {
        format!("{} 23:59:59", to_raw.trim())
    };

    let mut stmt = conn
        .prepare(
            "SELECT tr_id, tr_particulars, tr_openingbalance, tr_transactionamount,
                    tr_closingbalance, tr_date, tr_transactiontype, tr_mode, pay_type,
                    cus_id, tr_name
             FROM vm_transaction
             WHERE user_id = ?1 AND tr_isactive = 0
               AND (?2 = '' OR tr_date >= ?2)
               AND (?3 = '' OR tr_date <= ?3)
             ORDER BY tr_id ASC",
        )
        .map_err(|e| e.to_string())?;

    let mut entries: Vec<LedgerEntry> = stmt
        .query_map(params![admin, from.trim(), to], |r| {
            Ok(LedgerEntry {
                tr_id: r.get(0)?,
                tr_particulars: r.get(1)?,
                tr_openingbalance: r.get(2)?,
                tr_transactionamount: r.get(3)?,
                tr_closingbalance: r.get(4)?,
                tr_date: r.get(5)?,
                tr_transactiontype: r.get(6)?,
                tr_mode: r.get(7)?,
                pay_type: r.get(8)?,
                cus_id: r.get(9)?,
                tr_name: r.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let current_balance: f64 = conn
        .query_row(
            "SELECT tr_closingbalance FROM vm_transaction WHERE user_id = ?1 ORDER BY tr_id DESC LIMIT 1",
            params![admin],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or(0.0);

    let total_in: f64 = entries
        .iter()
        .filter(|e| e.tr_transactionamount > 0.0)
        .map(|e| e.tr_transactionamount)
        .sum();
    let total_out: f64 = entries
        .iter()
        .filter(|e| e.tr_transactionamount < 0.0)
        .map(|e| -e.tr_transactionamount)
        .sum();

    entries.reverse(); // newest first for display, matching Python

    Ok(LedgerData {
        entries,
        current_balance,
        total_in: round2(total_in),
        total_out: round2(total_out),
    })
}

// ---------------------------------------------------------------------
// Daybook (paired debit/credit journal -- administrator_daybook)
// ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct DaybookData {
    pub entries: Vec<DaybookEntry>,
    pub total_amount: f64,
}

#[tauri::command]
pub fn list_daybook(
    db: State<PlatformDb>,
    session: State<SessionState>,
    date_from: Option<String>,
    date_to: Option<String>,
) -> Result<DaybookData, String> {
    let (admin, _username) = guard(&session)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let from = date_from.unwrap_or_default();
    let to = date_to.unwrap_or_default();

    let mut stmt = conn
        .prepare(
            "SELECT refid, dayBookDate, debit, credit, dayBookAmount, description
             FROM administrator_daybook
             WHERE ad_branchid = ?1
               AND (?2 = '' OR dayBookDate >= ?2)
               AND (?3 = '' OR dayBookDate <= ?3)
             ORDER BY refid DESC",
        )
        .map_err(|e| e.to_string())?;

    let entries: Vec<DaybookEntry> = stmt
        .query_map(params![admin.to_string(), from.trim(), to.trim()], |r| {
            Ok(DaybookEntry {
                refid: r.get(0)?,
                day_book_date: r.get(1)?,
                debit: r.get(2)?,
                credit: r.get(3)?,
                day_book_amount: r.get(4)?,
                description: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let total_amount: f64 = round2(entries.iter().map(|e| e.day_book_amount).sum());

    Ok(DaybookData { entries, total_amount })
}
