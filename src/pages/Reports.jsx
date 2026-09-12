import React, { useEffect, useState } from "react";
import { gstReport, profitReport, customerReport, exportGstr1 } from "../api.js";

// Phase 5: read-only reports, ported from routes/reports.py. One screen
// with three sub-tabs (GST / Profit / Customers), same shape as
// Accounts.jsx's Vouchers/Ledger/Daybook split -- these are three
// different views over the same billing/returns data, not three
// unrelated features.
//
// Loading-state JSX pattern (see PROJECT_STATUS.md §2's last bullet,
// found the hard way in Phase 4): every sub-tab below renders its error
// banner in the filter panel UNCONDITIONALLY, separate from the `!data`
// check that only gates the results table underneath it. Never nest the
// error banner inside `if (!data) return <Loading/>` -- that swallows
// the very error that explains why data never arrived.

const money = (v) => (Number(v) || 0).toFixed(2);

export default function Reports() {
  const [subTab, setSubTab] = useState("gst");
  return (
    <div>
      <h1>Reports</h1>
      <div className="toolbar no-print">
        <button className={subTab === "gst" ? "primary" : "secondary"} onClick={() => setSubTab("gst")}>
          GST
        </button>
        <button className={subTab === "profit" ? "primary" : "secondary"} onClick={() => setSubTab("profit")}>
          Profit
        </button>
        <button className={subTab === "customers" ? "primary" : "secondary"} onClick={() => setSubTab("customers")}>
          Customers
        </button>
      </div>
      {subTab === "gst" && <GstReport />}
      {subTab === "profit" && <ProfitReport />}
      {subTab === "customers" && <CustomerReport />}
    </div>
  );
}

function DateFilterBar({ dateFrom, setDateFrom, dateTo, setDateTo, onApply, children }) {
  return (
    <div className="toolbar no-print">
      <div>
        <label>From</label>
        <input type="date" value={dateFrom} onChange={(e) => setDateFrom(e.target.value)} />
      </div>
      <div>
        <label>To</label>
        <input type="date" value={dateTo} onChange={(e) => setDateTo(e.target.value)} />
      </div>
      <button className="secondary" style={{ marginTop: 18 }} onClick={onApply}>
        Apply
      </button>
      {children}
    </div>
  );
}

