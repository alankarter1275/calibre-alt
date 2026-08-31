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
        let clean = word.trim();
        if clean.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let variants = dictionary_query_variants(clean);
        let lim = limit as i64;
        // Try exact and prefix matches for every normalized form before using
        // the older substring fallback. This lets `running` find `run` even
        // when a definition happens to contain the word "running".
        for variant in &variants {
            let out = self.search_dict_exact_or_prefix(variant, lim)?;
            if !out.is_empty() {
                return Ok(out);
            }
        }

        let normalized = normalize_dictionary_term(clean);
        let fallback = if normalized.is_empty() {
            clean
        } else {
            normalized.as_str()
        };
        self.search_dict_substring(fallback, lim)
    }

    fn search_dict_exact_or_prefix(&self, clean: &str, limit: i64) -> Result<Vec<DictEntry>> {
        let conn = self.conn();
        let mut stmt = conn.prepare_cached(
            "SELECT id, dict_id, word, definition FROM dict_entries
             WHERE word = ?1 COLLATE NOCASE
             ORDER BY word ASC LIMIT ?2",
        )?;
        let mut out = Vec::new();
        for r in stmt.query_map(params![clean, limit], |r| {
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

        let like = format!("{}%", escape_like(clean));
        let mut stmt2 = conn.prepare_cached(
            "SELECT id, dict_id, word, definition FROM dict_entries
             WHERE word LIKE ?1 ESCAPE '\\' COLLATE NOCASE
             ORDER BY LENGTH(word) ASC, word ASC LIMIT ?2",
        )?;
        for r in stmt2.query_map(params![like, limit], |r| {
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

    fn search_dict_substring(&self, clean: &str, limit: i64) -> Result<Vec<DictEntry>> {
        let conn = self.conn();
        let like = format!("%{}%", escape_like(clean));
        let mut stmt = conn.prepare_cached(
            "SELECT id, dict_id, word, definition FROM dict_entries
             WHERE word LIKE ?1 ESCAPE '\\' COLLATE NOCASE
                OR definition LIKE ?1 ESCAPE '\\'
             ORDER BY LENGTH(word) ASC LIMIT ?2",
        )?;
        let mut out = Vec::new();
        for r in stmt.query_map(params![like, limit], |r| {
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

fn dictionary_query_variants(term: &str) -> Vec<String> {
    let trimmed = term.trim();
    let normalized = normalize_dictionary_term(trimmed);
    let primary = if normalized.is_empty() {
        trimmed.to_string()
    } else {
        normalized.clone()
    };
    let mut variants = Vec::new();
    push_dictionary_variant(&mut variants, trimmed.to_string());
    push_dictionary_variant(&mut variants, primary.clone());

    let mut inflection_source = primary;
    if let Some(base) = dictionary_possessive_base(&inflection_source) {
        push_dictionary_variant(&mut variants, base.to_string());
        inflection_source = base.to_string();
    }

    if inflection_source.split_whitespace().count() == 1
        && inflection_source.chars().all(|ch| ch.is_alphabetic())
    {
        for variant in simple_inflection_variants(&inflection_source) {
            push_dictionary_variant(&mut variants, variant);
        }
    }
    variants
}

fn normalize_dictionary_term(term: &str) -> String {
    term.split_whitespace()
        .map(|token| token.trim_matches(|ch: char| !ch.is_alphanumeric()))
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn dictionary_possessive_base(term: &str) -> Option<&str> {
    term.strip_suffix("'s")
        .or_else(|| term.strip_suffix("’s"))
        .or_else(|| term.strip_suffix('\''))
        .or_else(|| term.strip_suffix('’'))
}

fn simple_inflection_variants(term: &str) -> Vec<String> {
    let mut variants = Vec::new();
    if let Some(stem) = term.strip_suffix("ies") {
        if stem.len() >= 2 {
            variants.push(format!("{stem}y"));
        }
    }
    if let Some(stem) = term.strip_suffix("ied") {
        if stem.len() >= 2 {
            variants.push(format!("{stem}y"));
        }
    }
    if let Some(stem) = term.strip_suffix("es") {
        if stem.len() >= 2 {
            variants.push(stem.to_string());
        }
    }
    if let Some(stem) = term.strip_suffix('s') {
        if stem.len() >= 3
            && !term.ends_with("ss")
            && !term.ends_with("us")
            && !term.ends_with("is")
        {
            variants.push(stem.to_string());
        }
    }
    if let Some(stem) = term.strip_suffix("ing") {
        if stem.len() >= 3 {
            add_simple_verb_stems(&mut variants, stem);
        }
    }
    if let Some(stem) = term.strip_suffix("ed") {
        if stem.len() >= 3 {
            add_simple_verb_stems(&mut variants, stem);
        }
    }
    variants
}

fn add_simple_verb_stems(variants: &mut Vec<String>, stem: &str) {
    let undoubled = undouble_final_letter(stem);
    variants.push(undoubled.clone());
    if let Some(last) = undoubled.chars().last() {
        if "bcdfghjklmnpqrstvwxyz".contains(last.to_ascii_lowercase()) {
            variants.push(format!("{undoubled}e"));
        }
    }
}

fn undouble_final_letter(term: &str) -> String {
    let mut chars = term.chars().collect::<Vec<_>>();
    if chars.len() >= 2 && chars[chars.len() - 1] == chars[chars.len() - 2] {
        chars.pop();
    }
    chars.into_iter().collect()
}

fn push_dictionary_variant(variants: &mut Vec<String>, candidate: String) {
    if !candidate.is_empty()
        && !variants
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&candidate))
    {
        variants.push(candidate);
    }
}

#[cfg(test)]
mod tests {
    use super::dictionary_query_variants;

    #[test]
    fn dictionary_variants_strip_outer_punctuation() {
        let variants = dictionary_query_variants("\u{201c}word,\u{201d}");
        assert!(variants.iter().any(|variant| variant == "word"));
    }

    #[test]
    fn dictionary_variants_handle_common_inflections() {
        let running = dictionary_query_variants("running");
        assert!(running.iter().any(|variant| variant == "run"));

        let studies = dictionary_query_variants("studies");
        assert!(studies.iter().any(|variant| variant == "study"));
    }

    #[test]
    fn dictionary_variants_handle_possessives() {
        let variants = dictionary_query_variants("children’s");
        assert!(variants.iter().any(|variant| variant == "children"));
    }
}
