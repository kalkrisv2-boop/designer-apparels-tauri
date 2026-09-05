# Designer Apparels — GST Invoice App

A fully local, offline desktop GST billing app built with **Tauri + Rust + React**, hardcoded for **DESIGNER APPARELS** (H.S. Road, Cherpulassery, Dist. Palakkad, Kerala – 679503, GSTIN: 32ANFPC7115A1ZW).

No server, no cloud, no external DB. Everything (settings, item catalog, invoices) lives in a local SQLite file next to the app, and PDFs are written to a folder you choose on first run.

---

## 1. Prerequisites

Install once on your development/build machine:

- **Rust** (stable) — https://rustup.rs
- **Node.js** 18+ and **npm**
- **Tauri CLI**: `cargo install tauri-cli --version "^2"`
- Platform build tools required by Tauri v2 (WebView2 on Windows is auto-installed; on Linux you need `libwebkit2gtk-4.1-dev`, `build-essential`, `libssl-dev`, `libgtk-3-dev`, `librsvg2-dev`; on macOS, Xcode command line tools)

## 2. Install dependencies

```bash
cd designer-apparels-invoice
npm install
```

## 3. App icons

A default icon set (Designer Apparels' initials on a maroon square) is already included in `src-tauri/icons/`, so `npm run tauri dev` and `npm run tauri build` both work out of the box — no separate icon-generation step needed.

To swap in your real logo later, generate a full set from any square PNG (512x512+ recommended) and overwrite the files in `src-tauri/icons/`:

```bash
npx @tauri-apps/cli icon path/to/your-logo.png
```

## 4. Run in development

```bash
npm run tauri dev
```

## 5. Build a distributable installer

```bash
npm run tauri build
```
This produces a native installer (`.msi`/`.exe` on Windows, `.dmg`/`.app` on macOS, `.deb`/`.AppImage` on Linux) in `src-tauri/target/release/bundle/`.

## 6. First run

1. Open **Settings** → fill in your bank name, account number, IFSC, branch, terms & conditions, and confirm the "Next Invoice Number" (defaults to **2958**, continuing from your sample invoice #2957).
2. Choose a **PDF output folder** (a native folder picker) — every generated invoice PDF is saved there automatically, named `INV-<number>.pdf`.
3. Go to **Items** to pre-load your product catalog (optional — you can also type items manually every time).
4. Go to **Billing** to create your first invoice.

---

## Project structure

```
designer-apparels-invoice/
├── src-tauri/                 # Rust backend (Tauri)
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   └── src/
│       ├── main.rs            # Tauri commands, app entry
│       ├── db.rs              # SQLite schema + queries (rusqlite)
│       ├── models.rs          # Shared structs
│       ├── words.rs           # Number → Indian words converter
│       └── pdf.rs             # printpdf invoice renderer
├── src/                        # React frontend
│   ├── main.jsx
│   ├── App.jsx
│   ├── api.js                 # thin wrapper over Tauri invoke()
│   ├── index.css
│   ├── pages/
│   │   ├── Billing.jsx
│   │   ├── History.jsx
│   │   ├── Items.jsx
│   │   └── Settings.jsx
│   └── components/
│       └── InvoiceItemRow.jsx
├── index.html
├── package.json
└── vite.config.js
```

## Data storage

- **Database**: SQLite file at the OS app-data directory (e.g. `%APPDATA%/com.designerapparels.invoice/invoices.db` on Windows, `~/.local/share/com.designerapparels.invoice/invoices.db` on Linux) — created automatically on first launch.
- **PDFs**: written to the folder you pick in Settings. Nothing ever leaves your machine.

## Notes / things you may want to tweak

- Company name, address, and GSTIN are **hardcoded** in `src-tauri/src/pdf.rs` and `models.rs` per your requirement that this build is exclusive to Designer Apparels. To reuse this app for another company, edit `COMPANY_*` constants in `models.rs`.
- GST rate is set **per line item** (not per-invoice), so a single invoice can mix rates.
- CGST+SGST is applied automatically when buyer state = Kerala (32); otherwise IGST is applied — this is controlled by the state you select for the buyer on each invoice.
- Amount-in-words uses the Indian numbering system (Lakh/Crore), matching your sample invoice's wording style.
