import React, { useEffect, useMemo, useState } from "react";
import { salesReturnsInit, salesReturnsSearchProducts, salesReturnsLookupBill, salesReturnsCheckout } from "../api.js";

// Sales Returns screen -- Phase 4, mirror of PurchaseReturns.jsx with two
// extra things Purchase Returns didn't need: a per-line Restock choice
// (resaleable vs written off), and a "look up original bill" shortcut
// that pulls a past sale's line items in at their original sold price.
//
// Unlike Purchase Returns (which deliberately posts nothing to
// Ledger/Daybook or the supplier's balance), Sales Returns DOES post to
// the books and adjusts the customer's balance on save -- see
// sales_returns.rs's module doc for why these two Returns flows aren't
// symmetric.

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

export default function SalesReturns() {
  const [init, setInit] = useState(null);
  const [error, setError] = useState("");
  const [success, setSuccess] = useState(null);

  const [customerId, setCustomerId] = useState(null);
  const [customerQuery, setCustomerQuery] = useState("");
  const [customerName, setCustomerName] = useState("");
  const [customerMobile, setCustomerMobile] = useState("");
  const [customerGstin, setCustomerGstin] = useState("");
  const [customerBalance, setCustomerBalance] = useState(null);

  const [gstType, setGstType] = useState("intra");
  const [originalBillNumber, setOriginalBillNumber] = useState("");
  const [lookupBusy, setLookupBusy] = useState(false);
  const [payMethod, setPayMethod] = useState("Cash");
  const [paidAmount, setPaidAmount] = useState(""); // refund amount
  const [overallDiscount, setOverallDiscount] = useState(0);
  const [note, setNote] = useState("");

  const [cart, setCart] = useState([]);

  const [productQuery, setProductQuery] = useState("");
  const [productResults, setProductResults] = useState([]);

  const [saving, setSaving] = useState(false);

  const refreshInit = async () => {
  setError("");

  const timeout = new Promise((_, reject) => {
    setTimeout(
      () => reject(new Error("Sales Returns could not load within 10 seconds.")),
      10_000
    );
  });

  try {
    const result = await Promise.race([salesReturnsInit(), timeout]);
    setInit(result);
  } catch (e) {
    console.error("sales_returns_init failed or timed out:", e);
    setError(String(e));
  }
};

  useEffect(() => {
    refreshInit();
  }, []);

  useEffect(() => {
    if (!productQuery.trim()) {
      setProductResults([]);
      return;
    }
    const t = setTimeout(() => {
      salesReturnsSearchProducts(productQuery).then(setProductResults).catch(() => setProductResults([]));
    }, 250);
    return () => clearTimeout(t);
  }, [productQuery]);

  const pickCustomer = (c) => {
    setCustomerId(c.cs_customerid);
    setCustomerQuery(c.cs_customername);
    setCustomerName(c.cs_customername);
    setCustomerMobile(c.cs_customerphone);
    setCustomerGstin(c.cs_tin_number);
    setCustomerBalance(c.cs_balance);
  };

  const clearCustomer = () => {
    setCustomerId(null);
    setCustomerQuery("");
    setCustomerName("");
    setCustomerMobile("");
    setCustomerGstin("");
    setCustomerBalance(null);
  };

  const filteredCustomers = useMemo(() => {
    if (!init || !customerQuery.trim() || customerId) return [];
    const q = customerQuery.trim().toLowerCase();
    return init.customers
      .filter((c) => c.cs_customername.toLowerCase().includes(q) || c.cs_customerphone.includes(q))
      .slice(0, 8);
  }, [init, customerQuery, customerId]);

  const addProductToCart = (product, qty = 1, priceOverride = null) => {
    setCart((prev) => [
      ...prev,
      {
        key: cartKeySeq++,
        product_id: product.pr_productid,
        name: product.pr_productname,
        size: product.pr_size,
        unit: product.pr_unit,
        stock: product.pr_stock,
        price: priceOverride !== null ? priceOverride : product.pr_saleprice || 0,
        qty,
        gst_pct: DEFAULT_GST_PCT,
        restock: true,
      },
    ]);
  };

  const updateCartLine = (key, patch) => {
    setCart((prev) => prev.map((l) => (l.key === key ? { ...l, ...patch } : l)));
  };

  const removeCartLine = (key) => setCart((prev) => prev.filter((l) => l.key !== key));

  const lookupOriginalBill = async () => {
    setError("");
    if (!originalBillNumber.trim()) return;
    setLookupBusy(true);
    try {
      const result = await salesReturnsLookupBill(originalBillNumber.trim());
      if (result.customer_id) {
        pickCustomer({
          cs_customerid: result.customer_id,
          cs_customername: result.customer_name,
          cs_customerphone: result.customer_mobile,
          cs_tin_number: "",
          cs_balance: 0,
        });
      } else {
        setCustomerName(result.customer_name || "Walk-in Customer");
        setCustomerMobile(result.customer_mobile || "");
      }
      setCart((prev) => [
        ...prev,
        ...result.items.map((it) => ({
          key: cartKeySeq++,
          product_id: it.bi_productid,
          name: it.pr_productname,
          size: it.pr_size,
          unit: it.pr_unit,
          stock: null,
          price: it.bi_price,
          qty: it.bi_quantity,
          gst_pct: DEFAULT_GST_PCT,
          restock: true,
        })),
      ]);
    } catch (e) {
      setError(String(e));
    } finally {
      setLookupBusy(false);
    }
  };

  const totals = useMemo(() => {
    let subtotal = 0;
    let totalGst = 0;
    for (const l of cart) {
      const { taxable, gstAmt } = calcLine(l);
      subtotal += taxable;
      totalGst += gstAmt;
    }
    const grandTotal = round2(subtotal + totalGst - (Number(overallDiscount) || 0));
    const refund = paidAmount === "" ? grandTotal : Number(paidAmount) || 0;
    const creditAmount = round2(grandTotal - refund);
    return { subtotal: round2(subtotal), totalGst: round2(totalGst), grandTotal, creditAmount };
  }, [cart, overallDiscount, paidAmount]);

  const resetAfterSave = () => {
    setCart([]);
    clearCustomer();
    setOriginalBillNumber("");
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
    for (const l of cart) {
      if (!Number(l.qty) || Number(l.qty) <= 0) {
        setError(`Enter a valid quantity for '${l.name}'.`);
        return;
      }
    }
    const refund = paidAmount === "" ? totals.grandTotal : Number(paidAmount) || 0;
    setSaving(true);
    try {
      const result = await salesReturnsCheckout({
        items: cart.map((l) => ({
          product_id: l.product_id,
          qty: Number(l.qty) || 0,
          price: Number(l.price) || 0,
          gst_pct: Number(l.gst_pct),
          restock: !!l.restock,
        })),
        gst_type: gstType,
        customer_name: customerName,
        customer_mobile: customerMobile,
        customer_gstin: customerGstin,
        customer_id: customerId,
        original_bill_number: originalBillNumber,
        pay_method: payMethod,
        overall_discount: Number(overallDiscount) || 0,
        paid_amount: refund,
        note,
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
        <h1>Sales Returns</h1>
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
      <h1>Sales Returns</h1>
      {error && <div className="error-banner">{error}</div>}
      {success && (
        <div className="success-banner">
          Return #{success.bill_number} saved — Grand Total ₹{success.grand_total.toFixed(2)}
          {success.credit_amount > 0.001 ? ` (₹${success.credit_amount.toFixed(2)} credited to customer account)` : ""}.
        </div>
      )}

      <div className="panel">
        <div className="toolbar">
          <div className="muted-note">Next return no. #{init.next_bill_number}</div>
        </div>
      </div>

      <div className="panel">
        <h2>Look Up Original Sale (optional)</h2>
        <div className="grid grid-3">
          <div>
            <label>Original Bill Number</label>
            <input value={originalBillNumber} onChange={(e) => setOriginalBillNumber(e.target.value)} placeholder="e.g. 1024" />
          </div>
          <div style={{ display: "flex", alignItems: "flex-end" }}>
            <button className="secondary" onClick={lookupOriginalBill} disabled={lookupBusy || !originalBillNumber.trim()}>
              {lookupBusy ? "Looking up…" : "Pull Items From This Bill"}
            </button>
          </div>
        </div>
        <div className="muted-note">
          Fills in the customer and cart from that sale at the price it was originally sold for. Optional — you can also just search products below for a walk-in return without a receipt.
        </div>
      </div>

      <div className="panel">
        <h2>Customer</h2>
        <div className="grid grid-3">
          <div style={{ position: "relative" }}>
            <label>Search / Select Customer</label>
            <input
              value={customerQuery}
              onChange={(e) => {
                setCustomerQuery(e.target.value);
                if (customerId) setCustomerId(null);
              }}
              placeholder="Name or phone — leave blank for walk-in"
            />
            {filteredCustomers.length > 0 && (
              <div className="search-dropdown">
                {filteredCustomers.map((c) => (
                  <div key={c.cs_customerid} className="search-dropdown-item" onClick={() => pickCustomer(c)}>
                    <strong>{c.cs_customername}</strong>
                    {c.cs_customerphone && <span className="muted"> · {c.cs_customerphone}</span>}
                  </div>
                ))}
              </div>
            )}
          </div>
          <div>
            <label>Customer Name</label>
            <input value={customerName} onChange={(e) => setCustomerName(e.target.value)} placeholder="Walk-in Customer" />
          </div>
          <div>
            <label>Mobile</label>
            <input value={customerMobile} onChange={(e) => setCustomerMobile(e.target.value)} />
          </div>
          <div>
            <label>GSTIN</label>
            <input value={customerGstin} onChange={(e) => setCustomerGstin(e.target.value)} />
          </div>
          <div>
            <label>GST Type</label>
            <select value={gstType} onChange={(e) => setGstType(e.target.value)}>
              <option value="intra">Intrastate (CGST+SGST)</option>
              <option value="inter">Interstate (IGST)</option>
            </select>
          </div>
        </div>
        <div style={{ marginTop: 8 }}>
          {customerId ? (
            <>
              <button className="icon-btn" onClick={clearCustomer}>
                ✕ Clear selected customer (switch to walk-in)
              </button>
              {customerBalance !== null && (
                <span className="muted-note" style={{ marginLeft: 10 }}>
                  Current balance: ₹{Number(customerBalance).toFixed(2)}
                </span>
              )}
            </>
          ) : null}
        </div>
      </div>

      <div className="panel">
        <h2>Add Items</h2>
        <div style={{ position: "relative" }}>
          <label>Search Products (code or name)</label>
          <input value={productQuery} onChange={(e) => setProductQuery(e.target.value)} placeholder="Type to search…" />
          {productResults.length > 0 && (
            <div className="search-dropdown">
              {productResults.map((p) => (
                <div key={p.pr_productid} className="search-dropdown-item">
                  <div>
                    <strong>{p.pr_productname}</strong>
                    {p.pr_size && <span className="muted"> ({p.pr_size})</span>}
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
          <div className="muted-note">No items yet — search above or pull from an original bill.</div>
        ) : (
          <table className="items-table">
            <thead>
              <tr>
                <th>Item</th>
                <th style={{ width: 70 }}>Qty</th>
                <th style={{ width: 90 }}>Price</th>
                <th style={{ width: 90 }}>GST %</th>
                <th style={{ width: 90 }}>Restock?</th>
                <th style={{ width: 90 }}>Amount</th>
                <th style={{ width: 30 }}></th>
              </tr>
            </thead>
            <tbody>
              {cart.map((l) => {
                const { lineTotal } = calcLine(l);
                return (
                  <tr key={l.key}>
                    <td>
                      {l.name}
                      {l.size && <div className="muted-note">{l.size}</div>}
                    </td>
                    <td>
                      <input
                        type="number"
                        min="0"
                        value={l.qty}
                        onChange={(e) => updateCartLine(l.key, { qty: e.target.value })}
                      />
                    </td>
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
                    <td className="text-center">
                      <input
                        type="checkbox"
                        checked={l.restock}
                        onChange={(e) => updateCartLine(l.key, { restock: e.target.checked })}
                        title="Resaleable — put back in stock"
                      />
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
        {cart.some((l) => !l.restock) && (
          <div className="muted-note">
            Items with Restock unchecked are written off as damaged/unsellable and will NOT be added back to stock.
          </div>
        )}

        <div className="grid grid-3" style={{ marginTop: 14 }}>
          <div>
            <label>Overall Discount (₹)</label>
            <input type="number" step="0.01" value={overallDiscount} onChange={(e) => setOverallDiscount(e.target.value)} />
          </div>
          <div>
            <label>Refund Amount (₹)</label>
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
              <option value="Credit">Credit (store credit only)</option>
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
          {totals.creditAmount > 0.001 && (
            <div className="totals-row">
              <span>Store Credit (to customer account)</span>
              <span>₹{totals.creditAmount.toFixed(2)}</span>
            </div>
          )}
        </div>
        {totals.creditAmount > 0.001 && !customerId && (
          <div className="muted-note">
            This return isn't fully refunded in cash/bank — select a customer from the list above so the credit can be tracked on their account.
          </div>
        )}
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
          <h2>Recent Sales Returns</h2>
          <table className="items-table">
            <thead>
              <tr>
                <th>Return #</th>
                <th>Customer</th>
                <th>Original Bill #</th>
                <th>Date</th>
                <th className="text-end">Grand Total</th>
              </tr>
            </thead>
            <tbody>
              {init.recent.map((r) => (
                <tr key={r.sre_billid}>
                  <td>{r.sre_billnumber}</td>
                  <td>{r.sre_customername}</td>
                  <td>{r.sre_rebill || "—"}</td>
                  <td>{r.sre_billdate}</td>
                  <td className="text-end">₹{r.sre_gtotal}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
