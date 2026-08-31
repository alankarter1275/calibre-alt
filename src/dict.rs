//! Offline dictionary handling — StarDict + Kalam SQLite packs.
#![allow(dead_code)]
//!
//! StarDict format (minimal parser):
//!   .ifo – metadata (wordcount)
//!   .idx – sorted list of [word\0][offset 32-bit BE][size 32-bit BE] (also supports 64-bit BE if sametypesequence has 'g'/'h' – we try both)
//!   .dict / .dict.dz – concatenated definitions
//! For simplicity P3 supports uncompressed .dict; .dict.dz is decompressed via flate2 if present.
//! SQLite pack: a SQLite file with table entries(word TEXT, definition TEXT) or (word, definition) naming variations.

use crate::db::Catalog;
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

/// Result of a dictionary search.
#[derive(Debug, Clone)]
pub struct DictSearchResult {
    pub word: String,
    pub definition: String,
    pub dict_name: String,
}

const BUNDLED_WORDNET_NAME: &str = "English WordNet 2025";
const BUNDLED_WORDNET_PREF: &str = "bundled_dictionary_english_wordnet_2025";
const BUNDLED_WORDNET_TSV_GZ: &[u8] =
    include_bytes!("../resources/dictionaries/english-wordnet-2025.tsv.gz");
const BUNDLED_IDIOMS_NAME: &str = "English Idioms and Expressions";
const BUNDLED_IDIOMS_PREF: &str = "bundled_dictionary_english_idioms_2024";
const BUNDLED_IDIOMS_TSV_GZ: &[u8] =
    include_bytes!("../resources/dictionaries/english-idioms-2024.tsv.gz");
const BUNDLED_SYNONYMS_NAME: &str = "English Synonyms (WordNet 3.0)";
const BUNDLED_SYNONYMS_PREF: &str = "bundled_dictionary_english_synonyms_3_0";
const BUNDLED_SYNONYMS_TSV_GZ: &[u8] =
    include_bytes!("../resources/dictionaries/english-synonyms-3.0.tsv.gz");
const BUNDLED_ANTONYMS_NAME: &str = "English Antonyms (WordNet 3.0)";
const BUNDLED_ANTONYMS_PREF: &str = "bundled_dictionary_english_antonyms_3_0";
const BUNDLED_ANTONYMS_TSV_GZ: &[u8] =
    include_bytes!("../resources/dictionaries/english-antonyms-3.0.tsv.gz");

/// Install the small, redistributable English dictionaries shipped with Kalam.
///
/// Each preference makes its pack a first-run action rather than a migration
/// that re-adds a pack after the user removes it. The compressed sources are
/// kept in the binary so the default dictionaries work without a download.
pub fn install_bundled_dictionaries(catalog: &Catalog) -> Result<()> {
    install_bundled_tsv(
        catalog,
        BUNDLED_WORDNET_NAME,
        BUNDLED_WORDNET_PREF,
        BUNDLED_WORDNET_TSV_GZ,
    )?;
    install_bundled_tsv(
        catalog,
        BUNDLED_IDIOMS_NAME,
        BUNDLED_IDIOMS_PREF,
        BUNDLED_IDIOMS_TSV_GZ,
    )?;
    install_bundled_tsv(
        catalog,
        BUNDLED_SYNONYMS_NAME,
        BUNDLED_SYNONYMS_PREF,
        BUNDLED_SYNONYMS_TSV_GZ,
    )?;
    install_bundled_tsv(
        catalog,
        BUNDLED_ANTONYMS_NAME,
        BUNDLED_ANTONYMS_PREF,
        BUNDLED_ANTONYMS_TSV_GZ,
    )?;
    Ok(())
}

