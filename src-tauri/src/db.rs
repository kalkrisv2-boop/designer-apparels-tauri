use crate::models::{CatalogItem, Invoice, InvoiceLineItem, Settings};
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Db(pub Mutex<Connection>);

pub fn db_path(app_data_dir: &PathBuf) -> PathBuf {
    app_data_dir.join("invoices.db")
}

pub fn init(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            bank_name TEXT NOT NULL DEFAULT '',
            bank_branch TEXT NOT NULL DEFAULT '',
            bank_account_number TEXT NOT NULL DEFAULT '',
            bank_ifsc TEXT NOT NULL DEFAULT '',
            terms_and_conditions TEXT NOT NULL DEFAULT '',
            next_invoice_number INTEGER NOT NULL DEFAULT 2958,
            pdf_output_folder TEXT NOT NULL DEFAULT '',
            mobile_no TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS catalog_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            description TEXT NOT NULL,
            hsn_sac TEXT NOT NULL DEFAULT '6212',
            default_rate REAL NOT NULL DEFAULT 0,
            default_gst_rate REAL NOT NULL DEFAULT 5
        );

        CREATE TABLE IF NOT EXISTS invoices (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            invoice_number INTEGER NOT NULL UNIQUE,
            invoice_date TEXT NOT NULL,
            buyer_name TEXT NOT NULL,
            buyer_address TEXT NOT NULL,
            buyer_gstin TEXT NOT NULL DEFAULT '',
            buyer_state TEXT NOT NULL,
            buyer_state_code TEXT NOT NULL,
            transport_name TEXT NOT NULL DEFAULT '',
            salesman TEXT NOT NULL DEFAULT '',
            is_interstate INTEGER NOT NULL,
            taxable_total REAL NOT NULL,
            cgst_total REAL NOT NULL,
            sgst_total REAL NOT NULL,
            igst_total REAL NOT NULL,
            round_off REAL NOT NULL,
            grand_total REAL NOT NULL,
            amount_in_words TEXT NOT NULL,
            pdf_path TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS invoice_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            invoice_id INTEGER NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
            sr INTEGER NOT NULL,
            description TEXT NOT NULL,
            hsn_sac TEXT NOT NULL,
            size_ratio TEXT NOT NULL DEFAULT '',
            qty REAL NOT NULL,
            rate REAL NOT NULL,
            gst_rate REAL NOT NULL,
            gst_mode TEXT NOT NULL DEFAULT 'exclusive',
            amount REAL NOT NULL
        );
        "#,
    )?;

    // Ensure a single settings row always exists.
    conn.execute(
        "INSERT OR IGNORE INTO settings (id, next_invoice_number) VALUES (1, 2958)",
        [],
    )?;

    Ok(())
}

// --------------------------- Settings ---------------------------

pub fn get_settings(conn: &Connection) -> Result<Settings> {
    conn.query_row(
        "SELECT bank_name, bank_branch, bank_account_number, bank_ifsc,
                terms_and_conditions, next_invoice_number, pdf_output_folder, mobile_no
         FROM settings WHERE id = 1",
        [],
        |row| {
            Ok(Settings {
                bank_name: row.get(0)?,
                bank_branch: row.get(1)?,
                bank_account_number: row.get(2)?,
                bank_ifsc: row.get(3)?,
                terms_and_conditions: row.get(4)?,
                next_invoice_number: row.get(5)?,
                pdf_output_folder: row.get(6)?,
                mobile_no: row.get(7)?,
            })
        },
    )
    .context("failed to load settings")
}

pub fn save_settings(conn: &Connection, s: &Settings) -> Result<()> {
    conn.execute(
        "UPDATE settings SET bank_name = ?1, bank_branch = ?2, bank_account_number = ?3,
            bank_ifsc = ?4, terms_and_conditions = ?5, next_invoice_number = ?6,
            pdf_output_folder = ?7, mobile_no = ?8 WHERE id = 1",
        params![
            s.bank_name,
            s.bank_branch,
            s.bank_account_number,
            s.bank_ifsc,
            s.terms_and_conditions,
            s.next_invoice_number,
            s.pdf_output_folder,
            s.mobile_no
        ],
    )?;
    Ok(())
}

// --------------------------- Catalog ---------------------------

