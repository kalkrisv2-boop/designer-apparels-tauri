import React, { useEffect, useMemo, useState } from "react";
import {
  billingInit,
  checkout,
  searchProducts,
  styleVariants,
  quickAddProduct,
  addCustomer,
  INDIAN_STATES,
} from "../api.js";

// POS checkout screen, rewritten against the Phase 1 platform schema
// (billing::checkout -> vm_billentry/vm_billitems), replacing the old
// single-tenant invoice-form version of this file. See
// PHASE_1_HANDOFF.md §2 for the three locked decisions this UI has to
// respect:
//   #1 GST type is auto-derived from buyer vs shop state, but the
//      cashier can override per bill (gstTypeOverride below).
//   #2 Per-line GST mode (exclusive/inclusive) is a real per-line
//      choice here, carried over from the old single-tenant app --
//      Python's billing.py has no such concept.
//   #7 Barcode printing (print-barcode-btn, bulk label sheet, reprint-
//      barcode-on-bill) is Phase 2 scope -- omitted entirely, not
//      stubbed, matching how Products.jsx already left it out.
//
// Client-side totals below are a *preview* only (same formulas as
// billing::checkout, round2 not integer round-off -- checkout has no
// round-off step, unlike the old create_invoice pipeline). The server
// is the source of truth; this just avoids a round-trip per keystroke.

const GST_RATES = [0, 5, 12, 18, 28];
const DEFAULT_GST_PCT = 5;

function round2(v) {
  return Math.round((Number(v) || 0) * 100) / 100;
}

function calcLine(l) {
  const preNet = (Number(l.qty) || 0) * (Number(l.price) || 0);
  const discountAmt = preNet * ((Number(l.discount_pct) || 0) / 100);
  let taxable = preNet - discountAmt;
  let gstAmt;
  if (l.gst_mode === "inclusive") {
    const t = taxable / (1 + (Number(l.gst_pct) || 0) / 100);
    gstAmt = taxable - t;
    taxable = t;
  } else {
    gstAmt = taxable * ((Number(l.gst_pct) || 0) / 100);
  }
  return { taxable, gstAmt, lineTotal: taxable + gstAmt };
}

let cartKeySeq = 1;

