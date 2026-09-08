import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Billing from "./pages/Billing.jsx";
import History from "./pages/History.jsx";
import InvoicePrint from "./pages/InvoicePrint.jsx";
import Accounts from "./pages/Accounts.jsx";
import Items from "./pages/Items.jsx";
import Products from "./pages/Products.jsx";
import Customers from "./pages/Customers.jsx";
import Suppliers from "./pages/Suppliers.jsx";
import Settings from "./pages/Settings.jsx";
import Activate from "./pages/Activate.jsx";
import Login from "./pages/Login.jsx";
import ShopPicker from "./pages/ShopPicker.jsx";
import DashboardHome from "./pages/DashboardHome.jsx";
import Sidebar from "./components/Sidebar.jsx";
import ErrorBoundary from "./components/ErrorBoundary.jsx";

// Top-level screen state machine -- Rust/Tauri equivalent of the Python
// app's before_request gates (require_activation -> login_required ->
// shop_selected), since there's no shared middleware layer here to
// intercept every "request" the way Flask's before_request did. See
// licensing_commands.rs's module doc for why this lives in the frontend
// instead.
//
// screen: "loading" | "activate" | "login" | "pick-shop" | "app"
export default function App() {
  const [screen, setScreen] = useState("loading");
  const [hardwareId, setHardwareId] = useState("");
  const [multiShopEnabled, setMultiShopEnabled] = useState(false);
  const [session, setSession] = useState(null);
  const [tab, setTab] = useState("dashboard");
  const [printBillNumber, setPrintBillNumber] = useState(null);

  useEffect(() => {
    (async () => {
      const clientConfig = await invoke("get_client_config");
      setMultiShopEnabled(clientConfig.multi_shop_enabled);

      const license = await invoke("check_license");
      if (!license.activated) {
        setHardwareId(license.hardware_id);
        setScreen("activate");
        return;
      }
      setScreen("login");
    })();
  }, []);

  async function refreshSession() {
    const s = await invoke("get_session");
    setSession(s);
    return s;
  }

  async function handleLoggedIn(next) {
    await refreshSession();
    setScreen(next === "pick-shop" ? "pick-shop" : "app");
  }

  async function handleShopChosen() {
    await refreshSession();
    setTab("dashboard");
    setScreen("app");
  }

  async function handleLogout() {
    await invoke("logout");
    setSession(null);
    setScreen("login");
  }

  if (screen === "loading") {
    return <div className="loading-screen">Loading...</div>;
  }

  if (screen === "activate") {
    return <Activate hardwareId={hardwareId} onActivated={() => setScreen("login")} />;
  }

  if (screen === "login") {
    return <Login onLoggedIn={handleLoggedIn} />;
  }

  if (screen === "pick-shop") {
    return <ShopPicker onShopChosen={handleShopChosen} />;
  }

  // screen === "app"
  if (!session) {
    // Defensive: shouldn't happen, but don't render a shell with no
    // session data rather than crash on session.shop_name below.
    return <div className="loading-screen">Loading...</div>;
  }

  return (
    <div className="app-shell">
      <Sidebar
        session={session}
        multiShopEnabled={multiShopEnabled}
        tab={tab}
        onTab={setTab}
        onLogout={handleLogout}
        onSwitchShop={async (action) => {
          if (action === "manage") {
            // switch_shop() cleared shop_selected -- go back to the picker
            setScreen("pick-shop");
            return;
          }
          // action === "chose": picked a different shop from the dropdown,
          // stay on the dashboard shell with the new shop's data
          await refreshSession();
          setTab("dashboard");
        }}
      />
      <main className="content">
        <ErrorBoundary key={tab}>
          {session.shop_type !== "designer_apparels" || tab === "dashboard" ? (
            <DashboardHome key={session.active_shop_id} />
          ) : (
            <>
              {tab === "billing" && (
                <Billing
                  onCheckoutSuccess={(billNumber) => {
                    setPrintBillNumber(billNumber);
                    setTab("invoice-print");
                  }}
                />
              )}
              {tab === "history" && <History />}
              {tab === "invoice-print" && (
                <InvoicePrint
                  initialBillNumber={printBillNumber}
                  onConsumeInitial={() => setPrintBillNumber(null)}
                />
              )}
              {tab === "accounts" && <Accounts />}
              {tab === "items" && <Items />}
              {tab === "products" && <Products />}
              {tab === "customers" && <Customers />}
              {tab === "suppliers" && <Suppliers />}
              {tab === "settings" && <Settings />}
            </>
          )}
        </ErrorBoundary>
      </main>
    </div>
  );
}
