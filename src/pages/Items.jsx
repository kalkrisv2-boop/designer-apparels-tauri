import React, { useEffect, useState } from "react";
import { listCatalogItems, saveCatalogItem, deleteCatalogItem, GST_RATES } from "../api.js";

const blank = { id: null, description: "", hsn_sac: "6212", default_rate: 0, default_gst_rate: 5 };

export default function Items() {
  const [items, setItems] = useState([]);
  const [form, setForm] = useState(blank);
  const [error, setError] = useState("");

  const refresh = () => listCatalogItems().then(setItems).catch((e) => setError(String(e)));

  useEffect(() => {
    refresh();
  }, []);

  const submit = async () => {
    if (!form.description.trim()) {
      setError("Description is required.");
      return;
    }
    setError("");
    try {
      await saveCatalogItem({
        ...form,
        default_rate: Number(form.default_rate) || 0,
        default_gst_rate: Number(form.default_gst_rate) || 0,
      });
      setForm(blank);
      refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const edit = (item) => setForm(item);
  const remove = async (id) => {
    await deleteCatalogItem(id);
    refresh();
  };

  return (
    <div>
      <h1>Item Catalog</h1>
      {error && <div className="error-banner">{error}</div>}

      <div className="panel">
        <h2>{form.id ? "Edit Item" : "Add New Item"}</h2>
        <div className="grid grid-4">
          <div>
            <label>Description</label>
            <input
              value={form.description}
              onChange={(e) => setForm({ ...form, description: e.target.value })}
            />
          </div>
          <div>
            <label>HSN/SAC</label>
            <input
              value={form.hsn_sac}
              onChange={(e) => setForm({ ...form, hsn_sac: e.target.value })}
            />
          </div>
          <div>
            <label>Default Rate</label>
            <input
              type="number"
              step="0.01"
              value={form.default_rate}
              onChange={(e) => setForm({ ...form, default_rate: e.target.value })}
            />
          </div>
          <div>
            <label>Default GST%</label>
            <select
              value={form.default_gst_rate}
              onChange={(e) => setForm({ ...form, default_gst_rate: e.target.value })}
            >
              {GST_RATES.map((r) => (
                <option key={r} value={r}>
                  {r}%
                </option>
              ))}
            </select>
          </div>
        </div>
        <div className="toolbar" style={{ marginTop: 12 }}>
          <button className="primary" onClick={submit}>
            {form.id ? "Update Item" : "Add Item"}
          </button>
          {form.id && (
            <button className="secondary" onClick={() => setForm(blank)}>
              Cancel Edit
            </button>
          )}
        </div>
      </div>

      <div className="panel">
        <h2>Catalog ({items.length})</h2>
        <table>
          <thead>
            <tr>
              <th>Description</th>
              <th>HSN</th>
              <th>Rate</th>
              <th>GST%</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {items.map((item) => (
              <tr key={item.id}>
                <td>{item.description}</td>
                <td>{item.hsn_sac}</td>
                <td>{item.default_rate.toFixed(2)}</td>
                <td>{item.default_gst_rate}%</td>
                <td style={{ display: "flex", gap: 6 }}>
                  <button className="icon-btn" onClick={() => edit(item)}>
                    Edit
                  </button>
                  <button className="icon-btn" onClick={() => remove(item.id)}>
                    Delete
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