fn install_bundled_tsv(
    catalog: &Catalog,
    dictionary_name: &str,
    installed_pref: &str,
    compressed_tsv: &[u8],
) -> Result<()> {
    if catalog.get_pref(installed_pref).as_deref() == Some("installed") {
        return Ok(());
    }

    // This also handles an upgrade from a build that seeded the row before it
    // stored the first-run marker.
    if catalog
        .list_dictionaries()?
        .iter()
        .any(|dict| dict.name == dictionary_name)
    {
        catalog.set_pref(installed_pref, "installed");
        return Ok(());
    }

    let decoder = flate2::read::GzDecoder::new(compressed_tsv);
    let reader = BufReader::new(decoder);
    let mut entries = Vec::new();
    for line in std::io::BufRead::lines(reader) {
        let line = line?;
        let Some((word, definition)) = line.split_once('\t') else {
            continue;
        };
        let word = word.trim();
        let definition = definition.trim();
        if !word.is_empty() && !definition.is_empty() {
            entries.push((word.to_string(), definition.to_string()));
        }
    }
    if entries.is_empty() {
        return Err(anyhow!(
            "bundled dictionary pack '{dictionary_name}' is empty"
        ));
    }

    let dict_id = catalog.insert_dictionary(dictionary_name, Some("en"), entries.len() as i64)?;
    catalog.clear_dict_entries(dict_id)?;
    for chunk in entries.chunks(2000) {
        catalog.batch_insert_dict_entries(dict_id, chunk)?;
    }
    catalog.set_pref(installed_pref, "installed");
    Ok(())
}

/// Import a dictionary pack into the catalog.
///
/// Supports:
/// - StarDict triple (.ifo + .idx + .dict[.dz]): provide any one file path, we find siblings.
/// - SQLite file with entries table.
///
/// Returns the dictionary name and entry count.
pub fn import_dictionary(catalog: &Catalog, path: &Path) -> Result<(String, i64)> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    // Detect StarDict by .ifo/.idx/.dict extension or by sibling presence
    if ext == "ifo" || ext == "idx" || ext == "dict" || ext == "dz" || ext == "dict" {
        return import_stardict(catalog, path);
    }

    // Try SQLite detection: if file is SQLite (starts with "SQLite format 3\0")
    if is_sqlite_file(path)? {
        return import_sqlite_pack(catalog, path);
    }

    // Fallback: try stardict detection from base name (user selected .ifo)
    if path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.to_ascii_lowercase().contains("stardict") || n.ends_with(".ifo"))
        .unwrap_or(false)
    {
        return import_stardict(catalog, path);
    }

    // Last try: plain text tab-separated dictionary (word<TAB>definition per line)
    if ext == "txt" || ext == "tab" || ext == "tsv" {
        return import_tsv(catalog, path);
    }

    Err(anyhow!(
        "unrecognized dictionary format for {} (.ifo/.idx/.dict, .db sqlite pack, or .txt tab-separated supported)",
        path.display()
    ))
}

fn is_sqlite_file(path: &Path) -> Result<bool> {
    let mut f = File::open(path)?;
    let mut header = [0u8; 16];
    let n = f.read(&mut header).unwrap_or(0);
    if n < 16 {
        return Ok(false);
    }
    Ok(&header[..16] == b"SQLite format 3\0")
}

