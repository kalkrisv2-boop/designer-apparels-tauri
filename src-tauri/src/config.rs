//! config.rs
//! ----------
//! Rust/Tauri equivalent of the Python app's `config.py`, adapted for the
//! "one binary, per-client runtime config" model decided in the migration
//! roadmap (§2): the Python app baked branding into the .exe at PyInstaller
//! build time via `profiles/<client>.json` + an env var. Tauri has no
//! compile-time templating step to hook that into anyway, so instead:
//!
//!   1. Compiled-in defaults (the `Default` impl below) -- the "factory
//!      default" for this build, equivalent to Python's `_DEFAULTS`.
//!   2. An OPTIONAL override file, `config.json`, sitting next to the
//!      .exe (see paths::persistent_path). If present, its fields are
//!      deep-merged on top of the defaults -- same semantics as Python's
//!      `_deep_merge`: you can override just `shop_display_name` without
//!      repeating the barcode settings.
//!
//! This is what lets ONE compiled binary serve every client: drop a
//! different `config.json` next to each client's copy of the .exe.
//! Loaded once at startup and stored in `AppState`; restart the app to
//! pick up a hand-edited config.json, exactly like the Python version.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::paths::persistent_path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarcodeConfig {
    pub symbology: String,
    pub label_width_mm: u32,
    pub label_height_mm: u32,
    pub labels_per_row: u32,
}

impl Default for BarcodeConfig {
    fn default() -> Self {
        Self {
            symbology: "code128".into(),
            label_width_mm: 50,
            label_height_mm: 25,
            labels_per_row: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub db_filename: String,
    pub shop_display_name: String,
    pub window_title: String,
    pub logo_path: String,
    pub multi_shop_enabled: bool,
    pub barcode: BarcodeConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            db_filename: "designer.db".into(),
            shop_display_name: "Designer Apparels".into(),
            window_title: "Designer Apparels".into(),
            logo_path: "".into(),
            multi_shop_enabled: false,
            barcode: BarcodeConfig::default(),
        }
    }
}

/// Merge `override_val` into `base_val`, recursing into nested JSON
/// objects (like "barcode") instead of replacing them wholesale --
/// direct port of Python config.py's `_deep_merge`.
fn deep_merge(base: &mut Value, over: &Value) {
    match (base, over) {
        (Value::Object(base_map), Value::Object(over_map)) => {
            for (k, v) in over_map {
                deep_merge(base_map.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (base_slot, over_val) => {
            *base_slot = over_val.clone();
        }
    }
}

#[derive(Serialize)]
pub struct ClientConfig {
    pub multi_shop_enabled: bool,
    pub shop_display_name: String,
    pub window_title: String,
}

/// The subset of AppConfig the frontend actually needs (sidebar
/// switcher visibility, window branding) -- deliberately not the full
/// struct, so nothing about barcode label sizing etc. needs a frontend
/// type just to answer "is multi-shop on".
#[tauri::command]
pub fn get_client_config(config: tauri::State<AppConfig>) -> ClientConfig {
    ClientConfig {
        multi_shop_enabled: config.multi_shop_enabled,
        shop_display_name: config.shop_display_name.clone(),
        window_title: config.window_title.clone(),
    }
}

/// Loads the effective config for this run: compiled-in defaults,
/// deep-merged with `config.json` next to the .exe if that file exists
/// and parses. Never fails/panics over a missing or malformed override
/// file -- same "never crash over config" guarantee as the Python
/// version, it just runs with defaults instead.
pub fn load_config() -> AppConfig {
    let default_val = serde_json::to_value(AppConfig::default())
        .expect("AppConfig::default() must always serialize");

    let override_path = persistent_path("config.json");
    let override_val: Option<Value> = std::fs::read_to_string(&override_path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok());

    let mut merged = default_val;
    if let Some(over) = override_val {
        deep_merge(&mut merged, &over);
    }

    serde_json::from_value(merged).unwrap_or_default()
}
