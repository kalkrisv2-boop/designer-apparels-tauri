use crate::models::{
    Invoice, COMPANY_ADDRESS_LINE1, COMPANY_ADDRESS_LINE2, COMPANY_GSTIN, COMPANY_NAME,
    COMPANY_STATE, COMPANY_STATE_CODE,
};
use anyhow::Result;
use printpdf::*;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

const PAGE_W: f64 = 210.0; // A4 mm
const PAGE_H: f64 = 297.0;
const MARGIN: f64 = 10.0;

struct Ctx<'a> {
    layer: PdfLayerReference,
    font_regular: &'a IndirectFontRef,
    font_bold: &'a IndirectFontRef,
}

impl<'a> Ctx<'a> {
    fn text(&self, s: &str, x: f64, y: f64, size: f64, bold: bool) {
        let font = if bold { self.font_bold } else { self.font_regular };
        self.layer
            .use_text(s, size, Mm(x), Mm(y), font);
    }

    fn text_centered(&self, s: &str, center_x: f64, y: f64, size: f64, bold: bool) {
        // Rough width estimate (Helvetica average glyph width ~0.5 * size in pt -> mm)
        let approx_width_mm = (s.len() as f64) * size * 0.45 * 0.3527;
        self.text(s, center_x - approx_width_mm / 2.0, y, size, bold);
    }

    fn hline(&self, x1: f64, x2: f64, y: f64, thickness: f64) {
        let line = Line {
            points: vec![
                (Point::new(Mm(x1), Mm(y)), false),
                (Point::new(Mm(x2), Mm(y)), false),
            ],
            is_closed: false,
            has_fill: false,
            has_stroke: true,
            is_clipping_path: false,
        };
        self.layer.set_outline_thickness(thickness);
        self.layer.add_shape(line);
    }

    fn vline(&self, x: f64, y1: f64, y2: f64, thickness: f64) {
        let line = Line {
            points: vec![
                (Point::new(Mm(x), Mm(y1)), false),
                (Point::new(Mm(x), Mm(y2)), false),
            ],
            is_closed: false,
            has_fill: false,
            has_stroke: true,
            is_clipping_path: false,
        };
        self.layer.set_outline_thickness(thickness);
        self.layer.add_shape(line);
    }

    fn rect(&self, x1: f64, y1: f64, x2: f64, y2: f64, thickness: f64) {
        self.hline(x1, x2, y1, thickness);
        self.hline(x1, x2, y2, thickness);
        self.vline(x1, y1, y2, thickness);
        self.vline(x2, y1, y2, thickness);
    }
}

