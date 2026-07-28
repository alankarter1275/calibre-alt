//! Prefs queries.
//!
//! Split out of a 3,400-line `db.rs` purely to make it navigable; these are
//! the same methods on the same `Catalog`, moved verbatim.

use super::*;

impl Catalog {
    // -----------------------------------------------------------------------
    // P4.1: preferences
    // -----------------------------------------------------------------------

    pub fn get_pref(&self, key: &str) -> Option<String> {
        let conn = self.conn.lock().ok()?;
        conn.query_row(
            "SELECT value FROM app_prefs WHERE key = ?1",
            params![key],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten()
    }

    pub fn set_pref(&self, key: &str, value: &str) {
        if let Ok(conn) = self.conn.lock() {
            let _ = conn.execute(
                "INSERT INTO app_prefs (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            );
        }
    }

    /// Convenience for numeric prefs; falls back when unset or unparsable.
    pub fn get_pref_i64(&self, key: &str, default: i64) -> i64 {
        self.get_pref(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }
}
