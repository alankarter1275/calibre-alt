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
// The shell is injected in both head and body so its CSS wins, but the
// JavaScript must only install one set of listeners and one selection layer.
if (!window.kalamReaderShellLoaded) {
  window.kalamReaderShellLoaded = true;
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
  var lastScrollTop = 0;
  var lastUiZone = 'both';
  window.addEventListener('scroll', function() {
    if (scrollT) cancelAnimationFrame(scrollT);
    scrollT = requestAnimationFrame(function(){
      pingProgress();
      maybeNext();
      var se = document.scrollingElement || document.documentElement;
      var cur = se.scrollTop || 0;
      if (cur > lastScrollTop + 8 && cur > 24) {
        kalamBridge({type:'reader-ui-hide'});
        lastUiZone = 'hidden';
      }
      lastScrollTop = cur;
    });
  }, {passive:true});

  document.addEventListener('mousemove', function(e) {
    var zone = null;
    if (e.clientY < 72) zone = 'top';
    else if (window.innerHeight - e.clientY < 92) zone = 'bottom';
    else return;
    if (zone === lastUiZone) return;
    lastUiZone = zone;
    kalamBridge({type: zone === 'top' ? 'reader-ui-show-back' : 'reader-ui-show-pill'});
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

  // ---- Temporary selection band ----
  // WebKit's native ::selection background does not expose a height control:
  // inline styles can make later line fragments taller than the first. Keep
  // native selection for copy/drag behavior, but paint the visible band with
  // one consistent height of our own.
  var selectionBandLayer = null;
  var selectionBandsVisible = false;
  var selectionBandFrame = null;
  var selectionBandPadding = 2;
  var selectionBandStackElements = [];
  var selectionInkStyles = [];

  function ensureSelectionBandLayer() {
    if (selectionBandLayer) return;
    selectionBandLayer = document.createElement('div');
    selectionBandLayer.id = 'kalam-selection-bands';
    selectionBandLayer.setAttribute('aria-hidden', 'true');
    document.body.appendChild(selectionBandLayer);
  }

  function clearSelectionBands() {
    if (!selectionBandLayer) return;
    while (selectionBandLayer.firstChild) {
      selectionBandLayer.removeChild(selectionBandLayer.firstChild);
    }
  }

  function clearSelectionInkStyles() {
    for (var i = 0; i < selectionInkStyles.length; i++) {
      var saved = selectionInkStyles[i];
      var style = saved.element.style;
      if (saved.color) style.setProperty('color', saved.color, saved.colorPriority);
      else style.removeProperty('color');
      if (saved.fill) style.setProperty('-webkit-text-fill-color', saved.fill, saved.fillPriority);
      else style.removeProperty('-webkit-text-fill-color');
    }
    selectionInkStyles = [];
  }

  function selectionRangeIntersectsElement(range, element) {
    try {
      var elementRange = document.createRange();
      elementRange.selectNodeContents(element);
      return range.compareBoundaryPoints(window.Range.END_TO_START, elementRange) > 0 &&
        range.compareBoundaryPoints(window.Range.START_TO_END, elementRange) < 0;
    } catch(e) {
      return false;
    }
  }

  function normalizeSelectionInkStyles(range) {
    clearSelectionInkStyles();
    if (!document.body || !range) return;

    var textColor = '';
    try { textColor = window.getComputedStyle(document.body).color; } catch(e) {}
    if (!textColor) return;

    var root = range.commonAncestorContainer;
    if (root && root.nodeType !== 1) root = root.parentElement;
    if (!root || !root.querySelectorAll) return;

    var elements = root.querySelectorAll('*');
    for (var i = 0; i < elements.length; i++) {
      var element = elements[i];
      if (element.closest && (element.closest('#kalam-chip') || element.closest('#kalam-dict-popup') || element.closest('#kalam-selection-bands') || element.closest('.kalam-selection-handle'))) continue;
      if (!selectionRangeIntersectsElement(range, element)) continue;
      var style = element.style;
      selectionInkStyles.push({
        element: element,
        color: style.getPropertyValue('color'),
        colorPriority: style.getPropertyPriority('color'),
        fill: style.getPropertyValue('-webkit-text-fill-color'),
        fillPriority: style.getPropertyPriority('-webkit-text-fill-color')
      });
      style.setProperty('color', textColor, 'important');
      style.setProperty('-webkit-text-fill-color', textColor, 'important');
    }
  }

  function setSelectionBandStacking(active) {
    var root = document.documentElement;
    if (!root) return;
    if (active) root.classList.add('kalam-selection-active');
    else root.classList.remove('kalam-selection-active');
  }

  function clearSelectionBandStacking() {
    for (var i = 0; i < selectionBandStackElements.length; i++) {
      var element = selectionBandStackElements[i];
      if (element.classList) element.classList.remove('kalam-selection-content-above');
    }
    selectionBandStackElements = [];
  }

  function addSelectionBandStacking(element) {
    if (!element || element === document.body || element === document.documentElement) return;
    if (!element.classList || selectionBandStackElements.indexOf(element) !== -1) return;
    element.classList.add('kalam-selection-content-above');
    selectionBandStackElements.push(element);
  }

  function selectionBlockFromNode(node) {
    var element = node && node.nodeType === 1 ? node : node && node.parentElement;
    if (!element) return null;
    var block = element.closest && element.closest('p, li, blockquote, pre, h1, h2, h3, h4, h5, h6, dt, dd, td, th');
    if (block) return block;
    return (element.closest && element.closest('div, section, article, main')) || null;
  }

  function stackSelectedContent(range) {
    clearSelectionBandStacking();
    if (!range) return;

    var root = range.commonAncestorContainer;
    if (root && root.nodeType !== 1) root = root.parentElement;
    if (!root || !root.querySelectorAll) return;

    var foundBlock = false;
    var blocks = root.querySelectorAll('p, li, blockquote, pre, h1, h2, h3, h4, h5, h6, dt, dd, td, th');
    for (var i = 0; i < blocks.length; i++) {
      if (selectionRangeIntersectsElement(range, blocks[i])) {
        addSelectionBandStacking(blocks[i]);
        foundBlock = true;
      }
    }

    var startBlock = selectionBlockFromNode(range.startContainer);
    var endBlock = selectionBlockFromNode(range.endContainer);
    if (startBlock) addSelectionBandStacking(startBlock);
    if (endBlock) addSelectionBandStacking(endBlock);
    if (!foundBlock && !startBlock && !endBlock) {
      addSelectionBandStacking(root);
    }
  }

  function hideSelectionBands() {
    selectionBandsVisible = false;
    if (selectionBandFrame !== null) {
      cancelAnimationFrame(selectionBandFrame);
      selectionBandFrame = null;
    }
    clearSelectionBands();
    clearSelectionInkStyles();
    clearSelectionBandStacking();
    setSelectionBandStacking(false);
    if (selectionBandLayer) selectionBandLayer.style.display = 'none';
  }

  function selectionTextBlock(range) {
    return selectionBlockFromNode(range.startContainer) || document.body;
  }

  function firstReadableTextNode(root) {
    if (!root) return null;
    try {
      var walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, null);
      var node;
      while ((node = walker.nextNode())) {
        if (!node.nodeValue || !node.nodeValue.trim()) continue;
        var parent = node.parentElement;
        if (parent && parent.closest && (parent.closest('#kalam-chip') || parent.closest('#kalam-dict-popup') || parent.closest('#kalam-selection-bands') || parent.closest('.kalam-selection-handle'))) continue;
        return node;
      }
    } catch(e) {}
    return null;
  }

  function selectionReferenceHeight(range, rects) {
    // Measure the first real character of the containing paragraph. This is
    // deliberately independent of where the user started selecting, so a
    // second-line or italic selection cannot choose its own taller height.
    try {
      var first = firstReadableTextNode(selectionTextBlock(range));
      if (first) {
        var value = first.nodeValue || '';
        var offset = 0;
        while (offset < value.length && /\s/.test(value.charAt(offset))) offset++;
        if (offset < value.length) {
          var firstChar = document.createRange();
          firstChar.setStart(first, offset);
          firstChar.setEnd(first, Math.min(offset + 1, value.length));
          var firstRects = firstChar.getClientRects();
          if (firstRects && firstRects.length && firstRects[0].height > 0) {
            return firstRects[0].height;
          }
        }
      }
    } catch(e) {}

    try {
      var element = selectionTextBlock(range);
      var lineHeight = parseFloat(window.getComputedStyle(element).lineHeight);
      if (isFinite(lineHeight) && lineHeight > 0) return lineHeight;
    } catch(e) {}

    return rects && rects.length ? rects[0].height : 0;
  }

  function positionSelectionBands() {
    selectionBandFrame = null;
    if (!selectionBandsVisible || !selectionBandLayer) return;

    var data = getSelectionData();
    var sel = window.getSelection();
    if (!data || !sel || sel.rangeCount === 0 || sel.isCollapsed) {
      hideSelectionBands();
      return;
    }

    var range = sel.getRangeAt(0);
    stackSelectedContent(range);
    normalizeSelectionInkStyles(range);
    var rects;
    try {
      rects = range.getClientRects();
    } catch(e) {
      hideSelectionBands();
      return;
    }
    if (!rects || !rects.length) {
      hideSelectionBands();
      return;
    }

    var referenceHeight = selectionReferenceHeight(range, rects);
    if (!referenceHeight || referenceHeight <= 0) {
      hideSelectionBands();
      return;
    }

    clearSelectionBands();
    selectionBandLayer.style.display = 'block';
    // Give the band a little breathing room above and below the glyph line,
    // while keeping every selected line exactly the same height.
    var bandHeight = referenceHeight + selectionBandPadding * 2;
    var scrollX = window.scrollX || 0;
    var scrollY = window.scrollY || 0;
    for (var i = 0; i < rects.length; i++) {
      var rect = rects[i];
      if (!rect || rect.width <= 0 || rect.height <= 0) continue;
      var band = document.createElement('div');
      band.className = 'kalam-selection-band';
      band.style.left = (scrollX + rect.left) + 'px';
      band.style.top = (scrollY + rect.top + (rect.height - bandHeight) / 2) + 'px';
      band.style.width = Math.max(1, rect.width) + 'px';
      band.style.height = Math.max(1, bandHeight) + 'px';
      selectionBandLayer.appendChild(band);
    }
  }

  function scheduleSelectionBandPosition() {
    if (!selectionBandsVisible || selectionBandFrame !== null) return;
    selectionBandFrame = requestAnimationFrame(positionSelectionBands);
  }

  function showSelectionBands() {
    ensureSelectionBandLayer();
    setSelectionBandStacking(true);
    selectionBandsVisible = true;
    scheduleSelectionBandPosition();
  }

  // ---- Temporary selection handles ----
  // The browser owns the selection itself. These two small elements mirror its
  // endpoint edges without replacing native selection, copy, or highlight behavior.
  var selectionHandleStart = null;
  var selectionHandleEnd = null;
  var selectionHandlesVisible = false;
  var selectionHandleFrame = null;

  function ensureSelectionHandles() {
    if (selectionHandleStart && selectionHandleEnd) return;
    selectionHandleStart = document.createElement('div');
    selectionHandleStart.className = 'kalam-selection-handle kalam-selection-handle-start';
    selectionHandleStart.setAttribute('aria-hidden', 'true');
    selectionHandleEnd = document.createElement('div');
    selectionHandleEnd.className = 'kalam-selection-handle kalam-selection-handle-end';
    selectionHandleEnd.setAttribute('aria-hidden', 'true');
    document.body.appendChild(selectionHandleStart);
    document.body.appendChild(selectionHandleEnd);
  }

  function hideSelectionHandles() {
    selectionHandlesVisible = false;
    if (selectionHandleFrame !== null) {
      cancelAnimationFrame(selectionHandleFrame);
      selectionHandleFrame = null;
    }
    if (selectionHandleStart) selectionHandleStart.style.display = 'none';
    if (selectionHandleEnd) selectionHandleEnd.style.display = 'none';
  }

  function selectionEndpointRect(range, which) {
    // Use the selection's own line rectangles first. A collapsed range at an
    // endpoint can be shorter or vertically misplaced on a later wrapped line
    // in WebKit; the first/last selection rect is the actual painted line box.
    try {
      var rects = range.getClientRects();
      if (rects && rects.length) {
        return which === 'start' ? rects[0] : rects[rects.length - 1];
      }
    } catch(e) {}

    // Fallback for a WebKit version that exposes no rect for the selection.
    try {
      var point = document.createRange();
      if (which === 'start') {
        point.setStart(range.startContainer, range.startOffset);
      } else {
        point.setStart(range.endContainer, range.endOffset);
      }
      point.collapse(true);
      var pointRects = point.getClientRects();
      if (pointRects && pointRects.length) return pointRects[0];
      var fallback = range.getBoundingClientRect();
      if (fallback) return fallback;
    } catch(e) {}
    return null;
  }

  function positionSelectionHandles() {
    selectionHandleFrame = null;
    if (!selectionHandlesVisible || !selectionHandleStart || !selectionHandleEnd) return;

    var data = getSelectionData();
    if (!data) {
      hideSelectionHandles();
      return;
    }

    var sel = window.getSelection();
    if (!sel || sel.rangeCount === 0 || sel.isCollapsed) {
      hideSelectionHandles();
      return;
    }
    var range = sel.getRangeAt(0);
    var startRect = selectionEndpointRect(range, 'start');
    var endRect = selectionEndpointRect(range, 'end');
    if (!startRect || !endRect) {
      hideSelectionHandles();
      return;
    }

    // Use the same base reference and breathing room as the selection bands.
    // A later line can report a taller fragment in WebKit, so it must not make
    // either edge grow on line two.
    var selectionRects = null;
    try { selectionRects = range.getClientRects(); } catch(e) {}
    var referenceHeight = selectionReferenceHeight(range, selectionRects);
    if (!referenceHeight || referenceHeight <= 0) {
      referenceHeight = startRect.height > 0 ? startRect.height : endRect.height;
    }
    var handleHeight = referenceHeight + selectionBandPadding * 2;
    var startTop = startRect.top + (startRect.height - handleHeight) / 2;
    var endTop = endRect.top + (endRect.height - handleHeight) / 2;
    var endX = endRect.width > 0 ? endRect.right : endRect.left;
    selectionHandleStart.style.left = (window.scrollX + startRect.left - 1) + 'px';
    selectionHandleStart.style.top = (window.scrollY + startTop) + 'px';
    selectionHandleStart.style.height = Math.max(1, handleHeight) + 'px';
    selectionHandleEnd.style.left = (window.scrollX + endX - 1) + 'px';
    selectionHandleEnd.style.top = (window.scrollY + endTop) + 'px';
    selectionHandleEnd.style.height = Math.max(1, handleHeight) + 'px';
    selectionHandleStart.style.display = 'block';
    selectionHandleEnd.style.display = 'block';
  }

  function scheduleSelectionHandlePosition() {
    if (!selectionHandlesVisible || selectionHandleFrame !== null) return;
    selectionHandleFrame = requestAnimationFrame(positionSelectionHandles);
  }

  function showSelectionHandles() {
    ensureSelectionHandles();
    selectionHandlesVisible = true;
    scheduleSelectionHandlePosition();
  }

  function excerptText(text) {
    return String(text || '').replace(/\s+/g, ' ').trim();
  }

  function isWordCharacter(ch) {
    if (!ch) return false;
    return ch.toLowerCase() !== ch.toUpperCase() || /[0-9_]/.test(ch);
  }

  function ignoredExcerptNode(node) {
    var parent = node && node.parentElement;
    if (!parent || !parent.closest) return false;
    return !!parent.closest('script, style, noscript, template, #kalam-chip, #kalam-dict-popup, #kalam-selection-bands, #kalam-annotation-focus-layer, .kalam-selection-handle');
  }

  function excerptBlock(node) {
    var parent = node && node.parentElement;
    if (!parent || !parent.closest) return null;
    return parent.closest('p, li, blockquote, pre, h1, h2, h3, h4, h5, h6, dt, dd, td, th, div, section, article, main');
  }

  function appendExcerptCharacter(normalized, entries, character, startNode, startOffset, endNode, endOffset) {
    if (/\s/.test(character)) {
      if (normalized.value && normalized.value.charAt(normalized.value.length - 1) === ' ') {
        entries[entries.length - 1].endNode = endNode;
        entries[entries.length - 1].endOffset = endOffset;
        return;
      }
      character = ' ';
    }
    normalized.value += character;
    entries.push({
      startNode: startNode,
      startOffset: startOffset,
      endNode: endNode,
      endOffset: endOffset,
    });
  }

  function rangeFromExcerpt(excerpt) {
    var needle = excerptText(excerpt);
    if (!needle || !document.body) return null;

    var walker;
    try {
      walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, null);
    } catch(e) {
      return null;
    }

    var nodes = [];
    var node;
    while ((node = walker.nextNode())) {
      if (!node.nodeValue || !node.nodeValue.length || ignoredExcerptNode(node)) continue;
      nodes.push(node);
    }
    if (!nodes.length) return null;

    var normalized = {value: ''};
    var entries = [];
    var previousNode = null;
    for (var n = 0; n < nodes.length; n++) {
      node = nodes[n];
      var value = node.nodeValue || '';
      if (previousNode) {
        var previousValue = previousNode.nodeValue || '';
        var previousBlock = excerptBlock(previousNode);
        var currentBlock = excerptBlock(node);
        if (previousBlock && currentBlock && previousBlock !== currentBlock &&
            previousValue.length && value.length &&
            !/\s/.test(previousValue.charAt(previousValue.length - 1)) &&
            !/\s/.test(value.charAt(0))) {
          appendExcerptCharacter(normalized, entries, ' ', previousNode, previousValue.length, node, 0);
        }
      }
      for (var i = 0; i < value.length; i++) {
        appendExcerptCharacter(normalized, entries, value.charAt(i), node, i, node, i + 1);
      }
      previousNode = node;
    }

    var index = normalized.value.indexOf(needle);
    while (index >= 0) {
      var before = index > 0 ? normalized.value.charAt(index - 1) : '';
      var afterIndex = index + needle.length;
      var after = afterIndex < normalized.value.length ? normalized.value.charAt(afterIndex) : '';
      var startsInsideWord = isWordCharacter(before) && isWordCharacter(needle.charAt(0));
      var endsInsideWord = isWordCharacter(after) && isWordCharacter(needle.charAt(needle.length - 1));
      if (!startsInsideWord && !endsInsideWord) {
        var first = entries[index];
        var last = entries[afterIndex - 1];
        if (first && last) {
          try {
            var range = document.createRange();
            range.setStart(first.startNode, first.startOffset);
            range.setEnd(last.endNode, last.endOffset);
            if (!range.collapsed) return range;
          } catch(e) {}
        }
      }
      index = normalized.value.indexOf(needle, index + 1);
    }
    return null;
  }

  function validNodeOffset(node, offset) {
    var numeric = Number(offset);
    if (!node || !isFinite(numeric) || numeric < 0 || Math.floor(numeric) !== numeric) return false;
    var limit = node.nodeType === 3 ? (node.nodeValue || '').length : (node.childNodes ? node.childNodes.length : 0);
    return numeric <= limit;
  }

  function rangeFromPaths(startPath, startOffset, endPath, endOffset) {
    var startNode = nodeFromPath(startPath);
    var endNode = nodeFromPath(endPath);
    if (!validNodeOffset(startNode, startOffset) || !validNodeOffset(endNode, endOffset)) return null;
    try {
      var range = document.createRange();
      range.setStart(startNode, Number(startOffset));
      range.setEnd(endNode, Number(endOffset));
      return range.collapsed ? null : range;
    } catch(e) {
      return null;
    }
  }

  function rangeMatchesExcerpt(range, excerpt) {
    var needle = excerptText(excerpt);
    if (!needle) return true;
    try {
      return excerptText(range.toString()) === needle;
    } catch(e) {
      return false;
    }
  }

  function annotationRange(startPath, startOffset, endPath, endOffset, excerpt) {
    // Keep the original path/offset anchor authoritative when it still points
    // to the saved text. The excerpt is used when the saved DOM location no
    // longer exists or now points at different text.
    var pathRange = rangeFromPaths(startPath, startOffset, endPath, endOffset);
    if (pathRange && rangeMatchesExcerpt(pathRange, excerpt)) return pathRange;
    return rangeFromExcerpt(excerpt);
  }

  function wrapRangeByPaths(startPath, startOffset, endPath, endOffset, color, annId, excerpt) {
    if (document.querySelector('span.kalam-hl[data-annotation-id=\"'+annId+'\"]')) return true;
    var range = annotationRange(startPath, startOffset, endPath, endOffset, excerpt);
    if (!range) {
      console.log('kalam wrap: saved anchor and excerpt not found', startPath, endPath);
      return false;
    }
    try {
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
          var range2 = annotationRange(startPath, startOffset, endPath, endOffset, excerpt);
          if (!range2) return false;
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

  window.kalamRecolorHighlight = function(annId, color) {
    var allowed = ['yellow', 'green', 'blue', 'pink', 'orange'];
    if (allowed.indexOf(color) === -1) return;
    var nodes = document.querySelectorAll('span.kalam-hl[data-annotation-id=\"'+annId+'\"]');
    nodes.forEach(function(n){
      for (var i = 0; i < allowed.length; i++) {
        n.classList.remove('kalam-hl-' + allowed[i]);
      }
      n.classList.add('kalam-hl-' + color);
      n.dataset.color = color;
    });
  };

  // ---- UI: selection toolbar ----
  function ensureChip() {
    var chip = document.getElementById('kalam-chip');
    if (chip) return chip;
    chip = document.createElement('div');
    chip.id = 'kalam-chip';
    chip.style.display = 'none';
    chip.innerHTML = '<button type=\"button\" class=\"kalam-chip-action kalam-chip-action-accent\" id=\"kalam-chip-highlight\" title=\"Highlight\" aria-label=\"Highlight\" aria-expanded=\"false\">'
      + '<svg class=\"kalam-chip-icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\" focusable=\"false\">'
      + '<path d=\"m15 4 5 5-9 9H6v-5l9-9Z\"></path><path d=\"m13 6 5 5\"></path><path d=\"M4 20h8\"></path>'
      + '</svg></button>'
      + '<div class=\"kalam-chip-colors\" id=\"kalam-chip-colors\">'
      + '<button type=\"button\" class=\"kalam-chip-btn\" data-color=\"yellow\" title=\"Highlight yellow\" aria-label=\"Highlight yellow\" style=\"background:#f4d35e\"></button>'
      + '<button type=\"button\" class=\"kalam-chip-btn\" data-color=\"green\" title=\"Highlight green\" aria-label=\"Highlight green\" style=\"background:#8acb9c\"></button>'
      + '<button type=\"button\" class=\"kalam-chip-btn\" data-color=\"blue\" title=\"Highlight blue\" aria-label=\"Highlight blue\" style=\"background:#8bb7f2\"></button>'
      + '<button type=\"button\" class=\"kalam-chip-btn\" data-color=\"pink\" title=\"Highlight pink\" aria-label=\"Highlight pink\" style=\"background:#e99bbd\"></button>'
      + '<button type=\"button\" class=\"kalam-chip-btn\" data-color=\"orange\" title=\"Highlight orange\" aria-label=\"Highlight orange\" style=\"background:#f2ae72\"></button>'
      + '</div>'
      + '<div class=\"kalam-chip-sep\"></div>'
      + '<button type=\"button\" class=\"kalam-chip-action\" id=\"kalam-chip-quote\" title=\"Save quote\" aria-label=\"Save quote\">'
      + '<svg class=\"kalam-chip-icon kalam-chip-icon-fill\" viewBox=\"0 0 24 24\" aria-hidden=\"true\" focusable=\"false\">'
      + '<path d=\"M4 11V8h4v3c0 3-1.3 5-4 6v-2.1c1.1-.5 1.8-1.3 2-2.9H4Zm10 0V8h4v3c0 3-1.3 5-4 6v-2.1c1.1-.5 1.8-1.3 2-2.9h-2Z\"></path>'
      + '</svg></button>'
      + '<div class=\"kalam-chip-sep\"></div>'
      + '<button type=\"button\" class=\"kalam-chip-action\" id=\"kalam-chip-dict\" title=\"Dictionary (D)\" aria-label=\"Dictionary (D)\">'
      + '<svg class=\"kalam-chip-icon kalam-chip-icon-letters\" viewBox=\"0 0 24 24\" aria-hidden=\"true\" focusable=\"false\"><text x=\"2.5\" y=\"16\" font-size=\"12\" font-weight=\"700\">A</text><text x=\"13\" y=\"19\" font-size=\"9\" font-weight=\"600\">a</text></svg></button>'
      + '<div class=\"kalam-chip-sep\"></div>'
      + '<button type=\"button\" class=\"kalam-chip-action\" id=\"kalam-chip-copy\" title=\"Copy\" aria-label=\"Copy\">'
      + '<svg class=\"kalam-chip-icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\" focusable=\"false\"><rect x=\"8\" y=\"8\" width=\"11\" height=\"12\" rx=\"2\"></rect><path d=\"M16 8V6a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h3\"></path></svg></button>';
    document.body.appendChild(chip);
    chip.querySelectorAll('.kalam-chip-btn').forEach(function(b){
      b.addEventListener('click', function(){
        var color = b.dataset.color;
        kalamHandleHighlight(color);
      });
    });
    var highlight = document.getElementById('kalam-chip-highlight');
    if (highlight) highlight.addEventListener('click', function(){
      var colors = document.getElementById('kalam-chip-colors');
      if (colors) {
        var visible = colors.classList.toggle('visible');
        highlight.setAttribute('aria-expanded', visible ? 'true' : 'false');
      }
    });
    var q = document.getElementById('kalam-chip-quote');
    if (q) q.addEventListener('click', function(){ kalamHandleQuote(); });
    var d = document.getElementById('kalam-chip-dict');
    if (d) d.addEventListener('click', function(){ kalamHandleDict(); });
    var c = document.getElementById('kalam-chip-copy');
    if (c) c.addEventListener('click', function(){
      var sel = window.getSelection(); if (sel) { try{ document.execCommand('copy'); }catch(e){} }
      hideChip();
      hideSelectionBands();
      hideSelectionHandles();
    });
    return chip;
  }
  function showChipAt(rect) {
    var chip = ensureChip();
    var colors = document.getElementById('kalam-chip-colors');
    if (colors) colors.classList.remove('visible');
    var highlight = document.getElementById('kalam-chip-highlight');
    if (highlight) highlight.setAttribute('aria-expanded', 'false');
    chip.style.display = 'flex';
    var chipWidth = chip.offsetWidth || 168;
    var chipHeight = chip.offsetHeight || 40;
    var margin = 12;
    var top, left;
    if (rect) {
      top = (window.scrollY + rect.y - chipHeight - 14);
      left = (window.scrollX + rect.x + (rect.w || 0) / 2 - chipWidth / 2);
      if (top < window.scrollY + margin) top = window.scrollY + rect.y + rect.h + margin;
    } else {
      top = window.scrollY + 120;
      left = window.scrollX + 80;
    }
    var minLeft = window.scrollX + margin;
    var maxLeft = window.scrollX + window.innerWidth - chipWidth - margin;
    if (maxLeft < minLeft) maxLeft = minLeft;
    if (left < minLeft) left = minLeft;
    if (left > maxLeft) left = maxLeft;
    chip.style.top = top + 'px';
    chip.style.left = left + 'px';
  }
  function hideChip() {
    var chip = document.getElementById('kalam-chip');
    if (chip) chip.style.display = 'none';
    var colors = document.getElementById('kalam-chip-colors');
    if (colors) colors.classList.remove('visible');
    var highlight = document.getElementById('kalam-chip-highlight');
    if (highlight) highlight.setAttribute('aria-expanded', 'false');
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
    hideSelectionBands();
    hideSelectionHandles();
    kalamBridge({type:'highlight', color:color, text:data.text, startPath:data.startPath, startOffset:data.startOffset, endPath:data.endPath, endOffset:data.endOffset, tmpId:provisional});
  };
  window.kalamHandleQuote = function() {
    var data = getSelectionData();
    if (!data) return;
    hideChip();
    hideSelectionBands();
    hideSelectionHandles();
    kalamBridge({type:'quote', text:data.text, startPath:data.startPath, startOffset:data.startOffset, endPath:data.endPath, endOffset:data.endOffset});
  };
  window.kalamHandleDict = function() {
    var data = getSelectionData();
    var word = '';
    var ctx = '';
    var rect = null;
    if (data) {
      // Keep a short phrase intact. The dictionary can contain multi-word
      // entries, and reducing the selection to its first word made phrase
      // lookup depend on an arbitrary selection boundary.
      word = data.text.replace(/\s+/g, ' ').trim();
      ctx = data.text;
      rect = data.rect;
    } else {
      var sel = window.getSelection();
      if (sel && sel.toString()) {
        var selectedText = sel.toString();
        word = selectedText.replace(/\s+/g, ' ').trim();
        ctx = selectedText;
        try { var r = sel.getRangeAt(0).getBoundingClientRect(); rect = {x:r.left, y:r.top, w:r.width, h:r.height, bottom:r.bottom}; } catch(e){}
      }
    }
    if (!word) return;
    hideChip();
    hideSelectionBands();
    hideSelectionHandles();
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
  function showDictPopup(word, results, rect) {
    var p = ensureDictPopup();
    function esc(s){ return (s||'').replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;'); }
    var entries = [];
    if (Array.isArray(results)) {
      results.forEach(function(entry) {
        if (entry && entry.word && entry.definition) {
          entries.push({word:String(entry.word), definition:String(entry.definition)});
        }
      });
    } else if (typeof results === 'string' && results.trim()) {
      // Keep compatibility with older callers that supplied one definition.
      entries.push({word:word, definition:results});
    }
    var countLabel = entries.length === 1 ? '1 result' : entries.length + ' results';
    var resultHtml = '';
    entries.forEach(function(entry, index) {
      resultHtml += '<article class="kalam-dict-result">'
        + '<div class="kalam-dict-result-head"><div class="kalam-dict-word">'+esc(entry.word)+'</div><div class="kalam-dict-sub">Result '+(index + 1)+'</div></div>'
        + '<div class="kalam-dict-body">'+esc(entry.definition)+'</div>'
        + '<div class="kalam-dict-actions"><button class="kalam-dict-save" data-result-index="'+index+'">Save word</button><button class="kalam-dict-copy" data-result-index="'+index+'">Copy</button></div>'
        + '</article>';
    });
    p.innerHTML = '<div class="kalam-dict-head"><div><div class="kalam-dict-word">'+esc(word)+'</div><div class="kalam-dict-sub">Dictionary · '+countLabel+'</div></div><button class="kalam-dict-close" onclick="window.kalamHideDict()">Close</button></div>'
      + '<div class="kalam-dict-results">'+resultHtml+'</div>';
    p.style.display = 'block';
    var popupHeight = Math.min(p.offsetHeight || 220, Math.max(120, window.innerHeight - 16));
    var popupWidth = Math.min(300, window.innerWidth * 0.84);
    var minLeft = window.scrollX + 8;
    var maxLeft = window.scrollX + window.innerWidth - popupWidth - 8;
    var top, left;
    if (rect) {
      top = window.scrollY + rect.y + rect.h + 10;
      left = window.scrollX + rect.x;
      if (left < minLeft) left = minLeft;
      if (left > maxLeft) left = Math.max(minLeft, maxLeft);
      if (top + popupHeight > window.scrollY + window.innerHeight - 8) {
        top = window.scrollY + rect.y - popupHeight - 10;
      }
      if (top < window.scrollY + 8) top = window.scrollY + 8;
    } else {
      top = window.scrollY + 180;
      left = minLeft + 32;
    }
    p.style.top = top + 'px';
    p.style.left = left + 'px';
    var saves = p.querySelectorAll('.kalam-dict-save');
    for (var i = 0; i < saves.length; i++) {
      saves[i].addEventListener('click', function() {
        var entry = entries[Number(this.getAttribute('data-result-index'))];
        if (!entry) return;
        kalamBridge({type:'save-word', word:entry.word, definition:entry.definition});
        hideDict();
      });
    }
    var copies = p.querySelectorAll('.kalam-dict-copy');
    for (var j = 0; j < copies.length; j++) {
      copies[j].addEventListener('click', function() {
        var entry = entries[Number(this.getAttribute('data-result-index'))];
        if (!entry) return;
        try{ navigator.clipboard.writeText(entry.definition); }catch(e){}
        hideDict();
      });
    }
  }
  function hideDict() {
    var p = document.getElementById('kalam-dict-popup');
    if (p) p.style.display='none';
  }
  window.kalamHideDict = hideDict;
  window.kalamShowDict = function(word, results, rectJson) {
    var rect = null;
    try { if (rectJson) rect = JSON.parse(rectJson); } catch(e){}
    showDictPopup(word, results, rect);
  };

  // ---- Highlights injection from Rust ----
  window.kalamInjectHighlights = function(jsonStr) {
    try {
      var arr = JSON.parse(jsonStr);
      arr.forEach(function(a){
        if (document.querySelector('span.kalam-hl[data-annotation-id=\"'+a.id+'\"]')) return;
        wrapRangeByPaths(a.start_path, a.start_offset, a.end_path, a.end_offset, a.color, a.id, a.text_excerpt);
      });
    } catch(e){ console.log('kalam inject highlights failed', e, jsonStr?.slice(0,200)); }
  };
  window.kalamInjectSingleHighlight = function(aJson) {
    try {
      var a = JSON.parse(aJson);
      wrapRangeByPaths(a.start_path, a.start_offset, a.end_path, a.end_offset, a.color, a.id, a.text_excerpt);
    } catch(e){ console.log('single inject failed', e); }
  };

  // ---- Exact annotation navigation + temporary emphasis ----
  // Prefer the persisted highlight span after chapter annotations are injected.
  // This keeps the jump stable even though wrapping a range changes child-node
  // positions. The saved path remains a fallback for older or failed wraps.
  var annotationFocusLayer = null;
  var annotationFocusTimer = null;

  function ensureAnnotationFocusLayer() {
    if (annotationFocusLayer) return;
    annotationFocusLayer = document.createElement('div');
    annotationFocusLayer.id = 'kalam-annotation-focus-layer';
    annotationFocusLayer.setAttribute('aria-hidden', 'true');
    document.body.appendChild(annotationFocusLayer);
  }

  function clearAnnotationFocus() {
    if (annotationFocusTimer !== null) {
      clearTimeout(annotationFocusTimer);
      annotationFocusTimer = null;
    }
    if (!annotationFocusLayer) return;
    while (annotationFocusLayer.firstChild) {
      annotationFocusLayer.removeChild(annotationFocusLayer.firstChild);
    }
    annotationFocusLayer.style.display = 'none';
  }

  function showAnnotationFocus(target, range) {
    ensureAnnotationFocusLayer();
    clearAnnotationFocus();
    var rects = null;
    try {
      rects = target ? target.getClientRects() : range.getClientRects();
    } catch(e) {}
    if (!rects || !rects.length) return;

    var scrollX = window.scrollX || 0;
    var scrollY = window.scrollY || 0;
    for (var i = 0; i < rects.length; i++) {
      var rect = rects[i];
      if (!rect || rect.width <= 0 || rect.height <= 0) continue;
      var focus = document.createElement('div');
      focus.className = 'kalam-annotation-focus';
      focus.style.left = (scrollX + rect.left - 3) + 'px';
      focus.style.top = (scrollY + rect.top - 3) + 'px';
      focus.style.width = (rect.width + 6) + 'px';
      focus.style.height = (rect.height + 6) + 'px';
      annotationFocusLayer.appendChild(focus);
    }
    if (!annotationFocusLayer.firstChild) return;
    annotationFocusLayer.style.display = 'block';
    annotationFocusTimer = setTimeout(clearAnnotationFocus, 1500);
  }

  function scrollAnnotationNearTop(rect) {
    if (!rect) return;
    var topInset = Math.max(64, Math.min(112, window.innerHeight * 0.14));
    var topDelta = rect.top - topInset;
    try {
      window.scrollBy({top: topDelta, left: 0, behavior:'auto'});
    } catch(e) {
      window.scrollBy(0, topDelta);
    }
  }

  function showAnnotationFocusAfterScroll(target, range) {
    if (window.requestAnimationFrame) {
      window.requestAnimationFrame(function(){ showAnnotationFocus(target, range); });
    } else {
      setTimeout(function(){ showAnnotationFocus(target, range); }, 0);
    }
  }

  window.kalamRevealAnnotation = function(annotation) {
    if (!annotation) return false;
    var target = null;
    var spans = document.querySelectorAll('span.kalam-hl[data-annotation-id]');
    for (var i = 0; i < spans.length; i++) {
      if (String(spans[i].dataset.annotationId) === String(annotation.id)) {
        target = spans[i];
        break;
      }
    }
    if (target) {
      try {
        var targetRect = target.getBoundingClientRect();
        scrollAnnotationNearTop(targetRect);
        showAnnotationFocusAfterScroll(target, null);
        return true;
      } catch(e) {}
    }

    // Use the saved path and offsets next, then fall back to the saved excerpt
    // when the chapter's DOM has changed since the annotation was created.
    var range = annotationRange(
      annotation.start_path,
      annotation.start_offset,
      annotation.end_path,
      annotation.end_offset,
      annotation.text_excerpt
    );
    if (!range) return false;
    try {
      var rect = range.getBoundingClientRect();
      if (!rect || (!rect.width && !rect.height)) return false;
      scrollAnnotationNearTop(rect);
      showAnnotationFocusAfterScroll(null, range);
      return true;
    } catch(e) {
      return false;
    }
  };

  // ---- Selection listeners ----
  var selTimeout = null;
  document.addEventListener('mouseup', function(e){
    if (e.target.closest && (e.target.closest('#kalam-chip') || e.target.closest('#kalam-dict-popup'))) return;
    clearTimeout(selTimeout);
    selTimeout = setTimeout(function(){
      var data = getSelectionData();
      if (data && data.text && data.text.trim().length>0 && data.text.trim().length < 2000) {
        showSelectionBands();
        showSelectionHandles();
        showChipAt(data.rect);
        kalamBridge({type:'selection', text:data.text});
      } else {
        hideSelectionBands();
        hideSelectionHandles();
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
    hideSelectionBands();
    hideSelectionHandles();
    // don't hide dict on mousedown inside content
  });

  document.addEventListener('selectionchange', function(){
    if (!selectionBandsVisible && !selectionHandlesVisible) return;
    var data = getSelectionData();
    if (data && data.text && data.text.trim()) {
      if (selectionBandsVisible) scheduleSelectionBandPosition();
      if (selectionHandlesVisible) scheduleSelectionHandlePosition();
    } else {
      hideSelectionBands();
      hideSelectionHandles();
    }
  });
  window.addEventListener('scroll', function(){
    scheduleSelectionBandPosition();
    scheduleSelectionHandlePosition();
  }, {passive:true});
  window.addEventListener('resize', function(){
    scheduleSelectionBandPosition();
    scheduleSelectionHandlePosition();
  }, {passive:true});

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
      hideSelectionBands();
      hideSelectionHandles();
      hideDict();
    }
  });

  // ---- Existing progress restore already handled above ----
  })();
}
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
pub fn reading_css(
    theme: ReadingTheme,
    font_px: u32,
    line_height: f32,
    column_px: u32,
    chrome_theme: crate::theme::Theme,
) -> String {
    let (bg, fg) = theme.swatch();
    let (selection_bg, handle_color) = theme.selection_style();
    // The custom selection band is layered under the chapter content so the
    // highlight does not tint the glyphs. The blend mode keeps the band
    // readable against each page theme.
    let selection_blend = match theme {
        ReadingTheme::Light | ReadingTheme::Sepia => "multiply",
        ReadingTheme::Dark | ReadingTheme::Ink => "screen",
    };

    // Many EPUBs ship chapter headings, ornaments and diagrams as PNG/JPEG with
    // a baked-in **white** background. CSS cannot repaint pixels inside an
    // image, so on a themed page those land as mismatched slabs.
    //
    // The trick is a blend mode chosen per theme, so the artwork's white
    // background takes the page colour and the ink stays legible:
    //
    // * Light / Sepia — `multiply`. White × cream = cream (background vanishes);
    //   black ink × cream = black (ink survives).
    // * Dark — `multiply` alone would be wrong: the white background does go
    //   dark, but the black lettering goes black-on-black and disappears. So we
    //   `invert()` first (black ink → white, white background → black) and then
    //   `screen`, where black is the no-op colour: the background drops out and
    //   the lettering comes through **white**.
    //
    // Photographs must be excluded — inverting a photo produces a colour
    // negative — so cover/photo/figure images only get a gentle dim.
    let image_css = match theme {
        ReadingTheme::Dark | ReadingTheme::Ink => {
            r#"
/* Line art / text-as-image: invert then screen so ink renders white and the
   baked-in white background drops out to the page colour. */
img, svg, image, picture > img, object[type^="image"] {
  filter: invert(1) brightness(1.06) contrast(1.04) !important;
  mix-blend-mode: screen !important;
  background: transparent !important;
}
/* Photographs would become colour negatives — dim them instead. */
img[class*="cover" i], img[id*="cover" i], img[src*="cover" i],
img[class*="photo" i], img[class*="figure" i], img[class*="illus" i],
img[src*="photo" i], figure > img[alt]:not([alt=""]) {
  filter: brightness(0.86) !important;
  mix-blend-mode: normal !important;
}
"#
        }
        _ => {
            r#"
/* White artwork background takes the page tint; ink stays dark. */
img, svg, image, picture > img, object[type^="image"] {
  mix-blend-mode: multiply !important;
  background: transparent !important;
}
img[class*="cover" i], img[id*="cover" i], img[src*="cover" i],
img[class*="photo" i], img[src*="photo" i] {
  mix-blend-mode: normal !important;
}
"#
        }
    };

    // Blending composites against the nearest painted backdrop, so any author
    // wrapper that paints its own white would swallow the effect.
    let image_backdrop_css = r#"
html, body {
  background-color: %BG% !important;
}
div, p, section, article, figure, figcaption, span, td, table, tbody, tr,
main, header, blockquote {
  background-color: transparent !important;
  background-image: none !important;
}
"#
    .replace("%BG%", bg);

    let image_css = format!("{image_css}{image_backdrop_css}");

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
  max-width: {column_px}px;
  margin-left: auto !important;
  margin-right: auto !important;
  padding: 56px 28px 96px 28px !important;
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
/* The native selection remains active for copy and future dragging, but its
   background is hidden because WebKit gives different line fragments
   different heights. Kalam paints the visible band below. */
::selection,
*::selection,
body *::selection,
body * ::selection,
html.kalam-selection-active body *::selection,
html.kalam-selection-active body * ::selection {{
  background: transparent !important;
  /* WebKit can keep a different foreground colour for a nested italic run
     during a paragraph selection. Paint the selected glyphs through one
     theme-coloured, zero-offset shadow so every inline run matches. */
  color: transparent !important;
  -webkit-text-fill-color: transparent !important;
  text-shadow: 0 0 0 {fg} !important;
}}

/* ── temporary selection band ── */
/* Keep only the selected text blocks above the band. Applying this to every
   body child would create new compositing groups and break themed images that
   rely on their existing blend backdrop. */
.kalam-selection-content-above {{
  position: relative !important;
  z-index: 1 !important;
}}
#kalam-selection-bands {{
  position: absolute !important;
  top: 0 !important;
  left: 0 !important;
  width: 0 !important;
  height: 0 !important;
  overflow: visible !important;
  z-index: 0 !important;
  pointer-events: none !important;
}}
.kalam-selection-band {{
  position: absolute !important;
  display: block !important;
  min-width: 1px !important;
  min-height: 1px !important;
  padding: 0 !important;
  margin: 0 !important;
  border: none !important;
  border-radius: 2px !important;
  background: {selection_bg} !important;
  background-color: {selection_bg} !important;
  mix-blend-mode: {selection_blend} !important;
  pointer-events: none !important;
}}

/* ── temporary selection handles ──
   These are deliberately a different colour from the selection band: near
   black on Light/Sepia, bright on Dark/Ink. The edge line spans the complete
   line box at the selection endpoint; the teardrop is intentionally tiny.
   This first pass keeps pointer events out of the way of normal selection. */
.kalam-selection-handle {{
  position: absolute !important;
  z-index: 999997 !important;
  display: none;
  width: 2px !important;
  height: 0;
  padding: 0 !important;
  margin: 0 !important;
  pointer-events: none !important;
  background: {handle_color} !important;
  border: none !important;
  border-radius: 999px !important;
}}
.kalam-selection-handle::after {{
  content: '' !important;
  position: absolute !important;
  left: 50% !important;
  width: 5px !important;
  height: 5px !important;
  background: {handle_color} !important;
  border: none !important;
  border-radius: 50% 50% 50% 0 !important;
}}
.kalam-selection-handle-start::after {{
  top: -3px !important;
  transform: translateX(-50%) rotate(-45deg) !important;
}}
.kalam-selection-handle-end::after {{
  bottom: -3px !important;
  transform: translateX(-50%) rotate(135deg) !important;
}}

/* ── temporary annotation focus ── */
#kalam-annotation-focus-layer {{
  position: absolute !important;
  top: 0 !important;
  left: 0 !important;
  width: 0 !important;
  height: 0 !important;
  overflow: visible !important;
  z-index: 999996 !important;
  pointer-events: none !important;
}}
.kalam-annotation-focus {{
  position: absolute !important;
  display: block !important;
  border: 2px solid {handle_color} !important;
  border-radius: 4px !important;
  background: transparent !important;
  box-shadow: none !important;
  pointer-events: none !important;
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
  background: rgba(244, 211, 94, 0.64) !important;
  background-color: rgba(244, 211, 94, 0.64) !important;
}}
.kalam-hl-green {{
  background: rgba(138, 203, 156, 0.64) !important;
  background-color: rgba(138, 203, 156, 0.64) !important;
}}
.kalam-hl-blue {{
  background: rgba(139, 183, 242, 0.64) !important;
  background-color: rgba(139, 183, 242, 0.64) !important;
}}
.kalam-hl-pink {{
  background: rgba(233, 155, 189, 0.68) !important;
  background-color: rgba(233, 155, 189, 0.68) !important;
}}
.kalam-hl-orange {{
  background: rgba(242, 174, 114, 0.64) !important;
  background-color: rgba(242, 174, 114, 0.64) !important;
}}
.kalam-hl:hover {{
  filter: brightness(0.98) !important;
}}

