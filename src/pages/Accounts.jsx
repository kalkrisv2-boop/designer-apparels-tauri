import React, { useEffect, useState } from "react";
import { accountsInit, addVoucher, listLedger, listDaybook } from "../api.js";

// Phase 2: Vouchers / Ledger / Daybook, ported from routes/accounts.py.
// One screen with three sub-tabs rather than three separate sidebar
// entries, since they're really one workflow: Vouchers is the entry
// form, Ledger and Daybook are two different views of what got entered
// (a running cash/bank balance vs a paired debit/credit journal).
//
// Balance convention (confirmed, carried over exactly from Python):
// linking a voucher to a customer/supplier REDUCES what they owe --
// see accounts.rs's module doc for the full reasoning, including why
// this is the opposite direction from an unpaid sale in Billing.

const money = (v) => (Number(v) || 0).toFixed(2);

export default function Accounts() {
  const [subTab, setSubTab] = useState("vouchers");
  return (
    <div>
      <h1>Accounts</h1>
      <div className="toolbar no-print">
        <button className={subTab === "vouchers" ? "primary" : "secondary"} onClick={() => setSubTab("vouchers")}>
          Vouchers
        </button>
        <button className={subTab === "ledger" ? "primary" : "secondary"} onClick={() => setSubTab("ledger")}>
          Ledger
        </button>
        <button className={subTab === "daybook" ? "primary" : "secondary"} onClick={() => setSubTab("daybook")}>
          Daybook
        </button>
      </div>
      {subTab === "vouchers" && <Vouchers />}
      {subTab === "ledger" && <Ledger />}
      {subTab === "daybook" && <Daybook />}
    </div>
  );
}

