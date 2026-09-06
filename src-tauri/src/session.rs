//! session.rs
//! -----------
//! Rust/Tauri equivalent of the Python app's Flask server-side session
//! cookie. There's no cookie here -- it's one process, one user, one
//! window -- so this is just an in-memory struct behind a Mutex inside
//! `AppState`, holding exactly the fields `routes/auth.py` and
//! `routes/shops.py` used to stash on `flask.session`:
//!
//!     logged_in     -- credentials were verified (set at login, never
//!                       touched again until logout)
//!     auth_shop_id  -- which vm_shopprofile row the login credentials
//!                       themselves belonged to (kept for reference;
//!                       not used for data scoping)
//!     active_shop_id -- the ACTIVE shop's sp_shopid. Every future
//!                       vertical command's SQL queries filter by this
//!                       (WHERE user_id = ?) -- same as Python's
//!                       session["admin"].
//!     shop_name      -- the ACTIVE shop's display name (sidebar)
//!     shop_type      -- the ACTIVE shop's sp_shop_type, used by the
//!                       shop_type access-guard (see shop_guard.rs) and
//!                       by the React sidebar to decide which nav items
//!                       to show
//!     shop_selected  -- true once a shop has actually been chosen this
//!                       session; mirrors Python's session["shop_selected"]
//!
//! Multi-shop mode note: single-shop deployments (config.json
//! "multi_shop_enabled": false) set active_shop_id/shop_selected
//! immediately at login, exactly like the Python version -- there is no
//! separate "picker" step in that mode.

use serde::Serialize;
use std::sync::Mutex;

/// Tauri-managed wrapper around the session, same pattern as the
/// existing invoicing app's `Db(pub Mutex<Connection>)` in db.rs.
pub struct SessionState(pub Mutex<Session>);

#[derive(Debug, Clone, Default, Serialize)]
pub struct Session {
    pub logged_in: bool,
    pub username: String,
    pub auth_shop_id: Option<i64>,
    pub active_shop_id: Option<i64>,
    pub shop_name: String,
    pub acnttype: i64,
    pub shop_type: String,
    pub shop_selected: bool,
}

impl Session {
    pub fn clear(&mut self) {
        *self = Session::default();
    }
}

/// Equivalent of Python's `login_required` decorator, called at the top
/// of every command that needs an authenticated + shop-scoped session.
/// Check order matters, same as the Python version:
///   1. not logged in at all -> caller should route back to login
///   2. logged in but no shop chosen yet (multi-shop mode only) ->
///      caller should route to the shop picker
///   3. otherwise -> proceed
/// Returned as a typed reason string rather than an HTTP redirect,
/// since the frontend (not this layer) owns which screen to show.
pub enum LoginGuard {
    Ok,
    NotLoggedIn,
    NoShopSelected,
}

pub fn check_login(session: &Session) -> LoginGuard {
    if !session.logged_in {
        return LoginGuard::NotLoggedIn;
    }
    if !session.shop_selected || session.active_shop_id.is_none() {
        return LoginGuard::NoShopSelected;
    }
    LoginGuard::Ok
}
