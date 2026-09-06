import React, { useEffect, useMemo, useState } from "react";
import {
  createInvoice,
  listCatalogItems,
  openFile,
  INDIAN_STATES,
  GST_RATES,
} from "../api.js";

const COMPANY_STATE_CODE = "32"; // Kerala — Designer Apparels' home state

function emptyLine(sr) {
  return {
    sr,
    description: "",
    hsn_sac: "6212",
    size_ratio: "",
    qty: 1,
    rate: 0,
    gst_rate: 5,
    gst_mode: "exclusive",
    amount: 0,
  };
}

function todayDdMmYyyy() {
  const d = new Date();
  const dd = String(d.getDate()).padStart(2, "0");
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  return `${dd}/${mm}/${d.getFullYear()}`;
}

// Parses a Size/Ratio string into its total quantity.
// Each comma-separated segment can be "BAND/QTY" (e.g. "32/12") or
// "CUP/BAND/QTY" (e.g. "A/32/6", "D/36/12") — the quantity is always the
// last slash-separated value. Returns null if no segment yields a valid
// number (e.g. the field is empty or still mid-typing).
function sumQtyFromSizeRatio(str) {
  if (!str || !str.trim()) return null;
  const segments = str.split(",");
  let total = 0;
  let sawValid = false;
  for (const seg of segments) {
    const trimmed = seg.trim();
    if (!trimmed) continue;
    const parts = trimmed.split("/");
    const qtyPart = parts[parts.length - 1].trim();
    const qty = parseFloat(qtyPart);
    if (!isNaN(qty)) {
      total += qty;
      sawValid = true;
    }
  }
  return sawValid ? total : null;
}

