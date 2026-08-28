#!/usr/bin/env python3
"""P5.5 mockup — Kalam Settings, reconciled design.

Renders the settings window in the new design language:
  - 52px app rail (logo top, nav centred, Settings bottom)
  - 210px grouped settings nav (APPEARANCE / LIBRARY / SOURCES / APP)
  - section cards (icon + title + desc + divider + rows)
  - theme picker grouped by family with mini UI previews
  - every row is a REAL shipped feature (nothing faked)

Outputs:
  docs/design/settings_window.png   — 960x640 window, Appearance panel
  docs/design/settings_panels.png   — all other panels, full width, stacked
"""
from PIL import Image, ImageDraw, ImageFont, ImageColor

S = 2  # supersample factor; final image is downscaled for antialiasing

# ── One Dark Darker (app default) — values straight from src/theme.rs ──
T = {
    "canvas":  "#0d0f12",
    "bg":      "#1b1e24",
    "sidebar": "#15171c",
    "surface": "#21242b",
    "surface2": "#282c34",
    "border":  "#343842",
    "text":    "#abb2bf",
    "dim":     "#767d8a",
    "accent":  "#61afef",
    "danger":  "#e06c75",
    "success": "#98c379",
    "warning": "#e5c07b",
    "info":    "#c678dd",
}

FONT = {
    "ui":   "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "uib":  "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    "mono": "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "monob": "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf",
}
_cf = {}
def F(kind, px):
    key = (kind, px)
    if key not in _cf:
        _cf[key] = ImageFont.truetype(FONT[kind], int(px * S))
    return _cf[key]

def P(v): return int(round(v * S))

def blend(fg, bg, a):
    """Blend fg over bg by alpha a (0..1). Returns hex."""
    f = ImageColor.getrgb(fg); b = ImageColor.getrgb(bg)
    return "#%02x%02x%02x" % tuple(round(f[i]*a + b[i]*(1-a)) for i in range(3))

def rrect(d, box, r, fill=None, outline=None, width=1):
    d.rounded_rectangle([P(v) for v in box], radius=P(r),
                        fill=fill, outline=outline, width=P(width))

def text(d, xy, s, kind, px, fill, anchor=None):
    d.text([P(xy[0]), P(xy[1])], s, font=F(kind, px), fill=fill, anchor=anchor)

def text_w(d, xy, s, kind, px, fill):
    """Return right edge x of drawn text (logical px)."""
    return xy[0] + d.textlength(s, font=F(kind, px)) / S

def text_h(kind, px):
    bbox = F(kind, px).getbbox("Ag")
    return (bbox[3] - bbox[1]) / S

def tracked(d, xy, s, kind, px, fill, tracking=0.07):
    """Uppercase overline with letter-spacing."""
    x = xy[0]
    for ch in s:
        text(d, (x, xy[1]), ch, kind, px, fill)
        x += d.textlength(ch, font=F(kind, px)) / S + px * tracking
    return x

def center(d, cx, y, s, kind, px, fill):
    w = d.textlength(s, font=F(kind, px)) / S
    text(d, (cx - w/2, y), s, kind, px, fill)

def ellipsize(d, s, kind, px, maxw):
    if d.textlength(s, font=F(kind, px)) / S <= maxw:
        return s
    out = s
    while out and d.textlength(out + "…", font=F(kind, px)) / S > maxw:
        out = out[:-1]
    return out + "…"

def wrap_lines(d, s, kind, px, maxw):
    """Greedy word wrap; returns list of lines."""
    lines, cur = [], ""
    for wd in s.split():
        t = (cur + " " + wd).strip()
        if d.textlength(t, font=F(kind, px)) / S <= maxw:
            cur = t
        else:
            if cur:
                lines.append(cur)
            cur = wd
    if cur:
        lines.append(cur)
    return lines or [""]

# ── component library ────────────────────────────────────────────────

def chip(d, x, y, s, bg, fg):
    w = d.textlength(s, font=F("uib", 10)) / S + 2*9
    rrect(d, (x, y, x + w, y + 18), 9, fill=bg)
    center(d, x + w/2, y + 4, s, "uib", 10, fg)
    return x + w