function Vouchers() {
  const [init, setInit] = useState(null);
  const [error, setError] = useState("");
  const [success, setSuccess] = useState("");

  const [voucherType, setVoucherType] = useState("receipt");
  const [mode, setMode] = useState("Cash");
  const [amount, setAmount] = useState("");
  const [date, setDate] = useState("");
  const [partyType, setPartyType] = useState("none");
  const [partyId, setPartyId] = useState("");
  const [accountName, setAccountName] = useState("");
  const [description, setDescription] = useState("");
  const [saving, setSaving] = useState(false);

  const load = () =>
    accountsInit()
      .then((data) => {
        setInit(data);
        setDate((d) => d || data.today);
      })
      .catch((e) => setError(String(e)));

  useEffect(() => {
    load();
  }, []);

  const submit = async () => {
    setError("");
    setSuccess("");
    if (!amount || Number(amount) <= 0) {
      setError("Enter an amount greater than zero.");
      return;
    }
    if (partyType === "none" && !accountName) {
      setError("Select an account, or link this voucher to a customer/supplier.");
      return;
    }
    setSaving(true);
    try {
      const result = await addVoucher({
        voucher_type: voucherType,
        mode,
        amount: Number(amount),
        date,
        party_type: partyType,
        party_id: partyType === "none" ? null : Number(partyId) || null,
        account_name: partyType === "none" ? accountName : "",
        description,
      });
      setSuccess(`Saved — running balance is now ₹${money(result.closing_balance)}.`);
      setAmount("");
      setDescription("");
      setPartyId("");
      setAccountName("");
      load();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  if (!init) return <div>Loading…</div>;

  const parties = partyType === "customer" ? init.customers : partyType === "supplier" ? init.suppliers : [];

  return (
    <div>
      <div className="panel">
        <h2>New Voucher</h2>
        {error && <div className="error-banner">{error}</div>}
        {success && <div className="success-banner">{success}</div>}
        <div className="grid grid-3">
          <div>
            <label>Type</label>
            <select value={voucherType} onChange={(e) => setVoucherType(e.target.value)}>
              <option value="receipt">Receipt (money in)</option>
              <option value="payment">Payment (money out)</option>
            </select>
          </div>
          <div>
            <label>Mode</label>
            <select value={mode} onChange={(e) => setMode(e.target.value)}>
              <option value="Cash">Cash</option>
              <option value="Bank">Bank</option>
            </select>
          </div>
          <div>
            <label>Amount (₹)</label>
            <input type="number" step="0.01" value={amount} onChange={(e) => setAmount(e.target.value)} />
          </div>
          <div>
            <label>Date</label>
            <input type="date" value={date} onChange={(e) => setDate(e.target.value)} />
          </div>
          <div>
            <label>Link To</label>
            <select
              value={partyType}
              onChange={(e) => {
                setPartyType(e.target.value);
                setPartyId("");
                setAccountName("");
              }}
            >
              <option value="none">An Account (not a customer/supplier)</option>
              <option value="customer">A Customer</option>
              <option value="supplier">A Supplier</option>
            </select>
          </div>
          {partyType === "none" ? (
            <div>
              <label>Account</label>
              <select value={accountName} onChange={(e) => setAccountName(e.target.value)}>
                <option value="">— select —</option>
                {init.accounts.map((a) => (
                  <option key={a.refid} value={a.acc_name}>
                    {a.acc_name}
                  </option>
                ))}
              </select>
            </div>
          ) : (
            <div>
              <label>{partyType === "customer" ? "Customer" : "Supplier"}</label>
              <select value={partyId} onChange={(e) => setPartyId(e.target.value)}>
                <option value="">— select —</option>
                {parties.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
              </select>
            </div>
          )}
          <div style={{ gridColumn: "span 3" }}>
            <label>Description (optional)</label>
            <input value={description} onChange={(e) => setDescription(e.target.value)} />
          </div>
        </div>
        <div style={{ marginTop: 10 }}>
          <button className="primary" onClick={submit} disabled={saving}>
            {saving ? "Saving…" : `Save ${voucherType === "receipt" ? "Receipt" : "Payment"}`}
          </button>
        </div>
        {partyType !== "none" && (
          <div className="muted-note">
            This reduces what this {partyType} owes (settling a debt), regardless of Receipt/Payment — same
            convention your Python app used.
          </div>
        )}
      </div>

      <div className="panel">
        <h2>Recent Entries</h2>
        {init.recent.length === 0 ? (
          <div className="muted-note">No entries yet.</div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Date</th>
                <th>Debit</th>
                <th>Credit</th>
                <th className="text-end">Amount</th>
                <th>Description</th>
              </tr>
            </thead>
            <tbody>
              {init.recent.map((r) => (
                <tr key={r.refid}>
                  <td>{r.day_book_date}</td>
                  <td>{r.debit}</td>
                  <td>{r.credit}</td>
                  <td className="text-end">{money(r.day_book_amount)}</td>
                  <td>{r.description}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

function DateFilterBar({ dateFrom, setDateFrom, dateTo, setDateTo, onApply }) {
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
    </div>
  );
}

function Ledger() {
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [data, setData] = useState(null);
  const [error, setError] = useState("");

  const load = () =>
    listLedger(dateFrom || undefined, dateTo || undefined)
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
          <div className="totals-box" style={{ maxWidth: 320 }}>
            <div className="totals-row">
              <span>Total In</span>
              <span>₹{money(data.total_in)}</span>
            </div>
            <div className="totals-row">
              <span>Total Out</span>
              <span>₹{money(data.total_out)}</span>
            </div>
            <div className="totals-row grand">
              <span>Current Balance</span>
              <span>₹{money(data.current_balance)}</span>
            </div>
          </div>
        )}
      </div>
      <div className="panel">
        {!data ? (
          <div>Loading…</div>
        ) : data.entries.length === 0 ? (
          <div className="muted-note">No ledger entries in this range.</div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Date</th>
                <th>Particulars</th>
                <th>Mode</th>
                <th>Party</th>
                <th className="text-end">Opening</th>
                <th className="text-end">Amount</th>
                <th className="text-end">Closing</th>
              </tr>
            </thead>
            <tbody>
              {data.entries.map((e) => (
                <tr key={e.tr_id}>
                  <td>{e.tr_date}</td>
                  <td>{e.tr_particulars}</td>
                  <td>{e.pay_type}</td>
                  <td>{e.tr_name}</td>
                  <td className="text-end">{money(e.tr_openingbalance)}</td>
                  <td className="text-end">{e.tr_transactionamount >= 0 ? "+" : ""}{money(e.tr_transactionamount)}</td>
                  <td className="text-end">{money(e.tr_closingbalance)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

function Daybook() {
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [data, setData] = useState(null);
  const [error, setError] = useState("");

  const load = () =>
    listDaybook(dateFrom || undefined, dateTo || undefined)
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
          <div className="totals-box" style={{ maxWidth: 320 }}>
            <div className="totals-row grand">
              <span>Total</span>
              <span>₹{money(data.total_amount)}</span>
            </div>
          </div>
        )}
      </div>
      <div className="panel">
        {!data ? (
          <div>Loading…</div>
        ) : data.entries.length === 0 ? (
          <div className="muted-note">No daybook entries in this range.</div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Date</th>
                <th>Debit</th>
                <th>Credit</th>
                <th className="text-end">Amount</th>
                <th>Description</th>
              </tr>
            </thead>
            <tbody>
              {data.entries.map((e) => (
                <tr key={e.refid}>
                  <td>{e.day_book_date}</td>
                  <td>{e.debit}</td>
                  <td>{e.credit}</td>
                  <td className="text-end">{money(e.day_book_amount)}</td>
                  <td>{e.description}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