export default function Billing() {
  const [catalog, setCatalog] = useState([]);
  const [header, setHeader] = useState({
    invoice_date: todayDdMmYyyy(),
    buyer_name: "",
    buyer_address: "",
    buyer_gstin: "",
    buyer_state: "Kerala",
    buyer_state_code: COMPANY_STATE_CODE,
    transport_name: "",
    salesman: "",
  });
  const [lines, setLines] = useState([emptyLine(1)]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [success, setSuccess] = useState("");

  useEffect(() => {
    listCatalogItems().then(setCatalog).catch(() => {});
  }, []);

  const isInterstate = header.buyer_state_code.trim() !== COMPANY_STATE_CODE;

  const computedLines = useMemo(() => {
    return lines.map((l) => {
      const gross = (Number(l.qty) || 0) * (Number(l.rate) || 0);
      let taxable, tax;
      if (l.gst_mode === "inclusive") {
        taxable = gross / (1 + (Number(l.gst_rate) || 0) / 100);
        tax = gross - taxable;
      } else {
        taxable = gross;
        tax = gross * ((Number(l.gst_rate) || 0) / 100);
      }
      return { ...l, _taxable: taxable, _tax: tax };
    });
  }, [lines]);

  const totals = useMemo(() => {
    let taxable = 0, cgst = 0, sgst = 0, igst = 0;
    for (const l of computedLines) {
      taxable += l._taxable;
      if (isInterstate) igst += l._tax;
      else {
        cgst += l._tax / 2;
        sgst += l._tax / 2;
      }
    }
    const preRound = taxable + cgst + sgst + igst;
    const grand = Math.round(preRound);
    const roundOff = grand - preRound;
    return { taxable, cgst, sgst, igst, grand, roundOff };
  }, [computedLines, isInterstate]);

  const updateLine = (idx, patch) => {
    setLines((prev) => prev.map((l, i) => (i === idx ? { ...l, ...patch } : l)));
  };

  const addLine = () => setLines((prev) => [...prev, emptyLine(prev.length + 1)]);

  const removeLine = (idx) => {
    setLines((prev) => {
      const next = prev.filter((_, i) => i !== idx);
      return next.map((l, i) => ({ ...l, sr: i + 1 }));
    });
  };

  const pickFromCatalog = (idx, catalogId) => {
    const item = catalog.find((c) => String(c.id) === String(catalogId));
    if (!item) return;
    updateLine(idx, {
      description: item.description,
      hsn_sac: item.hsn_sac,
      rate: item.default_rate,
      gst_rate: item.default_gst_rate,
    });
  };

  const handleKeyDown = (e, idx, isLastField) => {
    // Tab on the last field of the last row adds a new row automatically.
    if (e.key === "Enter") {
      e.preventDefault();
      if (isLastField && idx === lines.length - 1) addLine();
    }
  };

  const resetForm = () => {
    setHeader({
      invoice_date: todayDdMmYyyy(),
      buyer_name: "",
      buyer_address: "",
      buyer_gstin: "",
      buyer_state: "Kerala",
      buyer_state_code: COMPANY_STATE_CODE,
      transport_name: "",
      salesman: "",
    });
    setLines([emptyLine(1)]);
  };

  const submit = async () => {
    setError("");
    setSuccess("");
    if (!header.buyer_name.trim()) {
      setError("Buyer name is required.");
      return;
    }
    if (lines.length === 0 || lines.every((l) => !l.description.trim())) {
      setError("Add at least one item.");
      return;
    }

    setSaving(true);
    try {
      const payload = {
        ...header,
        items: lines
          .filter((l) => l.description.trim())
          .map((l) => ({
            sr: l.sr,
            description: l.description,
            hsn_sac: l.hsn_sac,
            size_ratio: l.size_ratio,
            qty: Number(l.qty) || 0,
            rate: Number(l.rate) || 0,
            gst_rate: Number(l.gst_rate) || 0,
            gst_mode: l.gst_mode,
            amount: 0, // computed server-side
          })),
      };
      const invoice = await createInvoice(payload);
      setSuccess(
        `Invoice #${invoice.invoice_number} saved and PDF generated (₹${invoice.grand_total}).`
      );
      resetForm();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const openLastPdf = async (path) => {
    try {
      await openFile(path);
    } catch (e) {
      // ignore — user can find it in the output folder
    }
  };

  return (
    <div>
      <h1>New Invoice</h1>
      {error && <div className="error-banner">{error}</div>}
      {success && <div className="success-banner">{success}</div>}

      <div className="panel">
        <h2>Buyer Details</h2>
        <div className="grid grid-3">
          <div>
            <label>Invoice Date</label>
            <input
              value={header.invoice_date}
              onChange={(e) => setHeader({ ...header, invoice_date: e.target.value })}
              placeholder="DD/MM/YYYY"
            />
          </div>
          <div>
            <label>Buyer Name *</label>
            <input
              value={header.buyer_name}
              onChange={(e) => setHeader({ ...header, buyer_name: e.target.value })}
              autoFocus
            />
          </div>
          <div>
            <label>Buyer GSTIN</label>
            <input
              value={header.buyer_gstin}
              onChange={(e) => setHeader({ ...header, buyer_gstin: e.target.value })}
            />
          </div>
          <div style={{ gridColumn: "span 2" }}>
            <label>Buyer Address</label>
            <input
              value={header.buyer_address}
              onChange={(e) => setHeader({ ...header, buyer_address: e.target.value })}
            />
          </div>
          <div>
            <label>Buyer State</label>
            <select
              value={header.buyer_state_code}
              onChange={(e) => {
                const st = INDIAN_STATES.find((s) => s.code === e.target.value);
                setHeader({
                  ...header,
                  buyer_state_code: e.target.value,
                  buyer_state: st ? st.name : "",
                });
              }}
            >
              {INDIAN_STATES.map((s) => (
                <option key={s.code} value={s.code}>
                  {s.name} ({s.code})
                </option>
              ))}
            </select>
          </div>
          <div>
            <label>Transport Name</label>
            <input
              value={header.transport_name}
              onChange={(e) => setHeader({ ...header, transport_name: e.target.value })}
            />
          </div>
          <div>
            <label>Salesman</label>
            <input
              value={header.salesman}
              onChange={(e) => setHeader({ ...header, salesman: e.target.value })}
            />
          </div>
        </div>
        <div className="muted-note">
          {isInterstate
            ? "Inter-state buyer → IGST will be applied."
            : "Intra-state buyer (Kerala) → CGST + SGST will be applied."}
        </div>
      </div>

      <div className="panel">
        <h2>Items</h2>
        <table className="items-table">
          <thead>
            <tr>
              <th style={{ width: 30 }}>Sr</th>
              <th>Catalog</th>
              <th>Description</th>
              <th style={{ width: 70 }}>HSN</th>
              <th style={{ width: 120 }}>Size/Ratio</th>
              <th style={{ width: 60 }}>Qty (auto)</th>
              <th style={{ width: 80 }}>Rate</th>
              <th style={{ width: 80 }}>GST%</th>
              <th style={{ width: 90 }}>Mode</th>
              <th style={{ width: 80 }}>Amount</th>
              <th style={{ width: 30 }}></th>
            </tr>
          </thead>
          <tbody>
            {computedLines.map((l, idx) => (
              <tr key={idx}>
                <td>{l.sr}</td>
                <td>
                  <select onChange={(e) => pickFromCatalog(idx, e.target.value)} defaultValue="">
                    <option value="" disabled>
                      pick…
                    </option>
                    {catalog.map((c) => (
                      <option key={c.id} value={c.id}>
                        {c.description}
                      </option>
                    ))}
                  </select>
                </td>
                <td>
                  <input
                    value={l.description}
                    onChange={(e) => updateLine(idx, { description: e.target.value })}
                  />
                </td>
                <td>
                  <input
                    value={l.hsn_sac}
                    onChange={(e) => updateLine(idx, { hsn_sac: e.target.value })}
                  />
                </td>
                <td>
                  <input
                    value={l.size_ratio}
                    placeholder="A/32/6, B/34/12, D/36/12"
                    onChange={(e) => {
                      const value = e.target.value;
                      const computedQty = sumQtyFromSizeRatio(value);
                      updateLine(idx, {
                        size_ratio: value,
                        ...(computedQty !== null ? { qty: computedQty } : {}),
                      });
                    }}
                  />
                </td>
                <td>
                  <input
                    type="number"
                    value={l.qty}
                    onChange={(e) => updateLine(idx, { qty: e.target.value })}
                  />
                </td>
                <td>
                  <input
                    type="number"
                    step="0.01"
                    value={l.rate}
                    onChange={(e) => updateLine(idx, { rate: e.target.value })}
                  />
                </td>
                <td>
                  <select
                    value={l.gst_rate}
                    onChange={(e) => updateLine(idx, { gst_rate: e.target.value })}
                  >
                    {GST_RATES.map((r) => (
                      <option key={r} value={r}>
                        {r}%
                      </option>
                    ))}
                  </select>
                </td>
                <td>
                  <select
                    value={l.gst_mode}
                    onChange={(e) => updateLine(idx, { gst_mode: e.target.value })}
                  >
                    <option value="exclusive">Exclusive</option>
                    <option value="inclusive">Inclusive</option>
                  </select>
                </td>
                <td>{l._taxable.toFixed(2)}</td>
                <td>
                  <button className="icon-btn" onClick={() => removeLine(idx)} title="Remove row">
                    ✕
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        <div className="muted-note" style={{ marginBottom: 8 }}>
          Size/Ratio format: <code>CUP/BAND/QTY</code> per size, comma-separated — e.g.{" "}
          <code>A/32/6, B/34/12, D/36/12</code> auto-fills Qty as 6+12+12 = <strong>30</strong>.
          Cup letter is optional (<code>32/12</code> also works). You can still overwrite the
          Qty field by hand if a row needs a different total.
        </div>
        <div style={{ marginTop: 10 }}>
          <button className="secondary" onClick={addLine}>
            + Add Item
          </button>
        </div>

        <div className="totals-box">
          <div className="totals-row">
            <span>Taxable Total</span>
            <span>₹{totals.taxable.toFixed(2)}</span>
          </div>
          {isInterstate ? (
            <div className="totals-row">
              <span>IGST</span>
              <span>₹{totals.igst.toFixed(2)}</span>
            </div>
          ) : (
            <>
              <div className="totals-row">
                <span>CGST</span>
                <span>₹{totals.cgst.toFixed(2)}</span>
              </div>
              <div className="totals-row">
                <span>SGST</span>
                <span>₹{totals.sgst.toFixed(2)}</span>
              </div>
            </>
          )}
          <div className="totals-row">
            <span>Round Off</span>
            <span>₹{totals.roundOff.toFixed(2)}</span>
          </div>
          <div className="totals-row grand">
            <span>Grand Total</span>
            <span>₹{totals.grand.toFixed(2)}</span>
          </div>
        </div>
      </div>

      <div className="toolbar">
        <button className="primary" onClick={submit} disabled={saving}>
          {saving ? "Saving…" : "Save & Generate PDF"}
        </button>
        <button className="secondary" onClick={resetForm} disabled={saving}>
          Clear
        </button>
      </div>
    </div>
  );
}