/// Renders one copy (Original/Duplicate/Triplicate) of the invoice onto a fresh page.
fn render_copy(ctx: &Ctx, inv: &Invoice, copy_label: &str) -> Result<()> {
    // printpdf's coordinate origin is bottom-left. We track `y` as the actual
    // PDF y-coordinate (from the bottom of the page) and decrement it as we
    // draw further down the page.
    let mut y = PAGE_H - MARGIN;

    let left = MARGIN;
    let right = PAGE_W - MARGIN;

    // Outer border for the whole invoice block
    let block_top = y;
    let block_bottom = MARGIN;
    ctx.rect(left, block_bottom, right, block_top, 0.6);

    // ---- Header: company name / address / GSTIN, copy label ----
    y -= 6.0;
    ctx.text_centered(COMPANY_NAME, PAGE_W / 2.0, y, 16.0, true);
    y -= 5.0;
    ctx.text_centered(COMPANY_ADDRESS_LINE1, PAGE_W / 2.0, y, 9.0, false);
    y -= 4.0;
    ctx.text_centered(COMPANY_ADDRESS_LINE2, PAGE_W / 2.0, y, 9.0, false);
    y -= 4.5;
    ctx.text_centered(
        &format!("GSTIN: {}", COMPANY_GSTIN),
        PAGE_W / 2.0,
        y,
        9.0,
        true,
    );
    y -= 4.5;
    ctx.text_centered("BRA - PANTY - SLIPS - LEGGINGS - KURTIS - NIGHTIES", PAGE_W / 2.0, y, 8.0, false);

    y -= 5.0;
    ctx.hline(left, right, y, 0.4);
    y -= 4.5;
    ctx.text_centered("TAX INVOICE", PAGE_W / 2.0, y, 11.0, true);
    y -= 4.0;
    ctx.text_centered(copy_label, PAGE_W / 2.0, y, 8.0, false);

    y -= 4.0;
    ctx.hline(left, right, y, 0.4);

    // ---- Bill To + Invoice meta (two columns) ----
    let mid_x = PAGE_W / 2.0;
    let col_top = y;
    let bill_to_bottom = col_top - 28.0;
    ctx.vline(mid_x, bill_to_bottom, col_top, 0.4);
    ctx.hline(left, right, bill_to_bottom, 0.4);

    let mut ly = col_top - 4.5;
    ctx.text("Bill To:", left + 2.0, ly, 8.5, true);
    ly -= 4.2;
    ctx.text(&format!("Name: {}", inv.buyer_name), left + 2.0, ly, 8.5, false);
    ly -= 4.2;
    ctx.text(&format!("Address: {}", inv.buyer_address), left + 2.0, ly, 8.0, false);
    ly -= 4.2;
    ctx.text(&format!("GSTIN: {}", inv.buyer_gstin), left + 2.0, ly, 8.0, false);
    ly -= 4.2;
    ctx.text(
        &format!("State: {}   Code: {}", inv.buyer_state, inv.buyer_state_code),
        left + 2.0,
        ly,
        8.0,
        false,
    );

    let mut ry = col_top - 4.5;
    ctx.text(&format!("Invoice No: {}", inv.invoice_number), mid_x + 2.0, ry, 8.5, true);
    ry -= 4.2;
    ctx.text(&format!("Invoice Date: {}", inv.invoice_date), mid_x + 2.0, ry, 8.5, false);
    ry -= 4.2;
    ctx.text(&format!("Transport: {}", inv.transport_name), mid_x + 2.0, ry, 8.0, false);
    ry -= 4.2;
    ctx.text(&format!("Salesman: {}", inv.salesman), mid_x + 2.0, ry, 8.0, false);
    ry -= 4.2;
    ctx.text(
        &format!("Supplier State: {} ({})", COMPANY_STATE, COMPANY_STATE_CODE),
        mid_x + 2.0,
        ry,
        8.0,
        false,
    );

    y = bill_to_bottom;

    // ---- Item table ----
    // Columns: Sr | Description | HSN/SAC | Size Ratio | Qty | Rate | GST% | Amount
    let col_x = [
        left,           // Sr start
        left + 8.0,     // Description start
        left + 65.0,    // HSN/SAC start
        left + 82.0,    // Size Ratio start
        left + 130.0,   // Qty start
        left + 145.0,   // Rate start
        left + 163.0,   // GST% start
        left + 175.0,   // Amount start
        right,          // end
    ];

    let header_h = 6.0;
    let header_top = y;
    let header_bottom = header_top - header_h;
    for x in &col_x {
        ctx.vline(*x, header_bottom, header_top, 0.35);
    }
    ctx.hline(left, right, header_top, 0.4);
    ctx.hline(left, right, header_bottom, 0.4);

    let hy = header_bottom + 2.0;
    ctx.text("Sr", col_x[0] + 1.0, hy, 7.5, true);
    ctx.text("Item Description", col_x[1] + 1.0, hy, 7.5, true);
    ctx.text("HSN", col_x[2] + 1.0, hy, 7.5, true);
    ctx.text("Size/Ratio", col_x[3] + 1.0, hy, 7.5, true);
    ctx.text("Qty", col_x[4] + 1.0, hy, 7.5, true);
    ctx.text("Rate", col_x[5] + 1.0, hy, 7.5, true);
    ctx.text("GST%", col_x[6] + 1.0, hy, 7.5, true);
    ctx.text("Amount", col_x[7] + 1.0, hy, 7.5, true);

    let mut row_top = header_bottom;
    let row_h = 6.0;

    for item in &inv.items {
        let row_bottom = row_top - row_h;
        for x in &col_x {
            ctx.vline(*x, row_bottom, row_top, 0.25);
        }
        ctx.hline(left, right, row_bottom, 0.25);

        let ty = row_bottom + 2.0;
        ctx.text(&format!("{}", item.sr), col_x[0] + 1.0, ty, 7.5, false);

        // Wrap long descriptions crudely by truncation (keeps single-line rows).
        let desc = if item.description.len() > 32 {
            format!("{}...", &item.description[..32])
        } else {
            item.description.clone()
        };
        ctx.text(&desc, col_x[1] + 1.0, ty, 7.5, false);
        ctx.text(&item.hsn_sac, col_x[2] + 1.0, ty, 7.5, false);

        let size_ratio = if item.size_ratio.len() > 30 {
            format!("{}...", &item.size_ratio[..30])
        } else {
            item.size_ratio.clone()
        };
        ctx.text(&size_ratio, col_x[3] + 1.0, ty, 6.3, false);

        ctx.text(&format!("{:.0}", item.qty), col_x[4] + 1.0, ty, 7.5, false);
        ctx.text(&format!("{:.2}", item.rate), col_x[5] + 1.0, ty, 7.5, false);
        ctx.text(&format!("{:.1}", item.gst_rate), col_x[6] + 1.0, ty, 7.5, false);
        ctx.text(&format!("{:.2}", item.amount), col_x[7] + 1.0, ty, 7.5, false);

        row_top = row_bottom;
    }

    y = row_top;

    // ---- Totals block ----
    let totals_top = y;
    let totals_h = if inv.is_interstate { 22.0 } else { 26.0 };
    let totals_bottom = totals_top - totals_h;
    ctx.rect(left, totals_bottom, right, totals_top, 0.35);
    let label_x = right - 60.0;
    ctx.vline(label_x, totals_bottom, totals_top, 0.25);

    let mut ty = totals_top - 4.0;
    ctx.text("Taxable Total:", label_x + 2.0, ty, 8.0, false);
    ctx.text(&format!("{:.2}", inv.taxable_total), right - 22.0, ty, 8.0, false);

    if inv.is_interstate {
        ty -= 4.5;
        ctx.text("IGST:", label_x + 2.0, ty, 8.0, false);
        ctx.text(&format!("{:.2}", inv.igst_total), right - 22.0, ty, 8.0, false);
    } else {
        ty -= 4.5;
        ctx.text("CGST:", label_x + 2.0, ty, 8.0, false);
        ctx.text(&format!("{:.2}", inv.cgst_total), right - 22.0, ty, 8.0, false);
        ty -= 4.5;
        ctx.text("SGST:", label_x + 2.0, ty, 8.0, false);
        ctx.text(&format!("{:.2}", inv.sgst_total), right - 22.0, ty, 8.0, false);
    }

    ty -= 4.5;
    ctx.text("Round Off:", label_x + 2.0, ty, 8.0, false);
    ctx.text(&format!("{:.2}", inv.round_off), right - 22.0, ty, 8.0, false);

    ty -= 5.0;
    ctx.hline(label_x, right, ty + 1.5, 0.3);
    ctx.text("Grand Total:", label_x + 2.0, ty, 9.5, true);
    ctx.text(&format!("{:.2}", inv.grand_total), right - 22.0, ty, 9.5, true);

    // Amount in words, left side of totals block
    ctx.text("Amount in Words:", left + 2.0, totals_top - 4.0, 8.0, true);
    let words = &inv.amount_in_words;
    // simple wrap at ~55 chars per line within available width
    for (i, chunk) in wrap_text(words, 45).into_iter().enumerate() {
        ctx.text(&chunk, left + 2.0, totals_top - 9.0 - (i as f64 * 4.0), 7.5, false);
    }

    y = totals_bottom;

    // ---- Bank details + signature ----
    let bank_h = 20.0;
    let bank_bottom = y - bank_h;
    ctx.rect(left, bank_bottom, right, y, 0.35);
    let sig_x = right - 55.0;
    ctx.vline(sig_x, bank_bottom, y, 0.25);

    let mut by = y - 4.0;
    ctx.text("Bank Details:", left + 2.0, by, 8.0, true);
    by -= 4.2;
    ctx.text(
        &format!("Bank: (see Settings)"),
        left + 2.0,
        by,
        7.5,
        false,
    );
    by -= 4.2;
    ctx.text("A/C No. / IFSC: (see Settings)", left + 2.0, by, 7.5, false);
    by -= 4.2;
    ctx.text("Branch: (see Settings)", left + 2.0, by, 7.5, false);

    ctx.text_centered(&format!("For {}", COMPANY_NAME), sig_x + (right - sig_x) / 2.0, y - 6.0, 8.5, true);
    ctx.text_centered("Authorised Signatory", sig_x + (right - sig_x) / 2.0, y - bank_h + 3.0, 7.5, false);

    y = bank_bottom;

    // ---- Terms and conditions ----
    ctx.text("Terms & Conditions:", left + 2.0, y - 4.0, 7.5, true);
    ctx.text(
        "1. Subject to Cherpulassery jurisdiction. 2. Goods once sold shall not be taken back. 3. Responsibility ceases once goods are handed to carrier.",
        left + 2.0,
        y - 8.0,
        6.5,
        false,
    );

    Ok(())
}

