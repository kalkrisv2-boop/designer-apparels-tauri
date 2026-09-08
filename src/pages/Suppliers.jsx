import React, { useEffect, useState } from "react";
import { listSuppliers, addSupplier, updateSupplier, deleteSupplier } from "../api.js";

// Direct port of templates/suppliers.html onto the Phase 1 backend.
// Note rs_balance is TEXT on this schema (same convention Python used),
// so it's handled as a plain string here, not coerced to a number.

const blank = { company_name: "", contact_name: "", phone: "", mobile: "", address: "", email: "", balance: "0", tin: "" };

export default function Suppliers() {
  const [suppliers, setSuppliers] = useState([]);
  const [form, setForm] = useState(blank);
  const [editingId, setEditingId] = useState(null);
  const [error, setError] = useState("");

  const refresh = () => listSuppliers().then(setSuppliers).catch((e) => setError(String(e)));

  useEffect(() => {
    refresh();
  }, []);

  const submit = async () => {
    if (!form.company_name.trim()) {
      setError("Company name is required.");
      return;
    }
    setError("");
    try {
      if (editingId) {
        await updateSupplier(editingId, form);
      } else {
        await addSupplier(form);
      }
      setForm(blank);
      setEditingId(null);
      refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const edit = (s) => {
    setForm({
      company_name: s.rs_company_name,
      contact_name: s.rs_name,
      phone: s.rs_phone,
      mobile: s.rs_mobile,
      address: s.rs_address,
      email: s.rs_email,
      balance: s.rs_balance,
      tin: s.rs_tinnum,
    });
    setEditingId(s.rs_supplierid);
  };

  const remove = async (id) => {
    if (!confirm("Delete this supplier?")) return;
    await deleteSupplier(id);
    refresh();
  };

  return (
    <div>
      <h1>Suppliers ({suppliers.length})</h1>
      {error && <div className="error-banner">{error}</div>}

      <div className="panel">
        <h2>{editingId ? "Edit Supplier" : "Add Supplier"}</h2>
        <div className="grid grid-4">
          <div>
            <label>Company Name</label>
            <input value={form.company_name} onChange={(e) => setForm({ ...form, company_name: e.target.value })} />
          </div>
          <div>
            <label>Contact Person</label>
            <input value={form.contact_name} onChange={(e) => setForm({ ...form, contact_name: e.target.value })} />
          </div>
          <div>
            <label>Phone</label>
            <input value={form.phone} onChange={(e) => setForm({ ...form, phone: e.target.value })} />
          </div>
          <div>
            <label>Mobile</label>
            <input value={form.mobile} onChange={(e) => setForm({ ...form, mobile: e.target.value })} />
          </div>
          <div>
            <label>Email</label>
            <input value={form.email} onChange={(e) => setForm({ ...form, email: e.target.value })} />
          </div>
          <div>
            <label>GSTIN</label>
            <input value={form.tin} onChange={(e) => setForm({ ...form, tin: e.target.value })} />
          </div>
          <div style={{ gridColumn: "span 2" }}>
            <label>Address</label>
            <input value={form.address} onChange={(e) => setForm({ ...form, address: e.target.value })} />
          </div>
          <div>
            <label>Opening Balance (₹)</label>
            <input value={form.balance} onChange={(e) => setForm({ ...form, balance: e.target.value })} />
          </div>
        </div>
        <div className="toolbar" style={{ marginTop: 12 }}>
          <button className="primary" onClick={submit}>
            {editingId ? "Update Supplier" : "Add Supplier"}
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
        <table>
          <thead>
            <tr>
              <th>Company</th>
              <th>Contact Person</th>
              <th>Phone</th>
              <th>Email</th>
              <th className="text-end">Balance</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {suppliers.map((s) => (
              <tr key={s.rs_supplierid}>
                <td>{s.rs_company_name}</td>
                <td>{s.rs_name}</td>
                <td>{s.rs_phone || s.rs_mobile}</td>
                <td>{s.rs_email}</td>
                <td className="text-end">₹{(parseFloat(s.rs_balance) || 0).toFixed(2)}</td>
                <td style={{ display: "flex", gap: 6 }}>
                  <button className="icon-btn" onClick={() => edit(s)}>Edit</button>
                  <button className="icon-btn" onClick={() => remove(s.rs_supplierid)}>Delete</button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
