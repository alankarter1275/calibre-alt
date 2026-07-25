//! Minimal EPUB import: OPF metadata + cover extraction.

use crate::db::{self, Catalog};
use crate::models::BookFormat;
use crate::paths::{book_dir, ensure_data_dirs};
use anyhow::{anyhow, Context, Result};
use roxmltree::Document;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use uuid::Uuid;
use zip::ZipArchive;

#[derive(Debug)]
pub struct ImportResult {
    pub book_id: i64,
    pub title: String,
    pub duplicate: bool,
}

#[derive(Debug, Default)]
struct OpfMeta {
    title: Option<String>,
    authors: Vec<String>,
    description: Option<String>,
    series: Option<String>,
    subjects: Vec<String>,
    cover_href: Option<String>,
    cover_id: Option<String>,
    /// manifest id -> href
    manifest: Vec<(String, String, Option<String>)>, // id, href, media-type
}

/// Import an EPUB path into the catalog. Copies into library storage.
pub fn import_epub(catalog: &Catalog, source: &Path) -> Result<ImportResult> {
    ensure_data_dirs()?;
    if !source.is_file() {
        return Err(anyhow!("not a file: {}", source.display()));
    }
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext != "epub" {
        return Err(anyhow!("only .epub is supported in P1 (got .{ext})"));
    }

    let hash = db::hash_file(source)?;
    if let Some(existing) = catalog.find_by_hash(&hash)? {
        let title = catalog
            .get_book(existing)?
            .map(|b| b.title)
            .unwrap_or_else(|| "Existing book".into());
        return Ok(ImportResult {
            book_id: existing,
            title,
            duplicate: true,
        });
    }

    let meta = parse_epub_meta(source)?;
    let title = meta
        .title
        .as_ref()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .unwrap_or_else(|| {
            source
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Untitled".into())
        });
    let authors = if meta.authors.is_empty() {
        "Unknown".to_string()
    } else {
        meta.authors.join(", ")
    };
    let description = meta.description.clone().unwrap_or_default();
    let series = meta.series.clone();
    let tags = meta.subjects.clone();

    let uuid = Uuid::new_v4().to_string();
    let dest_dir = book_dir(&uuid);
    fs::create_dir_all(&dest_dir)?;

    let file_name = "book.epub";
    let dest_epub = dest_dir.join(file_name);
    fs::copy(source, &dest_epub)
        .with_context(|| format!("copy {} → {}", source.display(), dest_epub.display()))?;

    let cover_name = extract_cover(source, &meta, &dest_dir)?;

    let id = catalog.insert_book(
        &uuid,
        &title,
        &authors,
        series.as_deref(),
        &description,
        BookFormat::Epub,
        file_name,
        &hash,
        cover_name.as_deref(),
        &tags,
    )?;

    Ok(ImportResult {
        book_id: id,
        title,
        duplicate: false,
    })
}

fn parse_epub_meta(path: &Path) -> Result<OpfMeta> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let opf_path = find_opf_path(&mut archive)?;
    let opf_xml = read_zip_string(&mut archive, &opf_path)?;
    let opf_dir = parent_zip_path(&opf_path);
    let mut meta = parse_opf(&opf_xml)?;
    // Resolve cover href relative to OPF directory.
    if let Some(href) = meta.cover_href.clone() {
        meta.cover_href = Some(join_zip_path(&opf_dir, &href));
    } else if let Some(id) = &meta.cover_id {
        if let Some((_, href, _)) = meta.manifest.iter().find(|(mid, _, _)| mid == id) {
            meta.cover_href = Some(join_zip_path(&opf_dir, href));
        }
    } else {
        // Heuristic: first image manifest item with "cover" in id/href.
        for (id, href, mt) in &meta.manifest {
            let is_image = mt
                .as_deref()
                .map(|m| m.starts_with("image/"))
                .unwrap_or_else(|| {
                    let h = href.to_ascii_lowercase();
                    h.ends_with(".jpg")
                        || h.ends_with(".jpeg")
                        || h.ends_with(".png")
                        || h.ends_with(".webp")
                        || h.ends_with(".gif")
                });
            if is_image
                && (id.to_ascii_lowercase().contains("cover")
                    || href.to_ascii_lowercase().contains("cover"))
            {
                meta.cover_href = Some(join_zip_path(&opf_dir, href));
                break;
            }
        }
    }
    Ok(meta)
}

fn find_opf_path<R: Read + std::io::Seek>(archive: &mut ZipArchive<R>) -> Result<String> {
    // container.xml
    let container = read_zip_string(archive, "META-INF/container.xml")
        .or_else(|_| read_zip_string(archive, "meta-inf/container.xml"))?;
    let doc = Document::parse(&container).context("parse container.xml")?;
    for node in doc.descendants() {
        if node.tag_name().name() == "rootfile" {
            if let Some(full) = node.attribute("full-path") {
                return Ok(full.to_string());
            }
        }
    }
    Err(anyhow!("no rootfile in container.xml"))
}