def btn(d, x, y, label, kind="outlined", sm=False):
    px, pad_x, pad_y = (11.5, 14, 6) if sm else (12.5, 18, 8)
    fnt = "uib" if kind in ("filled",) else "ui"
    w = d.textlength(label, font=F(fnt, px)) / S + 2*pad_x
    h = 2*pad_y + text_h(fnt, px)
    if kind == "filled":
        rrect(d, (x, y, x+w, y+h), h/2, fill=T["accent"])
        text(d, (x+w/2, y+pad_y-0.5), label, fnt, px, T["bg"], anchor="ma")
    elif kind == "outlined":
        rrect(d, (x, y, x+w, y+h), h/2, outline=T["border"], width=1)
        text(d, (x+w/2, y+pad_y-0.5), label, fnt, px, T["text"], anchor="ma")
    elif kind == "danger":
        rrect(d, (x, y, x+w, y+h), h/2, outline=T["danger"], width=1)
        text(d, (x+w/2, y+pad_y-0.5), label, fnt, px, T["danger"], anchor="ma")
    elif kind == "ghost":
        text(d, (x+w/2, y+pad_y-0.5), label, fnt, px, T["dim"], anchor="ma")
    return w, h

def switch(d, x, y, on=True):
    w, h = 40, 22
    rrect(d, (x, y, x+w, y+h), h/2,
          fill=T["accent"] if on else T["border"])
    kx = x + (w - 19 if on else 3)
    d.ellipse([P(kx), P(y+3), P(kx+16), P(y+19)], fill="#ffffff")
    return w

def path_box(d, x, y, w, s):
    rrect(d, (x, y, x+w, y+32), 8, fill=T["surface2"], outline=T["border"])
    text(d, (x+12, y+8), ellipsize(d, s, "mono", 11, w-24), "mono", 11, T["dim"])

def icon_block(d, x, y, glyph, bg, fg, size=32):
    rrect(d, (x, y, x+size, y+size), 8, fill=bg)
    center(d, x+size/2, y+size/2-8, glyph, "ui", 15, fg)
    return size

def badge(d, x, y, s, kind="ok"):
    cols = {"ok": (blend(T["success"], T["surface"], .16), T["success"]),
            "err": (blend(T["danger"], T["surface"], .16), T["danger"]),
            "info": (blend(T["info"], T["surface"], .16), T["info"]),
            "neutral": (T["surface2"], T["dim"])}
    return chip(d, x, y, s, *cols[kind])

# ── panels ───────────────────────────────────────────────────────────

def page_head(d, x, y, w, title, sub):
    text(d, (x, y), title, "uib", 20, T["text"])
    lines = wrap_lines(d, sub, "ui", 12.5, w)
    for i, ln in enumerate(lines):
        text(d, (x, y + 30 + i*18), ln, "ui", 12.5, T["dim"])
    return y + 30 + (len(lines)-1)*18 + 24

def section_card(d, x, y, w, glyph, title, desc, rows, footer=None):
    """rows: list of draw fns (d, x, y, w) -> height. Returns y after card."""
    desc_lines = wrap_lines(d, desc, "ui", 12, w-88) if desc else []
    head_h = (46 + len(desc_lines)*17) if desc else 40
    body_y = y + head_h + 19  # head + divider margins

    # measure pass on a scratch canvas so the card bg can be drawn first
    scratch = Image.new("RGB", (P(w + 80), P(12000)), "#000000")
    sd = ImageDraw.Draw(scratch)
    ry = body_y
    for i, row in enumerate(rows):
        ry += row(sd, x+20, ry, w-40)
        if i < len(rows) - 1:
            ry += 19
    if footer:
        ry += footer(sd, x+20, ry, w-40)
    ry += 20  # bottom padding

    rrect(d, (x, y, x+w, ry), 16, fill=T["surface"], outline=T["border"])
    d.rectangle([P(x+20), P(y+head_h-2), P(x+w-20), P(y+head_h-1)],
                fill=T["border"])
    text(d, (x+20, y+17), glyph, "ui", 16, T["accent"])
    text(d, (x+20+24, y+15), title, "uib", 14, T["text"])
    for i, ln in enumerate(desc_lines):
        text(d, (x+20+24, y+35+i*17), ln, "ui", 12, T["dim"])
    ry = body_y
    for i, row in enumerate(rows):
        ry += row(d, x+20, ry, w-40)
        if i < len(rows) - 1:
            d.rectangle([P(x+20), P(ry+9), P(x+w-20), P(ry+10)],
                        fill=T["border"])
            ry += 19
    if footer:
        ry += footer(d, x+20, ry, w-40)
    return ry + 20 + 16