fn wrap_text(s: &str, max_chars: usize) -> Vec<String> {
    let words: Vec<&str> = s.split_whitespace().collect();
    let mut lines = vec![];
    let mut current = String::new();
    for w in words {
        if current.len() + w.len() + 1 > max_chars {
            lines.push(current.clone());
            current.clear();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(w);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Generates the full 3-copy invoice PDF (Original / Duplicate / Triplicate) and
/// writes it to `output_path`. Returns the path on success.
pub fn generate_invoice_pdf(inv: &Invoice, output_path: &Path) -> Result<()> {
    let (doc, page1, layer1) = PdfDocument::new(
        format!("Invoice {}", inv.invoice_number),
        Mm(PAGE_W),
        Mm(PAGE_H),
        "Layer 1",
    );

    let font_regular = doc.add_builtin_font(BuiltinFont::Helvetica)?;
    let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;

    let copies = [
        "Original for Recipient",
        "Duplicate for Supplier/Transporter",
        "Triplicate for Office",
    ];

    // First copy on page1/layer1
    {
        let layer = doc.get_page(page1).get_layer(layer1);
        let ctx = Ctx {
            layer,
            font_regular: &font_regular,
            font_bold: &font_bold,
        };
        render_copy(&ctx, inv, copies[0])?;
    }

    // Remaining copies on their own pages
    for label in &copies[1..] {
        let (page, layer) = doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Layer 1");
        let layer_ref = doc.get_page(page).get_layer(layer);
        let ctx = Ctx {
            layer: layer_ref,
            font_regular: &font_regular,
            font_bold: &font_bold,
        };
        render_copy(&ctx, inv, label)?;
    }

    doc.save(&mut BufWriter::new(File::create(output_path)?))?;
    Ok(())
}