/// Import SQLite pack: look for table `entries` or `dict` or `words`
fn import_sqlite_pack(catalog: &Catalog, path: &Path) -> Result<(String, i64)> {
    use rusqlite::Connection;

    let conn = Connection::open(path).context("open sqlite dict")?;
    // Find a suitable table
    let mut tables = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
    )?;
    for r in stmt.query_map([], |row| row.get::<_, String>(0))? {
        tables.push(r?);
    }

    let chosen = if tables.contains(&"entries".to_string()) {
        "entries"
    } else if tables.contains(&"dict".to_string()) {
        "dict"
    } else if tables.contains(&"words".to_string()) {
        "words"
    } else if !tables.is_empty() {
        &tables[0]
    } else {
        return Err(anyhow!("sqlite dict has no tables"));
    };

    // Determine column names
    let mut columns_stmt = conn.prepare(&format!("PRAGMA table_info({chosen})"))?;
    let mut cols = Vec::new();
    for r in columns_stmt.query_map([], |row| row.get::<_, String>(1))? {
        cols.push(r?);
    }
    let word_col = if cols.iter().any(|c| c.eq_ignore_ascii_case("word")) {
        cols.iter()
            .find(|c| c.eq_ignore_ascii_case("word"))
            .unwrap()
    } else if cols.iter().any(|c| c.eq_ignore_ascii_case("term")) {
        cols.iter()
            .find(|c| c.eq_ignore_ascii_case("term"))
            .unwrap()
    } else {
        &cols[0]
    };
    let def_col = if cols.iter().any(|c| c.eq_ignore_ascii_case("definition")) {
        cols.iter()
            .find(|c| c.eq_ignore_ascii_case("definition"))
            .unwrap()
    } else if cols.iter().any(|c| c.eq_ignore_ascii_case("meaning")) {
        cols.iter()
            .find(|c| c.eq_ignore_ascii_case("meaning"))
            .unwrap()
    } else if cols.iter().any(|c| c.eq_ignore_ascii_case("def")) {
        cols.iter().find(|c| c.eq_ignore_ascii_case("def")).unwrap()
    } else if cols.len() >= 2 {
        &cols[1]
    } else {
        &cols[0]
    };

    let mut stmt = conn.prepare(&format!("SELECT {}, {} FROM {}", word_col, def_col, chosen))?;
    let mut entries = Vec::new();
    for r in stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })? {
        let (w, d) = r?;
        let w = w.trim();
        let d = d.trim();
        if !w.is_empty() && !d.is_empty() {
            entries.push((w.to_string(), d.to_string()));
        }
    }

    if entries.is_empty() {
        return Err(anyhow!("sqlite dict {} has no entries", path.display()));
    }

    let dict_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("ImportedDict")
        .to_string();

    let dict_id = catalog
        .insert_dictionary(&dict_name, None, entries.len() as i64)
        .map_err(|e| anyhow!("insert dict meta: {e}"))?;
    catalog
        .clear_dict_entries(dict_id)
        .map_err(|e| anyhow!("clear dict: {e}"))?;
    catalog
        .batch_insert_dict_entries(dict_id, &entries)
        .map_err(|e| anyhow!("batch insert: {e}"))?;

    Ok((dict_name, entries.len() as i64))
}

fn import_tsv(catalog: &Catalog, path: &Path) -> Result<(String, i64)> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    for line in std::io::BufRead::lines(reader) {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(tab) = line.find('\t') {
            let w = line[..tab].trim();
            let d = line[tab + 1..].trim();
            if !w.is_empty() && !d.is_empty() {
                entries.push((w.to_string(), d.to_string()));
            }
        } else if let Some(sep) = line.find("  ") {
            let w = line[..sep].trim();
            let d = line[sep..].trim();
            if !w.is_empty() && !d.is_empty() {
                entries.push((w.to_string(), d.to_string()));
            }
        }
    }
    if entries.is_empty() {
        return Err(anyhow!("TSV dict empty"));
    }
    let dict_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("TSVDict")
        .to_string();
    let dict_id = catalog
        .insert_dictionary(&dict_name, None, entries.len() as i64)
        .map_err(|e| anyhow!("insert dict: {e}"))?;
    catalog
        .clear_dict_entries(dict_id)
        .map_err(|e| anyhow!("clear: {e}"))?;
    catalog
        .batch_insert_dict_entries(dict_id, &entries)
        .map_err(|e| anyhow!("batch: {e}"))?;
    Ok((dict_name, entries.len() as i64))
}

// ---------------------------------------------------------------------------
// StarDict
// ---------------------------------------------------------------------------