def row(d, x, y, w, label, desc, control, ctrl_w):
    """Standard settings row: label+desc left, control right-aligned."""
    lh = text_h("ui", 13) + 2
    dh = 0
    if desc:
        words, lines, cur = desc.split(), [], ""
        for wd in words:
            t = (cur + " " + wd).strip()
            if d.textlength(t, font=F("ui", 11.5)) / S <= w - ctrl_w - 16:
                cur = t
            else:
                lines.append(cur); cur = wd
        lines.append(cur)
        dh = len(lines) * 16
        for i, ln in enumerate(lines):
            text(d, (x, y + lh + 2 + i*16), ln, "ui", 11.5, T["dim"])
    text(d, (x, y), label, "uib", 13, T["text"])
    h = max(lh + dh, control(d, x + w, y + max(0, (lh + dh - 30)//2)))
    return h + 24

# ── Appearance: theme picker ─────────────────────────────────────────

THEMES = [
    ("One Dark", "Warm-tinted greys, soft blue accent. The default.",
     [("Normal", None, "#282c34", "#21252b", "#2c313a", "#333842", "#3e4451",
       "#abb2bf", "#7f8794", "#61afef"),
      ("Darker", True, "#1b1e24", "#15171c", "#21242b", "#282c34", "#343842",
       "#abb2bf", "#767d8a", "#61afef")]),
    ("Tokyo Night", "Deep indigo, high-chroma violet and blue accents.",
     [("Normal", None, "#1a1b26", "#16161e", "#1f2335", "#272b3f", "#2f334d",
       "#c0caf5", "#787c99", "#7aa2f7"),
      ("Night", None, "#16161e", "#101014", "#1a1b26", "#22232f", "#292e42",
       "#c0caf5", "#737aa2", "#7aa2f7")]),
    ("Everforest", "Low-saturation greens and warm greys. Easy on the eyes.",
     [("Normal", None, "#2d353b", "#272e33", "#343f44", "#3d484d", "#475258",
       "#d3c6aa", "#9da9a0", "#a7c080"),
      ("Hard", None, "#232a2e", "#1e2326", "#2d353b", "#343f44", "#3d484d",
       "#d3c6aa", "#859289", "#a7c080")]),
    ("Catppuccin Mocha", "Soft pastels on a near-black lavender base.",
     [("Normal", None, "#1e1e2e", "#181825", "#232338", "#313244", "#45475a",
       "#cdd6f4", "#9399b2", "#89b4fa"),
      ("Crust", None, "#11111b", "#0b0b13", "#181825", "#1e1e2e", "#313244",
       "#cdd6f4", "#7f849c", "#89b4fa")]),
    ("Gruvbox", "Warm retro browns and ochres — a classic.",
     [("Normal", None, "#282828", "#1d2021", "#32302f", "#3c3836", "#504945",
       "#ebdbb2", "#a89984", "#83a598"),
      ("Hard", None, "#1d2021", "#141617", "#282828", "#32302f", "#3c3836",
       "#ebdbb2", "#928374", "#83a598")]),
    ("Ayu", "Muted slate with a distinctive amber accent.",
     [("Mirage", None, "#1f2430", "#1a1f29", "#242936", "#282e3a", "#3a4251",
       "#cbccc6", "#7d8491", "#ffcc66"),
      ("Dark", None, "#0f1419", "#0b0e13", "#151a1e", "#1c2228", "#273038",
       "#bfbdb6", "#7b8288", "#e6b450")]),
    ("Nord", "Cool arctic blue-greys. No darker variant — already deep.",
     [("Nord", None, "#2e3440", "#272b35", "#3b4252", "#434c5e", "#4c566a",
       "#eceff4", "#a0a8b7", "#88c0d0")]),
]

def theme_card(d, x, y, w, name, selected, bg, sidebar, surface, surface2,
               border, text_c, dim, accent):
    h = 108
    card_bg = blend(accent, bg, .05) if selected else bg
    rrect(d, (x, y, x+w, y+h), 8, fill=card_bg,
          outline=accent if selected else border)
    text(d, (x+10, y+8), name, "uib", 11.5, T["text"])
    if selected:
        nx = x + 10 + d.textlength(name, font=F("uib", 11.5)) / S
        text(d, (nx+4, y+8), "default" if name == "Darker" else "current",
             "ui", 10, T["dim"])
    py = y + 24
    pw = w - 20
    # mini preview
    rrect(d, (x+10, py, x+10+pw, py+40), 6, fill=sidebar, outline=blend("#000000", bg, .35))
    rail_w = 16
    d.rectangle([P(x+10), P(py), P(x+10+rail_w), P(py+40)], fill=sidebar)
    dot_y = py + 6
    d.ellipse([P(x+10+5), P(dot_y), P(x+10+11), P(dot_y+6)], fill=accent)
    d.ellipse([P(x+10+5), P(dot_y+10), P(x+10+11), P(dot_y+16)], fill=border)
    mx = x + 10 + rail_w
    mw = pw - rail_w
    text_bg = blend(surface2, sidebar, .85)
    rrect(d, (mx+6, py+6, mx+mw-6, py+6+4), 2, fill=text_bg)
    rrect(d, (mx+6, py+13, mx+mw-6, py+13+14), 4, fill=surface, outline=border)
    rrect(d, (mx+6, py+30, mx+mw-6, py+30+4), 2, fill=text_bg)
    # swatches
    sy = py + 40 + 6
    sw = (pw - 4*4) / 5
    for i, c in enumerate([surface2, surface, border, accent, text_c]):
        rrect(d, (x+10+i*(sw+4), sy, x+10+i*(sw+4)+sw, sy+5), 2.5, fill=c)
    if selected:
        cx, cy = x + w - 18, y + 7
        d.ellipse([P(cx), P(cy), P(cx+16), P(cy+16)], fill=accent)
        text(d, (cx+8, cy+8), "✓", "uib", 9, bg, anchor="mm")
    return h

def family_block(d, x, y, w, fam):
    name, desc, variants = fam
    pad = 14
    inner = w - 2*pad
    card_w = (inner - 8) / 2
    lines = wrap_lines(d, desc, "ui", 11, inner)
    cy = y + 30 + len(lines)*15 + 8
    h = (cy - y) + 108 + 14
    rrect(d, (x, y, x+w, y+h), 12, fill=T["surface2"], outline=T["border"])
    text(d, (x+pad, y+14), name, "uib", 12, T["text"])
    for i, ln in enumerate(lines):
        text(d, (x+pad, y+32+i*15), ln, "ui", 11, T["dim"])
    for i, v in enumerate(variants):
        theme_card(d, x + pad + i*(card_w+8), cy, card_w, *v)
    return h

def appearance_panel(d, x, y, w):
    y = page_head(d, x, y, w, "Appearance",
                  "Every theme is dark. Changes apply immediately.")
    for row_i in range(0, 7, 2):
        pair = THEMES[row_i:row_i+2]
        col_w = (w - 12) / 2
        maxh = 0
        rendered = []
        for j, fam in enumerate(pair):
            fx = x + j*(col_w+12)
            h = family_block(d, fx, y, col_w, fam)
            rendered.append((fx, h))
            maxh = max(maxh, h)
        y += maxh + 12
    return y

def storage_panel(d, x, y, w):
    y = page_head(d, x, y, w, "Storage & Backup",
                  "Where Kalam keeps your books and catalog.")
    rows = []
    def path_row(d, x, y, w, label, desc, path):
        def control(d, cx, cy):
            path_box(d, cx - 340, cy, 340, path)
            return 32
        return row(d, x, y, w, label, desc, control, 340)
    rows.append(lambda d, x, y, w: path_row(d, x, y, w,
        "Data Directory", "Base folder for Kalam application data",
        "~/.local/share/kalam"))
    rows.append(lambda d, x, y, w: path_row(d, x, y, w,
        "Catalog Database",
        "SQLite database storing library books, shelves, tags, and reading progress",
        "~/.local/share/kalam/catalog.db"))
    rows.append(lambda d, x, y, w: path_row(d, x, y, w,
        "Library Files", "Directory where EPUB books and extracted covers are stored",
        "~/.local/share/kalam/library"))
    rows.append(lambda d, x, y, w: path_row(d, x, y, w,
        "Dictionaries Directory",
        "Location for offline StarDict, SQLite, and TSV dictionary packs",
        "~/.local/share/kalam/dictionaries"))
    y = section_card(d, x, y, w, "☷", "Data locations",
                     "These paths are set at first run. Moving data requires copying the files manually.",
                     rows)

    def backup_row(d, x, y, w):
        def control(d, cx, cy):
            bw, bh = btn(d, cx-140, cy, "Back up library…", "outlined")
            return bh
        return row(d, x, y, w, "Back up library database",
                   "Creates a snapshot of catalog.db containing all metadata, annotations, shelves, ratings, and reading history.",
                   control, 140)
    def cache_row(d, x, y, w):
        def control(d, cx, cy):
            chip(d, cx-190, cy, "12.4 MB", T["surface2"], T["dim"])
            bw, bh = btn(d, cx-100, cy+6, "Clear cache", "danger")
            return bh
        return row(d, x, y, w, "Extracted EPUB cache",
                   "Temporary files extracted for the WebKitGTK reader. Safe to clear; books re-extract on open.",
                   control, 190)
    y = section_card(d, x, y, w, "↓", "Backup & cache", None,
                     [backup_row, cache_row])

    def export_row(d, x, y, w):
        def control(d, cx, cy):
            bw, bh = btn(d, cx-120, cy, "Export quotes", "outlined")
            return bh
        return row(d, x, y, w, "Export quotes to Markdown",
                   "Saves all saved quotes to ~/Quotes.md", control, 120)
    y = section_card(d, x, y, w, "⧉", "Export", None, [export_row])
    return y

def dict_panel(d, x, y, w):
    y = page_head(d, x, y, w, "Dictionaries",
                  "Offline packs used for word lookup in the reader.")
    def dict_row(d, x, y, w, name, meta):
        icon_block(d, x, y+2, "📖", blend(T["info"], T["surface"], .14), T["info"])
        text(d, (x+42, y+1), name, "ui", 13, T["text"])
        text(d, (x+42, y+20), meta, "mono", 11, T["dim"])
        bw, bh = btn(d, x+w-74, y+6, "Remove", "danger", sm=True)
        return 36 + 24
    def import_footer(d, x, y, w):
        bw, bh = btn(d, x, y, "+  Import dictionary", "filled")
        return bh + 10
    rows = [lambda d, x, y, w: dict_row(d, x, y, w, "English — WordNet 3.1",
                                         "SQLite · 147,000 entries"),
            lambda d, x, y, w: dict_row(d, x, y, w, "English — Webster 1913",
                                         "StarDict · 99,100 entries")]
    y = section_card(d, x, y, w, "✎", "Installed packs",
                     "StarDict (.ifo/.idx/.dict), SQLite (.db), and TSV formats are supported.",
                     rows, footer=import_footer)
    return y

def bookfiles_panel(d, x, y, w):
    y = page_head(d, x, y, w, "Book Files",
                  "How Kalam handles imported files and metadata writeback.")
    def write_row(d, x, y, w):
        def control(d, cx, cy):
            return switch(d, cx-40, cy+2)
        return row(d, x, y, w, "Also write metadata into the EPUB file",
                   "On: saving in Edit metadata also updates the book file, so Calibre and other readers see your changes. The untouched original is kept once as <name>.epub.orig.",
                   control, 40)
    def backups_row(d, x, y, w):
        def control(d, cx, cy):
            chip(d, cx-190, cy, "3 originals · 4.2 MB", T["surface2"], T["dim"])
            bw, bh = btn(d, cx-110, cy+6, "Delete backups", "danger")
            return bh
        return row(d, x, y, w, "Original backups",
                   "Kept once per book when writeback is on. Deleting is permanent: you lose the ability to undo metadata written into those files.",
                   control, 190)
    y = section_card(d, x, y, w, "☰", "EPUB writeback", None,
                     [write_row, backups_row])
    return y

def sources_panel(d, x, y, w):
    y = page_head(d, x, y, w, "Metadata Sources",
                  "Used by Edit metadata. Results from every enabled source are merged and badged with their origin.")
    def ol_row(d, x, y, w):
        def control(d, cx, cy):
            return switch(d, cx-40, cy+2)
        return row(d, x, y, w, "Enable Open Library lookup",
                   "Adds a search panel inside the metadata editor.", control, 40)
    y = section_card(d, x, y, w, "★", "Open Library",
                     "Internet Archive. No key needed. Strong on older and public-domain titles.",
                     [ol_row])
    def gb_row(d, x, y, w):
        def control(d, cx, cy):
            return switch(d, cx-40, cy+2)
        return row(d, x, y, w, "Enable Google Books lookup",
                   "Adds results from Google Books to the metadata editor.", control, 40)
    def key_row(d, x, y, w):
        def control(d, cx, cy):
            rrect(d, (cx-300, cy, cx-78, cy+32), 8, fill=T["surface2"],
                  outline=T["border"])
            text(d, (cx-290, cy+8), "••••••••••••••", "mono", 11, T["dim"])
            bw, bh = btn(d, cx-70, cy, "Save key", "outlined", sm=True)
            return bh
        return row(d, x, y, w, "API key",
                   "Optional — lifts the shared rate limit. Free from console.cloud.google.com.",
                   control, 300)
    def country_row(d, x, y, w):
        def control(d, cx, cy):
            rrect(d, (cx-160, cy, cx-104, cy+32), 8, fill=T["surface2"],
                  outline=T["border"])
            text(d, (cx-148, cy+8), "IN", "mono", 11, T["dim"])
            bw, bh = btn(d, cx-96, cy, "Save", "outlined", sm=True)
            return bh
        return row(d, x, y, w, "Country",
                   "Two-letter code. Google only serves results for countries it has rights in.",
                   control, 160)
    y = section_card(d, x, y, w, "★", "Google Books",
                     "Broad coverage, good for recent and non-English books. Works without a key, but anonymous requests share a global quota and can be rate limited.",
                     [gb_row, key_row, country_row])
    return y

def notif_panel(d, x, y, w):
    y = page_head(d, x, y, w, "Notifications",
                  "History log of recent activity, alerts, and toasts.")
    def entry(d, x, y, w, kind, title, detail, when):
        badge(d, x, y+2, kind, {"OK": "ok", "ERR": "err", "i": "info"}[kind])
        text(d, (x+44, y+1), title, "ui", 13, T["text"])
        text(d, (x+44, y+20), ellipsize(d, detail, "ui", 11.5, w-160),
             "ui", 11.5, T["dim"])
        text(d, (x+w-40, y+1), when, "ui", 11, T["dim"], anchor="ra")
        return 48
    rows = [
        lambda d, x, y, w: entry(d, x, y, w, "OK", "Theme changed",
                                 "One Dark Darker", "14:32"),
        lambda d, x, y, w: entry(d, x, y, w, "OK", "Library backed up",
                                 "24.1 MB · ~/Backups/kalam-backup-2026-07-28.db", "11:47"),
        lambda d, x, y, w: entry(d, x, y, w, "ERR", "Backup failed",
                                 "Permission denied: /run/media/kalam-usb", "09:15"),
        lambda d, x, y, w: entry(d, x, y, w, "OK", "Added to reading list",
                                 "The Name of the Wind", "09:02"),
        lambda d, x, y, w: entry(d, x, y, w, "i", "Cleaned up reader cache",
                                 "Freed 312.4 MB", "Yesterday"),
    ]
    def clear_footer(d, x, y, w):
        bw, bh = btn(d, x, y, "Clear history", "ghost", sm=True)
        return bh + 6
    y = section_card(d, x, y, w, "●", "Activity log",
                     "The last 25 events this session. Toasts fade; this keeps the record.",
                     rows, footer=clear_footer)
    return y

# ── chrome: app rail + settings nav ──────────────────────────────────

def app_rail(d, ox, oy, h, active="settings"):
    rrect(d, (ox, oy, ox+52, oy+h), 16, fill=T["sidebar"],
          outline=T["border"])
    d.rectangle([P(ox+52), P(oy), P(ox+53), P(oy+h)], fill=T["border"])
    # logo
    rrect(d, (ox+12, oy+14, ox+40, oy+42), 8, fill=blend(T["accent"], T["sidebar"], .15))
    center(d, ox+26, oy+20, "K", "uib", 13, T["accent"])
    items = [("⌂", False), ("☰", False), ("▦", False), ("↓", False)]
    y = oy + 56
    for glyph, act in items:
        bg = blend(T["accent"], T["sidebar"], .15) if act else None
        d.ellipse([P(ox+8), P(y), P(ox+44), P(y+36)], fill=bg)
        center(d, ox+26, y+7, glyph, "ui", 16,
               T["accent"] if act else T["dim"])
        y += 40
    y = oy + h - 46
    d.ellipse([P(ox+8), P(y), P(ox+44), P(y+36)],
              fill=blend(T["accent"], T["sidebar"], .15))
    center(d, ox+26, y+7, "⚙", "ui", 16, T["accent"])

SETTINGS_NAV = [
    ("APPEARANCE", [("◎", "Appearance", True)]),
    ("LIBRARY", [("☷", "Storage & Backup", False),
                 ("✎", "Dictionaries", False),
                 ("☰", "Book Files", False)]),
    ("SOURCES", [("★", "Metadata Sources", False)]),
    ("APP", [("●", "Notifications", False)]),
]

def settings_nav(d, ox, oy, h):
    d.rectangle([P(ox), P(oy), P(ox+210), P(oy+h)], fill=T["surface"])
    d.rectangle([P(ox+209), P(oy), P(ox+210), P(oy+h)], fill=T["border"])
    y = oy + 20
    first = True
    for group, items in SETTINGS_NAV:
        if not first:
            y += 12
        first = False
        tracked(d, (ox+14, y), group, "uib", 10, T["dim"], tracking=0.07)
        y += 18
        for glyph, label, active in items:
            if active:
                rrect(d, (ox+12, y, ox+198, y+36), 12,
                      fill=blend(T["accent"], T["surface"], .15))
            text(d, (ox+24, y+8), glyph, "ui", 16,
                 T["accent"] if active else T["dim"])
            text(d, (ox+46, y+9), label, "uib" if active else "ui", 13,
                 T["accent"] if active else T["dim"])
            y += 40
        y += 6

# ── window assembly ──────────────────────────────────────────────────

def render_window(path):
    W, H = 960, 640
    CW, CH = 1200, 840
    img = Image.new("RGB", (P(CW), P(CH)), T["canvas"])
    d = ImageDraw.Draw(img)
    wx, wy = (CW - W)//2, (CH - H)//2

    # window surface
    d.rounded_rectangle([P(wx), P(wy), P(wx+W), P(wy+H)], radius=P(20),
                        fill=T["bg"], outline=T["border"], width=P(1))
    app_rail(d, wx, wy, H)
    settings_nav(d, wx+52, wy, H)
    # content
    cx, cw = wx + 52 + 210, W - 52 - 210
    inner_x, inner_w = cx + 32, cw - 64
    content = Image.new("RGB", (P(cw), P(H)), T["bg"])
    cd = ImageDraw.Draw(content)
    y = appearance_panel(cd, 32, 28, inner_w)
    # scrollbar hint
    track_h = H - 24
    thumb_h = max(60, int(track_h * (track_h / max(y, 1))))
    thumb_y = 8
    cd.rounded_rectangle([P(cw-5), P(thumb_y), P(cw), P(thumb_y+thumb_h)],
                         radius=P(2.5), fill=T["border"])
    img.paste(content, (P(cx), P(wy)))
    img.save(path)
    img = img.resize((CW, CH), Image.LANCZOS)
    img.save(path)

def render_panels(path):
    W = 820
    panels = [("Appearance", appearance_panel),
              ("Storage & Backup", storage_panel),
              ("Dictionaries", dict_panel),
              ("Book Files", bookfiles_panel),
              ("Metadata Sources", sources_panel),
              ("Notifications", notif_panel)]
    # measure heights first
    pad = 32
    x, w = 40, W - 80
    ys = []
    y = 40
    heights = []
    tmp = Image.new("RGB", (10, 10))
    for i, (name, fn) in enumerate(panels):
        label_h = 40
        y += label_h
        # measure by drawing into a tall scratch
        scratch = Image.new("RGB", (P(w+80), P(20000)))
        sd = ImageDraw.Draw(scratch)
        end = fn(sd, 0, 0, w)
        heights.append(end)
        y += end + 24
    H = int(y + 40)
    img = Image.new("RGB", (P(W), P(H)), T["canvas"])
    d = ImageDraw.Draw(img)
    y = 40
    for i, (name, fn) in enumerate(panels):
        tracked(d, (x, y+8), name, "uib", 11, T["dim"], tracking=0.07)
        fn(d, x, y + 40, w)
        y += 40 + heights[i] + 24
    img = img.resize((W, H), Image.LANCZOS)
    img.save(path)

if __name__ == "__main__":
    import os
    out = os.path.join(os.path.dirname(os.path.abspath(__file__)))
    render_window(os.path.join(out, "settings_window.png"))
    render_panels(os.path.join(out, "settings_panels.png"))
    print("wrote settings_window.png and settings_panels.png")
