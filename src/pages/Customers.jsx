import React, { useEffect, useState } from "react";
import { listCustomers, addCustomer, updateCustomer, deleteCustomer } from "../api.js";

// Direct port of templates/customers.html onto the Phase 1 backend.

const blank = { customername: "", phone: "", address: "", email: "", tin: "", balance: 0 };

export default function Customers() {
  const [customers, setCustomers] = useState([]);
  const [form, setForm] = useState(blank);
  const [editingId, setEditingId] = useState(null);
  const [error, setError] = useState("");

  const refresh = () => listCustomers().then(setCustomers).catch((e) => setError(String(e)));

  useEffect(() => {
    refresh();
  }, []);

  const submit = async () => {
    if (!form.customername.trim()) {
      setError("Customer name is required.");
      return;
    }
    setError("");
    const input = { ...form, balance: Number(form.balance) || 0 };
    try {
      if (editingId) {
        await updateCustomer(editingId, input);
      } else {
        await addCustomer(input);
      }
      setForm(blank);
      setEditingId(null);
      refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const edit = (c) => {
    setForm({
      customername: c.cs_customername,
      phone: c.cs_customerphone,
      address: c.cs_address,
      email: c.cs_email,
      tin: c.cs_tin_number,
      balance: c.cs_balance,
    });
    setEditingId(c.cs_customerid);
  };

  const remove = async (id) => {
    if (!confirm("Delete this customer?")) return;
    await deleteCustomer(id);
    refresh();
  };

  return (
    <div>
      <h1>Customers ({customers.length})</h1>
      {error && <div className="error-banner">{error}</div>}

      <div className="panel">
        <h2>{editingId ? "Edit Customer" : "Add Customer"}</h2>
        <div className="grid grid-4">
          <div>
            <label>Name</label>
            <input value={form.customername} onChange={(e) => setForm({ ...form, customername: e.target.value })} />
          </div>
          <div>
            <label>Phone</label>
            <input value={form.phone} onChange={(e) => setForm({ ...form, phone: e.target.value })} />
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
            <input type="number" step="0.01" value={form.balance} onChange={(e) => setForm({ ...form, balance: e.target.value })} />
          </div>
        </div>
        <div className="toolbar" style={{ marginTop: 12 }}>
          <button className="primary" onClick={submit}>
            {editingId ? "Update Customer" : "Add Customer"}
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
              <th>Name</th>
              <th>Phone</th>
              <th>Email</th>
              <th>Address</th>
              <th className="text-end">Balance</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {customers.map((c) => (
              <tr key={c.cs_customerid}>
                <td>{c.cs_customername || "Unnamed Customer"}</td>
                <td>{c.cs_customerphone}</td>
                <td>{c.cs_email}</td>
                <td>{c.cs_address}</td>
                <td className="text-end">₹{(c.cs_balance || 0).toFixed(2)}</td>
                <td style={{ display: "flex", gap: 6 }}>
                  <button className="icon-btn" onClick={() => edit(c)}>Edit</button>
                  <button className="icon-btn" onClick={() => remove(c.cs_customerid)}>Delete</button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
