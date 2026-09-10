import React, { useEffect, useMemo, useState } from "react";
import { purchaseReturnsInit, purchasesSearchProductsForReturn, purchaseReturnsCheckout } from "../api.js";

// Purchase Returns (stock OUT, back to supplier) -- Phase 3. Product
// search here IS limited to in-stock items -- you can only return what
// you actually have (see purchases_search_products_for_return).
//
// NOTE (matches Python's routes/purchases.py::returns_checkout exactly,
// see purchases.rs's module doc): saving a return does NOT post to the
// Ledger/Daybook and does NOT adjust the supplier's balance -- only the
// return bill is written and stock is decreased. Not fixed here, just
// carried over faithfully from the original app.

const GST_RATES = [0, 5, 12, 18, 28];
const DEFAULT_GST_PCT = 5;

function round2(v) {
  return Math.round((Number(v) || 0) * 100) / 100;
}

function calcLine(l) {
  const taxable = (Number(l.qty) || 0) * (Number(l.price) || 0);
  const gstAmt = taxable * ((Number(l.gst_pct) || 0) / 100);
  return { taxable, gstAmt, lineTotal: taxable + gstAmt };
}

let cartKeySeq = 1;

export default function PurchaseReturns() {
  const [init, setInit] = useState(null);
  const [error, setError] = useState("");
  const [success, setSuccess] = useState(null);

  const [supplierId, setSupplierId] = useState(0);
  const [supplierQuery, setSupplierQuery] = useState("");
  const [supplierName, setSupplierName] = useState("");

  const [gstType, setGstType] = useState("intra");
  const [originalBillNumber, setOriginalBillNumber] = useState("");
  const [invoiceNumber, setInvoiceNumber] = useState("");
  const [invoiceDate, setInvoiceDate] = useState("");
  const [vehicleNumber, setVehicleNumber] = useState("");
  const [payMethod, setPayMethod] = useState("Cash");
  const [paidAmount, setPaidAmount] = useState("");
  const [overallDiscount, setOverallDiscount] = useState(0);
  const [note, setNote] = useState("");

  const [cart, setCart] = useState([]);

  const [productQuery, setProductQuery] = useState("");
  const [productResults, setProductResults] = useState([]);

  const [saving, setSaving] = useState(false);

  const refreshInit = () =>
    purchaseReturnsInit()
      .then(setInit)
      .catch((e) => {
        console.error("purchase_returns_init failed:", e);
        setError(String(e));
      });

  useEffect(() => {
    refreshInit();
  }, []);

  useEffect(() => {
    if (!productQuery.trim()) {
      setProductResults([]);
      return;
    }
    const t = setTimeout(() => {
      purchasesSearchProductsForReturn(productQuery).then(setProductResults).catch(() => setProductResults([]));
    }, 250);
    return () => clearTimeout(t);
  }, [productQuery]);

  const pickSupplier = (s) => {
    setSupplierId(s.id);
    setSupplierQuery(s.name);
    setSupplierName(s.name);
  };

  const clearSupplier = () => {
    setSupplierId(0);
    setSupplierQuery("");
    setSupplierName("");
  };

  const filteredSuppliers = useMemo(() => {
    if (!init || !supplierQuery.trim() || supplierId) return [];
    const q = supplierQuery.trim().toLowerCase();
    return init.suppliers.filter((s) => s.name.toLowerCase().includes(q)).slice(0, 8);
  }, [init, supplierQuery, supplierId]);

  const addProductToCart = (product, qty = 1) => {
    setCart((prev) => [
      ...prev,
      {
        key: cartKeySeq++,
        product_id: product.pr_productid,
        name: product.pr_productname,
        stock: product.pr_stock,
        price: product.pr_purchaseprice || 0,
        qty,
        gst_pct: DEFAULT_GST_PCT,
      },
    ]);
  };

  const updateCartLine = (key, patch) => {
    setCart((prev) => prev.map((l) => (l.key === key ? { ...l, ...patch } : l)));
  };

  const removeCartLine = (key) => setCart((prev) => prev.filter((l) => l.key !== key));

  const totals = useMemo(() => {
    let subtotal = 0;
    let totalGst = 0;
    for (const l of cart) {
      const { taxable, gstAmt } = calcLine(l);
      subtotal += taxable;
      totalGst += gstAmt;
    }
    const grandTotal = round2(subtotal + totalGst - (Number(overallDiscount) || 0));
    const paid = paidAmount === "" ? grandTotal : Number(paidAmount) || 0;
    const balance = round2(grandTotal - paid);
    return { subtotal: round2(subtotal), totalGst: round2(totalGst), grandTotal, balance };
  }, [cart, overallDiscount, paidAmount]);

  const resetAfterSave = () => {
    setCart([]);
    clearSupplier();
    setOriginalBillNumber("");
    setInvoiceNumber("");
    setInvoiceDate("");
    setVehicleNumber("");
    setPaidAmount("");
    setOverallDiscount(0);
    setNote("");
    setGstType("intra");
  };

  const submit = async () => {
    setError("");
    setSuccess(null);
    if (cart.length === 0) {
      setError("No items in this return.");
      return;
    }
    if (!supplierName.trim()) {
      setError("Supplier is required.");
      return;
    }
    for (const l of cart) {
      if (!Number(l.qty) || Number(l.qty) <= 0) {
        setError(`Enter a valid quantity for '${l.name}'.`);
        return;
      }
      if (Number(l.qty) > Number(l.stock)) {
        setError(`Cannot return ${l.qty} of '${l.name}' — only ${l.stock} in stock.`);
        return;
      }
    }
    const paid = paidAmount === "" ? totals.grandTotal : Number(paidAmount) || 0;
    setSaving(true);
    try {
      const result = await purchaseReturnsCheckout({
        items: cart.map((l) => ({
          product_id: l.product_id,
          qty: Number(l.qty) || 0,
          price: Number(l.price) || 0,
          gst_pct: Number(l.gst_pct),
        })),
        gst_type: gstType,
        supplier_id: supplierId || 0,
        supplier_name: supplierName,
        invoice_number: invoiceNumber,
        invoice_date: invoiceDate,
        vehicle_number: vehicleNumber,
        pay_method: payMethod,
        overall_discount: Number(overallDiscount) || 0,
        paid_amount: paid,
        note,
        original_bill_number: originalBillNumber,
      });
      setSuccess(result);
      resetAfterSave();
      refreshInit();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  if (!init) {
    return (
      <div>
        <h1>Purchase Returns</h1>
        {error ? (
          <div className="error-banner">
            {error}
            <div style={{ marginTop: 8 }}>
              <button className="secondary" onClick={refreshInit}>
                Retry
              </button>
            </div>
          </div>
        ) : (
          <div>Loading…</div>
        )}
      </div>
    );
  }

  return (
    <div>
      <h1>Purchase Returns</h1>
      {error && <div className="error-banner">{error}</div>}
      {success && (
        <div className="success-banner">
          Return #{success.bill_number} saved — Grand Total ₹{success.grand_total.toFixed(2)}.
        </div>
      )}

      <div className="panel">
        <div className="toolbar">
          <div className="muted-note">Next return no. #{init.next_bill_number}</div>
        </div>
      </div>

      <div className="panel">
        <h2>Supplier</h2>
        <div className="grid grid-3">
          <div style={{ position: "relative" }}>
            <label>Search / Select Supplier</label>
            <input
              value={supplierQuery}
              onChange={(e) => {
                setSupplierQuery(e.target.value);
                setSupplierName(e.target.value);
                if (supplierId) setSupplierId(0);
              }}
              placeholder="Name — or type a new one"
            />
            {filteredSuppliers.length > 0 && (
              <div className="search-dropdown">
                {filteredSuppliers.map((s) => (
                  <div key={s.id} className="search-dropdown-item" onClick={() => pickSupplier(s)}>
                    <strong>{s.name}</strong>
                  </div>
                ))}
              </div>
            )}
          </div>
          <div>
            <label>Original Bill Number</label>
            <input value={originalBillNumber} onChange={(e) => setOriginalBillNumber(e.target.value)} placeholder="Purchase bill this return is against" />
          </div>
          <div>
            <label>Invoice Number</label>
            <input value={invoiceNumber} onChange={(e) => setInvoiceNumber(e.target.value)} />
          </div>
          <div>
            <label>Invoice Date</label>
            <input type="date" value={invoiceDate} onChange={(e) => setInvoiceDate(e.target.value)} />
          </div>
          <div>
            <label>Vehicle Number</label>
            <input value={vehicleNumber} onChange={(e) => setVehicleNumber(e.target.value)} />
          </div>
          <div>
            <label>GST Type</label>
            <select value={gstType} onChange={(e) => setGstType(e.target.value)}>
              <option value="intra">Intrastate (CGST+SGST)</option>
              <option value="inter">Interstate (IGST)</option>
            </select>
          </div>
        </div>
        {supplierId ? (
          <div style={{ marginTop: 8 }}>
            <button className="icon-btn" onClick={clearSupplier}>
              ✕ Clear selected supplier
            </button>
          </div>
        ) : null}
      </div>

      <div className="panel">
        <h2>Add Items</h2>
        <div style={{ position: "relative" }}>
          <label>Search Products (in-stock only)</label>
          <input value={productQuery} onChange={(e) => setProductQuery(e.target.value)} placeholder="Type to search…" />
          {productResults.length > 0 && (
            <div className="search-dropdown">
              {productResults.map((p) => (
                <div key={p.pr_productid} className="search-dropdown-item">
                  <div>
                    <strong>{p.pr_productname}</strong>
                    <span className="muted"> · stock {p.pr_stock}</span>
                  </div>
                  <button className="secondary" onClick={() => addProductToCart(p, 1)}>
                    Add
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      <div className="panel">
        <h2>Cart</h2>
        {cart.length === 0 ? (
          <div className="muted-note">No items yet — search above to add products.</div>
        ) : (
          <table className="items-table">
            <thead>
              <tr>
                <th>Item</th>
                <th style={{ width: 70 }}>Qty</th>
                <th style={{ width: 90 }}>In Stock</th>
                <th style={{ width: 90 }}>Price</th>
                <th style={{ width: 90 }}>GST %</th>
                <th style={{ width: 90 }}>Amount</th>
                <th style={{ width: 30 }}></th>
              </tr>
            </thead>
            <tbody>
              {cart.map((l) => {
                const { lineTotal } = calcLine(l);
                const over = Number(l.qty) > Number(l.stock);
                return (
                  <tr key={l.key}>
                    <td>{l.name}</td>
                    <td>
                      <input
                        type="number"
                        min="0"
                        value={l.qty}
                        onChange={(e) => updateCartLine(l.key, { qty: e.target.value })}
                        style={over ? { borderColor: "var(--danger, #c0392b)" } : undefined}
                      />
                    </td>
                    <td className="text-end">{l.stock}</td>
                    <td>
                      <input
                        type="number"
                        step="0.01"
                        value={l.price}
                        onChange={(e) => updateCartLine(l.key, { price: e.target.value })}
                      />
                    </td>
                    <td>
                      <select value={l.gst_pct} onChange={(e) => updateCartLine(l.key, { gst_pct: Number(e.target.value) })}>
                        {GST_RATES.map((r) => (
                          <option key={r} value={r}>
                            {r}%
                          </option>
                        ))}
                      </select>
                    </td>
                    <td className="text-end">{lineTotal.toFixed(2)}</td>
                    <td>
                      <button className="icon-btn" onClick={() => removeCartLine(l.key)} title="Remove">
                        ✕
                      </button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}

        <div className="grid grid-3" style={{ marginTop: 14 }}>
          <div>
            <label>Overall Discount (₹)</label>
            <input type="number" step="0.01" value={overallDiscount} onChange={(e) => setOverallDiscount(e.target.value)} />
          </div>
          <div>
            <label>Paid Amount (₹)</label>
            <input
              type="number"
              step="0.01"
              value={paidAmount}
              placeholder={totals.grandTotal.toFixed(2)}
              onChange={(e) => setPaidAmount(e.target.value)}
            />
          </div>
          <div>
            <label>Pay Method</label>
            <select value={payMethod} onChange={(e) => setPayMethod(e.target.value)}>
              <option value="Cash">Cash</option>
              <option value="Card">Card</option>
              <option value="UPI">UPI</option>
              <option value="Credit">Credit (on account)</option>
            </select>
          </div>
        </div>

        <div className="totals-box">
          <div className="totals-row">
            <span>Subtotal</span>
            <span>₹{totals.subtotal.toFixed(2)}</span>
          </div>
          <div className="totals-row">
            <span>{gstType === "inter" ? "IGST" : "CGST + SGST"}</span>
            <span>₹{totals.totalGst.toFixed(2)}</span>
          </div>
          <div className="totals-row grand">
            <span>Grand Total</span>
            <span>₹{totals.grandTotal.toFixed(2)}</span>
          </div>
        </div>
      </div>

      <div className="panel">
        <h2>Note (optional)</h2>
        <input value={note} onChange={(e) => setNote(e.target.value)} />
      </div>

      <div className="toolbar">
        <button className="primary" onClick={submit} disabled={saving || cart.length === 0}>
          {saving ? "Saving…" : "Save Return"}
        </button>
      </div>

      {init.recent.length > 0 && (
        <div className="panel">
          <h2>Recent Returns</h2>
          <table className="items-table">
            <thead>
              <tr>
                <th>Return #</th>
                <th>Supplier</th>
                <th>Invoice #</th>
                <th>Date</th>
                <th className="text-end">Grand Total</th>
              </tr>
            </thead>
            <tbody>
              {init.recent.map((r) => (
                <tr key={r.pre_billid}>
                  <td>{r.pre_billnumber}</td>
                  <td>{r.pre_customername}</td>
                  <td>{r.pre_invoice_number}</td>
                  <td>{r.pre_billdate}</td>
                  <td className="text-end">₹{r.pre_gtotal.toFixed(2)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
