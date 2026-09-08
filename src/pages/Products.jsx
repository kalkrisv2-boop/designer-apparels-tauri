import React, { useEffect, useState } from "react";
import { listProducts, addProduct, updateProduct, deleteProduct } from "../api.js";

// Direct port of templates/products.html onto the Phase 1 backend --
// same fields, same low-stock/out-of-stock summary cards, same inline
// add/edit form pattern as Items.jsx (this app's existing convention).
// NOT ported: barcode printing (print-barcode-btn, bulk label sheet) --
// that's Phase 2 (Barcode generation) per the roadmap, not built yet.

const CUPSIZES = ["A", "B", "C", "D", "E"];
const COMMON_SIZES = ["32", "34", "36", "38", "40"];

const blank = {
  productcode: "",
  productname: "",
  hsn: "",
  purchaseprice: 0,
  saleprice: 0,
  wholesale: "",
  stock: 0,
  unit: "",
  model: "",
  cupsize: "",
  size: "",
  description: "",
  type: "",
  percentage: "",
  barcode: "",
};

export default function Products() {
  const [summary, setSummary] = useState(null);
  const [form, setForm] = useState(blank);
  const [editingId, setEditingId] = useState(null);
  const [error, setError] = useState("");
  const [lowStockOnly, setLowStockOnly] = useState(false);
  const [search, setSearch] = useState("");

  const refresh = () => listProducts().then(setSummary).catch((e) => setError(String(e)));

  useEffect(() => {
    refresh();
  }, []);

  const submit = async () => {
    if (!form.productcode.trim() || !form.productname.trim()) {
      setError("Product code and name are required.");
      return;
    }
    setError("");
    const input = {
      ...form,
      purchaseprice: Number(form.purchaseprice) || 0,
      saleprice: Number(form.saleprice) || 0,
      stock: Number(form.stock) || 0,
    };
    try {
      if (editingId) {
        await updateProduct(editingId, input);
      } else {
        await addProduct(input);
      }
      setForm(blank);
      setEditingId(null);
      refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const edit = (p) => {
    setForm({
      productcode: p.pr_productcode,
      productname: p.pr_productname,
      hsn: p.pr_hsn,
      purchaseprice: p.pr_purchaseprice,
      saleprice: p.pr_saleprice,
      wholesale: p.pr_wholesale,
      stock: p.pr_stock,
      unit: p.pr_unit,
      model: p.pr_model,
      cupsize: p.pr_cupsize,
      size: p.pr_size,
      description: p.pr_description,
      type: p.pr_type,
      percentage: p.pr_pecentage,
      barcode: p.pr_barcode,
    });
    setEditingId(p.pr_productid);
  };

  const remove = async (id) => {
    if (!confirm("Delete this product?")) return;
    await deleteProduct(id);
    refresh();
  };

  if (!summary) return <div>Loading...</div>;

  const rows = summary.products.filter((p) => {
    if (lowStockOnly && p.stock_status === "ok") return false;
    if (!search.trim()) return true;
    const q = search.trim().toLowerCase();
    return p.pr_productcode.toLowerCase().includes(q) || p.pr_productname.toLowerCase().includes(q) || p.pr_model.toLowerCase().includes(q);
  });

  return (
    <div>
      <h1>Products &amp; Inventory</h1>
      {error && <div className="error-banner">{error}</div>}

      <div className="summary-cards" style={{ marginBottom: 16 }}>
        <div className="summary-card">
          <div className="summary-value">{summary.total_count}</div>
          <div className="summary-label">Total Products</div>
        </div>
        <div className="summary-card">
          <div className="summary-value">{summary.low_stock_count}</div>
          <div className="summary-label">Low Stock (&lt; {summary.low_stock_threshold})</div>
        </div>
        <div className="summary-card">
          <div className="summary-value">{summary.out_of_stock_count}</div>
          <div className="summary-label">Out of Stock</div>
        </div>
        <div className="summary-card">
          <div className="summary-value">₹{summary.total_stock_value.toFixed(2)}</div>
          <div className="summary-label">Total Stock Value</div>
        </div>
      </div>

      <div className="panel">
        <h2>{editingId ? "Edit Product" : "Add Product"}</h2>
        <div className="grid grid-4">
          <div>
            <label>Product Code</label>
            <input value={form.productcode} onChange={(e) => setForm({ ...form, productcode: e.target.value })} />
          </div>
          <div>
            <label>Product Name</label>
            <input value={form.productname} onChange={(e) => setForm({ ...form, productname: e.target.value })} />
          </div>
          <div>
            <label>Barcode</label>
            <input value={form.barcode} onChange={(e) => setForm({ ...form, barcode: e.target.value })} />
          </div>
          <div>
            <label>HSN</label>
            <input value={form.hsn} onChange={(e) => setForm({ ...form, hsn: e.target.value })} />
          </div>
          <div>
            <label>Model</label>
            <input value={form.model} onChange={(e) => setForm({ ...form, model: e.target.value })} placeholder="e.g. Sajna" />
          </div>
          <div>
            <label>Cupsize</label>
            <select value={form.cupsize} onChange={(e) => setForm({ ...form, cupsize: e.target.value })}>
              <option value="">--</option>
              {CUPSIZES.map((c) => (
                <option key={c} value={c}>{c}</option>
              ))}
            </select>
          </div>
          <div>
            <label>Size</label>
            <input
              list="common-sizes"
              value={form.size}
              onChange={(e) => setForm({ ...form, size: e.target.value })}
              placeholder="32, 34... or custom"
            />
            <datalist id="common-sizes">
              {COMMON_SIZES.map((s) => (
                <option key={s} value={s} />
              ))}
            </datalist>
          </div>
          <div style={{ gridColumn: "span 2" }}>
            <label>Description</label>
            <input
              value={form.description}
              onChange={(e) => setForm({ ...form, description: e.target.value })}
              placeholder="e.g. red color cotton"
            />
          </div>
          <div>
            <label>Unit</label>
            <input value={form.unit} onChange={(e) => setForm({ ...form, unit: e.target.value })} placeholder="pcs" />
          </div>
          <div>
            <label>Purchase Price (₹)</label>
            <input type="number" step="0.01" value={form.purchaseprice} onChange={(e) => setForm({ ...form, purchaseprice: e.target.value })} />
          </div>
          <div>
            <label>Sale Price (₹)</label>
            <input type="number" step="0.01" value={form.saleprice} onChange={(e) => setForm({ ...form, saleprice: e.target.value })} />
          </div>
          <div>
            <label>Wholesale Price (₹)</label>
            <input value={form.wholesale} onChange={(e) => setForm({ ...form, wholesale: e.target.value })} />
          </div>
          <div>
            <label>Stock</label>
            <input type="number" step="1" value={form.stock} onChange={(e) => setForm({ ...form, stock: e.target.value })} />
          </div>
        </div>
        <div className="toolbar" style={{ marginTop: 12 }}>
          <button className="primary" onClick={submit}>
            {editingId ? "Update Product" : "Add Product"}
          </button>
          {editingId && (
            <button
              className="secondary"
              onClick={() => {
                setForm(blank);
                setEditingId(null);
              }}
            >
              Cancel Edit
            </button>
          )}
        </div>
      </div>

      <div className="panel">
        <div className="toolbar" style={{ justifyContent: "space-between" }}>
          <h2 style={{ margin: 0 }}>Products ({rows.length})</h2>
          <div style={{ display: "flex", gap: 12, alignItems: "center" }}>
            <input placeholder="Search code or name..." value={search} onChange={(e) => setSearch(e.target.value)} />
            <label>
              <input type="checkbox" checked={lowStockOnly} onChange={(e) => setLowStockOnly(e.target.checked)} /> Low/out of stock only
            </label>
          </div>
        </div>
        <table>
          <thead>
            <tr>
              <th>Code</th>
              <th>Name</th>
              <th>Model</th>
              <th>Cup</th>
              <th>Size</th>
              <th className="text-end">Purchase ₹</th>
              <th className="text-end">Sale ₹</th>
              <th className="text-end">Stock</th>
              <th className="text-end">Value ₹</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {rows.map((p) => (
              <tr key={p.pr_productid} className={p.stock_status === "out" ? "row-danger" : p.stock_status === "low" ? "row-warning" : ""}>
                <td>{p.pr_productcode}</td>
                <td>{p.pr_productname}</td>
                <td>{p.pr_model}</td>
                <td>{p.pr_cupsize}</td>
                <td>{p.pr_size}</td>
                <td className="text-end">{p.pr_purchaseprice.toFixed(2)}</td>
                <td className="text-end">{p.pr_saleprice.toFixed(2)}</td>
                <td className="text-end">
                  {p.pr_stock}
                  {p.stock_status === "out" && <span className="badge badge-danger"> Out</span>}
                  {p.stock_status === "low" && <span className="badge badge-warning"> Low</span>}
                </td>
                <td className="text-end">{p.stock_value.toFixed(2)}</td>
                <td style={{ display: "flex", gap: 6 }}>
                  <button className="icon-btn" onClick={() => edit(p)}>Edit</button>
                  <button className="icon-btn" onClick={() => remove(p.pr_productid)}>Delete</button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
