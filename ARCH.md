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

## EPUB reader (P2 + P3 enhancements)

- WebKitGTK  
- **Chapter-wise** continuous scroll  
- Prefetch next chapter near ~85–90% scroll (P2: manual N/›; auto-next disabled for stability)  
- Keep at most ~3 chapters mounted  
- Instant CSS (fonts, theme, margins) — reload chapter on theme/font change  
- Highlights / quotes / dictionary in P3:
  - Selection → `#kalam-chip` floating inside WebView (colors, quote, dict, copy)
  - `wrapRangeByPaths` via nodePath (child index path) + offsets for persistence
  - Reinject via `kalamInjectHighlights` on `LoadEvent::Finished`
  - JS bridge: `window.webkit.messageHandlers.kalam.postMessage(JSON)` + fallback `kalam://` iframe + title notify; Rust: `UCM::register_script_message_handler("kalam", None)` + `connect_script_message_received` + `decide_policy`
  - Dictionary: Settings → import StarDict/SQLite/TSV → `dict_entries` → search (exact → prefix → substring) → popup near rect via `kalamShowDict`

## Shelves engine (P4)

Two kinds share one table, separated by `kind`:

- **Manual** — rows in `shelf_books`, hand-ordered by `position`
- **Smart** — a JSON rule document in `shelves.rules`, compiled at query time

Rule documents are deliberately **flat**: a list of `{field, op, value}` plus a
single `match: all | any`. `shelf_rules::RuleSet::to_sql()` turns that into a
parameterised `WHERE` fragment over `books`; tag rules become
`EXISTS (SELECT 1 FROM book_tags …)`, negations wrap in `NOT`. Unknown fields or
blank values are skipped rather than failing, and an empty rule set compiles to
`0 = 1` so a half-built shelf matches nothing instead of the whole library.

Nested boolean groups were considered and deferred — the JSON can gain a
`groups` key later without a schema migration.

## History & time tracking (P4)

- `reading_events` is append-only: `opened | finished | unfinished | imported`.
  Repeat opens inside the same hour are collapsed so flipping in and out of the
  reader doesn't flood the log.
- `reading_sessions` gets one row per reader mount, closed in `shutdown()`.
  Durations are clamped to 6h — a suspended laptop must not claim a marathon.
- Auto-finish fires once at ≥99% progress; `finished_at` guards re-firing.

## Data dirs (P4 actual)

```text
~/.local/share/kalam/
  catalog.db                 # + P4: shelves, shelf_books, reading_list,
                             #   reading_events, reading_sessions
  library/<uuid>/            # book.epub + cover.*
  dictionaries/              # (placeholder dir, actual entries in catalog.db)
  cache/reader/<uuid>/       # extracted EPUB for WebView
~/Quotes.md                  # exported quotes Markdown
~/.config/kalam/config.toml  (future)
```

## Out of scope

- Z-Library / unauthorized shadow libraries  
- Calibre-style multi-app suite  
- Content server, fetch news  

## Phase map

| Phase | Deliverable |
|-------|-------------|
| P0 | Shell + nav + sample shelves/books ✅ |
| P1 | SQLite + EPUB import + covers ✅ |
| P2 | Reader (chapter scroll, fonts, progress) ✅ |
| P3 | Highlights, quotes, offline dictionary ✅ |
| P4 | Shelves engine, lists, history, tags, analytics ✅ ← **you are here** |
| P5+ | Sources (AO3, FF), comics, convert |
