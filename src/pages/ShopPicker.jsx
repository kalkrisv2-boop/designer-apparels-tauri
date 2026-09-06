import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

const SHOP_TYPES = [
  ["designer_apparels", "Designer Apparels (Retail)"],
  ["staff_hr", "Staff / HR"],
  ["student_management", "Student Management"],
  ["garments", "Garments"],
  ["service", "Service"],
];

// Ported from templates/select_shop.html / routes/shops.py. Only ever
// reached in multi-shop mode -- login() returns next: "dashboard"
// directly in single-shop mode, so this screen is skipped entirely then.
export default function ShopPicker({ onShopChosen }) {
  const [shops, setShops] = useState([]);
  const [error, setError] = useState(null);
  const [showAdd, setShowAdd] = useState(false);
  const [newName, setNewName] = useState("");
  const [newType, setNewType] = useState("designer_apparels");

  async function refresh() {
    try {
      setShops(await invoke("list_shops"));
    } catch (err) {
      setError(typeof err === "string" ? err : "Could not load shops.");
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  async function choose(shopId) {
    try {
      await invoke("choose_shop", { shopId });
      onShopChosen();
    } catch (err) {
      setError(typeof err === "string" ? err : "Could not switch shop.");
    }
  }

  async function addShop(e) {
    e.preventDefault();
    try {
      await invoke("add_shop", { shopName: newName, shopType: newType });
      onShopChosen();
    } catch (err) {
      setError(typeof err === "string" ? err : "Could not add shop.");
    }
  }

  return (
    <div className="auth-screen">
      <div className="auth-card wide">
        <h2>Select a Shop</h2>
        {error && <div className="form-error">{error}</div>}

        <ul className="shop-list">
          {shops.map((s) => (
            <li key={s.shop_id}>
              <button className="shop-list-item" onClick={() => choose(s.shop_id)}>
                <span>{s.shop_name}</span>
                <span className="muted">
                  {SHOP_TYPES.find(([k]) => k === s.shop_type)?.[1] ?? s.shop_type}
                </span>
              </button>
            </li>
          ))}
          {shops.length === 0 && <li className="muted">No shops yet.</li>}
        </ul>

        {!showAdd ? (
          <button onClick={() => setShowAdd(true)}>+ Add / Manage Shops</button>
        ) : (
          <form onSubmit={addShop} className="add-shop-form">
            <label>Shop name</label>
            <input value={newName} onChange={(e) => setNewName(e.target.value)} autoFocus />
            <label>Vertical</label>
            <select value={newType} onChange={(e) => setNewType(e.target.value)}>
              {SHOP_TYPES.map(([k, label]) => (
                <option key={k} value={k}>
                  {label}
                </option>
              ))}
            </select>
            <button type="submit">Create Shop</button>
          </form>
        )}
      </div>
    </div>
  );
}
