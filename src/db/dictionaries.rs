//! Dictionaries queries.
//!
//! Split out of a 3,400-line `db.rs` purely to make it navigable; these are
//! the same methods on the same `Catalog`, moved verbatim.

use super::*;

impl Catalog {
    // -----------------------------------------------------------------------
    // P3: Dictionaries
    // -----------------------------------------------------------------------

    pub fn list_dictionaries(&self) -> Result<Vec<Dictionary>> {
        let conn = self.conn();
        let mut stmt = conn.prepare_cached(
            "SELECT id, name, lang, entry_count, added_at FROM dictionaries ORDER BY name ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Dictionary {
                id: r.get(0)?,
                name: r.get(1)?,
                lang: r.get(2)?,
                entry_count: r.get(3)?,
                added_at: r.get(4)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn insert_dictionary(
        &self,
        name: &str,
        lang: Option<&str>,
        entry_count: i64,
    ) -> Result<i64> {
        let conn = self.conn();
        let now = chrono_like_now();
        conn.execute(
            "INSERT INTO dictionaries (name, lang, entry_count, added_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(name) DO UPDATE SET lang=excluded.lang, entry_count=excluded.entry_count",
            params![name, lang, entry_count, now],
        )?;
        let id = conn.query_row(
            "SELECT id FROM dictionaries WHERE name = ?1",
            params![name],
            |r| r.get(0),
        )?;
        Ok(id)
    }

    pub fn delete_dictionary(&self, id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute("DELETE FROM dictionaries WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn set_dictionary_entry_count(&self, id: i64, count: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE dictionaries SET entry_count = ?1 WHERE id = ?2",
            params![count, id],
        )?;
        Ok(())
    }

    pub fn insert_dict_entry(&self, dict_id: i64, word: &str, definition: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO dict_entries (dict_id, word, definition) VALUES (?1, ?2, ?3)",
            params![dict_id, word, definition],
        )?;
        Ok(())
    }

    pub fn batch_insert_dict_entries(
        &self,
        dict_id: i64,
        entries: &[(String, String)],
    ) -> Result<()> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO dict_entries (dict_id, word, definition) VALUES (?1, ?2, ?3)",
            )?;
            for (w, d) in entries {
                stmt.execute(params![dict_id, w, d])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn clear_dict_entries(&self, dict_id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM dict_entries WHERE dict_id = ?1",
            params![dict_id],
        )?;
        Ok(())
    }

    pub fn search_dict(&self, word: &str, limit: usize) -> Result<Vec<DictEntry>> {
        let conn = self.conn();
        let clean = word.trim();
        if clean.is_empty() {
            return Ok(Vec::new());
        }
        // Exact match first, then prefix, then LIKE fallback
        let mut stmt = conn.prepare_cached(
            "SELECT id, dict_id, word, definition FROM dict_entries
             WHERE word = ?1 COLLATE NOCASE
             ORDER BY word ASC LIMIT ?2",
        )?;
        let mut out = Vec::new();
        let lim = limit as i64;
        for r in stmt.query_map(params![clean, lim], |r| {
            Ok(DictEntry {
                id: r.get(0)?,
                dict_id: r.get(1)?,
                word: r.get(2)?,
                definition: r.get(3)?,
            })
        })? {
            out.push(r?);
        }
        if !out.is_empty() {
            return Ok(out);
        }

        // Prefix search
        let like = format!("{}%", escape_like(clean));
        let mut stmt2 = conn.prepare_cached(
            "SELECT id, dict_id, word, definition FROM dict_entries
             WHERE word LIKE ?1 ESCAPE '\\' COLLATE NOCASE
             ORDER BY LENGTH(word) ASC, word ASC LIMIT ?2",
        )?;
        for r in stmt2.query_map(params![like, lim], |r| {
            Ok(DictEntry {
                id: r.get(0)?,
                dict_id: r.get(1)?,
                word: r.get(2)?,
                definition: r.get(3)?,
            })
        })? {
            out.push(r?);
        }
        if !out.is_empty() {
            return Ok(out);
        }

        // Substring fallback
        let like2 = format!("%{}%", escape_like(clean));
        let mut stmt3 = conn.prepare_cached(
            "SELECT id, dict_id, word, definition FROM dict_entries
             WHERE word LIKE ?1 ESCAPE '\\' COLLATE NOCASE
                OR definition LIKE ?1 ESCAPE '\\'
             ORDER BY LENGTH(word) ASC LIMIT ?2",
        )?;
        for r in stmt3.query_map(params![like2, lim], |r| {
            Ok(DictEntry {
                id: r.get(0)?,
                dict_id: r.get(1)?,
                word: r.get(2)?,
                definition: r.get(3)?,
            })
        })? {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn dict_entry_count(&self) -> Result<i64> {
        let conn = self.conn();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM dict_entries", [], |r| r.get(0))?;
        Ok(n)
    }
}