/* ── selection toolbar (inside WebView) ── */
#kalam-chip {{
  position: absolute !important;
  z-index: 999999 !important;
  background: var(--kalam-chip-bg) !important;
  border: 1px solid var(--kalam-chip-border) !important;
  border-radius: 999px !important;
  padding: 4px !important;
  display: none;
  flex-direction: row !important;
  align-items: center !important;
  gap: 2px !important;
  box-shadow: none !important;
  box-shadow: 0 10px 28px color-mix(in srgb, var(--kalam-chip-shadow) 26%, transparent) !important;
  font-family: -apple-system, BlinkMacSystemFont, "Inter", sans-serif !important;
  backdrop-filter: blur(16px) !important;
  -webkit-backdrop-filter: blur(16px) !important;
}}
#kalam-chip, #kalam-chip * {{
  box-sizing: border-box !important;
}}
.kalam-chip-colors {{
  display: none !important;
  align-items: center !important;
  gap: 6px !important;
  padding: 0 4px !important;
}}
.kalam-chip-colors.visible {{
  display: flex !important;
}}
.kalam-chip-btn {{
  width: 20px !important;
  height: 20px !important;
  border-radius: 999px !important;
  border: 2px solid var(--kalam-chip-border) !important;
  border-color: color-mix(in srgb, var(--kalam-chip-border) 70%, transparent) !important;
  cursor: pointer !important;
  padding: 0 !important;
  margin: 0 !important;
}}
.kalam-chip-btn:hover {{
  transform: scale(1.18) !important;
  border-color: var(--kalam-chip-border) !important;
}}
.kalam-chip-sep {{
  width: 1px !important;
  height: 20px !important;
  background: var(--kalam-chip-border) !important;
  opacity: 0.18 !important;
  margin: 0 2px !important;
}}
.kalam-chip-action {{
  width: 32px !important;
  height: 32px !important;
  border-radius: 999px !important;
  border: none !important;
  background: transparent !important;
  color: var(--kalam-chip-text) !important;
  cursor: pointer !important;
  padding: 0 !important;
  margin: 0 !important;
  display: inline-flex !important;
  align-items: center !important;
  justify-content: center !important;
  flex: 0 0 32px !important;
}}
.kalam-chip-action:hover {{
  background: var(--kalam-chip-hover) !important;
  color: var(--kalam-chip-text) !important;
}}
.kalam-chip-action:focus-visible {{
  outline: 2px solid var(--kalam-chip-accent) !important;
  outline-offset: 1px !important;
}}
.kalam-chip-icon {{
  display: block !important;
  width: 17px !important;
  height: 17px !important;
  min-width: 17px !important;
  min-height: 17px !important;
  overflow: visible !important;
  opacity: 1 !important;
  visibility: visible !important;
  color: var(--kalam-chip-text) !important;
  fill: none !important;
  stroke: var(--kalam-chip-text) !important;
  stroke-width: 1.8 !important;
  stroke-linecap: round !important;
  stroke-linejoin: round !important;
  pointer-events: none !important;
}}
.kalam-chip-icon path, .kalam-chip-icon rect {{
  stroke: var(--kalam-chip-text) !important;
  stroke-width: 1.8 !important;
  stroke-linecap: round !important;
  stroke-linejoin: round !important;
}}
.kalam-chip-icon-fill, .kalam-chip-icon-letters {{
  fill: var(--kalam-chip-text) !important;
  stroke: none !important;
}}
.kalam-chip-icon-fill path, .kalam-chip-icon-letters text {{
  fill: var(--kalam-chip-text) !important;
  stroke: none !important;
}}
.kalam-chip-icon-letters {{
  font-family: -apple-system, BlinkMacSystemFont, "Inter", sans-serif !important;
}}
.kalam-chip-action-accent {{
  color: var(--kalam-chip-accent) !important;
}}
.kalam-chip-action-accent .kalam-chip-icon {{
  color: var(--kalam-chip-accent) !important;
  stroke: var(--kalam-chip-accent) !important;
}}
.kalam-chip-action-accent .kalam-chip-icon path {{
  stroke: var(--kalam-chip-accent) !important;
}}
.kalam-chip-action-accent .kalam-chip-icon-fill path,
.kalam-chip-action-accent .kalam-chip-icon-letters text {{
  fill: var(--kalam-chip-accent) !important;
}}

