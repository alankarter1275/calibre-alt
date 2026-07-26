//! Open an on-disk EPUB for reading: spine, TOC, chapter HTML.
//! P3 adds highlight CSS + selection chip + dictionary JS.

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
    let core_js = r#"
(function() {
  window.kalam = window.kalam || {};
  window.kalam._lastProgress = -1;
  window.kalam._advanced = false;
  window.kalam._restoreFrac = %RESTORE%;

  function fraction() {
    var se = document.scrollingElement || document.documentElement;
    var max = Math.max(1, se.scrollHeight - se.clientHeight);
    return se.scrollTop / max;
  }
  function setScroll(frac) {
    var se = document.scrollingElement || document.documentElement;
    var max = Math.max(0, se.scrollHeight - se.clientHeight);
    se.scrollTop = max * Math.min(1, Math.max(0, frac || 0));
  }
  function kalamBridge(payload) {
    try {
      var json = typeof payload === 'string' ? payload : JSON.stringify(payload);
      if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.kalam) {
        window.webkit.messageHandlers.kalam.postMessage(json);
      } else {
        // fallback iframe scheme (old P2)
        var i = document.createElement('iframe');
        i.style.display = 'none';
        i.src = 'kalam://' + encodeURIComponent(json);
        document.documentElement.appendChild(i);
        setTimeout(function(){ try{ i.remove(); }catch(e){} }, 30);
      }
    } catch(e) {}
  }
  window.kalamBridge = kalamBridge;

  // ---- Progress reporting (P2) ----
  function pingProgress() {
    var f = fraction();
    if (Math.abs(f - window.kalam._lastProgress) < 0.01) return;
    window.kalam._lastProgress = f;
    kalamBridge({type:'progress', fraction:f});
  }
  function maybeNext() {
    if (window.kalam._advanced) return;
    if (fraction() > 0.90) {
      window.kalam._advanced = true;
      kalamBridge({type:'next'});
    }
  }
  var scrollT = null;
  window.addEventListener('scroll', function() {
    if (scrollT) cancelAnimationFrame(scrollT);
    scrollT = requestAnimationFrame(function(){ pingProgress(); maybeNext(); });
  }, {passive:true});

  function tryRestore() {
    if (window.kalam._restoreFrac > 0) setScroll(window.kalam._restoreFrac);
    window.kalam._restoreFrac = 0;
    window.kalam._advanced = false;
    window.kalam._lastProgress = -1;
    pingProgress();
  }
  if (document.readyState === 'complete') setTimeout(tryRestore, 80);
  else window.addEventListener('load', function(){ setTimeout(tryRestore, 80); });

  // ---- Selection & paths ----
  function nodePath(node) {
    var path = [];
    var cur = node;
    while (cur && cur !== document.body && cur.parentNode) {
      var idx = Array.prototype.indexOf.call(cur.parentNode.childNodes, cur);
      if (idx < 0) break;
      path.unshift(idx);
      cur = cur.parentNode;
      if (!cur) break;
    }
    return path.join('/');
  }
  function nodeFromPath(path) {
    if (path === '' || path === null || path === undefined) return document.body;
    var parts = path.split('/').filter(function(s){return s.length>0;}).map(function(n){return parseInt(n,10);});
    var cur = document.body;
    for (var i=0;i<parts.length;i++) {
      var idx = parts[i];
      if (!cur || !cur.childNodes || idx <0 || idx >= cur.childNodes.length) return null;
      cur = cur.childNodes[idx];
    }
    return cur;
  }
  window.kalamNodePath = nodePath;
  window.kalamNodeFromPath = nodeFromPath;

  function getSelectionData() {
    var sel = window.getSelection();
    if (!sel || sel.rangeCount===0 || sel.isCollapsed) return null;
    var range = sel.getRangeAt(0);
    var text = sel.toString();
    if (!text || !text.trim()) return null;
    // Avoid chip/dict UI
    var anc = range.commonAncestorContainer;
    if (anc && anc.nodeType !== 1) anc = anc.parentElement;
    if (anc && anc.closest && (anc.closest('#kalam-chip') || anc.closest('#kalam-dict-popup') || anc.closest('#kalam-note-pop'))) return null;
    var rect = null;
    try { var r = range.getBoundingClientRect(); if (r) rect = {x:r.left, y:r.top, w:r.width, h:r.height, bottom:r.bottom}; } catch(e){}
    return {
      text: text,
      startPath: nodePath(range.startContainer),
      startOffset: range.startOffset,
      endPath: nodePath(range.endContainer),
      endOffset: range.endOffset,
      rect: rect
    };
  }
  window.kalamGetSelectionData = getSelectionData;

  function wrapRangeByPaths(startPath, startOffset, endPath, endOffset, color, annId) {
    var startNode = nodeFromPath(startPath);
    var endNode = nodeFromPath(endPath);
    if (!startNode || !endNode) { console.log('kalam wrap: nodes not found', startPath, endPath); return false; }
    try {
      var range = document.createRange();
      range.setStart(startNode, startOffset);
      range.setEnd(endNode, endOffset);
      if (range.collapsed) return false;
      // Do not wrap if already inside same annotation
      var existing = range.commonAncestorContainer;
      if (existing && existing.nodeType !== 1) existing = existing.parentElement;
      if (existing && existing.closest && existing.closest('span.kalam-hl[data-annotation-id=\"'+annId+'\"]')) {
        return true;
      }
      var span = document.createElement('span');
      span.className = 'kalam-hl kalam-hl-' + color;
      span.dataset.annotationId = annId;
      span.dataset.color = color;
      try {
        var frag = range.extractContents();
        // avoid empty
        if (!frag || frag.textContent.trim() === '') return false;
        span.appendChild(frag);
        range.insertNode(span);
      } catch(e) {
        try {
          var range2 = document.createRange();
          range2.setStart(nodeFromPath(startPath), startOffset);
          range2.setEnd(nodeFromPath(endPath), endOffset);
          var span2 = document.createElement('span');
          span2.className = 'kalam-hl kalam-hl-' + color;
          span2.dataset.annotationId = annId;
          span2.dataset.color = color;
          range2.surroundContents(span2);
        } catch(e2) {
          console.log('kalam wrap fallback failed', e, e2);
          return false;
        }
      }
      return true;
    } catch(e) {
      console.log('kalam wrapRangeByPaths error', e);
      return false;
    }
  }
  window.kalamWrapRangeByPaths = wrapRangeByPaths;

  window.kalamRemoveHighlight = function(annId) {
    var nodes = document.querySelectorAll('span.kalam-hl[data-annotation-id=\"'+annId+'\"]');
    nodes.forEach(function(n){
      var parent = n.parentNode;
      if (!parent) return;
      while (n.firstChild) parent.insertBefore(n.firstChild, n);
      parent.removeChild(n);
      parent.normalize();
    });
  };

  // ---- UI: selection chip ----
  function ensureChip() {
    var chip = document.getElementById('kalam-chip');
    if (chip) return chip;
    chip = document.createElement('div');
    chip.id = 'kalam-chip';
    chip.style.display = 'none';
    chip.innerHTML = '<button class=\"kalam-chip-btn\" data-color=\"yellow\" title=\"Highlight yellow\" style=\"background:#fef08a\"></button>'
      + '<button class=\"kalam-chip-btn\" data-color=\"green\" title=\"Highlight green\" style=\"background:#bbf7d0\"></button>'
      + '<button class=\"kalam-chip-btn\" data-color=\"blue\" title=\"Highlight blue\" style=\"background:#bfdbfe\"></button>'
      + '<button class=\"kalam-chip-btn\" data-color=\"pink\" title=\"Highlight pink\" style=\"background:#fbcfe8\"></button>'
      + '<button class=\"kalam-chip-btn\" data-color=\"orange\" title=\"Highlight orange\" style=\"background:#fed7aa\"></button>'
      + '<div class=\"kalam-chip-sep\"></div>'
      + '<button class=\"kalam-chip-action\" id=\"kalam-chip-quote\" title=\"Save quote\">❝</button>'
      + '<button class=\"kalam-chip-action\" id=\"kalam-chip-dict\" title=\"Dictionary (D)\">Aa</button>'
      + '<button class=\"kalam-chip-action\" id=\"kalam-chip-copy\" title=\"Copy\">⧉</button>';
    document.body.appendChild(chip);
    chip.querySelectorAll('.kalam-chip-btn').forEach(function(b){
      b.addEventListener('click', function(){
        var color = b.dataset.color;
        kalamHandleHighlight(color);
      });
    });
    var q = document.getElementById('kalam-chip-quote');
    if (q) q.addEventListener('click', function(){ kalamHandleQuote(); });
    var d = document.getElementById('kalam-chip-dict');
    if (d) d.addEventListener('click', function(){ kalamHandleDict(); });
    var c = document.getElementById('kalam-chip-copy');
    if (c) c.addEventListener('click', function(){
      var sel = window.getSelection(); if (sel) { try{ document.execCommand('copy'); }catch(e){} }
      hideChip();
    });
    return chip;
  }
  function showChipAt(rect) {
    var chip = ensureChip();
    var top, left;
    if (rect) {
      top = (window.scrollY + rect.y - 52);
      left = (window.scrollX + rect.x);
      // keep in viewport
      if (left < 12) left = 12;
      if (top < 12) top = window.scrollY + rect.y + rect.h + 8;
    } else {
      top = window.scrollY + 120;
      left = window.scrollX + 80;
    }
    // clamp
    var maxLeft = window.scrollX + window.innerWidth - 260;
    if (left > maxLeft) left = maxLeft;
    chip.style.top = top + 'px';
    chip.style.left = left + 'px';
    chip.style.display = 'flex';
  }
  function hideChip() {
    var chip = document.getElementById('kalam-chip');
    if (chip) chip.style.display = 'none';
  }
  window.kalamHideChip = hideChip;
  window.kalamShowChipAt = showChipAt;

  window.kalamHandleHighlight = function(color) {
    var data = getSelectionData();
    if (!data) return;
    var provisional = 'tmp_'+Date.now()+'_'+Math.random().toString(36).slice(2,7);
    var ok = wrapRangeByPaths(data.startPath, data.startOffset, data.endPath, data.endOffset, color, provisional);
    if (!ok) return;
    hideChip();
    kalamBridge({type:'highlight', color:color, text:data.text, startPath:data.startPath, startOffset:data.startOffset, endPath:data.endPath, endOffset:data.endOffset, tmpId:provisional});
  };
  window.kalamHandleQuote = function() {
    var data = getSelectionData();
    if (!data) return;
    hideChip();
    kalamBridge({type:'quote', text:data.text, startPath:data.startPath, startOffset:data.startOffset, endPath:data.endPath, endOffset:data.endOffset});
  };
  window.kalamHandleDict = function() {
    var data = getSelectionData();
    var word = '';
    var ctx = '';
    var rect = null;
    if (data) {
      word = data.text.trim().split(/\s+/)[0] || '';
      ctx = data.text;
      rect = data.rect;
    } else {
      var sel = window.getSelection();
      if (sel && sel.toString()) {
        word = sel.toString().trim().split(/\s+/)[0];
        ctx = sel.toString();
        try { var r = sel.getRangeAt(0).getBoundingClientRect(); rect = {x:r.left, y:r.top, w:r.width, h:r.height, bottom:r.bottom}; } catch(e){}
      }
    }
    if (!word) return;
    hideChip();
    kalamBridge({type:'dict-lookup', word:word, context:ctx, rect:rect});
  };

  // ---- Dictionary popup ----
  function ensureDictPopup() {
    var p = document.getElementById('kalam-dict-popup');
    if (p) return p;
    p = document.createElement('div');
    p.id = 'kalam-dict-popup';
    p.style.display='none';
    document.body.appendChild(p);
    return p;
  }
  function showDictPopup(word, definition, rect) {
    var p = ensureDictPopup();
    function esc(s){ return (s||'').replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;'); }
    p.innerHTML = '<div class=\"kalam-dict-head\"><span class=\"kalam-dict-word\">'+esc(word)+'</span><button class=\"kalam-dict-close\" onclick=\"window.kalamHideDict()\">✕</button></div>'
      + '<div class=\"kalam-dict-body\">'+esc(definition)+'</div>'
      + '<div class=\"kalam-dict-actions\"><button id=\"kalam-dict-save\" class=\"kalam-dict-save\">Save word</button><button id=\"kalam-dict-copy\" class=\"kalam-dict-copy\">Copy</button></div>';
    p.dataset.word = word;
    p.dataset.definition = definition;
    var top, left;
    if (rect) {
      top = window.scrollY + rect.y + rect.h + 10;
      left = window.scrollX + rect.x;
      if (left < 8) left = 8;
      var maxLeft = window.scrollX + window.innerWidth - 320;
      if (left > maxLeft) left = maxLeft;
      if (top + 180 > window.scrollY + window.innerHeight) {
        top = window.scrollY + rect.y - 200;
      }
    } else {
      top = window.scrollY + 180;
      left = window.scrollX + 40;
    }
    p.style.top = top + 'px';
    p.style.left = left + 'px';
    p.style.display = 'block';
    var save = document.getElementById('kalam-dict-save');
    if (save) save.addEventListener('click', function(){ kalamBridge({type:'save-word', word:p.dataset.word, definition:p.dataset.definition}); hideDict(); });
    var cp = document.getElementById('kalam-dict-copy');
    if (cp) cp.addEventListener('click', function(){ try{ navigator.clipboard.writeText(p.dataset.definition); }catch(e){} hideDict(); });
  }
  function hideDict() {
    var p = document.getElementById('kalam-dict-popup');
    if (p) p.style.display='none';
  }
  window.kalamHideDict = hideDict;
  window.kalamShowDict = function(word, definition, rectJson) {
    var rect = null;
    try { if (rectJson) rect = JSON.parse(rectJson); } catch(e){}
    showDictPopup(word, definition, rect);
  };

  // ---- Highlights injection from Rust ----
  window.kalamInjectHighlights = function(jsonStr) {
    try {
      var arr = JSON.parse(jsonStr);
      arr.forEach(function(a){
        if (document.querySelector('span.kalam-hl[data-annotation-id=\"'+a.id+'\"]')) return;
        wrapRangeByPaths(a.start_path, a.start_offset, a.end_path, a.end_offset, a.color, a.id);
      });
    } catch(e){ console.log('kalam inject highlights failed', e, jsonStr?.slice(0,200)); }
  };
  window.kalamInjectSingleHighlight = function(aJson) {
    try {
      var a = JSON.parse(aJson);
      wrapRangeByPaths(a.start_path, a.start_offset, a.end_path, a.end_offset, a.color, a.id);
    } catch(e){ console.log('single inject failed', e); }
  };

  // ---- Selection listeners ----
  var selTimeout = null;
  document.addEventListener('mouseup', function(e){
    if (e.target.closest && (e.target.closest('#kalam-chip') || e.target.closest('#kalam-dict-popup'))) return;
    clearTimeout(selTimeout);
    selTimeout = setTimeout(function(){
      var data = getSelectionData();
      if (data && data.text && data.text.trim().length>0 && data.text.trim().length < 2000) {
        showChipAt(data.rect);
        kalamBridge({type:'selection', text:data.text});
      } else {
        // do not hide immediately if dict is open
        var dict = document.getElementById('kalam-dict-popup');
        if (!dict || dict.style.display==='none') {
          // keep chip if already visible? hide after delay
          // hideChip();
        }
      }
    }, 160);
  });
  document.addEventListener('mousedown', function(e){
    if (e.target.closest && (e.target.closest('#kalam-chip') || e.target.closest('#kalam-dict-popup'))) return;
    hideChip();
    // don't hide dict on mousedown inside content
  });
  document.addEventListener('keydown', function(e){
    if ((e.key === 'd' || e.key === 'D') && !e.ctrlKey && !e.metaKey && !e.altKey) {
      // only if selection exists or chip visible
      var data = getSelectionData();
      if (data || document.getElementById('kalam-chip')?.style.display==='flex') {
        e.preventDefault();
        window.kalamHandleDict();
      } else {
        // try word under caret? fallback to dictionary shortcut via bridge
        kalamBridge({type:'dict-shortcut'});
      }
    }
    if (e.key === 'Escape') {
      hideChip();
      hideDict();
    }
  });

  // ---- Existing progress restore already handled above ----
})();
"#;
    let js = core_js.replace("%RESTORE%", &restore.to_string());

    let inject = format!(
        r#"<base href="{base}">
<style id="kalam-reading-css">{css}</style>
<script>{js}</script>"#,
        base = base_url,
        css = reading_css,
        js = js,
    );

    // Put our skin at the *end* of the document so it wins over author CSS
    // (equal !important → later rule wins). Also keep a head copy for early paint.
    let lower = raw_html.to_ascii_lowercase();
    let head_inject = inject.clone();
    let mut out = if let Some(pos) = lower.find("<head>") {
        let insert_at = pos + 6;
        let mut s = String::with_capacity(raw_html.len() + inject.len() * 2);
        s.push_str(&raw_html[..insert_at]);
        s.push_str(&head_inject);
        s.push_str(&raw_html[insert_at..]);
        s
    } else if let Some(pos) = lower.find("<head ") {
        if let Some(gt) = raw_html[pos..].find('>') {
            let insert_at = pos + gt + 1;
            let mut s = String::with_capacity(raw_html.len() + inject.len() * 2);
            s.push_str(&raw_html[..insert_at]);
            s.push_str(&head_inject);
            s.push_str(&raw_html[insert_at..]);
            s
        } else {
            format!("<!DOCTYPE html><html><head>{head_inject}</head><body>{raw_html}</body></html>")
        }
    } else {
        format!("<!DOCTYPE html><html><head>{head_inject}</head><body>{raw_html}</body></html>")
    };

    // Append a second copy before </body> for cascade victory.
    let lower2 = out.to_ascii_lowercase();
    if let Some(pos) = lower2.rfind("</body>") {
        out.insert_str(pos, &inject);
    } else {
        out.push_str(&inject);
    }
    out
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
///
/// Many commercial EPUBs wrap almost every paragraph in `<a>` with blue
/// link styling. We nuke link chrome entirely for reading. P3 adds highlight
/// and chip styling.
pub fn reading_css(theme: ReadingTheme, font_px: u32, line_height: f32, margin_em: f32) -> String {
    let (bg, fg) = match theme {
        ReadingTheme::Light => ("#faf8f5", "#1c1917"),
        ReadingTheme::Sepia => ("#f4ecd8", "#3e3226"),
        ReadingTheme::Dark => ("#1a1b1e", "#e7e5e4"),
    };
    // Injected at the *end* of <body> so it wins over author stylesheets.
    format!(
        r#"
/* kalam reading skin — highest priority overrides */
html {{
  background: {bg} !important;
}}
html, body, body * {{
  color: {fg} !important;
  -webkit-text-fill-color: {fg} !important;
  text-decoration: none !important;
  text-decoration-line: none !important;
  text-decoration-color: transparent !important;
  border-bottom: none !important;
}}
html, body {{
  background: {bg} !important;
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
p, div, span, li, td, th, blockquote, h1, h2, h3, h4, h5, h6,
section, article, main, font, a, a:link, a:visited, a:hover, a:active {{
  color: {fg} !important;
  -webkit-text-fill-color: {fg} !important;
  background: transparent !important;
  background-color: transparent !important;
  line-height: {lh} !important;
  text-decoration: none !important;
  text-decoration-line: none !important;
  border-bottom-width: 0 !important;
  border-bottom-style: none !important;
}}
h1, h2, h3, h4, h5, h6 {{
  font-weight: 650 !important;
  line-height: 1.25 !important;
  margin-top: 1.4em !important;
}}
a[href], a[href]:link, a[href]:visited, a[href]:hover, a[href]:active {{
  color: {fg} !important;
  -webkit-text-fill-color: {fg} !important;
  text-decoration: none !important;
  cursor: text !important;
}}
img, svg {{
  max-width: 100% !important;
  height: auto !important;
  -webkit-text-fill-color: initial !important;
}}
/* selection tint pink-ish (Apple Books like) */
::selection {{
  background: rgba(244, 114, 182, 0.38) !important;
  color: {fg} !important;
  -webkit-text-fill-color: {fg} !important;
}}

/* ── P3 highlights ── */
.kalam-hl {{
  border-radius: 3px !important;
  padding: 0.08em 0.12em !important;
  margin: 0 -0.08em !important;
  cursor: pointer !important;
  box-decoration-break: clone !important;
  -webkit-box-decoration-break: clone !important;
}}
.kalam-hl-yellow {{
  background: rgba(254, 240, 138, 0.62) !important;
  background-color: rgba(254, 240, 138, 0.62) !important;
}}
.kalam-hl-green {{
  background: rgba(187, 247, 208, 0.62) !important;
  background-color: rgba(187, 247, 208, 0.62) !important;
}}
.kalam-hl-blue {{
  background: rgba(191, 219, 254, 0.62) !important;
  background-color: rgba(191, 219, 254, 0.62) !important;
}}
.kalam-hl-pink {{
  background: rgba(251, 207, 232, 0.70) !important;
  background-color: rgba(251, 207, 232, 0.70) !important;
}}
.kalam-hl-orange {{
  background: rgba(254, 215, 170, 0.62) !important;
  background-color: rgba(254, 215, 170, 0.62) !important;
}}
.kalam-hl:hover {{
  filter: brightness(0.98) !important;
}}

/* ── selection chip (inside WebView) ── */
#kalam-chip {{
  position: absolute !important;
  z-index: 999999 !important;
  background: rgba(28, 25, 23, 0.92) !important;
  border: 1px solid rgba(255, 255, 255, 0.14) !important;
  border-radius: 999px !important;
  padding: 6px 8px !important;
  display: none;
  flex-direction: row !important;
  align-items: center !important;
  gap: 6px !important;
  box-shadow: 0 10px 28px rgba(0,0,0,0.45) !important;
  font-family: -apple-system, BlinkMacSystemFont, "Inter", sans-serif !important;
  backdrop-filter: blur(8px) !important;
}}
.kalam-chip-btn {{
  width: 22px !important;
  height: 22px !important;
  border-radius: 999px !important;
  border: 1.5px solid rgba(255,255,255,0.85) !important;
  cursor: pointer !important;
  padding: 0 !important;
  margin: 0 !important;
}}
.kalam-chip-btn:hover {{
  transform: scale(1.12) !important;
}}
.kalam-chip-sep {{
  width: 1px !important;
  height: 18px !important;
  background: rgba(255,255,255,0.15) !important;
  margin: 0 4px !important;
}}
.kalam-chip-action {{
  min-width: 22px !important;
  height: 22px !important;
  border-radius: 999px !important;
  border: none !important;
  background: rgba(255,255,255,0.10) !important;
  color: #f5f5f4 !important;
  font-size: 12px !important;
  font-weight: 700 !important;
  cursor: pointer !important;
  padding: 0 6px !important;
}}
.kalam-chip-action:hover {{
  background: rgba(255,255,255,0.20) !important;
}}

/* ── dictionary popup inside WebView ── */
#kalam-dict-popup {{
  position: absolute !important;
  z-index: 999998 !important;
  width: 300px !important;
  max-width: 84vw !important;
  background: #1c1917 !important;
  color: #fafaf9 !important;
  border: 1px solid rgba(255,255,255,0.12) !important;
  border-radius: 14px !important;
  box-shadow: 0 18px 48px rgba(0,0,0,0.45) !important;
  padding: 0 !important;
  overflow: hidden !important;
  font-family: -apple-system, BlinkMacSystemFont, "Inter", sans-serif !important;
}}
.kalam-dict-head {{
  display: flex !important;
  justify-content: space-between !important;
  align-items: center !important;
  padding: 10px 12px 6px 12px !important;
  font-weight: 700 !important;
  font-size: 0.92rem !important;
  background: rgba(255,255,255,0.04) !important;
}}
.kalam-dict-word {{
  color: #fafaf9 !important;
}}
.kalam-dict-close {{
  background: transparent !important;
  border: none !important;
  color: rgba(250,250,249,0.6) !important;
  cursor: pointer !important;
  font-size: 0.9rem !important;
}}
.kalam-dict-body {{
  padding: 10px 12px !important;
  font-size: 0.86rem !important;
  line-height: 1.45 !important;
  color: rgba(231,229,228,0.92) !important;
  max-height: 180px !important;
  overflow-y: auto !important;
  white-space: pre-wrap !important;
}}
.kalam-dict-actions {{
  display: flex !important;
  gap: 8px !important;
  padding: 8px 12px 10px 12px !important;
  border-top: 1px solid rgba(255,255,255,0.08) !important;
}}
.kalam-dict-save, .kalam-dict-copy {{
  border: none !important;
  border-radius: 999px !important;
  padding: 6px 12px !important;
  font-size: 0.78rem !important;
  font-weight: 600 !important;
  cursor: pointer !important;
  background: rgba(255,255,255,0.12) !important;
  color: #fafaf9 !important;
}}
.kalam-dict-save:hover, .kalam-dict-copy:hover {{
  background: rgba(255,255,255,0.20) !important;
}}

/* Final override — ensure Kalam UI inside WebView stays visible in light/sepia/dark regardless of aggressive resets */
#kalam-chip, #kalam-dict-popup {{
  color-scheme: dark !important;
}}
#kalam-chip, #kalam-chip *, #kalam-dict-popup, #kalam-dict-popup * {{
  color: #f5f5f4 !important;
  -webkit-text-fill-color: #f5f5f4 !important;
}}
#kalam-chip {{
  background: rgba(28,25,23,0.92) !important;
  background-color: rgba(28,25,23,0.92) !important;
}}
#kalam-dict-popup {{
  background: #1c1917 !important;
  background-color: #1c1917 !important;
}}
"#,
        bg = bg,
        fg = fg,
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
