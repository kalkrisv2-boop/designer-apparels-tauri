import React, { useEffect, useState } from "react";
import { listInvoices, openFile, regenerateInvoicePdf } from "../api.js";

export default function History() {
  const [invoices, setInvoices] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  const refresh = () => {
    setLoading(true);
    listInvoices()
      .then(setInvoices)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  };

  useEffect(refresh, []);

  const handleOpen = async (path) => {
    try {
      await openFile(path);
    } catch (e) {
      setError("Could not open PDF: " + String(e));
    }
  };

  const handleRegenerate = async (id) => {
    try {
      const path = await regenerateInvoicePdf(id);
      await openFile(path);
      refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div>
      <h1>Invoice History</h1>
      {error && <div className="error-banner">{error}</div>}
      <div className="panel">
        {loading ? (
          <p>Loading…</p>
        ) : invoices.length === 0 ? (
          <p className="muted-note">No invoices yet. Create one from the Billing tab.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Invoice #</th>
                <th>Date</th>
                <th>Buyer</th>
                <th>State</th>
                <th>Tax Type</th>
                <th>Grand Total</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {invoices.map((inv) => (
                <tr key={inv.id}>
                  <td>{inv.invoice_number}</td>
                  <td>{inv.invoice_date}</td>
                  <td>{inv.buyer_name}</td>
                  <td>{inv.buyer_state}</td>
                  <td>{inv.is_interstate ? "IGST" : "CGST+SGST"}</td>
                  <td>₹{inv.grand_total.toFixed(2)}</td>
                  <td style={{ display: "flex", gap: 6 }}>
                    {inv.pdf_path ? (
                      <button className="icon-btn" onClick={() => handleOpen(inv.pdf_path)}>
                        Open PDF
                      </button>
                    ) : (
                      <span className="muted-note">no pdf</span>
                    )}
                    <button className="icon-btn" onClick={() => handleRegenerate(inv.id)}>
                      Regenerate
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