fn import_stardict(catalog: &Catalog, any_path: &Path) -> Result<(String, i64)> {
    let base = stardict_base_path(any_path)?;
    let ifo_path = base.with_extension("ifo");
    let idx_path = base.with_extension("idx");
    let dict_path = base.with_extension("dict");
    let dict_dz_path = PathBuf::from(format!("{}.dict.dz", base.display()));

    let ifo_path = if ifo_path.exists() {
        ifo_path
    } else if any_path.extension().map(|e| e == "ifo").unwrap_or(false) {
        any_path.to_path_buf()
    } else if Path::new(&format!("{}.ifo", base.display())).exists() {
        PathBuf::from(format!("{}.ifo", base.display()))
    } else {
        find_sibling_with_ext(any_path, "ifo")
            .ok_or_else(|| anyhow!("StarDict .ifo not found for {}", any_path.display()))?
    };

    let idx_path = if idx_path.exists() {
        idx_path
    } else {
        find_sibling_with_ext(&ifo_path, "idx").ok_or_else(|| anyhow!("StarDict .idx not found"))?
    };

    let dict_path_opt = if dict_path.exists() {
        Some(dict_path)
    } else if dict_dz_path.exists() {
        Some(dict_dz_path)
    } else if let Some(p) = find_sibling_with_ext(&ifo_path, "dict") {
        Some(p)
    } else if let Some(p) = find_sibling_with_ext(&ifo_path, "dz") {
        Some(p)
    } else {
        find_file_ending_with(
            ifo_path.parent().unwrap_or_else(|| Path::new(".")),
            ".dict.dz",
        )
    };

    let dict_path = dict_path_opt.ok_or_else(|| anyhow!("StarDict .dict/.dict.dz not found"))?;

    let meta = parse_ifo(&ifo_path)?;
    let dict_name = meta.get("bookname").cloned().unwrap_or_else(|| {
        base.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("StarDict")
            .to_string()
    });

    let entries_meta = parse_idx(&idx_path, &meta)?;
    let dict_bytes = read_dict_file(&dict_path)?;

    let mut entries = Vec::with_capacity(entries_meta.len());
    for em in entries_meta {
        if em.offset as usize + em.size as usize <= dict_bytes.len() {
            let slice = &dict_bytes[em.offset as usize..em.offset as usize + em.size as usize];
            let def = String::from_utf8_lossy(slice)
                .trim_end_matches('\0')
                .trim()
                .to_string();
            if !def.is_empty() {
                entries.push((em.word, def));
            }
        }
    }

    if entries.is_empty() {
        return Err(anyhow!("StarDict produced no entries"));
    }

    let dict_id = catalog
        .insert_dictionary(&dict_name, None, entries.len() as i64)
        .map_err(|e| anyhow!("insert dict meta: {e}"))?;
    catalog
        .clear_dict_entries(dict_id)
        .map_err(|e| anyhow!("clear: {e}"))?;
    for chunk in entries.chunks(2000) {
        catalog
            .batch_insert_dict_entries(dict_id, chunk)
            .map_err(|e| anyhow!("batch insert failed: {e}"))?;
    }

    Ok((dict_name, entries.len() as i64))
}

fn stardict_base_path(any_path: &Path) -> Result<PathBuf> {
    let s = any_path.to_string_lossy();
    let mut base = s.to_string();
    for ext in &[".ifo", ".idx", ".dict.dz", ".dict", ".dz"] {
        if base.to_ascii_lowercase().ends_with(ext) {
            base = base[..base.len() - ext.len()].to_string();
            break;
        }
    }
    Ok(PathBuf::from(base))
}

fn find_sibling_with_ext(base: &Path, ext: &str) -> Option<PathBuf> {
    let parent = base.parent()?;
    let stem = base.file_stem()?.to_str()?;
    let candidate = parent.join(format!("{stem}.{ext}"));
    if candidate.exists() {
        return Some(candidate);
    }
    if let Ok(dir) = std::fs::read_dir(parent) {
        for entry in dir.flatten() {
            let p = entry.path();
            let ext_match = p
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case(ext))
                .unwrap_or(false);
            let name_match = p
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.contains(stem))
                .unwrap_or(false);
            if ext_match && name_match {
                return Some(p);
            }
            if ext == "dz" && p.to_string_lossy().ends_with(".dict.dz") {
                return Some(p);
            }
            if ext == "dict"
                && (p.to_string_lossy().ends_with(".dict")
                    || p.to_string_lossy().ends_with(".dict.dz"))
            {
                return Some(p);
            }
        }
    }
    None
}