pub fn list_catalog_items(conn: &Connection) -> Result<Vec<CatalogItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, description, hsn_sac, default_rate, default_gst_rate
         FROM catalog_items ORDER BY description ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(CatalogItem {
            id: row.get(0)?,
            description: row.get(1)?,
            hsn_sac: row.get(2)?,
            default_rate: row.get(3)?,
            default_gst_rate: row.get(4)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn upsert_catalog_item(conn: &Connection, item: &CatalogItem) -> Result<i64> {
    match item.id {
        Some(id) => {
            conn.execute(
                "UPDATE catalog_items SET description=?1, hsn_sac=?2, default_rate=?3, default_gst_rate=?4 WHERE id=?5",
                params![item.description, item.hsn_sac, item.default_rate, item.default_gst_rate, id],
            )?;
            Ok(id)
        }
        None => {
            conn.execute(
                "INSERT INTO catalog_items (description, hsn_sac, default_rate, default_gst_rate) VALUES (?1, ?2, ?3, ?4)",
                params![item.description, item.hsn_sac, item.default_rate, item.default_gst_rate],
            )?;
            Ok(conn.last_insert_rowid())
        }
    }
}

pub fn delete_catalog_item(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM catalog_items WHERE id = ?1", params![id])?;
    Ok(())
}

// --------------------------- Invoices ---------------------------

pub fn insert_invoice(conn: &Connection, inv: &Invoice) -> Result<i64> {
    conn.execute(
        "INSERT INTO invoices (
            invoice_number, invoice_date, buyer_name, buyer_address, buyer_gstin,
            buyer_state, buyer_state_code, transport_name, salesman, is_interstate,
            taxable_total, cgst_total, sgst_total, igst_total, round_off, grand_total,
            amount_in_words, pdf_path
        ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
        params![
            inv.invoice_number,
            inv.invoice_date,
            inv.buyer_name,
            inv.buyer_address,
            inv.buyer_gstin,
            inv.buyer_state,
            inv.buyer_state_code,
            inv.transport_name,
            inv.salesman,
            inv.is_interstate as i64,
            inv.taxable_total,
            inv.cgst_total,
            inv.sgst_total,
            inv.igst_total,
            inv.round_off,
            inv.grand_total,
            inv.amount_in_words,
            inv.pdf_path,
        ],
    )?;
    let invoice_id = conn.last_insert_rowid();

    for item in &inv.items {
        conn.execute(
            "INSERT INTO invoice_items (
                invoice_id, sr, description, hsn_sac, size_ratio, qty, rate, gst_rate, gst_mode, amount
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                invoice_id,
                item.sr,
                item.description,
                item.hsn_sac,
                item.size_ratio,
                item.qty,
                item.rate,
                item.gst_rate,
                item.gst_mode,
                item.amount,
            ],
        )?;
    }

    // Advance the invoice counter.
    conn.execute(
        "UPDATE settings SET next_invoice_number = ?1 WHERE id = 1",
        params![inv.invoice_number + 1],
    )?;

    Ok(invoice_id)
}

pub fn update_invoice_pdf_path(conn: &Connection, invoice_id: i64, path: &str) -> Result<()> {
    conn.execute(
        "UPDATE invoices SET pdf_path = ?1 WHERE id = ?2",
        params![path, invoice_id],
    )?;
    Ok(())
}

pub fn list_invoices(conn: &Connection) -> Result<Vec<Invoice>> {
    let mut stmt = conn.prepare(
        "SELECT id, invoice_number, invoice_date, buyer_name, buyer_address, buyer_gstin,
                buyer_state, buyer_state_code, transport_name, salesman, is_interstate,
                taxable_total, cgst_total, sgst_total, igst_total, round_off, grand_total,
                amount_in_words, pdf_path
         FROM invoices ORDER BY invoice_number DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Invoice {
            id: row.get(0)?,
            invoice_number: row.get(1)?,
            invoice_date: row.get(2)?,
            buyer_name: row.get(3)?,
            buyer_address: row.get(4)?,
            buyer_gstin: row.get(5)?,
            buyer_state: row.get(6)?,
            buyer_state_code: row.get(7)?,
            transport_name: row.get(8)?,
            salesman: row.get(9)?,
            is_interstate: {
                let v: i64 = row.get(10)?;
                v != 0
            },
            taxable_total: row.get(11)?,
            cgst_total: row.get(12)?,
            sgst_total: row.get(13)?,
            igst_total: row.get(14)?,
            round_off: row.get(15)?,
            grand_total: row.get(16)?,
            amount_in_words: row.get(17)?,
            pdf_path: row.get(18)?,
            items: vec![], // populated separately when a single invoice is fetched
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_invoice_with_items(conn: &Connection, invoice_id: i64) -> Result<Invoice> {
    let mut inv = conn.query_row(
        "SELECT id, invoice_number, invoice_date, buyer_name, buyer_address, buyer_gstin,
                buyer_state, buyer_state_code, transport_name, salesman, is_interstate,
                taxable_total, cgst_total, sgst_total, igst_total, round_off, grand_total,
                amount_in_words, pdf_path
         FROM invoices WHERE id = ?1",
        params![invoice_id],
        |row| {
            Ok(Invoice {
                id: row.get(0)?,
                invoice_number: row.get(1)?,
                invoice_date: row.get(2)?,
                buyer_name: row.get(3)?,
                buyer_address: row.get(4)?,
                buyer_gstin: row.get(5)?,
                buyer_state: row.get(6)?,
                buyer_state_code: row.get(7)?,
                transport_name: row.get(8)?,
                salesman: row.get(9)?,
                is_interstate: {
                    let v: i64 = row.get(10)?;
                    v != 0
                },
                taxable_total: row.get(11)?,
                cgst_total: row.get(12)?,
                sgst_total: row.get(13)?,
                igst_total: row.get(14)?,
                round_off: row.get(15)?,
                grand_total: row.get(16)?,
                amount_in_words: row.get(17)?,
                pdf_path: row.get(18)?,
                items: vec![],
            })
        },
    )?;

    let mut stmt = conn.prepare(
        "SELECT sr, description, hsn_sac, size_ratio, qty, rate, gst_rate, gst_mode, amount
         FROM invoice_items WHERE invoice_id = ?1 ORDER BY sr ASC",
    )?;
    let items = stmt
        .query_map(params![invoice_id], |row| {
            Ok(InvoiceLineItem {
                sr: row.get(0)?,
                description: row.get(1)?,
                hsn_sac: row.get(2)?,
                size_ratio: row.get(3)?,
                qty: row.get(4)?,
                rate: row.get(5)?,
                gst_rate: row.get(6)?,
                gst_mode: row.get(7)?,
                amount: row.get(8)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    inv.items = items;
    Ok(inv)
}
