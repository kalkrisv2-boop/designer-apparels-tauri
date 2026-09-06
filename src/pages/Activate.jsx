import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

// Ported from templates/activate.html. Shown before anything else if
// licensing_commands::check_license reports activated: false -- the
// frontend enforces the "don't even let an unlicensed install look
// around" rule that Python's before_request gate used to, since Tauri
// commands aren't intercepted by shared middleware (see
// licensing_commands.rs's module doc).
export default function Activate({ hardwareId, onActivated }) {
  const [key, setKey] = useState("");
  const [error, setError] = useState(null);
  const [busy, setBusy] = useState(false);

  async function submit(e) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await invoke("activate_license", { key });
      onActivated();
    } catch (err) {
      setError(typeof err === "string" ? err : "Activation failed.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="auth-screen">
      <form className="auth-card" onSubmit={submit}>
        <h2>Activate Designer Apparels</h2>
        <p className="muted">
          This machine's Hardware ID is shown below. Send it to your
          vendor to receive an activation key.
        </p>
        <div className="hardware-id">{hardwareId}</div>

        <label>Activation Key</label>
        <input
          type="text"
          value={key}
          onChange={(e) => setKey(e.target.value)}
          placeholder="Enter activation key"
          autoFocus
        />

        {error && <div className="form-error">{error}</div>}

        <button type="submit" disabled={busy}>
          {busy ? "Activating..." : "Activate"}
        </button>
      </form>
    </div>
  );
}
