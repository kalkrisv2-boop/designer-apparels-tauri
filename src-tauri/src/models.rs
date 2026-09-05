use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Hardcoded company identity — this build is exclusive to Designer Apparels.
// To repurpose the app for a different company, change these constants only.
// ---------------------------------------------------------------------------
pub const COMPANY_NAME: &str = "DESIGNER APPARELS";
pub const COMPANY_ADDRESS_LINE1: &str = "H.S.ROAD, CHERPULASSERY,";
pub const COMPANY_ADDRESS_LINE2: &str = "DIST-PALAKKAD, KERALA-679503";
pub const COMPANY_GSTIN: &str = "32ANFPC7115A1ZW";
pub const COMPANY_STATE: &str = "Kerala";
pub const COMPANY_STATE_CODE: &str = "32";

// ---------------------------------------------------------------------------
// Settings (single-row table): everything that IS user-editable.
// ---------------------------------------------------------------------------
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub bank_name: String,
    pub bank_branch: String,
    pub bank_account_number: String,
    pub bank_ifsc: String,
    pub terms_and_conditions: String, // newline separated
    pub next_invoice_number: i64,
    pub pdf_output_folder: String, // absolute path chosen by user
    pub mobile_no: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            bank_name: String::new(),
            bank_branch: String::new(),
            bank_account_number: String::new(),
            bank_ifsc: String::new(),
            terms_and_conditions:
                "1. Subject to Cherpulassery jurisdiction.\n2. Goods once sold shall not be taken back on any condition.\n3. All responsibility ceases, once goods are handed over to carriers."
                    .to_string(),
            next_invoice_number: 2958,
            pdf_output_folder: String::new(),
            mobile_no: String::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Item catalog (reusable products you can pick from during billing)
// ---------------------------------------------------------------------------
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CatalogItem {
    pub id: Option<i64>,
    pub description: String,
    pub hsn_sac: String,
    pub default_rate: f64,
    pub default_gst_rate: f64, // e.g. 5.0, 12.0, 18.0
}

// ---------------------------------------------------------------------------
// A single invoice line item
// ---------------------------------------------------------------------------
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InvoiceLineItem {
    pub sr: i64,
    pub description: String,
    pub hsn_sac: String,
    pub size_ratio: String, // free text, e.g. "32/12 , 34/12 , 36/12"
    pub qty: f64,
    pub rate: f64,
    pub gst_rate: f64,  // percent, e.g. 5.0
    pub gst_mode: String, // "exclusive" | "inclusive"
    pub amount: f64,    // computed taxable amount (pre-tax) for this line
}

// ---------------------------------------------------------------------------
// Buyer / invoice header
// ---------------------------------------------------------------------------
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InvoiceInput {
    pub invoice_date: String, // "DD/MM/YYYY"
    pub buyer_name: String,
    pub buyer_address: String,
    pub buyer_gstin: String,
    pub buyer_state: String,
    pub buyer_state_code: String,
    pub transport_name: String,
    pub salesman: String,
    pub items: Vec<InvoiceLineItem>,
}

// ---------------------------------------------------------------------------
// Fully computed invoice, as persisted + returned to the frontend
// ---------------------------------------------------------------------------
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Invoice {
    pub id: Option<i64>,
    pub invoice_number: i64,
    pub invoice_date: String,
    pub buyer_name: String,
    pub buyer_address: String,
    pub buyer_gstin: String,
    pub buyer_state: String,
    pub buyer_state_code: String,
    pub transport_name: String,
    pub salesman: String,
    pub is_interstate: bool,
    pub taxable_total: f64,
    pub cgst_total: f64,
    pub sgst_total: f64,
    pub igst_total: f64,
    pub round_off: f64,
    pub grand_total: f64,
    pub amount_in_words: String,
    pub pdf_path: Option<String>,
    pub items: Vec<InvoiceLineItem>,
}