fn parse_opf(xml: &str) -> Result<OpfMeta> {
    let doc = Document::parse(xml).context("parse OPF")?;
    let mut meta = OpfMeta::default();

    for node in doc.descendants() {
        let name = node.tag_name().name();
        match name {
            "title" if node.parent().map(|p| p.tag_name().name()) == Some("metadata") => {
                if meta.title.is_none() {
                    let t = node.text().unwrap_or("").trim();
                    if !t.is_empty() {
                        meta.title = Some(t.to_string());
                    }
                }
            }
            "creator" => {
                let t = node.text().unwrap_or("").trim();
                if !t.is_empty() {
                    meta.authors.push(t.to_string());
                }
            }
            "description" => {
                if meta.description.is_none() {
                    let t = node.text().unwrap_or("").trim();
                    if !t.is_empty() {
                        meta.description = Some(collapse_ws(t));
                    }
                }
            }
            "subject" => {
                let t = node.text().unwrap_or("").trim();
                if !t.is_empty() {
                    meta.subjects.push(t.to_string());
                }
            }
            "meta" => {
                let name_attr = node.attribute("name").unwrap_or("");
                let prop = node.attribute("property").unwrap_or("");
                let content = node
                    .attribute("content")
                    .map(|s| s.to_string())
                    .or_else(|| node.text().map(|t| t.trim().to_string()))
                    .unwrap_or_default();
                if name_attr.eq_ignore_ascii_case("cover") && meta.cover_id.is_none() {
                    meta.cover_id = Some(content);
                } else if prop == "belongs-to-collection" && meta.series.is_none() {
                    if !content.is_empty() {
                        meta.series = Some(content);
                    }
                } else if name_attr.eq_ignore_ascii_case("calibre:series") && meta.series.is_none()
                {
                    if !content.is_empty() {
                        meta.series = Some(content);
                    }
                }
            }
            "item" => {
                let id = node.attribute("id").unwrap_or("").to_string();
                let href = node.attribute("href").unwrap_or("").to_string();
                let mt = node.attribute("media-type").map(|s| s.to_string());
                let props = node.attribute("properties").unwrap_or("");
                if !id.is_empty() && !href.is_empty() {
                    if props.split_whitespace().any(|p| p == "cover-image") {
                        meta.cover_href = Some(href.clone());
                    }
                    meta.manifest.push((id, href, mt));
                }
            }
            _ => {}
        }
    }
    Ok(meta)
}

fn extract_cover(source: &Path, meta: &OpfMeta, dest_dir: &Path) -> Result<Option<String>> {
    let Some(href) = &meta.cover_href else {
        return Ok(None);
    };
    let file = File::open(source)?;
    let mut archive = ZipArchive::new(file)?;
    let href_norm = href.trim_start_matches("./");
    // Try a few path variants (zip entries vary).
    let candidates = [
        href_norm.to_string(),
        href.replace('\\', "/"),
        percent_decode(href_norm),
    ];
    for cand in &candidates {
        if let Ok(bytes) = read_zip_bytes(&mut archive, cand) {
            let ext = Path::new(cand)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("jpg")
                .to_ascii_lowercase();
            let ext = match ext.as_str() {
                "jpeg" | "jpg" | "png" | "gif" | "webp" | "svg" => ext,
                _ => "img".into(),
            };
            let name = format!("cover.{ext}");
            let mut out = File::create(dest_dir.join(&name))?;
            out.write_all(&bytes)?;
            return Ok(Some(name));
        }
    }
    Ok(None)
}

fn read_zip_string<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<String> {
    let bytes = read_zip_bytes(archive, name)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn read_zip_bytes<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<Vec<u8>> {
    // Case-insensitive search fallback.
    let idx = find_zip_index(archive, name)?;
    let mut file = archive.by_index(idx)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

fn find_zip_index<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<usize> {
    let target = name.replace('\\', "/");
    let target_l = target.to_ascii_lowercase();
    for i in 0..archive.len() {
        let f = archive.by_index(i)?;
        let n = f.name().replace('\\', "/");
        if n == target || n.trim_start_matches("./") == target.trim_start_matches("./") {
            return Ok(i);
        }
        if n.to_ascii_lowercase() == target_l
            || n.to_ascii_lowercase().trim_start_matches("./") == target_l.trim_start_matches("./")
        {
            return Ok(i);
        }
    }
    Err(anyhow!("zip entry not found: {name}"))
}

fn parent_zip_path(path: &str) -> String {
    let p = path.replace('\\', "/");
    match p.rfind('/') {
        Some(i) => p[..i].to_string(),
        None => String::new(),
    }
}

fn join_zip_path(dir: &str, href: &str) -> String {
    let href = href.trim_start_matches("./").replace('\\', "/");
    if href.starts_with('/') {
        return href.trim_start_matches('/').to_string();
    }
    if dir.is_empty() {
        return href;
    }
    // Resolve .. segments lightly.
    let mut parts: Vec<&str> = dir.split('/').filter(|s| !s.is_empty()).collect();
    for seg in href.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    parts.join("/")
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(a), Some(b)) = (from_hex(bytes[i + 1]), from_hex(bytes[i + 2])) {
                out.push((a << 4) | b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn from_hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
