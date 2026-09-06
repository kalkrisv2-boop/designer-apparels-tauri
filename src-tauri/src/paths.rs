//! paths.rs
//! ---------
//! Rust/Tauri equivalent of the Python app's `paths.py`.
//!
//! The Python app drew a hard line between two kinds of paths because
//! PyInstaller's --onefile mode re-extracts a temp bundle on every launch:
//!   - bundle_path()     -- read-only assets shipped with the app
//!                          (templates, static, schema.sql)
//!   - persistent_path()  -- files that must survive between runs
//!                          (designer.db, license.json, config.json)
//!
//! Tauri does not have that problem: there is no PyInstaller-style temp
//! re-extraction step, and the schema/default-config values are compiled
//! directly into the binary via `include_str!` (see schema.rs / config.rs).
//! So the only thing this module needs to provide is the equivalent of
//! `persistent_path()`: a real, permanent folder next to the running
//! .exe, so a client install is still "one folder" that's trivial to
//! back up (db file, config.json override, license.json all sit next to
//! DesignerApparels.exe, exactly like the Python version).
//!
//! In dev (`cargo tauri dev`), `current_exe()` resolves to a path deep in
//! `target/debug/`, which is fine for local testing -- it behaves the
//! same way `paths.py` falls back to "this project's own folder" when
//! not frozen.

use std::path::PathBuf;

/// Folder containing the running executable. Config.json (override),
/// license.json, and the client's SQLite database file all live here --
/// never inside any OS-managed app-data directory, so a client's entire
/// install + data is one folder that can be copied/zipped as a backup,
/// matching the Python app's distribution model on purpose.
pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn persistent_path(filename: &str) -> PathBuf {
    exe_dir().join(filename)
}
