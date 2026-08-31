//! Dictionaries queries.
//!
//! Split out of a 3,400-line `db.rs` purely to make it navigable; these are
//! the same methods on the same `Catalog`, moved verbatim.

use super::*;
use std::collections::HashMap;
use std::sync::OnceLock;
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

/// Princeton WordNet 3.0 morphological exception lists (noun/verb/adj/adv),
/// gzipped. Surface form → lemmas, one entry per line. See
/// `resources/dictionaries/wordnet-3.0-exc.NOTICE.txt` for source, licence
/// and checksums.
const WORDNET_NOUN_EXC: &[u8] =
    include_bytes!("../../resources/dictionaries/wordnet-3.0-noun.exc.gz");
const WORDNET_VERB_EXC: &[u8] =
    include_bytes!("../../resources/dictionaries/wordnet-3.0-verb.exc.gz");
const WORDNET_ADJ_EXC: &[u8] =
    include_bytes!("../../resources/dictionaries/wordnet-3.0-adj.exc.gz");
const WORDNET_ADV_EXC: &[u8] =
    include_bytes!("../../resources/dictionaries/wordnet-3.0-adv.exc.gz");

/// WordNet exception lists, parsed once into surface form (lowercase) →
/// lemma candidates. Consulted before the suffix-rule fallback so irregulars
/// like `went → go`, `mice → mouse` and `better → good` resolve to a real
/// headword instead of a dead end. Loaded lazily: a lookup that never needs
/// lemmatization never pays the parse.
fn wordnet_exceptions() -> &'static HashMap<String, Vec<String>> {
    static EXC: OnceLock<HashMap<String, Vec<String>>> = OnceLock::new();
    EXC.get_or_init(|| {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for bytes in [
            WORDNET_NOUN_EXC,
            WORDNET_VERB_EXC,
            WORDNET_ADJ_EXC,
            WORDNET_ADV_EXC,
        ] {
            let decoder = flate2::read::GzDecoder::new(bytes);
            let reader = std::io::BufReader::new(decoder);
            for line in std::io::BufRead::lines(reader).map_while(|line| line.ok()) {
                let mut parts = line.split_whitespace();
                let Some(surface) = parts.next() else { continue };
                let lemmas: Vec<String> = parts.map(str::to_string).collect();
                if !lemmas.is_empty() {
                    map.entry(surface.to_string()).or_default().extend(lemmas);
                }
            }
        }
        map
    })
}

