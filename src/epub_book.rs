//! Open an on-disk EPUB for reading: spine, TOC, chapter HTML.

use anyhow::{anyhow, Context, Result};
use roxmltree::Document;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

#[derive(Debug, Clone)]
pub struct TocEntry {
    pub label: String,
    #[allow(dead_code)]
    pub href: String,
    /// Spine index if this href maps to a spine item.
    pub spine_index: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct SpineItem {
    #[allow(dead_code)]
    pub id: String,
    pub href: String,
    /// Absolute path on disk after extract (under cache dir).
    pub path: PathBuf,
    pub title: String,
}

#[derive(Debug)]
pub struct OpenBook {
    #[allow(dead_code)]
    pub title: String,
    pub extract_dir: PathBuf,
    pub spine: Vec<SpineItem>,
    pub toc: Vec<TocEntry>,
    #[allow(dead_code)]
    opf_dir: String,
}

impl OpenBook {
    /// Empty book used when open fails (reader still mounts chrome).
    pub fn empty_placeholder() -> Self {
        Self {
            title: String::new(),
            extract_dir: PathBuf::new(),
            spine: Vec::new(),
            toc: Vec::new(),
            opf_dir: String::new(),
        }
    }

    /// Unzip EPUB into `cache_dir/uuid/` (or reuse if present) and parse spine/TOC.
    pub fn open(epub_path: &Path, cache_dir: &Path) -> Result<Self> {
        fs::create_dir_all(cache_dir)?;
        // Marker so we know extract finished.
        let marker = cache_dir.join(".kalam_extracted");
        if !marker.exists() {
            extract_zip(epub_path, cache_dir)?;
            File::create(&marker)?;
        }

        let container = read_file_string(&cache_dir.join("META-INF/container.xml"))
            .or_else(|_| read_file_string(&cache_dir.join("meta-inf/container.xml")))
            .context("container.xml")?;
        let opf_rel = find_opf_path(&container)?;
        let opf_path = cache_dir.join(&opf_rel);
        let opf_xml = read_file_string(&opf_path)?;
        let opf_dir = parent_zip_path(&opf_rel);

        let (title, manifest, spine_ids) = parse_opf_spine(&opf_xml)?;
        let mut spine = Vec::new();
        for (i, id) in spine_ids.iter().enumerate() {
            let href = manifest
                .iter()
                .find(|(mid, _, _)| mid == id)
                .map(|(_, h, _)| h.clone())
                .ok_or_else(|| anyhow!("spine id {id} missing from manifest"))?;
            let rel = join_zip_path(&opf_dir, &href);
            let path = cache_dir.join(&rel);
            let chap_title = format!("Chapter {}", i + 1);
            spine.push(SpineItem {
                id: id.clone(),
                href: rel,
                path,
                title: chap_title,
            });
        }

        let toc = parse_nav_or_ncx(cache_dir, &opf_dir, &opf_xml, &spine)?;
        // Improve spine titles from TOC when possible.
        for entry in &toc {
            if let Some(idx) = entry.spine_index {
                if let Some(item) = spine.get_mut(idx) {
                    if !entry.label.is_empty() {
                        item.title = entry.label.clone();
                    }
                }
            }
        }

        if spine.is_empty() {
            return Err(anyhow!("EPUB has empty spine"));
        }

        Ok(Self {
            title,
            extract_dir: cache_dir.to_path_buf(),
            spine,
            toc,
            opf_dir,
        })
    }

    pub fn chapter_count(&self) -> usize {
        self.spine.len()
    }

    /// HTML document for a spine chapter, with reading CSS injected and base href set.
    pub fn chapter_html(
        &self,
        index: usize,
        reading_css: &str,
        restore_fraction: f64,
    ) -> Result<String> {
        let item = self
            .spine
            .get(index)
            .ok_or_else(|| anyhow!("chapter index {index} out of range"))?;
        let raw = fs::read_to_string(&item.path)
            .with_context(|| format!("read chapter {}", item.path.display()))?;
        let base = path_to_file_url(item.path.parent().unwrap_or_else(|| Path::new(".")));
        Ok(inject_reading_shell(
            &raw,
            &base,
            reading_css,
            restore_fraction,
        ))
    }
}

fn extract_zip(epub: &Path, dest: &Path) -> Result<()> {
    let file = File::open(epub)?;
    let mut archive = ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().replace('\\', "/");
        if name.ends_with('/') {
            fs::create_dir_all(dest.join(&name))?;
            continue;
        }
        let out_path = dest.join(&name);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out)?;
    }
    Ok(())
}

