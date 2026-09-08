// Prevents additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod billing;
mod config;
mod customers;
mod dashboard;
mod db;
mod invoices;
mod licensing;
mod licensing_commands;
mod models;
mod paths;
mod pdf;
mod platform_db;
mod products;
mod schema;
mod session;
mod shop_guard;
mod shops;
mod suppliers;
mod words;

use db::Db;
use models::{CatalogItem, Invoice, InvoiceInput, Settings};
use platform_db::PlatformDb;
use rusqlite::Connection;
use session::{Session, SessionState};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};

// ---------------------------------------------------------------------------
// Tax calculation
// ---------------------------------------------------------------------------

const COMPANY_STATE_CODE: &str = models::COMPANY_STATE_CODE;

fn compute_invoice(input: InvoiceInput, invoice_number: i64) -> Invoice {
    let is_interstate = input.buyer_state_code.trim() != COMPANY_STATE_CODE;

    let mut items = input.items;
    let mut taxable_total = 0.0_f64;
    let mut cgst_total = 0.0_f64;
    let mut sgst_total = 0.0_f64;
    let mut igst_total = 0.0_f64;

    for item in items.iter_mut() {
        let gross = item.qty * item.rate;
        let (taxable, tax_amount) = if item.gst_mode.eq_ignore_ascii_case("inclusive") {
            // gross already includes GST: taxable = gross / (1 + rate/100)
            let taxable = gross / (1.0 + item.gst_rate / 100.0);
            (taxable, gross - taxable)
        } else {
            // exclusive: gross is the taxable value, tax added on top
            (gross, gross * item.gst_rate / 100.0)
        };

        item.amount = round2(taxable);
        taxable_total += taxable;

        if is_interstate {
            igst_total += tax_amount;
        } else {
            cgst_total += tax_amount / 2.0;
            sgst_total += tax_amount / 2.0;
        }
    }

    taxable_total = round2(taxable_total);
    cgst_total = round2(cgst_total);
    sgst_total = round2(sgst_total);
    igst_total = round2(igst_total);

    let pre_round_total = taxable_total + cgst_total + sgst_total + igst_total;
    let grand_total = pre_round_total.round();
    let round_off = round2(grand_total - pre_round_total);

    let amount_in_words = words::amount_to_words(grand_total);

    Invoice {
        id: None,
        invoice_number,
        invoice_date: input.invoice_date,
        buyer_name: input.buyer_name,
        buyer_address: input.buyer_address,
        buyer_gstin: input.buyer_gstin,
        buyer_state: input.buyer_state,
        buyer_state_code: input.buyer_state_code,
        transport_name: input.transport_name,
        salesman: input.salesman,
        is_interstate,
        taxable_total,
        cgst_total,
        sgst_total,
        igst_total,
        round_off,
        grand_total,
        amount_in_words,
        pdf_path: None,
        items,
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_settings(db: State<Db>) -> Result<Settings, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::get_settings(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_settings(db: State<Db>, settings: Settings) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::save_settings(&conn, &settings).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_catalog_items(db: State<Db>) -> Result<Vec<CatalogItem>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::list_catalog_items(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_catalog_item(db: State<Db>, item: CatalogItem) -> Result<i64, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::upsert_catalog_item(&conn, &item).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_catalog_item(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::delete_catalog_item(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_invoices(db: State<Db>) -> Result<Vec<Invoice>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::list_invoices(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_invoice(db: State<Db>, id: i64) -> Result<Invoice, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::get_invoice_with_items(&conn, id).map_err(|e| e.to_string())
}

/// Creates the invoice (computing taxes + amount-in-words), persists it,
/// generates the 3-copy PDF into the configured output folder, and returns
/// the fully computed invoice (with id + pdf_path filled in).
#[tauri::command]
fn create_invoice(db: State<Db>, input: InvoiceInput) -> Result<Invoice, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let settings = db::get_settings(&conn).map_err(|e| e.to_string())?;
    if settings.pdf_output_folder.trim().is_empty() {
        return Err("Please choose a PDF output folder in Settings before billing.".to_string());
    }

    let invoice_number = settings.next_invoice_number;
    let mut invoice = compute_invoice(input, invoice_number);

    let invoice_id = db::insert_invoice(&conn, &invoice).map_err(|e| e.to_string())?;
    invoice.id = Some(invoice_id);

    let filename = format!("INV-{}.pdf", invoice_number);
    let output_path = PathBuf::from(&settings.pdf_output_folder).join(&filename);

    pdf::generate_invoice_pdf(&invoice, &output_path).map_err(|e| e.to_string())?;

    let path_str = output_path.to_string_lossy().to_string();
    db::update_invoice_pdf_path(&conn, invoice_id, &path_str).map_err(|e| e.to_string())?;
    invoice.pdf_path = Some(path_str);

    Ok(invoice)
}

/// Re-generates the PDF for an already-saved invoice (e.g. after editing
/// bank details in Settings, or if the file was deleted).
#[tauri::command]
fn regenerate_invoice_pdf(db: State<Db>, invoice_id: i64) -> Result<String, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let invoice = db::get_invoice_with_items(&conn, invoice_id).map_err(|e| e.to_string())?;
    let settings = db::get_settings(&conn).map_err(|e| e.to_string())?;

    if settings.pdf_output_folder.trim().is_empty() {
        return Err("No PDF output folder configured in Settings.".to_string());
    }

    let filename = format!("INV-{}.pdf", invoice.invoice_number);
    let output_path = PathBuf::from(&settings.pdf_output_folder).join(&filename);
    pdf::generate_invoice_pdf(&invoice, &output_path).map_err(|e| e.to_string())?;

    let path_str = output_path.to_string_lossy().to_string();
    db::update_invoice_pdf_path(&conn, invoice_id, &path_str).map_err(|e| e.to_string())?;
    Ok(path_str)
}

// ---------------------------------------------------------------------------
// App entry
// ---------------------------------------------------------------------------

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            std::fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");

            let conn = Connection::open(db::db_path(&app_data_dir))
                .expect("failed to open sqlite database");
            db::init(&conn).expect("failed to initialize database schema");

            app.manage(Db(Mutex::new(conn)));

            // --- Phase 0 foundations: runtime config, full ERP schema,
            // session state. See platform_db.rs for why this is a
            // separate connection/file from the invoicing db above for
            // now (Phase 1 reconciles them).
            let app_config = config::load_config();
            let platform_conn = platform_db::open(&app_config)
                .expect("failed to open/initialize platform database");

            app.manage(PlatformDb(Mutex::new(platform_conn)));
            app.manage(SessionState(Mutex::new(Session::default())));
            app.manage(app_config);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            list_catalog_items,
            save_catalog_item,
            delete_catalog_item,
            list_invoices,
            get_invoice,
            create_invoice,
            regenerate_invoice_pdf,
            licensing_commands::check_license,
            licensing_commands::activate_license,
            auth::login,
            auth::logout,
            auth::get_session,
            shops::list_shops,
            shops::choose_shop,
            shops::add_shop,
            shops::switch_shop,
            dashboard::get_dashboard,
            config::get_client_config,
            products::list_products,
            products::add_product,
            products::update_product,
            products::delete_product,
            products::search_products,
            products::style_variants,
            products::quick_add_product,
            customers::list_customers,
            customers::add_customer,
            customers::update_customer,
            customers::delete_customer,
            suppliers::list_suppliers,
            suppliers::add_supplier,
            suppliers::update_supplier,
            suppliers::delete_supplier,
            billing::billing_init,
            billing::checkout,
            invoices::find_bill_by_number,
            invoices::get_invoice_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