/* ── dictionary popup inside WebView ── */
#kalam-dict-popup {{
  position: absolute !important;
  z-index: 999998 !important;
  width: 300px !important;
  max-width: 84vw !important;
  background: rgba(22,24,30,0.97) !important;
  color: #fafaf9 !important;
  border: 1px solid rgba(255,255,255,0.10) !important;
  border-radius: 16px !important;
  box-shadow: 0 18px 48px rgba(0,0,0,0.45) !important;
  padding: 0 !important;
  overflow: hidden !important;
  font-family: -apple-system, BlinkMacSystemFont, "Inter", sans-serif !important;
  backdrop-filter: blur(22px) !important;
  -webkit-backdrop-filter: blur(22px) !important;
}}
.kalam-dict-head {{
  display: flex !important;
  justify-content: space-between !important;
  align-items: flex-start !important;
  padding: 12px 14px 8px 14px !important;
  background: rgba(255,255,255,0.04) !important;
}}
.kalam-dict-word {{
  color: #fafaf9 !important;
  font-size: 15px !important;
  font-weight: 700 !important;
  overflow-wrap: anywhere !important;
}}
.kalam-dict-result-head {{
  padding: 10px 14px 0 14px !important;
}}
.kalam-dict-results {{
  max-height: 420px !important;
  overflow-y: auto !important;
}}
.kalam-dict-result + .kalam-dict-result {{
  border-top: 1px solid rgba(255,255,255,0.10) !important;
}}
.kalam-dict-sub {{
  color: rgba(255,255,255,0.45) !important;
  font-size: 10px !important;
  text-transform: uppercase !important;
  letter-spacing: 0.08em !important;
  margin-top: 2px !important;
}}
.kalam-dict-close {{
  background: transparent !important;
  border: none !important;
  color: rgba(250,250,249,0.62) !important;
  cursor: pointer !important;
  font-size: 11px !important;
  font-weight: 600 !important;
  border-radius: 999px !important;
  padding: 4px 8px !important;
}}
.kalam-dict-close:hover {{
  background: rgba(255,255,255,0.08) !important;
  color: #ffffff !important;
}}
.kalam-dict-body {{
  padding: 12px 14px !important;
  font-size: 13px !important;
  line-height: 1.55 !important;
  color: rgba(231,229,228,0.92) !important;
  max-height: 190px !important;
  overflow-y: auto !important;
  white-space: pre-wrap !important;
}}
.kalam-dict-actions {{
  display: flex !important;
  gap: 8px !important;
  padding: 8px 14px 14px 14px !important;
  border-top: 1px solid rgba(255,255,255,0.08) !important;
}}
.kalam-dict-save, .kalam-dict-copy {{
  border: none !important;
  border-radius: 999px !important;
  padding: 7px 12px !important;
  font-size: 12px !important;
  font-weight: 600 !important;
  cursor: pointer !important;
  background: rgba(255,255,255,0.12) !important;
  color: #fafaf9 !important;
}}
.kalam-dict-save:hover, .kalam-dict-copy:hover {{
  background: rgba(255,255,255,0.20) !important;
}}

