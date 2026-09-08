import React, { useEffect, useState } from "react";
import { findBillByNumber, getInvoiceData } from "../api.js";

// Invoice print / reprint view -- renders invoices::get_invoice_data's
// output in the same shape Python's invoice_print.html covered (seller
// header, buyer block, item table with per-line GST columns, totals +
// amount-in-words + bank details + terms), per PHASE_1_HANDOFF.md §2
// decision #4: "Print" is the webview's native print (window.print()),
// not a hand-drawn PDF. Sits alongside the old (legacy) History.jsx
// rather than replacing it -- that screen still serves the old
// single-tenant `invoices` table, this one serves the new
// vm_billentry/vm_billitems schema that billing::checkout() writes to.
//
// GST columns: is_interstate picks IGST vs CGST+SGST, matching decision
// #1 (auto-derived, but the *rate actually charged* is whatever was
// snapshotted onto the bill items at checkout time -- this view just
// displays what's stored, it doesn't recompute).
//
// Wholesale bills (is_wholesale) get the style-grouped rendering
// (style_groups) instead of a flat item table, per decision §4's
// grouping note in invoices.rs (bi_style_snapshot-keyed, first-seen
// order).

const money = (v) => (Number(v) || 0).toFixed(2);

export default function InvoicePrint({ initialBillNumber, onConsumeInitial }) {
  const [billNumber, setBillNumber] = useState(initialBillNumber ? String(initialBillNumber) : "");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [data, setData] = useState(null);

  const search = async (numberOverride) => {
    const number = numberOverride ?? billNumber;
    if (!String(number).trim()) {
      setError("Enter a bill number.");
      return;
    }
    setError("");
    setLoading(true);
    setData(null);
    try {
      const billId = await findBillByNumber(String(number));
      const invoiceData = await getInvoiceData(billId);
      setData(invoiceData);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  // Coming from Billing's "Print This Invoice" -- auto-search once, then
  // let the App-level state clear so revisiting this tab manually
  // doesn't keep re-triggering the same lookup.
  useEffect(() => {
    if (initialBillNumber) {
      setBillNumber(String(initialBillNumber));
      search(initialBillNumber);
      onConsumeInitial && onConsumeInitial();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialBillNumber]);

  const reset = () => {
    setData(null);
    setBillNumber("");
    setError("");
  };

  return (
    <div>
      <h1>Print / Reprint Invoice</h1>

      <div className="panel no-print">
        <div className="toolbar">
          <div style={{ width: 220 }}>
            <label>Bill Number</label>
            <input
              value={billNumber}
              onChange={(e) => setBillNumber(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && search()}
              placeholder="e.g. 1042"
            />
          </div>
          <button className="primary" style={{ marginTop: 18 }} onClick={() => search()} disabled={loading}>
            {loading ? "Searching…" : "Find Bill"}
          </button>
          {data && (
            <>
              <button className="secondary" style={{ marginTop: 18 }} onClick={reset}>
                New Search
              </button>
              <button className="secondary" style={{ marginTop: 18 }} onClick={() => window.print()}>
                Print
              </button>
            </>
          )}
        </div>
        {error && <div className="error-banner">{error}</div>}
      </div>

      {data && <InvoiceSheet data={data} />}
    </div>
  );
}

function InvoiceSheet({ data }) {
  const { bill, items, style_groups, shop, customer, is_interstate, is_wholesale, grand_total_words } = data;

  const hasTransport = bill.be_transport_name || bill.be_lr_number || bill.be_lr_date || bill.be_parcels;

  return (
    <div className="invoice-sheet panel">
      <div className="invoice-head">
        <div>
          <div className="invoice-shop-name">{shop.sp_shopname || "—"}</div>
          {shop.sp_tagline && <div className="invoice-shop-tagline">{shop.sp_tagline}</div>}
          <div className="invoice-shop-line">{shop.sp_shopaddress}</div>
          <div className="invoice-shop-line">
            {[shop.sp_phone, shop.sp_mobile].filter(Boolean).join("  |  ")}
          </div>
          {shop.sp_tin && <div className="invoice-shop-line">GSTIN: {shop.sp_tin}</div>}
        </div>
        <div className="invoice-title-block">
          <div className="invoice-title">TAX INVOICE</div>
          <div className="invoice-mode-badge">
            {is_wholesale ? "Wholesale" : "Retail"} · {is_interstate ? "Interstate (IGST)" : "Intrastate (CGST+SGST)"}
          </div>
          <table className="invoice-meta-table">
            <tbody>
              <tr>
                <td>Bill No.</td>
                <td>{bill.be_billnumber}</td>
              </tr>
              <tr>
                <td>Date</td>
                <td>{bill.be_billdate}</td>
              </tr>
              {bill.be_salesman && (
                <tr>
                  <td>Salesman</td>
                  <td>{bill.be_salesman}</td>
                </tr>
              )}
              {bill.be_booking && (
                <tr>
                  <td>Booking</td>
                  <td>{bill.be_booking}</td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>

      <div className="invoice-parties">
        <div className="invoice-party">
          <div className="invoice-party-label">Bill To</div>
          <div className="invoice-party-name">{bill.be_customername || "Walk-in Customer"}</div>
          {customer?.cs_address && <div>{customer.cs_address}</div>}
          {bill.be_customermobile && <div>Mobile: {bill.be_customermobile}</div>}
          {bill.be_customer_tin_num && <div>GSTIN: {bill.be_customer_tin_num}</div>}
          {bill.be_customer_pan && <div>PAN: {bill.be_customer_pan}</div>}
          {customer?.cs_statecode && (
            <div className="muted-note">State code (current on file): {customer.cs_statecode}</div>
          )}
        </div>
        {hasTransport && (
          <div className="invoice-party">
            <div className="invoice-party-label">Transport</div>
            {bill.be_transport_name && <div>Carrier: {bill.be_transport_name}</div>}
            {bill.be_lr_number && <div>LR No.: {bill.be_lr_number}</div>}
            {bill.be_lr_date && <div>LR Date: {bill.be_lr_date}</div>}
            {bill.be_parcels && <div>Parcels: {bill.be_parcels}</div>}
          </div>
        )}
      </div>

      {is_wholesale ? (
        <WholesaleTable groups={style_groups || []} isInterstate={is_interstate} />
      ) : (
        <RetailTable items={items} isInterstate={is_interstate} />
      )}

      <div className="totals-box">
        <div className="totals-row">
          <span>Total</span>
          <span>{money(bill.be_total)}</span>
        </div>
        {bill.be_discount > 0 && (
          <div className="totals-row">
            <span>Discount</span>
            <span>-{money(bill.be_discount)}</span>
          </div>
        )}
        <div className="totals-row">
          <span>Tax Total</span>
          <span>{money(bill.be_totvat)}</span>
        </div>
        <div className="totals-row grand">
          <span>Grand Total</span>
          <span>₹{bill.be_gtotal}</span>
        </div>
        <div className="totals-row">
          <span>Paid</span>
          <span>{money(bill.be_paidamount)}</span>
        </div>
        <div className="totals-row">
          <span>Balance</span>
          <span>{money(bill.be_balance)}</span>
        </div>
      </div>

      <div className="words-box">Amount in words: {grand_total_words}</div>

      {(shop.sp_bank || shop.sp_terms) && (
        <div className="invoice-footer">
          {shop.sp_bank && (
            <div className="invoice-bank">
              <div className="invoice-party-label">Bank Details</div>
              <div>{shop.sp_bank}</div>
              {shop.sp_branch && <div>Branch: {shop.sp_branch}</div>}
              {shop.sp_accno && <div>A/c No.: {shop.sp_accno}</div>}
              {shop.sp_ifsc && <div>IFSC: {shop.sp_ifsc}</div>}
            </div>
          )}
          {shop.sp_terms && (
            <div className="invoice-terms">
              <div className="invoice-party-label">Terms</div>
              <div>{shop.sp_terms}</div>
            </div>
          )}
        </div>
      )}

      <div className="invoice-signature">
        <div>Customer Signature</div>
        <div>For {shop.sp_shopname || "—"}</div>
      </div>
    </div>
  );
}

function gstHeaderCells(isInterstate) {
  return isInterstate ? (
    <th>IGST</th>
  ) : (
    <>
      <th>CGST</th>
      <th>SGST</th>
    </>
  );
}

function gstDataCells(item, isInterstate) {
  if (isInterstate) {
    return (
      <td className="text-end">
        {item.bi_igst ? `${item.bi_igst}% (${money(item.bi_igst_amt)})` : "—"}
      </td>
    );
  }
  return (
    <>
      <td className="text-end">{item.bi_cgst ? `${item.bi_cgst}% (${money(item.bi_cgst_amt)})` : "—"}</td>
      <td className="text-end">{item.bi_sgst ? `${item.bi_sgst}% (${money(item.bi_sgst_amt)})` : "—"}</td>
    </>
  );
}

function itemLabel(item) {
  const parts = [item.pr_model, item.pr_cupsize, item.pr_size].filter(Boolean);
  return parts.length ? `${item.pr_productname} (${parts.join("/")})` : item.pr_productname;
}

function RetailTable({ items, isInterstate }) {
  return (
    <table className="invoice-items-table">
      <thead>
        <tr>
          <th>#</th>
          <th>Item</th>
          <th>HSN</th>
          <th className="text-end">Qty</th>
          <th className="text-end">Rate</th>
          {gstHeaderCells(isInterstate)}
          <th className="text-end">Amount</th>
        </tr>
      </thead>
      <tbody>
        {items.map((it, i) => (
          <tr key={it.bi_billitemid}>
            <td>{i + 1}</td>
            <td>{itemLabel(it)}</td>
            <td>{it.pr_hsn}</td>
            <td className="text-end">
              {it.bi_quantity} {it.pr_unit}
            </td>
            <td className="text-end">{money(it.bi_price)}</td>
            {gstDataCells(it, isInterstate)}
            <td className="text-end">{money(it.bi_total)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function WholesaleTable({ groups, isInterstate }) {
  return (
    <table className="invoice-items-table">
      <thead>
        <tr>
          <th>#</th>
          <th>Item</th>
          <th>HSN</th>
          <th className="text-end">Qty</th>
          <th className="text-end">Rate</th>
          {gstHeaderCells(isInterstate)}
          <th className="text-end">Amount</th>
        </tr>
      </thead>
      <tbody>
        {groups.map((g, gi) => (
          <React.Fragment key={g.style || gi}>
            <tr className="invoice-style-group-row">
              <td colSpan={isInterstate ? 7 : 8}>
                <strong>{g.style || "(ungrouped)"}</strong>
              </td>
            </tr>
            {g.lines.map((it, i) => (
              <tr key={it.bi_billitemid}>
                <td>{gi + 1}.{i + 1}</td>
                <td>{itemLabel(it)}</td>
                <td>{it.pr_hsn}</td>
                <td className="text-end">
                  {it.bi_quantity} {it.pr_unit}
                </td>
                <td className="text-end">{money(it.bi_price)}</td>
                {gstDataCells(it, isInterstate)}
                <td className="text-end">{money(it.bi_total)}</td>
              </tr>
            ))}
            <tr className="invoice-style-group-total">
              <td colSpan={isInterstate ? 6 : 7} className="text-end">
                Subtotal ({g.total_qty})
              </td>
              <td className="text-end">{money(g.total_amount)}</td>
            </tr>
          </React.Fragment>
        ))}
      </tbody>
    </table>
  );
}
