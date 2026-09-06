# Phase 0 (Foundations) — Handoff Notes

Built in this conversation, on top of the existing
`designer-apparels-tauri` repo + the Python source
(`designer_apparels_py`). **Not yet compiled/run** — see "Verification"
below before you trust any of it.

## What's new

| File | Purpose |
|---|---|
| `src-tauri/schema.sql` | Byte-for-byte copy of the Python app's schema.sql (35 tables) |
| `src-tauri/src/paths.rs` | `exe_dir()` / `persistent_path()` — config.json, license.json, and the db file all sit next to the .exe |
| `src-tauri/src/config.rs` | Compiled-in defaults + optional `config.json` override, deep-merged (Rust port of `config.py`) |
| `src-tauri/src/schema.rs` | Embeds schema.sql, runs it, then applies the additive `SCHEMA_MIGRATIONS` list ported from `database.py` |
| `src-tauri/src/platform_db.rs` | Opens/seeds the platform database (separate file from the existing `invoices.db` — see "Key decisions" below) |
| `src-tauri/src/licensing.rs` + `licensing_commands.rs` | HMAC-SHA256 + hardware-ID activation, ported from `utils/licensing.py` / `routes/licensing.py` |
| `src-tauri/src/session.rs` | In-memory session (replaces Flask's cookie) |
| `src-tauri/src/shop_guard.rs` | Port of `BLUEPRINT_SHOP_TYPES` — not called by anything yet, wired up starting Phase 1 |
| `src-tauri/src/auth.rs` | `login` / `logout` / `get_session` commands, ported from `routes/auth.py` |
| `src-tauri/src/shops.rs` | `list_shops` / `choose_shop` / `add_shop` / `switch_shop`, ported from `routes/shops.py` |
| `src-tauri/src/dashboard.rs` | `get_dashboard` command, ported from `routes/dashboard.py` |
| `src/pages/Activate.jsx`, `Login.jsx`, `ShopPicker.jsx`, `DashboardHome.jsx` | New screens |
| `src/components/Sidebar.jsx` | Shop switcher + shop_type-scoped nav |
| `src/App.jsx` | Rewritten as a screen state machine: loading → activate → login → pick-shop → app shell |

`main.rs` and `Cargo.toml` were edited to wire all of the above in
alongside the existing invoicing commands (untouched).

## Key decisions made along the way (flag if you disagree)

1. **Platform DB is a separate file/connection from `invoices.db` for
   now.** The existing Billing/History/Items pages keep working exactly
   as before, against their own db. Reconciling that logic with the
   ported `vm_billentry`/`vm_billitems` tables is explicitly Phase 1
   work per the roadmap — I didn't want to fold it in silently under
   "foundations."
2. **Licensing is a fresh secret/keygen for this Rust app** — not
   byte-compatible with keys issued by the Python app's keygen. If a
   client needs one license to cover both a Python install and a Rust
   install during the migration window, that needs its own decision
   (shared secret + matching hardware-ID derivation on both sides).
3. **Default admin login is recreated fresh** (`admin` / `admin123`,
   bcrypt-hashed) on first run of the new platform db — no attempt to
   import/migrate existing Python-hashed passwords, since the hash
   formats aren't compatible (werkzeug vs bcrypt) and this is a new db
   file, not the same file being reopened.
4. **Shop_guard map is defined but unused** — nothing in Phase 0 needs
   it yet (no vertical commands exist). Phase 1+ commands should call
   `shop_guard::check_shop_type("billing", &session.shop_type)` etc. as
   their first line.

## Verification — do this before relying on any of this

This sandbox had no working Rust toolchain for Tauri v2 (apt's rustc
1.75 is too old — several of Tauri's own transitive dependencies now
require the `edition2024` Cargo feature). I could not get `cargo check`
to complete, so **none of this Rust code has been compiler-verified**.
I hand-reviewed every file against patterns already working in your
`db.rs`/`main.rs`, but please:

1. `cd src-tauri && cargo check` — fix whatever surfaces (my best guess,
   if anything: minor `State<T>` deref nits, or a rusqlite `params!`
   type mismatch somewhere in `auth.rs`/`shops.rs`/`dashboard.rs`).
2. `npm run tauri dev` and click through: Activate (copy the Hardware
   ID shown, run your own keygen against it, paste the key back) →
   Login (`admin` / `admin123`) → Dashboard.
3. Confirm `config.json` next to the dev binary actually overrides
   `shop_display_name` etc. when you drop one in.

No `Cargo.lock` is included — this sandbox's failed resolution attempts
would have left a broken one; let your own toolchain generate a clean
one on first `cargo check`.

## Status tracker update

- [x] Original single-tenant Tauri invoicing app — unchanged, still works
- [x] Phase 0 — Foundations — **built, not yet compiler-verified** (see above)
- [ ] Phase 1 — Designer Apparels core
- [ ] Phase 2 — Designer Apparels continued
- [ ] Phase 3 — Staff HR
- [ ] Phase 4 — Student Management
- [ ] Phase 5 — Garments
- [ ] Phase 6 — Service
- [ ] Phase 7 — Packaging & distribution per client
