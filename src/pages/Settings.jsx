import React, { useEffect, useState } from "react";
import { getSettings, saveSettings, pickFolder } from "../api.js";

export default function Settings() {
  const [settings, setSettings] = useState(null);
  const [error, setError] = useState("");
  const [success, setSuccess] = useState("");

  useEffect(() => {
    getSettings().then(setSettings).catch((e) => setError(String(e)));
  }, []);

  const choosePdfFolder = async () => {
    const folder = await pickFolder();
    if (folder) setSettings({ ...settings, pdf_output_folder: folder });
  };

  const submit = async () => {
    setError("");
    setSuccess("");
    try {
      await saveSettings(settings);
      setSuccess("Settings saved.");
    } catch (e) {
      setError(String(e));
    }
  };

  if (!settings) return <p>Loading…</p>;

  return (
    <div>
      <h1>Settings</h1>
      {error && <div className="error-banner">{error}</div>}
      {success && <div className="success-banner">{success}</div>}

      <div className="panel">
        <h2>Company (fixed)</h2>
        <p className="muted-note">
          This build is exclusive to <strong>DESIGNER APPARELS</strong> — the company name,
          address, and GSTIN are hardcoded into the app and cannot be changed here.
        </p>
      </div>

      <div className="panel">
        <h2>Bank Details (printed on invoice)</h2>
        <div className="grid grid-2">
          <div>
            <label>Bank Name</label>
            <input
              value={settings.bank_name}
              onChange={(e) => setSettings({ ...settings, bank_name: e.target.value })}
            />
          </div>
          <div>
            <label>Branch</label>
            <input
              value={settings.bank_branch}
              onChange={(e) => setSettings({ ...settings, bank_branch: e.target.value })}
            />
          </div>
          <div>
            <label>Account Number</label>
            <input
              value={settings.bank_account_number}
              onChange={(e) =>
                setSettings({ ...settings, bank_account_number: e.target.value })
              }
            />
          </div>
          <div>
            <label>IFSC</label>
            <input
              value={settings.bank_ifsc}
              onChange={(e) => setSettings({ ...settings, bank_ifsc: e.target.value })}
            />
          </div>
        </div>
      </div>

      <div className="panel">
        <h2>Invoicing</h2>
        <div className="grid grid-2">
          <div>
            <label>Next Invoice Number</label>
            <input
              type="number"
              value={settings.next_invoice_number}
              onChange={(e) =>
                setSettings({ ...settings, next_invoice_number: Number(e.target.value) })
              }
            />
          </div>
          <div>
            <label>Mobile No. (printed on invoice)</label>
            <input
              value={settings.mobile_no}
              onChange={(e) => setSettings({ ...settings, mobile_no: e.target.value })}
            />
          </div>
        </div>
        <div style={{ marginTop: 10 }}>
          <label>PDF Output Folder</label>
          <div style={{ display: "flex", gap: 8 }}>
            <input value={settings.pdf_output_folder} readOnly />
            <button className="secondary" onClick={choosePdfFolder}>
              Browse…
            </button>
          </div>
        </div>
        <div style={{ marginTop: 10 }}>
          <label>Terms &amp; Conditions (one per line)</label>
          <textarea
            rows={4}
            value={settings.terms_and_conditions}
            onChange={(e) =>
              setSettings({ ...settings, terms_and_conditions: e.target.value })
            }
          />
        </div>
      </div>

      <button className="primary" onClick={submit}>
        Save Settings
      </button>
    </div>
  );
}