fn find_file_ending_with(dir: &Path, suffix: &str) -> Option<PathBuf> {
    let rd = std::fs::read_dir(dir).ok()?;
    for e in rd.flatten() {
        let p = e.path();
        if p.to_string_lossy().ends_with(suffix) {
            return Some(p);
        }
    }
    None
}

fn parse_ifo(path: &Path) -> Result<HashMap<String, String>> {
    let content = std::fs::read_to_string(path).context("read .ifo")?;
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("StarDict's dict") {
            continue;
        }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim().to_string();
            let value = line[eq + 1..].trim().to_string();
            map.insert(key, value);
        }
    }
    Ok(map)
}

#[derive(Debug)]
struct IdxEntryMeta {
    word: String,
    offset: u64,
    size: u64,
}

fn parse_idx(path: &Path, _ifo_meta: &HashMap<String, String>) -> Result<Vec<IdxEntryMeta>> {
    let data = std::fs::read(path).context("read .idx")?;
    let mut out = Vec::new();
    let mut i = 0usize;
    let mut try_64 = false;
    if let Some(bits) = _ifo_meta.get("idxoffsetbits") {
        if bits.trim() == "64" {
            try_64 = true;
        }
    }
    if try_64 {
        while i < data.len() {
            let start = i;
            while i < data.len() && data[i] != 0 {
                i += 1;
            }
            if i >= data.len() {
                break;
            }
            let word_bytes = &data[start..i];
            let word = String::from_utf8_lossy(word_bytes).to_string();
            i += 1;
            if i + 16 > data.len() {
                break;
            }
            let offset = u64::from_be_bytes([
                data[i],
                data[i + 1],
                data[i + 2],
                data[i + 3],
                data[i + 4],
                data[i + 5],
                data[i + 6],
                data[i + 7],
            ]);
            let size = u64::from_be_bytes([
                data[i + 8],
                data[i + 9],
                data[i + 10],
                data[i + 11],
                data[i + 12],
                data[i + 13],
                data[i + 14],
                data[i + 15],
            ]);
            i += 16;
            out.push(IdxEntryMeta { word, offset, size });
        }
    } else {
        while i < data.len() {
            let start = i;
            while i < data.len() && data[i] != 0 {
                i += 1;
            }
            if i >= data.len() {
                break;
            }
            let word_bytes = &data[start..i];
            let word = String::from_utf8_lossy(word_bytes).to_string();
            i += 1;
            if i + 8 > data.len() {
                break;
            }
            let offset =
                u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as u64;
            let size =
                u32::from_be_bytes([data[i + 4], data[i + 5], data[i + 6], data[i + 7]]) as u64;
            i += 8;
            out.push(IdxEntryMeta { word, offset, size });
        }
    }
    Ok(out)
}

fn read_dict_file(path: &Path) -> Result<Vec<u8>> {
    let s = path.to_string_lossy().to_ascii_lowercase();
    if s.ends_with(".dz") || s.ends_with(".gz") {
        let file = File::open(path).context("open dict.dz")?;
        let mut gz = flate2::read::GzDecoder::new(file);
        let mut buf = Vec::new();
        gz.read_to_end(&mut buf).context("decompress dict.dz")?;
        Ok(buf)
    } else {
        std::fs::read(path).context("read .dict")
    }
}

// ---------------------------------------------------------------------------
// Utility for cleaning definition HTML to plain-ish text for GTK display
// ---------------------------------------------------------------------------

pub fn strip_dict_html(input: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in input.chars() {
        if in_tag {
            if ch == '>' {
                in_tag = false;
            }
            continue;
        }
        if ch == '<' {
            in_tag = true;
            continue;
        }
        out.push(ch);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
