#!/usr/bin/env python3
"""Regenerate docs/files/kalam_dictionary_popup_v3_preview.html.

Extracts the dictionary popup CSS and JS (including the Phase 6 block
marked [preview:phase6]) verbatim from src/epub_book.rs, substitutes the
app-chrome tokens with the mockup palette, and wraps them in a standalone
demo page. Run:  python3 docs/files/gen_kalam_dict_preview.py
"""
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
SRC = ROOT / "src" / "epub_book.rs"
OUT = ROOT / "docs" / "files" / "kalam_dictionary_popup_v3_preview.html"

text = SRC.read_text()

# ---- CSS: popup section (theme block included), braces unescaped ----
css_start = text.index("/* \u2500\u2500 dictionary popup inside WebView (Phase 5 redesign) \u2500\u2500")
css_end = text.index("/* \u2500\u2500 theme-specific image handling (appended last so it wins) \u2500\u2500 */")
css = text[css_start:css_end].replace("{{", "{").replace("}}", "}")
palette = {
    "{app_surface}": "#21242b",
    "{app_surface_2}": "#282c34",
    "{app_border}": "#343842",
    "{app_text}": "#abb2bf",
    "{app_accent}": "#61afef",
}
for token, color in sorted(palette.items(), key=lambda kv: -len(kv[0])):
    css = css.replace(token, color)

# ---- JS: popup block + Phase 6 block, verbatim ----
js_start = text.index("  function ensureDictPopup() {")
js_end = text.index("  // [preview:/phase6]") + len("  // [preview:/phase6]")
js = text[js_start:js_end]

bank_payload = {
    "word": "bank",
    "pos": ["noun", "verb"],
    "senses": [
        {"number": 1, "pos": "noun", "def": "sloping land (especially the slope beside a body of water)", "example": None},
        {"number": 2, "pos": "noun", "def": "a financial institution that accepts deposits and channels the money into lending activities", "example": None},
        {"number": 3, "pos": "noun", "def": "a long ridge or pile", "example": None},
        {"number": 4, "pos": "noun", "def": "an arrangement of similar objects in a row or in tiers", "example": None},
        {"number": 5, "pos": "noun", "def": "a supply or stock held in reserve for future use (especially in emergencies)", "example": None},
        {"number": 6, "pos": "noun", "def": "the funds held by a gambling house or the dealer in some gambling games", "example": None},
        {"number": 7, "pos": "noun", "def": "a slope in the turn of a road or track; the outside is higher than the inside in order to reduce the effects of centrifugal force", "example": None},
        {"number": 8, "pos": "noun", "def": "a container (usually with a slot in the top) for keeping money at home", "example": None},
        {"number": 9, "pos": "noun", "def": "a building in which the business of banking is transacted", "example": None},
        {"number": 10, "pos": "noun", "def": "a flight maneuver; aircraft tips laterally about its longitudinal axis (especially in turning)", "example": None},
        {"number": 11, "pos": "verb", "def": "tip laterally", "example": None},
        {"number": 12, "pos": "verb", "def": "enclose with a bank", "example": None},
        {"number": 13, "pos": "verb", "def": "do business with a bank or keep an account at a bank", "example": None},
        {"number": 14, "pos": "verb", "def": "act as the banker in a game or in gambling", "example": None},
        {"number": 15, "pos": "verb", "def": "be in the banking business", "example": None},
        {"number": 16, "pos": "verb", "def": "put into a bank account", "example": None},
        {"number": 17, "pos": "verb", "def": "cover with ashes so to control the rate of burning", "example": None},
        {"number": 18, "pos": "verb", "def": "have faith or confidence in", "example": None},
    ],
    "synonyms": ["bank building", "savings bank", "coin bank", "money box",
                 "depository financial institution", "banking concern",
                 "banking company", "cant", "camber", "trust", "swear", "rely"],
    "antonyms": [],
    "idioms": [{"phrase": "Piece of cake", "def": "Easy."}],
    "suggestions": [],
    "saved": True,
    "hint": 1,
    "pronunciation": "/\u02c8b\u00e6\u014bk/",
}
import json
payload_json = json.dumps(bank_payload, ensure_ascii=True)

