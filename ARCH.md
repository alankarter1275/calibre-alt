# Kalam — architecture notes (living doc)

## Product

Personal Linux-only ebook manager + reader. One process. No plugin host, no content server.

**Name:** Kalam  
**Stack:** Rust · GTK4 · Relm4 · (later) WebKitGTK · SQLite · MuPDF for PDF  

## Navigation (P0)

```text
Sidebar
  Home
  Library  → hub tiles → section lists → Book page
  Shelves  → 2-col grid → Shelf detail → Book page
  Downloads / Comics / AO3 / Fanfic / Settings  (placeholders)
```

Book can open as:

1. **Page** in the main column (stack + Back), or  
2. **Float** — separate non-modal window with the same book page.

## Shelves UX (agreed)

1. Sidebar **Shelves** → grid (2 columns) of all shelves  
2. Each card: name, book count, smart/manual, rule/description  
3. Click card → shelf detail (list of books)  
4. Click book → book page (or Float)

Smart shelf ≈ Calibre virtual library (saved rules).  
Manual shelf ≈ pinned book ids.  
Rule engine is **not** implemented in P0 (sample data only).

## EPUB reader (planned P2)

- WebKitGTK  
- **Chapter-wise** continuous scroll  
- Prefetch next chapter near ~85–90% scroll  
- Keep at most ~3 chapters mounted  
- Instant CSS (fonts, theme, margins)  
- Highlights / quotes / dictionary in P3  

## Data dirs (planned)

```text
~/.local/share/kalam/
  catalog.db
  library/<uuid>/
  dictionaries/
  cache/covers/
~/.config/kalam/config.toml
```

## Out of scope

- Z-Library / unauthorized shadow libraries  
- Calibre-style multi-app suite  
- Content server, fetch news  

## Phase map

| Phase | Deliverable |
|-------|-------------|
| P0 | Shell + nav + sample shelves/books ← **you are here** |
| P1 | SQLite + EPUB import + covers |
| P2 | Reader (chapter scroll, fonts, progress) |
| P3 | Highlights, quotes, offline dictionary |
| P4 | Real home / shelves / lists |
| P5+ | Sources (AO3, FF), comics, convert |