fn find_opf_path(container_xml: &str) -> Result<String> {
    let doc = Document::parse(container_xml)?;
    for node in doc.descendants() {
        if node.tag_name().name() == "rootfile" {
            if let Some(full) = node.attribute("full-path") {
                return Ok(full.to_string());
            }
        }
    }
    Err(anyhow!("no rootfile in container.xml"))
}

/// (id, href, media-type)
type ManifestItem = (String, String, Option<String>);

fn parse_opf_spine(opf: &str) -> Result<(String, Vec<ManifestItem>, Vec<String>)> {
    let doc = Document::parse(opf)?;
    let mut title = String::from("Untitled");
    let mut manifest = Vec::new();
    let mut spine = Vec::new();
    let mut title_set = false;

    for node in doc.descendants() {
        match node.tag_name().name() {
            "title" if !title_set => {
                let t = node.text().unwrap_or("").trim();
                if !t.is_empty() {
                    title = t.to_string();
                    title_set = true;
                }
            }
            "item" => {
                let id = node.attribute("id").unwrap_or("").to_string();
                let href = node.attribute("href").unwrap_or("").to_string();
                let mt = node.attribute("media-type").map(|s| s.to_string());
                if !id.is_empty() && !href.is_empty() {
                    manifest.push((id, href, mt));
                }
            }
            "itemref" => {
                if let Some(idref) = node.attribute("idref") {
                    spine.push(idref.to_string());
                }
            }
            _ => {}
        }
    }
    Ok((title, manifest, spine))
}

fn parse_nav_or_ncx(
    root: &Path,
    opf_dir: &str,
    opf_xml: &str,
    spine: &[SpineItem],
) -> Result<Vec<TocEntry>> {
    // Try EPUB3 nav
    if let Ok(nav_href) = find_nav_href(opf_xml) {
        let rel = join_zip_path(opf_dir, &nav_href);
        let path = root.join(&rel);
        if let Ok(html) = fs::read_to_string(&path) {
            let toc = parse_nav_html(&html, spine);
            if !toc.is_empty() {
                return Ok(toc);
            }
        }
    }
    // EPUB2 NCX
    if let Ok(ncx_href) = find_ncx_href(opf_xml) {
        let rel = join_zip_path(opf_dir, &ncx_href);
        let path = root.join(&rel);
        if let Ok(xml) = fs::read_to_string(&path) {
            let toc = parse_ncx(&xml, spine);
            if !toc.is_empty() {
                return Ok(toc);
            }
        }
    }
    // Fallback: one TOC entry per spine item
    Ok(spine
        .iter()
        .enumerate()
        .map(|(i, s)| TocEntry {
            label: s.title.clone(),
            href: s.href.clone(),
            spine_index: Some(i),
        })
        .collect())
}

fn find_nav_href(opf: &str) -> Result<String> {
    let doc = Document::parse(opf)?;
    for node in doc.descendants() {
        if node.tag_name().name() == "item" {
            let props = node.attribute("properties").unwrap_or("");
            if props.split_whitespace().any(|p| p == "nav") {
                if let Some(href) = node.attribute("href") {
                    return Ok(href.to_string());
                }
            }
        }
    }
    Err(anyhow!("no nav document"))
}

fn find_ncx_href(opf: &str) -> Result<String> {
    let doc = Document::parse(opf)?;
    for node in doc.descendants() {
        if node.tag_name().name() == "item" {
            let mt = node.attribute("media-type").unwrap_or("");
            if mt == "application/x-dtbncx+xml" {
                if let Some(href) = node.attribute("href") {
                    return Ok(href.to_string());
                }
            }
        }
    }
    Err(anyhow!("no ncx"))
}

