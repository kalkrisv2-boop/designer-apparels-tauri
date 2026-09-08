import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

// Ported from templates/sidebar.html. Nav items are gated by
// session.shop_type exactly like the Jinja {% if current_shop_type == ... %}
// branches in the Python template. Only "designer_apparels" has real
// pages wired up in this app so far (Billing/History/Items, carried over
// from the standalone invoicing tool -- see roadmap Phase 1 for
// reconciling them with the ported vm_billentry schema). The other four
// verticals show a "coming soon" placeholder until their own Phase
// (3/4/5/6) lands, same as Python's routes/placeholder.py did before
// each vertical was built out.
export default function Sidebar({ session, multiShopEnabled, tab, onTab, onLogout, onSwitchShop }) {
  const [shops, setShops] = useState([]);

  useEffect(() => {
    if (multiShopEnabled) {
      invoke("list_shops").then(setShops).catch(() => setShops([]));
    }
  }, [multiShopEnabled, session.shop_name]);

  const designerApparelsNav = [
    { id: "dashboard", label: "Dashboard" },
    { id: "billing", label: "Billing" },
    { id: "history", label: "Print Invoice / History" },
    { id: "items", label: "Item Catalog (legacy)" },
    { id: "products", label: "Products & Inventory" },
    { id: "customers", label: "Customers" },
    { id: "suppliers", label: "Suppliers" },
    { id: "settings", label: "Settings" },
  ];

  const otherVerticalNav = [{ id: "dashboard", label: "Dashboard" }];

  const nav = session.shop_type === "designer_apparels" ? designerApparelsNav : otherVerticalNav;

  return (
    <nav className="sidebar">
      {multiShopEnabled ? (
        <div className="shop-switcher">
          <div className="shop-switcher-current">{session.shop_name}</div>
          <select
            value={session.active_shop_id ?? ""}
            onChange={(e) =>
              invoke("choose_shop", { shopId: Number(e.target.value) }).then(() =>
                onSwitchShop("chose")
              )
            }
          >
            {shops.map((s) => (
              <option key={s.shop_id} value={s.shop_id}>
                {s.shop_name}
              </option>
            ))}
          </select>
          <button
            className="link-btn"
            onClick={() => invoke("switch_shop").then(() => onSwitchShop("manage"))}
          >
            + Add / Manage Shops
          </button>
        </div>
      ) : (
        <h5 className="shop-name-static">{session.shop_name}</h5>
      )}

      <ul className="nav-list">
        {nav.map((item) => (
          <li key={item.id}>
            <button
              className={"nav-btn" + (tab === item.id ? " active" : "")}
              onClick={() => onTab(item.id)}
            >
              {item.label}
            </button>
          </li>
        ))}
      </ul>

      <div className="sidebar-footer">
        <button className="nav-btn" onClick={onLogout}>
          Logout
        </button>
      </div>
    </nav>
  );
}