function GstReport() {
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [data, setData] = useState(null);
  const [error, setError] = useState("");
  const [exporting, setExporting] = useState(false);
  const [exportMsg, setExportMsg] = useState("");

  const load = () =>
    gstReport(dateFrom, dateTo)
      .then(setData)
      .catch((e) => setError(String(e)));

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const doExport = async () => {
    setError("");
    setExportMsg("");
    setExporting(true);
    try {
      const result = await exportGstr1(dateFrom, dateTo);
      if (!result) return; // user cancelled the save dialog
      let msg = `Exported ${result.b2b_count} B2B, ${result.b2c_large_count} B2C-Large, ${result.b2c_small_count} B2C-Small row(s) to ${result.savePath}.`;
      if (result.needs_review_count > 0) {
        msg += ` ${result.needs_review_count} bill(s) had no reliable Place of Supply and were put in the zip's needs_review CSV instead of being guessed at -- check those manually.`;
      }
      setExportMsg(msg);
    } catch (e) {
      setError(String(e));
    } finally {
      setExporting(false);
    }
  };

  return (
    <div>
      <div className="panel">
        <DateFilterBar dateFrom={dateFrom} setDateFrom={setDateFrom} dateTo={dateTo} setDateTo={setDateTo} onApply={load}>
          <button className="secondary" style={{ marginTop: 18 }} onClick={doExport} disabled={exporting}>
            {exporting ? "Exporting…" : "Export GSTR-1 (B2B/B2C CSVs)"}
          </button>
        </DateFilterBar>
        {error && <div className="error-banner">{error}</div>}
        {exportMsg && <div className="success-banner">{exportMsg}</div>}
        {data && (
          <div className="grid grid-3" style={{ maxWidth: 700 }}>
            <div className="totals-box">
              <div className="totals-row">
                <span>Taxable Value</span>
                <span>₹{money(data.totals.taxable)}</span>
              </div>
            </div>
            <div className="totals-box">
              <div className="totals-row grand">
                <span>Total GST Collected</span>
                <span>₹{money(data.totals.gst)}</span>
              </div>
            </div>
            <div className="totals-box">
              <div className="totals-row">
                <span>CGST / SGST / IGST</span>
                <span>
                  ₹{money(data.totals.cgst)} / ₹{money(data.totals.sgst)} / ₹{money(data.totals.igst)}
                </span>
              </div>
            </div>
          </div>
        )}
      </div>

      <div className="panel">
        <h2>By GST Rate</h2>
        {!data ? (
          <div>Loading…</div>
        ) : data.rate_summary.length === 0 ? (
          <div className="muted-note">No bills in this range.</div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Rate</th>
                <th className="text-end">Lines</th>
                <th className="text-end">Taxable</th>
                <th className="text-end">CGST</th>
                <th className="text-end">SGST</th>
                <th className="text-end">IGST</th>
                <th className="text-end">Total GST</th>
              </tr>
            </thead>
            <tbody>
              {data.rate_summary.map((b) => (
                <tr key={b.rate}>
                  <td>{b.rate}%</td>
                  <td className="text-end">{b.count}</td>
                  <td className="text-end">₹{money(b.taxable)}</td>
                  <td className="text-end">₹{money(b.cgst)}</td>
                  <td className="text-end">₹{money(b.sgst)}</td>
                  <td className="text-end">₹{money(b.igst)}</td>
                  <td className="text-end">₹{money(b.gst)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <div className="panel">
        <h2>Line-by-Line Detail</h2>
        {!data ? (
          <div>Loading…</div>
        ) : data.lines.length === 0 ? (
          <div className="muted-note">No bills in this range.</div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Bill #</th>
                <th>Date</th>
                <th>Customer</th>
                <th className="text-end">Taxable</th>
                <th className="text-end">Rate</th>
                <th className="text-end">GST Amt</th>
              </tr>
            </thead>
            <tbody>
              {data.lines.map((l, i) => (
                <tr key={i}>
                  <td>{l.be_billnumber}</td>
                  <td>{l.be_billdate}</td>
                  <td>{l.be_customername || "Walk-in"}</td>
                  <td className="text-end">₹{money(l.bi_total)}</td>
                  <td className="text-end">{l.bi_vatper}%</td>
                  <td className="text-end">₹{money(l.bi_vatamount)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

function ProfitReport() {
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [data, setData] = useState(null);
  const [error, setError] = useState("");

  const load = () =>
    profitReport(dateFrom, dateTo)
      .then(setData)
      .catch((e) => setError(String(e)));

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div>
      <div className="panel">
        <DateFilterBar dateFrom={dateFrom} setDateFrom={setDateFrom} dateTo={dateTo} setDateTo={setDateTo} onApply={load} />
        {error && <div className="error-banner">{error}</div>}
        {data && (
          <div className="grid grid-3" style={{ maxWidth: 700 }}>
            <div className="totals-box">
              <div className="totals-row">
                <span>Revenue (taxable)</span>
                <span>₹{money(data.total_revenue)}</span>
              </div>
            </div>
            <div className="totals-box">
              <div className="totals-row">
                <span>Cost of Goods Sold</span>
                <span>₹{money(data.total_cost)}</span>
              </div>
            </div>
            <div className="totals-box">
              <div className="totals-row grand">
                <span>Profit</span>
                <span>₹{money(data.total_profit)}</span>
              </div>
            </div>
          </div>
        )}
        {data && data.flagged_lines.length > 0 && (
          <div className="error-banner" style={{ marginTop: 10 }}>
            {data.flagged_lines.length} line item(s) excluded from the totals above due to a missing or zero cost
            basis -- treating that as "this sale cost nothing" would inflate profit, so it's not counted either way
            rather than guessed at. These lines represent ₹{money(data.flagged_revenue)} of revenue currently
            sitting outside this report entirely. See "Needs Review" below.
          </div>
        )}
      </div>

      <div className="panel">
        <h2>By Product</h2>
        {!data ? (
          <div>Loading…</div>
        ) : data.product_summary.length === 0 ? (
          <div className="muted-note">No sales in this range.</div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Product</th>
                <th className="text-end">Qty Sold</th>
                <th className="text-end">Revenue</th>
                <th className="text-end">Cost</th>
                <th className="text-end">Profit</th>
              </tr>
            </thead>
            <tbody>
              {data.product_summary.map((p, i) => (
                <tr key={i}>
                  <td>
                    {p.product_code} — {p.product_name}
                  </td>
                  <td className="text-end">{p.qty}</td>
                  <td className="text-end">₹{money(p.revenue)}</td>
                  <td className="text-end">₹{money(p.cost)}</td>
                  <td className="text-end">₹{money(p.profit)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <div className="panel">
        <h2>Line-by-Line Detail</h2>
        {!data ? (
          <div>Loading…</div>
        ) : data.lines.length === 0 ? (
          <div className="muted-note">No sales in this range.</div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Bill #</th>
                <th>Date</th>
                <th>Customer</th>
                <th>Product</th>
                <th className="text-end">Qty</th>
                <th className="text-end">Revenue</th>
                <th className="text-end">Cost</th>
                <th className="text-end">Profit</th>
              </tr>
            </thead>
            <tbody>
              {data.lines.map((l, i) => (
                <tr key={i}>
                  <td>{l.bill_number}</td>
                  <td>{l.bill_date}</td>
                  <td>{l.customer_name || "Walk-in"}</td>
                  <td>
                    {l.product_code} — {l.product_name}
                  </td>
                  <td className="text-end">{l.qty}</td>
                  <td className="text-end">₹{money(l.revenue)}</td>
                  <td className="text-end">₹{money(l.cost)}</td>
                  <td className="text-end">₹{money(l.profit)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {data && data.flagged_lines.length > 0 && (
        <div className="panel">
          <h2>Needs Review — Excluded from Totals</h2>
          <div className="muted-note">
            These lines have no usable cost basis on record, so they're kept out of the Revenue/Cost/Profit figures
            above entirely rather than being counted as 100% profit. Fix the product's purchase price and re-bill,
            or confirm it really was a zero-cost giveaway, on a case-by-case basis.
          </div>
          <table>
            <thead>
              <tr>
                <th>Bill #</th>
                <th>Date</th>
                <th>Customer</th>
                <th>Product</th>
                <th className="text-end">Qty</th>
                <th className="text-end">Revenue</th>
                <th>Reason</th>
              </tr>
            </thead>
            <tbody>
              {data.flagged_lines.map((l, i) => (
                <tr key={i}>
                  <td>{l.bill_number}</td>
                  <td>{l.bill_date}</td>
                  <td>{l.customer_name || "Walk-in"}</td>
                  <td>
                    {l.product_code} — {l.product_name}
                  </td>
                  <td className="text-end">{l.qty}</td>
                  <td className="text-end">₹{money(l.revenue)}</td>
                  <td>{l.reason}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function CustomerReport() {
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [customerId, setCustomerId] = useState(null);
  const [data, setData] = useState(null);
  const [error, setError] = useState("");

  const load = (cid) =>
    customerReport(dateFrom, dateTo, cid)
      .then(setData)
      .catch((e) => setError(String(e)));

  useEffect(() => {
    load(customerId);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const openStatement = (cid) => {
    setCustomerId(cid);
    load(cid);
  };
  const backToAll = () => {
    setCustomerId(null);
    load(null);
  };

  const sc = data?.selected_customer;

  return (
    <div>
      <div className="panel">
        <DateFilterBar dateFrom={dateFrom} setDateFrom={setDateFrom} dateTo={dateTo} setDateTo={setDateTo} onApply={() => load(customerId)} />
        {error && <div className="error-banner">{error}</div>}
      </div>

      {!data ? (
        <div className="panel">
          <div>Loading…</div>
        </div>
      ) : !sc ? (
        <>
          <div className="grid grid-3" style={{ maxWidth: 700 }}>
            <div className="totals-box">
              <div className="totals-row">
                <span>Active Customers</span>
                <span>{data.customers.length}</span>
              </div>
            </div>
            <div className="totals-box">
              <div className="totals-row grand">
                <span>Total Outstanding</span>
                <span>₹{money(data.total_outstanding)}</span>
              </div>
            </div>
            <div className="totals-box">
              <div className="totals-row">
                <span>Balance Reconciliation</span>
                <span>
                  {data.mismatch_count} mismatch{data.mismatch_count !== 1 ? "es" : ""}
                </span>
              </div>
            </div>
          </div>

          {data.mismatch_count > 0 && (
            <div className="panel" style={{ marginTop: 10 }}>
              <div className="muted-note">
                Expected balance = unpaid bills − unresolved return credit, compared against the customer's actual
                balance on file. A mismatch can be a real data problem, or can simply mean that customer's balance
                was adjusted by a standalone Voucher (Accounts) -- this check doesn't look at Vouchers, so that's a
                known, expected source of false positives here, not necessarily an error. Open a flagged customer's
                statement to see the numbers side by side.
              </div>
            </div>
          )}

          <div className="panel">
            <h2>Customer Balances</h2>
            {data.customers.length === 0 ? (
              <div className="muted-note">No active customers.</div>
            ) : (
              <table>
                <thead>
                  <tr>
                    <th>Customer</th>
                    <th>Mobile</th>
                    <th className="text-end">Balance</th>
                    <th className="text-center">Reconciled</th>
                    <th></th>
                  </tr>
                </thead>
                <tbody>
                  {data.customers.map((c) => (
                    <tr key={c.cs_customerid}>
                      <td>{c.cs_customername || "Unnamed Customer"}</td>
                      <td>{c.cs_customerphone || ""}</td>
                      <td className="text-end">₹{money(c.cs_balance)}</td>
                      <td className="text-center">
                        {c.reconciled ? "✓" : `✕ mismatch (expected ₹${money(c.expected_balance)})`}
                      </td>
                      <td className="text-end">
                        <button className="secondary" onClick={() => openStatement(c.cs_customerid)}>
                          View Statement
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </>
      ) : (
        <>
          <div className="panel">
            <button className="secondary" onClick={backToAll}>
              ← Back to all customers
            </button>
          </div>

          <div className="panel">
            <h2>{sc.cs_customername || "Unnamed Customer"}</h2>
            <div className="muted-note">{sc.cs_customerphone || ""}</div>
            <div style={{ marginTop: 8 }}>
              Current Balance: <strong>₹{money(sc.cs_balance)}</strong>
            </div>
            {sc.reconciled ? (
              <div className="muted-note">✓ Reconciles cleanly against bills and returns.</div>
            ) : (
              <div className="error-banner">
                Expected ₹{money(sc.expected_balance)} from bills/returns alone (actual balance is ₹
                {money(sc.cs_balance)}) -- the difference may be a standalone Voucher adjustment, which this check
                doesn't include.
              </div>
            )}
          </div>

          <div className="panel">
            <h2>Bills</h2>
            {data.bills.length === 0 ? (
              <div className="muted-note">No bills in this range.</div>
            ) : (
              <table>
                <thead>
                  <tr>
                    <th>Bill #</th>
                    <th>Date</th>
                    <th className="text-end">Total</th>
                    <th className="text-end">Paid</th>
                    <th className="text-end">Balance</th>
                  </tr>
                </thead>
                <tbody>
                  {data.bills.map((b) => (
                    <tr key={b.be_billid}>
                      <td>{b.be_billnumber}</td>
                      <td>{b.be_billdate}</td>
                      <td className="text-end">₹{b.be_gtotal}</td>
                      <td className="text-end">₹{money(b.be_paidamount)}</td>
                      <td className="text-end">₹{money(b.be_balance)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>

          <div className="panel">
            <h2>Sales Returns</h2>
            {data.returns.length === 0 ? (
              <div className="muted-note">No sales returns in this range.</div>
            ) : (
              <table>
                <thead>
                  <tr>
                    <th>Return #</th>
                    <th>Against Bill #</th>
                    <th>Date</th>
                    <th className="text-end">Total</th>
                    <th className="text-end">Refunded</th>
                    <th className="text-end">Credit (unresolved)</th>
                  </tr>
                </thead>
                <tbody>
                  {data.returns.map((r) => (
                    <tr key={r.sre_billid}>
                      <td>{r.sre_billnumber}</td>
                      <td>{r.sre_rebill || ""}</td>
                      <td>{r.sre_billdate}</td>
                      <td className="text-end">₹{r.sre_gtotal}</td>
                      <td className="text-end">₹{money(r.sre_paidamount)}</td>
                      <td className="text-end">₹{money(r.sre_balance)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>

          <div className="panel">
            <h2>Ledger Entries</h2>
            {data.ledger_entries.length === 0 ? (
              <div className="muted-note">No ledger entries in this range.</div>
            ) : (
              <table>
                <thead>
                  <tr>
                    <th>Date</th>
                    <th>Particulars</th>
                    <th>Mode</th>
                    <th className="text-end">Amount</th>
                    <th className="text-end">Balance</th>
                  </tr>
                </thead>
                <tbody>
                  {data.ledger_entries.map((t) => (
                    <tr key={t.tr_id}>
                      <td>{t.tr_date}</td>
                      <td>{t.tr_particulars}</td>
                      <td>{t.pay_type}</td>
                      <td className="text-end">₹{money(t.tr_transactionamount)}</td>
                      <td className="text-end">₹{money(t.tr_closingbalance)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </>
      )}
    </div>
  );
}