export default function Billing({ onCheckoutSuccess }) {
  const [init, setInit] = useState(null);
  const [error, setError] = useState("");
  const [success, setSuccess] = useState(null);

  const [billMode, setBillMode] = useState("retail");

  // Customer / buyer
  const [customerId, setCustomerId] = useState(null);
  const [customerQuery, setCustomerQuery] = useState("");
  const [customerName, setCustomerName] = useState("");
  const [customerMobile, setCustomerMobile] = useState("");
  const [customerGstin, setCustomerGstin] = useState("");
  const [customerPan, setCustomerPan] = useState("");
  const [buyerStateCode, setBuyerStateCode] = useState("");
  const [gstTypeOverride, setGstTypeOverride] = useState("auto"); // "auto" | "intra" | "inter"

  // Bill meta
  const [payMethod, setPayMethod] = useState("Cash");
  const [paidAmount, setPaidAmount] = useState("");
  const [overallDiscount, setOverallDiscount] = useState(0);
  const [note, setNote] = useState("");
  const [transportName, setTransportName] = useState("");
  const [lrNumber, setLrNumber] = useState("");
  const [lrDate, setLrDate] = useState("");
  const [parcels, setParcels] = useState("");
  const [salesman, setSalesman] = useState("");
  const [booking, setBooking] = useState("");

  // Cart
  const [cart, setCart] = useState([]);

  // Product search (retail + a-la-carte in wholesale mode)
  const [productQuery, setProductQuery] = useState("");
  const [productResults, setProductResults] = useState([]);

  // Style-variant "add whole size-ratio" flow (wholesale only)
  const [styleQuery, setStyleQuery] = useState("");
  const [styleMatches, setStyleMatches] = useState([]);
  const [selectedModel, setSelectedModel] = useState("");
  const [variants, setVariants] = useState([]);
  const [variantQtys, setVariantQtys] = useState({});

  // Quick-add product (retail only -- see decision note in module doc)
  const [quickAddOpen, setQuickAddOpen] = useState(false);
  const [quickAddForm, setQuickAddForm] = useState({ productname: "", hsn: "", unit: "pcs", saleprice: "" });

  // Inline new-customer
  const [newCustomerOpen, setNewCustomerOpen] = useState(false);
  const [newCustomerForm, setNewCustomerForm] = useState({ customername: "", phone: "", address: "", email: "", tin: "" });

  const [saving, setSaving] = useState(false);

  const refreshInit = () => billingInit().then(setInit).catch((e) => setError(String(e)));

  useEffect(() => {
    refreshInit();
  }, []);

  // --- Product search (debounced) ---
  useEffect(() => {
    if (!productQuery.trim()) {
      setProductResults([]);
      return;
    }
    const t = setTimeout(() => {
      searchProducts(productQuery).then(setProductResults).catch(() => setProductResults([]));
    }, 250);
    return () => clearTimeout(t);
  }, [productQuery]);

  // --- Style search (debounced, wholesale only) ---
  useEffect(() => {
    if (billMode !== "wholesale" || !styleQuery.trim()) {
      setStyleMatches([]);
      return;
    }
    const t = setTimeout(() => {
      searchProducts(styleQuery).then((results) => {
        const seen = new Set();
        const models = [];
        for (const p of results) {
          if (p.pr_model && !seen.has(p.pr_model)) {
            seen.add(p.pr_model);
            models.push(p.pr_model);
          }
        }
        setStyleMatches(models);
      }).catch(() => setStyleMatches([]));
    }, 250);
    return () => clearTimeout(t);
  }, [styleQuery, billMode]);

  const pickModel = (model) => {
    setSelectedModel(model);
    setVariantQtys({});
    styleVariants(model).then(setVariants).catch(() => setVariants([]));
  };

  const priceFor = (product) => {
    if (billMode === "wholesale") {
      const w = parseFloat(product.pr_wholesale);
      return isNaN(w) ? 0 : w;
    }
    return product.pr_saleprice;
  };

  const addProductToCart = (product, qty = 1) => {
    setCart((prev) => [
      ...prev,
      {
        key: cartKeySeq++,
        product_id: product.pr_productid,
        name: product.pr_productname,
        model: product.pr_model,
        cupsize: product.pr_cupsize,
        size: product.pr_size,
        unit: product.pr_unit,
        stock: product.pr_stock,
        price: priceFor(product),
        qty,
        discount_pct: 0,
        gst_pct: DEFAULT_GST_PCT,
        gst_mode: "exclusive",
      },
    ]);
  };

  const addSelectedVariants = () => {
    const toAdd = variants.filter((v) => Number(variantQtys[v.pr_productid]) > 0);
    if (toAdd.length === 0) return;
    setCart((prev) => [
      ...prev,
      ...toAdd.map((v) => ({
        key: cartKeySeq++,
        product_id: v.pr_productid,
        name: v.pr_productname,
        model: v.pr_model,
        cupsize: v.pr_cupsize,
        size: v.pr_size,
        unit: v.pr_unit,
        stock: v.pr_stock,
        price: priceFor(v),
        qty: Number(variantQtys[v.pr_productid]),
        discount_pct: 0,
        gst_pct: DEFAULT_GST_PCT,
        gst_mode: "exclusive",
      })),
    ]);
    setVariantQtys({});
  };

  const updateCartLine = (key, patch) => {
    setCart((prev) => prev.map((l) => (l.key === key ? { ...l, ...patch } : l)));
  };

  const removeCartLine = (key) => setCart((prev) => prev.filter((l) => l.key !== key));

  const switchBillMode = (mode) => {
    if (mode === billMode) return;
    if (cart.length > 0) {
      const ok = window.confirm(
        "Switching between Retail and Wholesale clears the cart (prices differ between the two modes). Continue?"
      );
      if (!ok) return;
    }
    setBillMode(mode);
    setCart([]);
    setSelectedModel("");
    setVariants([]);
    setVariantQtys({});
  };

  const pickCustomer = (c) => {
    setCustomerId(c.cs_customerid);
    setCustomerQuery(c.cs_customername);
    setCustomerName(c.cs_customername);
    setCustomerMobile(c.cs_customerphone);
    setCustomerGstin(c.cs_tin_number);
    setBuyerStateCode(c.cs_statecode || "");
  };

  const clearCustomer = () => {
    setCustomerId(null);
    setCustomerQuery("");
    setCustomerName("");
    setCustomerMobile("");
    setCustomerGstin("");
    setCustomerPan("");
    setBuyerStateCode("");
  };

  const filteredCustomers = useMemo(() => {
    if (!init || !customerQuery.trim() || customerId) return [];
    const q = customerQuery.trim().toLowerCase();
    return init.customers
      .filter((c) => c.cs_customername.toLowerCase().includes(q) || c.cs_customerphone.includes(q))
      .slice(0, 8);
  }, [init, customerQuery, customerId]);

  const effectiveGstType = useMemo(() => {
    if (gstTypeOverride !== "auto") return gstTypeOverride;
    if (!init) return "intra";
    const buyer = buyerStateCode.trim();
    if (!buyer) return "intra";
    return buyer === init.shop_state_code.trim() ? "intra" : "inter";
  }, [gstTypeOverride, buyerStateCode, init]);

  const isInterstate = effectiveGstType === "inter";

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

  const submitQuickAdd = async () => {
    setError("");
    try {
      const product = await quickAddProduct({
        productname: quickAddForm.productname,
        hsn: quickAddForm.hsn,
        unit: quickAddForm.unit,
        saleprice: Number(quickAddForm.saleprice) || 0,
      });
      addProductToCart(product, 1);
      setQuickAddForm({ productname: "", hsn: "", unit: "pcs", saleprice: "" });
      setQuickAddOpen(false);
    } catch (e) {
      setError(String(e));
    }
  };

  const submitNewCustomer = async () => {
    setError("");
    try {
      const id = await addCustomer({ ...newCustomerForm, balance: 0 });
      pickCustomer({
        cs_customerid: id,
        cs_customername: newCustomerForm.customername,
        cs_customerphone: newCustomerForm.phone,
        cs_tin_number: newCustomerForm.tin,
        cs_statecode: "",
      });
      setNewCustomerForm({ customername: "", phone: "", address: "", email: "", tin: "" });
      setNewCustomerOpen(false);
      refreshInit();
    } catch (e) {
      setError(String(e));
    }
  };

  const resetAfterSale = () => {
    setCart([]);
    clearCustomer();
    setCustomerPan("");
    setGstTypeOverride("auto");
    setPaidAmount("");
    setOverallDiscount(0);
    setNote("");
    setTransportName("");
    setLrNumber("");
    setLrDate("");
    setParcels("");
    setSalesman("");
    setBooking("");
  };

  const submit = async () => {
    setError("");
    setSuccess(null);
    if (cart.length === 0) {
      setError("Cart is empty.");
      return;
    }
    for (const l of cart) {
      if (!Number(l.qty) || Number(l.qty) <= 0) {
        setError(`Enter a valid quantity for '${l.name}'.`);
        return;
      }
    }
    const paid = paidAmount === "" ? totals.grandTotal : Number(paidAmount) || 0;
    setSaving(true);
    try {
      const result = await checkout({
        items: cart.map((l) => ({
          product_id: l.product_id,
          qty: Number(l.qty) || 0,
          discount_pct: Number(l.discount_pct) || 0,
          gst_pct: Number(l.gst_pct),
          gst_mode: l.gst_mode,
        })),
        customer_state_code: buyerStateCode.trim(),
        gst_type_override: gstTypeOverride === "auto" ? null : gstTypeOverride,
        customer_name: customerName,
        customer_mobile: customerMobile,
        customer_id: customerId,
        pay_method: payMethod,
        overall_discount: Number(overallDiscount) || 0,
        paid_amount: paid,
        note,
        bill_mode: billMode,
        transport_name: transportName,
        lr_number: lrNumber,
        lr_date: lrDate,
        parcels,
        salesman,
        booking,
        customer_pan: customerPan,
        customer_gstin: customerGstin,
      });
      setSuccess(result);
      resetAfterSale();
      refreshInit();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  if (!init) {
    return <div>Loading…</div>;
  }

  return (
    <div>
      <h1>Billing</h1>
      {error && <div className="error-banner">{error}</div>}
      {success && (
        <div className="success-banner">
          Bill #{success.bill_number} saved — Grand Total ₹{success.grand_total.toFixed(2)}
          {success.balance > 0.001 ? ` (balance ₹${success.balance.toFixed(2)} on account)` : ""}.{" "}
          {onCheckoutSuccess && (
            <button className="secondary" onClick={() => onCheckoutSuccess(success.bill_number)} style={{ marginLeft: 8 }}>
              Print This Invoice
            </button>
          )}
        </div>
      )}

      <div className="panel">
        <div className="toolbar">
          <button
            className={billMode === "retail" ? "primary" : "secondary"}
            onClick={() => switchBillMode("retail")}
          >
            Retail
          </button>
          <button
            className={billMode === "wholesale" ? "primary" : "secondary"}
            onClick={() => switchBillMode("wholesale")}
          >
            Wholesale
          </button>
          <div className="muted-note" style={{ marginLeft: "auto" }}>
            Next bill no. #{init.next_bill_number}
          </div>
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
            <label>PAN</label>
            <input value={customerPan} onChange={(e) => setCustomerPan(e.target.value)} />
          </div>
          <div>
            <label>Buyer State</label>
            <select value={buyerStateCode} onChange={(e) => setBuyerStateCode(e.target.value)}>
              <option value="">— not set —</option>
              {INDIAN_STATES.map((s) => (
                <option key={s.code} value={s.code}>
                  {s.name} ({s.code})
                </option>
              ))}
            </select>
          </div>
          <div>
            <label>GST Type</label>
            <select value={gstTypeOverride} onChange={(e) => setGstTypeOverride(e.target.value)}>
              <option value="auto">Auto ({isInterstate ? "Interstate" : "Intrastate"})</option>
              <option value="intra">Force Intrastate (CGST+SGST)</option>
              <option value="inter">Force Interstate (IGST)</option>
            </select>
          </div>
        </div>
        <div style={{ marginTop: 8 }}>
          {customerId ? (
            <button className="icon-btn" onClick={clearCustomer}>
              ✕ Clear selected customer (switch to walk-in)
            </button>
          ) : (
            <button className="icon-btn" onClick={() => setNewCustomerOpen((o) => !o)}>
              + New Customer
            </button>
          )}
        </div>
        {newCustomerOpen && !customerId && (
          <div className="grid grid-3" style={{ marginTop: 10, borderTop: "1px solid var(--border)", paddingTop: 10 }}>
            <div>
              <label>Name *</label>
              <input
                value={newCustomerForm.customername}
                onChange={(e) => setNewCustomerForm({ ...newCustomerForm, customername: e.target.value })}
              />
            </div>
            <div>
              <label>Phone</label>
              <input
                value={newCustomerForm.phone}
                onChange={(e) => setNewCustomerForm({ ...newCustomerForm, phone: e.target.value })}
              />
            </div>
            <div>
              <label>TIN/GSTIN</label>
              <input
                value={newCustomerForm.tin}
                onChange={(e) => setNewCustomerForm({ ...newCustomerForm, tin: e.target.value })}
              />
            </div>
            <div style={{ gridColumn: "span 2" }}>
              <label>Address</label>
              <input
                value={newCustomerForm.address}
                onChange={(e) => setNewCustomerForm({ ...newCustomerForm, address: e.target.value })}
              />
            </div>
            <div>
              <label>Email</label>
              <input
                value={newCustomerForm.email}
                onChange={(e) => setNewCustomerForm({ ...newCustomerForm, email: e.target.value })}
              />
            </div>
            <div style={{ gridColumn: "span 3" }}>
              <button className="primary" onClick={submitNewCustomer}>
                Save Customer
              </button>
              <div className="muted-note">
                Note: new customers have no state code on file yet — set Buyer State above manually for this bill.
              </div>
            </div>
          </div>
        )}
      </div>

      <div className="panel">
        <h2>Add Items</h2>
        <div style={{ position: "relative" }}>
          <label>Search Products (code, name, or model)</label>
          <input value={productQuery} onChange={(e) => setProductQuery(e.target.value)} placeholder="Type to search…" />
          {productResults.length > 0 && (
            <div className="search-dropdown">
              {productResults.map((p) => (
                <div key={p.pr_productid} className="search-dropdown-item">
                  <div>
                    <strong>{p.pr_productname}</strong>
                    {(p.pr_model || p.pr_cupsize || p.pr_size) && (
                      <span className="muted"> ({[p.pr_model, p.pr_cupsize, p.pr_size].filter(Boolean).join("/")})</span>
                    )}
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

        {billMode === "retail" && (
          <div style={{ marginTop: 8 }}>
            <button className="icon-btn" onClick={() => setQuickAddOpen((o) => !o)}>
              + Quick-Add New Product
            </button>
          </div>
        )}
        {quickAddOpen && billMode === "retail" && (
          <div className="grid grid-4" style={{ marginTop: 10, borderTop: "1px solid var(--border)", paddingTop: 10 }}>
            <div>
              <label>Name *</label>
              <input
                value={quickAddForm.productname}
                onChange={(e) => setQuickAddForm({ ...quickAddForm, productname: e.target.value })}
              />
            </div>
            <div>
              <label>HSN</label>
              <input value={quickAddForm.hsn} onChange={(e) => setQuickAddForm({ ...quickAddForm, hsn: e.target.value })} />
            </div>
            <div>
              <label>Unit</label>
              <input value={quickAddForm.unit} onChange={(e) => setQuickAddForm({ ...quickAddForm, unit: e.target.value })} />
            </div>
            <div>
              <label>Sale Price *</label>
              <input
                type="number"
                step="0.01"
                value={quickAddForm.saleprice}
                onChange={(e) => setQuickAddForm({ ...quickAddForm, saleprice: e.target.value })}
              />
            </div>
            <div style={{ gridColumn: "span 4" }}>
              <button className="primary" onClick={submitQuickAdd}>
                Add to Cart
              </button>
              <div className="muted-note">
                Stock is seeded at 1 and purchase price at 0 — edit properly on the Products screen later if this becomes a
                recurring item.
              </div>
            </div>
          </div>
        )}

        {billMode === "wholesale" && (
          <div style={{ marginTop: 16, borderTop: "1px solid var(--border)", paddingTop: 12 }}>
            <h2 style={{ marginTop: 0 }}>Add Whole Style (all sizes)</h2>
            <div style={{ position: "relative", maxWidth: 340 }}>
              <label>Search by Model</label>
              <input value={styleQuery} onChange={(e) => setStyleQuery(e.target.value)} placeholder="e.g. Sajna" />
              {styleMatches.length > 0 && (
                <div className="search-dropdown">
                  {styleMatches.map((m) => (
                    <div key={m} className="search-dropdown-item" onClick={() => pickModel(m)}>
                      {m}
                    </div>
                  ))}
                </div>
              )}
            </div>
            {selectedModel && variants.length > 0 && (
              <div style={{ marginTop: 10 }}>
                <table>
                  <thead>
                    <tr>
                      <th>Cupsize</th>
                      <th>Size</th>
                      <th className="text-end">Stock</th>
                      <th className="text-end">Wholesale Price</th>
                      <th className="text-end">Qty</th>
                    </tr>
                  </thead>
                  <tbody>
                    {variants.map((v) => (
                      <tr key={v.pr_productid}>
                        <td>{v.pr_cupsize || "—"}</td>
                        <td>{v.pr_size}</td>
                        <td className="text-end">{v.pr_stock}</td>
                        <td className="text-end">{v.pr_wholesale || "—"}</td>
                        <td className="text-end" style={{ width: 90 }}>
                          <input
                            type="number"
                            min="0"
                            value={variantQtys[v.pr_productid] || ""}
                            onChange={(e) => setVariantQtys({ ...variantQtys, [v.pr_productid]: e.target.value })}
                          />
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                <button className="secondary" style={{ marginTop: 8 }} onClick={addSelectedVariants}>
                  Add Selected Sizes to Cart
                </button>
              </div>
            )}
            {selectedModel && variants.length === 0 && (
              <div className="muted-note">No in-stock variants found for '{selectedModel}'.</div>
            )}
          </div>
        )}
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
                <th style={{ width: 90 }}>Rate</th>
                <th style={{ width: 80 }}>Disc %</th>
                <th style={{ width: 90 }}>GST %</th>
                <th style={{ width: 100 }}>GST Mode</th>
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
                      {(l.model || l.cupsize || l.size) && (
                        <div className="muted-note">{[l.model, l.cupsize, l.size].filter(Boolean).join("/")}</div>
                      )}
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
                      <input
                        type="number"
                        step="0.01"
                        value={l.discount_pct}
                        onChange={(e) => updateCartLine(l.key, { discount_pct: e.target.value })}
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
                    <td>
                      <select value={l.gst_mode} onChange={(e) => updateCartLine(l.key, { gst_mode: e.target.value })}>
                        <option value="exclusive">Exclusive</option>
                        <option value="inclusive">Inclusive</option>
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
            <input
              type="number"
              step="0.01"
              value={overallDiscount}
              onChange={(e) => setOverallDiscount(e.target.value)}
            />
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
            <span>{isInterstate ? "IGST" : "CGST + SGST"}</span>
            <span>₹{totals.totalGst.toFixed(2)}</span>
          </div>
          <div className="totals-row grand">
            <span>Grand Total</span>
            <span>₹{totals.grandTotal.toFixed(2)}</span>
          </div>
          {totals.balance > 0.001 && (
            <div className="totals-row">
              <span>Balance (on account)</span>
              <span>₹{totals.balance.toFixed(2)}</span>
            </div>
          )}
        </div>
        {totals.balance > 0.001 && !customerId && (
          <div className="muted-note">
            This bill won't be fully paid — select or add a customer above so the balance can be tracked.
          </div>
        )}
      </div>

      <div className="panel">
        <h2>Transport &amp; Notes (optional)</h2>
        <div className="grid grid-4">
          <div>
            <label>Transport Name</label>
            <input value={transportName} onChange={(e) => setTransportName(e.target.value)} />
          </div>
          <div>
            <label>LR Number</label>
            <input value={lrNumber} onChange={(e) => setLrNumber(e.target.value)} />
          </div>
          <div>
            <label>LR Date</label>
            <input value={lrDate} onChange={(e) => setLrDate(e.target.value)} placeholder="DD/MM/YYYY" />
          </div>
          <div>
            <label>Parcels</label>
            <input value={parcels} onChange={(e) => setParcels(e.target.value)} />
          </div>
          <div>
            <label>Salesman</label>
            <input value={salesman} onChange={(e) => setSalesman(e.target.value)} />
          </div>
          <div>
            <label>Booking</label>
            <input value={booking} onChange={(e) => setBooking(e.target.value)} />
          </div>
          <div style={{ gridColumn: "span 2" }}>
            <label>Note</label>
            <input value={note} onChange={(e) => setNote(e.target.value)} />
          </div>
        </div>
      </div>

      <div className="toolbar">
        <button className="primary" onClick={submit} disabled={saving || cart.length === 0}>
          {saving ? "Saving…" : "Checkout & Save Bill"}
        </button>
      </div>
    </div>
  );
}