/// Fold a headword into its canonical dictionary key.
///
/// Lowercases, strips diacritics (NFD + drop combining marks), collapses
/// whitespace and trims surrounding non-alphanumerics per token, so `Run`,
/// `run` and `rún` all fold to `run` and every lookup hits the same `key`
/// column through `idx_dict_entries_key` instead of a case-insensitive
/// scan over `word`.
pub fn fold_key(word: &str) -> String {
    normalize_dictionary_term(word)
        .to_lowercase()
        .nfd()
        .filter(|ch| !is_combining_mark(*ch))
        .collect()
}

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
            "INSERT INTO dict_entries (dict_id, word, definition, key) VALUES (?1, ?2, ?3, ?4)",
            params![dict_id, word, definition, fold_key(word)],
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
                "INSERT INTO dict_entries (dict_id, word, definition, key) VALUES (?1, ?2, ?3, ?4)",
            )?;
            for (w, d) in entries {
                stmt.execute(params![dict_id, w, d, fold_key(w)])?;
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

    /// One-time backfill of the `key` column for rows imported before the v11
    /// migration. Guarded by `key IS NULL` so it runs at most once per
    /// database and is a no-op on fresh installs (nothing imported yet).
    /// Batched in small transactions because existing libraries can hold a
    /// six-figure WordNet pack.
    pub(crate) fn backfill_dict_entry_keys(&self) -> Result<()> {
        let mut conn = self.conn();
        loop {
            let pending: Vec<(i64, String)> = {
                let mut stmt = conn.prepare_cached(
                    "SELECT id, word FROM dict_entries WHERE key IS NULL LIMIT 2000",
                )?;
                let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
                rows.collect::<std::result::Result<Vec<_>, _>>()?
            };
            if pending.is_empty() {
                return Ok(());
            }
            let tx = conn.transaction()?;
            {
                let mut stmt = tx.prepare("UPDATE dict_entries SET key = ?1 WHERE id = ?2")?;
                for (id, word) in &pending {
                    stmt.execute(params![fold_key(word), id])?;
                }
            }
            tx.commit()?;
        }
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

    /// Exact headword hit, then prefix hit, both against the precomputed
    /// `key` column (see `fold_key`) so the lookup is served by
    /// `idx_dict_entries_key` instead of a `COLLATE NOCASE` scan over `word`.
    /// Exact results always come first; the old `LENGTH(word)` tiebreak is
    /// gone — prefix results simply follow index order.
    fn search_dict_exact_or_prefix(&self, clean: &str, limit: i64) -> Result<Vec<DictEntry>> {
        let key = fold_key(clean);
        if key.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn();

        let like = format!("{}%", escape_like(&key));
        let mut out = Vec::new();
        let mut stmt = conn.prepare_cached(
            "SELECT id, dict_id, word, definition FROM dict_entries
             WHERE key = ?1 COLLATE NOCASE
             ORDER BY word ASC LIMIT ?2",
        )?;
        for r in stmt.query_map(params![key.as_str(), limit], |r| {
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

        let mut stmt2 = conn.prepare_cached(
            "SELECT id, dict_id, word, definition FROM dict_entries
             WHERE key LIKE ?1 ESCAPE '\\' COLLATE NOCASE
             ORDER BY key COLLATE NOCASE ASC, word ASC LIMIT ?2",
        )?;
        for r in stmt2.query_map(params![like.as_str(), limit], |r| {
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
        // Irregulars come from the WordNet exception lists first; the
        // suffix rules below are the fallback for forms the lists do not
        // cover. Both go through `push_dictionary_variant` so dedup stays
        // case-insensitive.
        let surface = inflection_source.to_lowercase();
        if let Some(lemmas) = wordnet_exceptions().get(&surface) {
            for lemma in lemmas {
                push_dictionary_variant(&mut variants, lemma.clone());
            }
        } else {
            for variant in simple_inflection_variants(&inflection_source) {
                push_dictionary_variant(&mut variants, variant);
            }
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
    use super::*;

    #[test]
    fn fold_key_normalizes_case_whitespace_and_diacritics() {
        assert_eq!(fold_key("Run"), "run");
        assert_eq!(fold_key("RUN"), "run");
        assert_eq!(fold_key("rún"), "run");
        assert_eq!(fold_key("ÉTÉ"), "ete");
        assert_eq!(fold_key("  Hello,   World!!  "), "hello world");
        assert_eq!(fold_key(""), "");
        assert_eq!(fold_key("!!! ..."), "");
    }

    #[test]
    fn inserted_entries_carry_folded_keys() {
        let cat = Catalog::open_in_memory().unwrap();
        let dict_id = cat.insert_dictionary("Test pack", Some("en"), 2).unwrap();
        cat.batch_insert_dict_entries(
            dict_id,
            &[
                ("Rún".to_string(), "an Irish hero".to_string()),
                ("Run".to_string(), "to move fast".to_string()),
            ],
        )
        .unwrap();
        let conn = cat.conn();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM dict_entries WHERE key = 'run'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 2);
    }

    #[test]
    fn search_dict_folds_case_and_diacritics_into_keys() {
        let cat = Catalog::open_in_memory().unwrap();
        let dict_id = cat.insert_dictionary("Test pack", Some("en"), 2).unwrap();
        cat.batch_insert_dict_entries(
            dict_id,
            &[
                ("run".to_string(), "to move fast".to_string()),
                (
                    "Rúnestone".to_string(),
                    "a stone carved with runes".to_string(),
                ),
            ],
        )
        .unwrap();

        // Exact hit regardless of case.
        let hits = cat.search_dict("Run", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].word, "run");

        // Diacritics fold away.
        let hits = cat.search_dict("rún", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].word, "run");

        // Prefix fallback through the key column.
        let hits = cat.search_dict("Rune", 10).unwrap();
        assert!(hits.iter().any(|hit| hit.word == "Rúnestone"));
    }

    #[test]
    fn exact_key_lookup_uses_the_key_index() {
        let cat = Catalog::open_in_memory().unwrap();
        let dict_id = cat.insert_dictionary("Test pack", Some("en"), 1).unwrap();
        cat.batch_insert_dict_entries(dict_id, &[("run".to_string(), "fast".to_string())])
            .unwrap();
        let conn = cat.conn();
        let plan: String = conn
            .query_row(
                "EXPLAIN QUERY PLAN
                 SELECT id, dict_id, word, definition FROM dict_entries
                 WHERE key = ?1 COLLATE NOCASE",
                params!["run"],
                |r| r.get(3),
            )
            .unwrap();
        assert!(
            plan.contains("idx_dict_entries_key"),
            "expected the key index in the query plan, got: {plan}"
        );
    }

    #[test]
    fn dictionary_variants_resolve_irregulars_from_wordnet_exceptions() {
        let went = dictionary_query_variants("went");
        assert!(went.iter().any(|variant| variant == "go"));

        let mice = dictionary_query_variants("mice");
        assert!(mice.iter().any(|variant| variant == "mouse"));

        // adj.exc lists two lemmas for `better`; both must surface.
        let better = dictionary_query_variants("better");
        assert!(better.iter().any(|variant| variant == "good"));
        assert!(better.iter().any(|variant| variant == "well"));

        // Possessive base resolves through the noun exceptions.
        let childrens = dictionary_query_variants("children’s");
        assert!(childrens.iter().any(|variant| variant == "child"));

        // Capitalized surface still resolves (case-insensitive map lookup).
        let went_cap = dictionary_query_variants("Went");
        assert!(went_cap.iter().any(|variant| variant == "go"));
    }

    #[test]
    fn dictionary_variants_keep_suffix_rules_as_fallback() {
        // Forms the exception lists do not cover still go through the suffix
        // rules (e.g. walked → walk).
        let walked = dictionary_query_variants("walked");
        assert!(walked.iter().any(|variant| variant == "walk"));

        // The suffix rules must not add junk on top of an irregular hit:
        // `better` resolves to its WordNet lemmas, not a guessed stem.
        let better = dictionary_query_variants("better");
        assert!(!better.iter().any(|variant| variant == "bett"));
    }

    #[test]
    fn search_dict_resolves_irregulars_to_headwords() {
        let cat = Catalog::open_in_memory().unwrap();
        let dict_id = cat.insert_dictionary("Test pack", Some("en"), 4).unwrap();
        cat.batch_insert_dict_entries(
            dict_id,
            &[
                ("go".to_string(), "to move from one place to another".to_string()),
                ("mouse".to_string(), "a small rodent".to_string()),
                ("good".to_string(), "having desirable qualities".to_string()),
                ("run".to_string(), "to move fast".to_string()),
            ],
        )
        .unwrap();

        let hits = cat.search_dict("went", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].word, "go");

        let hits = cat.search_dict("mice", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].word, "mouse");

        let hits = cat.search_dict("better", 10).unwrap();
        assert!(hits.iter().any(|hit| hit.word == "good"));

        // Regular inflection still resolves through the suffix fallback.
        let hits = cat.search_dict("running", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].word, "run");
    }

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