/* Final override — use the same system-theme tokens as the GTK reader chrome */
#kalam-chip {{
  --kalam-chip-bg: {app_surface};
  --kalam-chip-text: {app_text};
  --kalam-chip-border: {app_border};
  --kalam-chip-hover: {app_surface_2};
  --kalam-chip-shadow: {app_bg};
  --kalam-chip-accent: {app_accent};
  color: var(--kalam-chip-text) !important;
  background: var(--kalam-chip-bg) !important;
}}
#kalam-chip, #kalam-chip * {{
  color: var(--kalam-chip-text) !important;
  -webkit-text-fill-color: var(--kalam-chip-text) !important;
}}
#kalam-chip .kalam-chip-action-accent,
#kalam-chip .kalam-chip-action-accent * {{
  color: var(--kalam-chip-accent) !important;
  -webkit-text-fill-color: var(--kalam-chip-accent) !important;
}}
#kalam-chip .kalam-chip-action-accent .kalam-chip-icon path {{
  stroke: var(--kalam-chip-accent) !important;
}}
#kalam-chip .kalam-chip-action-accent .kalam-chip-icon-fill path,
#kalam-chip .kalam-chip-action-accent .kalam-chip-icon-letters text {{
  fill: var(--kalam-chip-accent) !important;
}}
#kalam-dict-popup, #kalam-dict-popup * {{
  color: #f5f5f4 !important;
  -webkit-text-fill-color: #f5f5f4 !important;
}}
#kalam-dict-popup {{
  background: #1c1917 !important;
  background-color: #1c1917 !important;
}}