fn parse_nav_html(html: &str, spine: &[SpineItem]) -> Vec<TocEntry> {
    // Lightweight: find <a href="...">label</a> inside nav
    let mut toc = Vec::new();
    let lower = html.to_ascii_lowercase();
    let mut search = html;
    // Prefer <nav epub:type="toc">
    if let Some(start) = lower.find("<nav") {
        search = &html[start..];
    }
    let bytes = search.as_bytes();
    let mut i = 0;
    while i + 2 < bytes.len() {
        // find href=
        if search[i..].to_ascii_lowercase().starts_with("href=") {
            let rest = &search[i + 5..];
            let quote = rest.chars().next().unwrap_or('"');
            if quote == '"' || quote == '\'' {
                if let Some(end) = rest[1..].find(quote) {
                    let href = &rest[1..1 + end];
                    // find > after tag
                    if let Some(gt) = search[i..].find('>') {
                        let after = &search[i + gt + 1..];
                        if let Some(close) = after.to_ascii_lowercase().find("</a>") {
                            let label = strip_tags(&after[..close]).trim().to_string();
                            if !label.is_empty() && !href.starts_with('#') {
                                let spine_index = spine_index_for(spine, href);
                                toc.push(TocEntry {
                                    label,
                                    href: href.to_string(),
                                    spine_index,
                                });
                            }
                            i += gt + close + 4;
                            continue;
                        }
                    }
                }
            }
        }
        i += 1;
    }
    toc
}

fn parse_ncx(xml: &str, spine: &[SpineItem]) -> Vec<TocEntry> {
    let mut toc = Vec::new();
    let Ok(doc) = Document::parse(xml) else {
        return toc;
    };
    for node in doc.descendants() {
        if node.tag_name().name() != "navPoint" {
            continue;
        }
        let mut label = String::new();
        let mut href = String::new();
        for child in node.descendants() {
            match child.tag_name().name() {
                "text" if label.is_empty() => {
                    label = child.text().unwrap_or("").trim().to_string();
                }
                "content" if href.is_empty() => {
                    href = child.attribute("src").unwrap_or("").to_string();
                }
                _ => {}
            }
        }
        if !label.is_empty() && !href.is_empty() {
            let spine_index = spine_index_for(spine, &href);
            toc.push(TocEntry {
                label,
                href,
                spine_index,
            });
        }
    }
    toc
}

fn spine_index_for(spine: &[SpineItem], href: &str) -> Option<usize> {
    let clean = href.split('#').next().unwrap_or(href);
    let clean = clean.trim_start_matches("./");
    spine.iter().position(|s| {
        s.href == clean
            || s.href.ends_with(clean)
            || clean.ends_with(&s.href)
            || Path::new(&s.href).file_name() == Path::new(clean).file_name()
    })
}

fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in s.chars() {
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
    out
}

fn inject_reading_shell(
    raw_html: &str,
    base_url: &str,
    reading_css: &str,
    restore_fraction: f64,
) -> String {
    let restore = restore_fraction.clamp(0.0, 1.0);
    let inject = format!(
        r#"<base href="{base}">
<style id="kalam-reading-css">{css}</style>
<script>
(function() {{
  var lastSent = -1;
  var advanced = false;
  var restoreFrac = {restore};
  function fraction() {{
    var se = document.scrollingElement || document.documentElement;
    var max = Math.max(1, se.scrollHeight - se.clientHeight);
    return se.scrollTop / max;
  }}
  function setScroll(frac) {{
    var se = document.scrollingElement || document.documentElement;
    var max = Math.max(0, se.scrollHeight - se.clientHeight);
    se.scrollTop = max * Math.min(1, Math.max(0, frac || 0));
  }}
  function bridge(path) {{
    try {{
      var i = document.createElement('iframe');
      i.style.display = 'none';
      i.src = 'kalam://' + path;
      document.documentElement.appendChild(i);
      setTimeout(function() {{ try {{ i.remove(); }} catch(e) {{}} }}, 0);
    }} catch (e) {{}}
  }}
  function pingProgress() {{
    var f = fraction();
    if (Math.abs(f - lastSent) < 0.01) return;
    lastSent = f;
    bridge('progress/' + f.toFixed(4));
  }}
  function maybeNext() {{
    if (advanced) return;
    if (fraction() > 0.90) {{
      advanced = true;
      bridge('next');
    }}
  }}
  var t = null;
  window.addEventListener('scroll', function() {{
    if (t) cancelAnimationFrame(t);
    t = requestAnimationFrame(function() {{
      pingProgress();
      maybeNext();
    }});
  }}, {{ passive: true }});
  function tryRestore() {{
    if (restoreFrac > 0) setScroll(restoreFrac);
    restoreFrac = 0;
    advanced = false;
    lastSent = -1;
    pingProgress();
  }}
  if (document.readyState === 'complete') setTimeout(tryRestore, 50);
  else window.addEventListener('load', function() {{ setTimeout(tryRestore, 50); }});
}})();
</script>"#,
        base = base_url,
        css = reading_css,
        restore = restore,
    );

    // Insert after <head> if present, else prepend.
    let lower = raw_html.to_ascii_lowercase();
    if let Some(pos) = lower.find("<head>") {
        let insert_at = pos + 6;
        let mut s = String::with_capacity(raw_html.len() + inject.len());
        s.push_str(&raw_html[..insert_at]);
        s.push_str(&inject);
        s.push_str(&raw_html[insert_at..]);
        s
    } else if let Some(pos) = lower.find("<head ") {
        if let Some(gt) = raw_html[pos..].find('>') {
            let insert_at = pos + gt + 1;
            let mut s = String::with_capacity(raw_html.len() + inject.len());
            s.push_str(&raw_html[..insert_at]);
            s.push_str(&inject);
            s.push_str(&raw_html[insert_at..]);
            s
        } else {
            format!("<!DOCTYPE html><html><head>{inject}</head><body>{raw_html}</body></html>")
        }
    } else {
        format!("<!DOCTYPE html><html><head>{inject}</head><body>{raw_html}</body></html>")
    }
}

