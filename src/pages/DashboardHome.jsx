import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

// Ported from templates/dashboard.html / routes/dashboard.py. Shows
// summary counts only for designer_apparels (the only vertical with
// real data yet) -- every other shop_type gets a plain welcome message,
// same as Python deliberately avoided showing misleading zero counts
// for verticals that don't have modules built yet.
export default function DashboardHome() {
  const [data, setData] = useState(null);
  const [error, setError] = useState(null);

  useEffect(() => {
    invoke("get_dashboard")
      .then(setData)
      .catch((err) => setError(typeof err === "string" ? err : "Could not load dashboard."));
  }, []);

  if (error) return <div className="form-error">{error}</div>;
  if (!data) return <div className="muted">Loading...</div>;

  return (
    <div className="dashboard-home">
      <h2>{data.shop_name}</h2>
      <p className="muted">{data.shop_type_label}</p>

      {data.counts ? (
        <div className="summary-cards">
          <div className="summary-card">
            <div className="summary-value">{data.counts.product_count}</div>
            <div className="summary-label">Products</div>
          </div>
          <div className="summary-card">
            <div className="summary-value">{data.counts.customer_count}</div>
            <div className="summary-label">Customers</div>
          </div>
          <div className="summary-card">
            <div className="summary-value">{data.counts.supplier_count}</div>
            <div className="summary-label">Suppliers</div>
          </div>
        </div>
      ) : (
        <p>
          Welcome. This vertical's dedicated screens are coming in a later
          phase of the migration -- for now this is just the shared
          dashboard shell.
        </p>
      )}
    </div>
  );
}
