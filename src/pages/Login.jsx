import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

// Ported from templates/login.html / routes/auth.py's login().
export default function Login({ onLoggedIn }) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState(null);
  const [busy, setBusy] = useState(false);

  async function submit(e) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const result = await invoke("login", { username, password });
      onLoggedIn(result.next);
    } catch (err) {
      setError(typeof err === "string" ? err : "Invalid username or password.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="auth-screen">
      <form className="auth-card" onSubmit={submit}>
        <h2>Designer Apparels</h2>
        <p className="muted">Sign in to continue</p>

        <label>Username</label>
        <input
          type="text"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          autoFocus
        />

        <label>Password</label>
        <input
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
        />

        {error && <div className="form-error">{error}</div>}

        <button type="submit" disabled={busy}>
          {busy ? "Signing in..." : "Sign in"}
        </button>
      </form>
    </div>
  );
}