fn path_to_file_url(path: &Path) -> String {
    let abs = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let s = abs.to_string_lossy();
    // file:///path/
    if s.starts_with('/') {
        format!("file://{}/", s.trim_end_matches('/'))
    } else {
        format!("file:///{}/", s.replace('\\', "/").trim_end_matches('/'))
    }
}

fn read_file_string(path: &Path) -> Result<String> {
    let mut f = File::open(path)?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    Ok(s)
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

/// Default reading stylesheet — book-like, not webpage-like.
/// Body text is never forced blue; links are subtle.
pub fn reading_css(theme: ReadingTheme, font_px: u32, line_height: f32, margin_em: f32) -> String {
    let (bg, fg, muted, link) = match theme {
        ReadingTheme::Light => ("#faf8f5", "#1c1917", "#57534e", "#44403c"),
        ReadingTheme::Sepia => ("#f4ecd8", "#3e3226", "#6b5a48", "#5c4a3a"),
        ReadingTheme::Dark => ("#1a1b1e", "#e7e5e4", "#a8a29e", "#d6d3d1"),
    };
    format!(
        r#"
html {{
  background: {bg} !important;
}}
html, body {{
  background: {bg} !important;
  color: {fg} !important;
  font-size: {font_px}px !important;
  line-height: {lh} !important;
  margin: 0 !important;
  padding: 0 !important;
}}
body {{
  max-width: 38rem;
  margin-left: auto !important;
  margin-right: auto !important;
  padding: {margin}em {margin}em 6em {margin}em !important;
  font-family: "Iowan Old Style", "Palatino Linotype", Palatino, "Book Antiqua",
    "Literata", Georgia, "Times New Roman", serif !important;
  -webkit-font-smoothing: antialiased;
}}
/* Kill common EPUB blue / gray overrides on body copy */
p, div, span, li, td, th, blockquote, h1, h2, h3, h4, h5, h6,
section, article, main, font {{
  color: inherit !important;
  line-height: {lh} !important;
  background: transparent !important;
}}
h1, h2, h3, h4, h5, h6 {{
  color: {fg} !important;
  font-weight: 650 !important;
  line-height: 1.25 !important;
  margin-top: 1.4em !important;
}}
/* Links: bookish, not browser-blue walls of text */
a, a:link, a:visited {{
  color: {fg} !important;
  text-decoration: underline !important;
  text-decoration-color: {muted} !important;
  text-underline-offset: 0.15em !important;
}}
a:hover {{
  color: {fg} !important;
  text-decoration-color: {fg} !important;
}}
img, svg {{
  max-width: 100% !important;
  height: auto !important;
}}
/* Selection preview (P3 will use proper highlights) */
::selection {{
  background: rgba(244, 114, 182, 0.35);
  color: inherit;
}}
"#,
        bg = bg,
        fg = fg,
        muted = muted,
        font_px = font_px,
        lh = line_height,
        margin = margin_em,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingTheme {
    Light,
    Sepia,
    Dark,
}
