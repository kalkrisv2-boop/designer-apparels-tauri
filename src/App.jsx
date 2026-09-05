import React, { useState } from "react";
import Billing from "./pages/Billing.jsx";
import History from "./pages/History.jsx";
import Items from "./pages/Items.jsx";
import Settings from "./pages/Settings.jsx";
import ErrorBoundary from "./components/ErrorBoundary.jsx";

const TABS = [
  { id: "billing", label: "Billing" },
  { id: "history", label: "History" },
  { id: "items", label: "Item Catalog" },
  { id: "settings", label: "Settings" },
];

export default function App() {
  const [tab, setTab] = useState("billing");

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-title">DESIGNER APPARELS</div>
          <div className="brand-sub">GST Invoicing</div>
        </div>
        <nav>
          {TABS.map((t) => (
            <button
              key={t.id}
              className={"nav-btn" + (tab === t.id ? " active" : "")}
              onClick={() => setTab(t.id)}
            >
              {t.label}
            </button>
          ))}
        </nav>
      </aside>
      <main className="content">
        {/* key={tab} forces the boundary to reset when switching tabs */}
        <ErrorBoundary key={tab}>
          {tab === "billing" && <Billing />}
          {tab === "history" && <History />}
          {tab === "items" && <Items />}
          {tab === "settings" && <Settings />}
        </ErrorBoundary>
      </main>
    </div>
  );
}