/* ── theme-specific image handling (appended last so it wins) ── */
{image_css}
"#,
        bg = bg,
        fg = fg,
        app_bg = chrome_theme.bg,
        app_surface = chrome_theme.surface,
        app_surface_2 = chrome_theme.surface_2,
        app_border = chrome_theme.border,
        app_text = chrome_theme.text,
        app_accent = chrome_theme.accent,
        selection_bg = selection_bg,
        selection_blend = selection_blend,
        handle_color = handle_color,
        font_px = font_px,
        lh = line_height,
        column_px = column_px,
        image_css = image_css,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingTheme {
    Light,
    Sepia,
    Dark,
    Ink,
}

impl ReadingTheme {
    pub fn as_str(self) -> &'static str {
        match self {
            ReadingTheme::Light => "light",
            ReadingTheme::Sepia => "sepia",
            ReadingTheme::Dark => "dark",
            ReadingTheme::Ink => "ink",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "light" => ReadingTheme::Light,
            "dark" => ReadingTheme::Dark,
            "ink" => ReadingTheme::Ink,
            _ => ReadingTheme::Sepia,
        }
    }

    /// Page and ink colours — shared by the reading CSS and the theme buttons
    /// in the typography popover, so a swatch always matches the real page.
    pub fn swatch(self) -> (&'static str, &'static str) {
        match self {
            ReadingTheme::Light => ("#faf8f5", "#1c1917"),
            ReadingTheme::Sepia => ("#f5f0e8", "#2c2820"),
            ReadingTheme::Dark => ("#1b1e24", "#abb2bf"),
            ReadingTheme::Ink => ("#0d0d0d", "#c8c8c8"),
        }
    }

    /// Temporary selection and its separate endpoint-handle colour. The
    /// handles are intentionally high-contrast controls rather than part of
    /// the selection band: near-black on light pages, bright on dark pages.
    pub fn selection_style(self) -> (&'static str, &'static str) {
        match self {
            ReadingTheme::Light => ("rgba(211, 137, 148, 0.42)", "#0b0b0b"),
            ReadingTheme::Sepia => ("rgba(202, 126, 136, 0.44)", "#0b0b0b"),
            ReadingTheme::Dark => ("rgba(184, 93, 112, 0.52)", "#ffd166"),
            ReadingTheme::Ink => ("rgba(204, 104, 132, 0.52)", "#ffd166"),
        }
    }
}