html = f"""<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>Kalam dictionary popup — Phase 6 preview</title>
<style>
html, body {{ margin:0; padding:0; background:#12141a; height:100%; }}
body {{ font-family: -apple-system, 'Segoe UI', Roboto, sans-serif; }}
#book {{ max-width: 640px; margin: 0 auto; padding: 60px 40px; color:#8b93a3; font-size:17px; line-height:1.7; }}
#book b {{ color:#d6dae3; }}
#book code {{ color:#e5c07b; }}
/* find-in-chapter hits (same rule as the reading skin, palette applied) */
.kalam-search-hit {{
  background: color-mix(in srgb, #61afef 32%, transparent) !important;
  background-color: color-mix(in srgb, #61afef 32%, transparent) !important;
  border-radius: 3px !important;
  box-shadow: 0 0 0 1px color-mix(in srgb, #61afef 55%, transparent) !important;
  padding: 0 1px !important;
  box-decoration-break: clone !important;
  -webkit-box-decoration-break: clone !important;
}}
{css}</style></head>
<body>
<div id="book">
<p>Select <b>bank</b> (or click the chips, or just <b>tap any word</b> in the paragraph below) to open
the dictionary popup. This preview renders the exact CSS and JS from <code>src/epub_book.rs</code> with the
mockup palette substituted for the app-chrome tokens (<code>{{app_surface}}</code> etc.), so what you see
here is what the reader shows in a dark theme.</p>
<p>Read: "I need to go to the <b>bank</b> to deposit a cheque." — the Lesk ranking excludes the word itself,
stems both sides, and the gloss evidence ("deposit") marks the financial sense as <i>likely here</i> (sense 2).</p>
<p id="demo-text">Tap any word here: the children played on the muddy bank of the stream. Then the bank manager opened a new bank account.</p>
<p>Open the popup below the word: <span id="here" style="font-size:20px;font-weight:600;color:#d6dae3;cursor:pointer">bank</span></p>
</div>
<script>
// Self-contained demo bridge: a real lookup responds with the bank entry
// (the webview's real bridge round-trips through Rust instead).
window.kalamBridge = function(payload) {{
  if (payload && payload.type === 'dict-lookup' && payload.word) {{
    var known = String(payload.word).toLowerCase() === 'bank';
    var p = known ? {payload_json} : {{word: payload.word, pos: [], senses: [], synonyms: [], antonyms: [], idioms: [], suggestions: [], saved: false}};
    window.kalamShowDict(p, JSON.stringify(payload.rect || null));
  }}
}};
// Preview-only stubs for the reader helpers the real webview provides.
function caretFromPoint(x, y) {{
  try {{
    if (document.caretRangeFromPoint) {{
      var range = document.caretRangeFromPoint(x, y);
      if (range) return {{node: range.startContainer, offset: range.startOffset}};
    }}
  }} catch(e) {{}}
  try {{
    if (document.caretPositionFromPoint) {{
      var position = document.caretPositionFromPoint(x, y);
      if (position) return {{node: position.offsetNode, offset: position.offset}};
    }}
  }} catch(e) {{}}
  return null;
}}
function getSelectionData() {{
  return {{
    text: window.__selText || 'bank',
    rect: {{x: 0, y: 40, w: 40, h: 24}}
  }};
}}
function hideChip() {{}}
function hideSelectionHandles() {{}}
{js}
window.kalamHandleDict = function() {{
    var data = getSelectionData();
    var word = '';
    var ctx = '';
    var rect = null;
    if (data) {{
      // Keep a short phrase intact. The dictionary can contain multi-word
      // entries, and reducing the selection to its first word made phrase
      // lookup depend on an arbitrary selection boundary.
      word = data.text.replace(/\\s+/g, ' ').trim();
      ctx = getContextSentence() || data.text;
      rect = data.rect;
    }} else {{
      var sel = window.getSelection();
      if (sel && sel.toString()) {{
        var selectedText = sel.toString();
        word = selectedText.replace(/\\s+/g, ' ').trim();
        ctx = getContextSentence() || selectedText;
        try {{ var r = sel.getRangeAt(0).getBoundingClientRect(); rect = {{x:r.left, y:r.top, w:r.width, h:r.height, bottom:r.bottom}}; }} catch(e){{}}
      }}
    }}
    if (!word) return;
    hideChip();
    hideSelectionHandles();
    clearSearchHits();
    // Keep the selection bands visible: the popup is placed clear of the
    // selection, so the lookup target stays highlighted underneath.
    kalamBridge({{type:'dict-lookup', word:word, context:ctx, rect:rect}});
  }}
function openPopup() {{
  var r = {{x: 0, y: 40, w: 40, h: 24}};
  window.kalamShowDict({payload_json}, JSON.stringify(r));
}}
document.getElementById('here').addEventListener('click', openPopup);
window.addEventListener('keydown', function(e){{
  if (handleDictPopupKey(e)) return;
  if (e.key === 'Escape') window.kalamHideDict();
}});
openPopup();
</script>
"""

OUT.write_text(html)
print(f"preview regenerated -> {OUT} ({len(html)} bytes)")
