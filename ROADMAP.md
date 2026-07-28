# Kalam — Detailed Roadmap

Living plan. Phases are **sequential gates**: we do not start phase N+1 until
phase N compiles on CI **and** you have run it on Arch and signed off (or filed
change requests).

This file is updated whenever product/UX decisions change.

---

## Working agreement

| Who | Does |
|-----|------|
| **Agent** | Implements the phase, pushes code, keeps CI green, updates this doc |
| **CI (GitHub Actions)** | `fmt` · `clippy` · `cargo build` on every push (no GUI) |
| **You** | (1) Apply workflow file changes when the agent asks (manual — App cannot push workflows). (2) Paste failed CI step logs when the agent cannot read them. (3) Run the app on Arch **once per completed phase** for UX feedback |

You do **not** need to build between small commits. Only at phase boundaries.

**Workflow files:** agent edits `docs/ci/github-actions-ci.yml` and gives copy
instructions; you install into `.github/workflows/ci.yml`. See `docs/ci/README.md`.

### Definition of Done (every phase)

1. CI green on the branch  
2. Feature list for that phase implemented (see below)  
3. README / ARCH / **this ROADMAP** updated if behaviour or UX targets changed  
4. You ran it on Arch (or explicitly deferred) and listed change requests  

### Non-goals (whole project)

- Z-Library / unauthorized shadow libraries  
- Calibre multi-app suite, content server, fetch news  
- Plugin API  
- Windows / macOS  

---

## North star

> One native Linux app: fast personal library for a few thousand books, excellent
> EPUB reading & annotation, honest metadata, modular sources (AO3, fanfic,
> comics) later — no bloat.

**Stack:** Rust · GTK4 · Relm4 · custom CSS · SQLite · WebKitGTK (EPUB) · image
pipeline (comics) · MuPDF later (PDF)

---

## Two readers (architecture — locked)

Kalam has **two distinct viewing modes**. Same library; `Read` routes by format.

| | **Text reader** | **Comics reader** |
|--|-----------------|-------------------|
| **Formats** | EPUB now; PDF later; AO3/fanfic as downloaded EPUB/HTML | CBZ/CBR local first; remote later |
| **Engine** | WebKitGTK (EPUB/HTML); MuPDF/Poppler later for PDF | Decode images from zip/rar on demand |
| **Motion** | Continuous long scroll, **chapter-wise** for EPUB | Page mode and/or webtoon long-strip |
| **Look target** | Immersive “tablet book” (cream/sepia page, floating chrome) | Immersive “Moku-like” (black stage, top meta + bottom scrub) |
| **Chrome** | Top-left close/crumb; bottom pill: ‹ ☰ ch Aa › | Top: ✕ title chapter pages; bottom: scrubber + zoom |
| **Progress** | Chapter index + in-chapter fraction → SQLite | Page index (+ chapter/volume) → SQLite |
| **Phase** | **P2** shipped (shell + read); **P3** annotations | **P8** local; **P9** sources |

**Text reader visual rules (agreed):**

- Default theme: **sepia** (cream paper, brown ink)  
- Body text is **never** browser-blue; EPUB `<a>` wrappers forced to ink color  
- **No underlines** on body/links while reading  
- Selection highlight tint reserved for P3 (pink-ish mark, Apple Books–like)  
- Heavy top toolbars avoided; controls live in floating pills  

**Comics reader visual rules (agreed — Moku reference):**

- Pure **black** letterbox; art centered  
- Thin **top bar**: close, prev/next chapter, title, page `i / N`, zoom %  
- **Bottom bar**: page scrubber, zoom control, prev/next page  
- Fit width / fit height / RTL; optional continuous strip mode  
- Memory-safe: only nearby pages decoded (4 GB RAM)  

---

## Phase map (overview)

```text
P0  Shell ───────────── sidebar, routing, float/detail shells     ✅ done
P1  Library core ────── SQLite, import EPUB, cover cards, search  ✅ done
P2  Text reader ─────── WebKit EPUB, themes, TOC, progress, UI    ✅ done
P3  Annotations ─────── highlights, quotes, offline dictionary    ✅ done
P4  Library depth ───── shelves engine, lists, tags, analytics    ✅ done
P5  Metadata ────────── edit metadata, cover pick, Open Library      ✅ done
P6  Downloads hub ───── unified queue + folder watch
P7  Fiction sources ─── AO3 first, then other fanfic adapters
P8  Comics local ────── CBZ/CBR + Moku-style comics reader
P9  Comics sources ──── browse/download (legal / self-hosted first)
P10 PDF ─────────────── MuPDF in text-reader family + basic marks
P11 Tools ───────────── convert (external), polish, Calibre import
```

**Order lock:** P0→P5 stay sequential. P6–P11 may reorder after P3 if priorities shift.
Comics UI design is frozen in this doc now; **implementation is P8**.

---

## P0 — Application shell  ✅ done

**Goal:** Real window + navigation skeleton.

### Shipped

- [x] `kalam` / `app.kalam.Kalam`
- [x] Slim sidebar: Home, Library, Shelves, Downloads, Comics, AO3, Fanfic, Settings
- [x] Settings bottom-pinned
- [x] Route stack + Back
- [x] Draft dark CSS; later refined with library/reader skins
- [x] CI workflow (manual install of `.github/workflows` by you)
- [x] Arch smoke-test signed off

### Out

Real DB, import, reader (those are P1/P2).

---

## P1 — Library core  ✅ done

**Goal:** Real EPUBs live in Kalam.

### Shipped

- [x] SQLite `~/.local/share/kalam/catalog.db`
- [x] Import EPUB (file dialog); copy to `library/<uuid>/`; cover extract
- [x] OPF title/authors/tags/description (HTML stripped to plain text)
- [x] Cover-card grid (Goodreads-like): fixed **1.6:1** cover, title+author under
- [x] Click cover → float; Ctrl+click → full book page
- [x] Search + sort (title / author / added)
- [x] Remove book (DB + files)
- [x] Home: continue + recently added (real data)
- [x] Float panel: Suwayomi-style compact detail (cover rail, badges, Read, meta)
- [x] Settings shows data paths

### UX notes locked in P1

- Cards must stay **uniform grid cells**; long titles **ellipsize**, full text on tooltip  
- No full-width stretched single covers  
- Float: no open/close morph animations for now (can return later)  

### Out (still later)

Shelves rules engine, network metadata, comics import.

---

## P2 — Text reader (EPUB)  ✅ done

**Goal:** Comfortable EPUB reading; position restores.

### Shipped

- [x] WebKitGTK 6 reader surface
- [x] Open from book page / float **Read**
- [x] Parse spine + NAV/NCX TOC; chapter-wise load
- [x] Themes: Light / Sepia / Dark (default **Sepia**)
- [x] Font size A± ; keyboard N/P, arrows, Esc
- [x] TOC via bottom pill popover
- [x] Progress in SQLite (`reading_progress` + `books.progress`)
- [x] Immersive chrome: sidebar/topbar hidden while reading
- [x] **Restyle (P2.1):** top-left close+crumb; bottom floating pill (‹ ☰ ch Aa ›)
- [x] Reading CSS: force ink color, kill blue + underlines on body/links
- [x] No background FlushProgress timer (crash fix on leave)

### Partial / deferred

- [ ] Near-end auto-advance (JS bridge removed for CI stability; use N/›)
- [ ] True multi-chapter DOM buffer (still one chapter WebView load)
- [ ] Instant CSS var updates without reload (currently reload chapter on Aa/theme)
- [ ] Selection toolbar (P3)

### Arch check

Read long EPUB, change theme/font, TOC jump, quit, reopen mid-book; no crash on leave; text not blue/underlined.

### Exit criteria

Daily-driver EPUB reading without annotations — **met for P2 scope**.

---

## P3 — Annotations & dictionary  ✅ done

**Goal:** “Editor in the viewer” on the text-reader surface.

### Shipped

- [x] Selection in WebView → floating chip: **Highlight** (yellow/green/blue/pink/orange) / **Save quote** (❝) / **Dictionary** (Aa) / copy
- [x] Shortcut **`d`** → dictionary popover near word (via JS + GTK popover search)
- [x] Offline dict packs: **StarDict** (.ifo/.idx/.dict[.dz]), **SQLite** .db with entries(word,definition), **TSV** (word<TAB>def)
- [x] Import via Settings → Offline dictionaries → + Import dictionary; list & remove
- [x] Persist annotations: chapter_index + DOM path (nodePath) + offsets, color, text_excerpt, note, kind
- [x] Reinject highlights on chapter load (`kalamInjectHighlights` + `wrapRangeByPaths`)
- [x] Annotations list: reader bottom pill **✎** shows highlights/quotes for current book, Jump & Delete
- [x] My Library → **Saved quotes** (real data, search, delete, Export Markdown → `~/Quotes.md`)
- [x] My Library → **Saved words** (real data, search, delete, saved from dict lookup with context)
- [x] Export quotes → Markdown with book title, chapter, color, timestamp, quote block
- [x] Selection chip UI: semi-transparent dark pill above selection, color dots + ❝ Aa ⧉
- [x] Dictionary popup inside WebView: shows definition near selection rect, Save word / Copy
- [x] Reader typography popover now includes dictionary search (prefix → substring fallback) + Save/Clear
- [x] Highlight storage: SQLite `annotations` table, `saved_words`, `dictionaries`, `dict_entries`
- [x] CSS: soft highlight tints (yellow 0.62, green, blue, pink 0.70, orange), chip & dict popup styling, badge colors for annotation list, P3 GTK rows

### Look

- Selection UI inspired by tablet readers (compact chip above selection) — implemented as `#kalam-chip` inside WebView
- Highlight colors soft (incl. pink/rose option like reference photo) — `kalam-hl-pink` rgba(251,207,232,0.70)
- Must not reintroduce blue underlines on body text — preserved via reading CSS `!important`

### Out (deferred)

- Full EPUB HTML editing, sync, collaborative notes, note editing UI (note field exists but no inline editor yet)
- CFI spec (using path+offset, robust enough for P3; CFI reserved for later)
- Dictionary definition HTML rendering (currently stripped to plain text for GTK popover, WebView popup escapes HTML)

### Arch check

- Highlight, quit, reopen → highlight persists and re-injects
- Offline dictionary import (StarDict + SQLite) → lookup via chip or D, save word, appears in Saved words
- Export quotes → `~/Quotes.md` with Markdown

### Exit criteria

Annotations trustworthy enough you stop using another app for EPUB markup — **met**: highlights survive reload, quotes & words saved, dictionary offline.

### Notes on implementation

- JS bridge via `window.webkit.messageHandlers.kalam.postMessage` + fallback `kalam://` iframe + `title` notify; Rust side via `UserContentManager::register_script_message_handler` (world None) + `connect_script_message_received` + `decide_policy` fallback
- Progress still via JS bridge (`progress` payload) + fraction restore
- `evaluate_javascript` signature: 5 args (script, world_name, source_uri, cancellable, callback) — fixed after CI errors
- `set_data`/`data` require unsafe blocks in gtk-rs 0.9; handled via `unsafe {}` 
- CI now auto-formats and pushes fix commits (`cargo fmt --all` + push) to avoid fmt blockers in sandbox without rustfmt binary
- Clippy -D warnings enforced; dead_code allowed for some P3 structs/methods still evolving

---

## P4 — Library depth  ✅ done

**Goal:** Home, shelves, lists feel like *your* library.

### Shipped

- [x] **Schema v4**: `shelves`, `shelf_books`, `reading_list`,
      `reading_events`, `reading_sessions`, plus `books.last_opened_at` /
      `books.finished_at` added via an idempotent `ALTER` helper
- [x] **Shelves engine**
  - Manual shelf (membership rows, hand-sortable with ↑/↓)
  - Smart shelf — **flat rule list + one All/Any switch** (decision B)
  - Fields: tag, author, series, format, progress, title, added
  - Operators: is / is not / contains / does not contain / in the last N
    days / more than N days ago
  - Rules stored as JSON, compiled to a parameterised SQL `WHERE`
  - Shelves grid: 2 columns, kind badge, live counts, rule summary
  - Rule editor with **live “N books match”** readout
- [x] **Reading list** — ordered TBR, ↑/↓ reorder, bulk picker, Read button
- [x] **History** — append-only event log (opened / finished / unfinished /
      imported), grouped by day, filterable, same-hour dedupe on opens
- [x] **Reading time tracking** — a `reading_sessions` row per reader visit,
      closed on shutdown, clamped at 6h so an idle window can't fake a marathon
- [x] **Tags browse** — usage-weighted tag cloud → per-tag book grid
- [x] **Analytics** — books/finished/reading/unread, time read (all time,
      7d, 30d), current & longest streak, highlights/quotes/words, 14-day
      reading bar chart, books-added-per-month, most read / top tags / top authors
- [x] **Home polish** — counts strip, multi-book Continue row driven by
      `last_opened_at`, Up-next peek from the reading list
- [x] **Book page** — add/remove reading list, Mark finished / Mark unread,
      manual-shelf checklist, shelf chips
- [x] Auto-finish at ≥99% progress (once per book), with manual override
- [x] Unit tests over an in-memory catalog: rule compilation, membership,
      reorder, auto-finish, session clamping, stats

### Design decisions locked in P4

- Smart shelves stay **flat** (no nested boolean groups). The stored JSON is
  forward compatible, so nested groups can arrive later without a migration.
- An empty smart shelf matches **nothing**, not everything — less surprising.
- Analytics never invents data: no "hours read" before sessions existed.
- Deleting a shelf never deletes books.

### Out

Online sources.

### Arch check

- Create a smart shelf (tag is X **and** progress is unread) → live count moves
  as you type → save → grid shows the count → open it
- Create a manual shelf → add books from the book page and the picker → reorder
- Read a book for a few minutes → History shows "Opened" → Analytics shows time
- Finish a book → it leaves the reading list and appears as Finished

---

## P5 — Metadata  ✅ done

**Goal:** Fix messy imports without leaving Kalam.

### Shipped

- [x] Edit metadata dialog: title, authors, series, tags, description
- [x] Replace cover from disk (PNG/JPEG/WebP/GIF, sniffed by magic bytes)
- [x] **Open Library** lookup: search, pick a candidate, pull description
      and cover
- [x] User-triggered only; results are staged into the form for review and
      nothing is written until you press Save
- [x] Sparse matches only fill fields they actually have, so a thin result
      cannot blank out good local metadata
- [x] Network on worker threads via async-channel — the dialog never blocks
- [x] uuid paths unchanged; covers get a fresh file name per replacement
- [x] Unit tests over captured Open Library payloads

### Notes

- `ureq` with rustls, so there is no OpenSSL system dependency to install
- Covers are written as `cover-<n>.<ext>` rather than overwritten: GTK caches
  textures by path, so reuse would show the old image until restart
- Open Library's `description` is sometimes a string and sometimes
  `{ "value": … }`; both are handled

### Out

Bulk metadata edit and cover refresh across many books — those live in P11.

---

## P5.5 — UI overhaul  ◀ in progress

**Goal:** Redesign the interface, one window at a time. The app grew screen by
screen and looks it; this is the pass that makes it feel like one product.

**Working method (agreed):** one window per round. The agent mocks the screen
up as an image first, the user looks at it, and only then does it become Rust.
The agent cannot see the GUI, and shipping layout blind has repeatedly wasted
rounds.

### Done
- **Colour system** — `src/theme.rs` owns every colour; `style.rs` holds only
  shape (padding, radii, type scale). Adding a theme is one struct.
- **13 dark themes**, grouped standard + darker per family: One Dark (default
  is One Dark Darker), Tokyo Night, Everforest, Catppuccin, Gruvbox, Ayu, and
  Nord (no darker variant). Light themes are out of scope.
- **Scrollbars** — invisible until hovered, thin pill, hugging the edge.
- **Sidebar** — 48px icon-only rail, logo pinned top, nav centred, Settings
  bottom, circular active state.
- **Logo** — `assets/logo.png`, embedded with `include_bytes!`.

### Next
1. **Settings** — currently one long unstructured column; needs grouping.
2. Home / dashboard — the two-column layout the design references imply.
3. Library, Book page, Reader chrome, dialogs.

### Hard-won rules
- **Never override `border`, `margin`, `padding` or `min-width` on
  `scrollbar slider`.** Adwaita draws it as a wide widget with a 4px
  *transparent* border plus `background-clip: padding-box`, so the visible pill
  is thin while the grab area stays usable. Replacing that geometry makes GTK
  subtract border/margin from an already ~3px collapsed allocation, and the
  result goes negative:
      *** BUG *** In pixman_region32_init_rect: Invalid rectangle passed
      GtkGizmo (slider) reported min width -12, but sizes must be >= 0
  Style the scrollbar with **colour only** — `background-color` on the slider,
  transparent at rest, tinted on `scrolledwindow:hover`. That gives the thin
  hidden-until-hovered bar with none of the breakage.
- `KALAM_NO_CSS=1` runs the app with no custom stylesheet. Use it first when a
  GTK rendering warning appears, before theorising about causes.

### Notes
- Reader *page* theming (Light/Sepia/Dark paper) stays separate from app
  chrome: a sepia page inside a dark app is a legitimate combination.
- The images in `docs/design/` are **inspiration the user collected**, not
  their own designs. Treat them as direction, not specification.

---

## P6 — Downloads hub

**Goal:** One place for inbound files/jobs.

### Scope

- Queue: queued / active / done / failed  
- Sidebar Downloads UI  
- Folder-watch import  
- Hooks for AO3/comics jobs  

---

## P7 — Fiction sources (AO3 first)

**Goal:** Search/read/track fanfiction inside Kalam.

### Scope

- `FictionSource` trait  
- AO3: search, detail, download EPUB into library, `source` + `remote_id`  
- Manual “Check updates”  
- Rate limits / clear errors  
- Open downloads in **text reader**  

### Out

Piracy sources.

---

## P8 — Comics local + Moku-style reader

**Goal:** Local CBZ/CBR with a dedicated comics viewer.

### Scope

- Import CBZ/CBR into same catalog (`format = cbz|cbr`)  
- **Comics reader UI (Moku reference — locked):**  
  - Black immersive stage, art centered  
  - Top bar: close, chapter/title, page `i / N`, zoom %  
  - Bottom: scrubber, zoom, prev/next page  
  - Page mode LTR/RTL; webtoon long-strip mode  
  - Fit width / fit height  
  - Tap center toggle chrome (optional)  
- Memory-safe decode (viewport ± neighbors only)  
- Progress per book  
- Book page / float: **Read** routes to comics viewer when format is comic  

### Out

Remote catalogues (P9).

### Arch check

Open large CBZ, scrub pages, zoom, RTL, quit/restore page; RAM stays reasonable.

---

## P9 — Comics sources

**Goal:** Browse/download into library → open in P8 viewer.

### Scope

- Source framework  
- Prefer **self-hosted / legitimate** backends first (OPDS, Komga, Kavita, own archive)  
- Downloads hub integration  
- Suwayomi-*like* module depth only as needed — not a full extension store on day one  

### Policy

Only sources you’re allowed to use. No unauthorized scraper assistance.

---

## P10 — PDF (text-reader family)

**Goal:** Read PDFs with light marks.

### Scope

- MuPDF (or Poppler) view inside **text-reader chrome family** (not comics shell)  
- Continuous or page mode  
- Basic highlight/underline stored like EPUB annotations where possible  
- Same library entry model  

---

## P11 — Tools

**Goal:** Occasional Calibre-class jobs.

### Scope

- Convert via external `ebook-convert` / `pandoc` if present  
- EPUB polish (strip junk CSS, etc.)  
- Batch metadata / cover refresh  
- Optional Calibre `metadata.db` one-shot import  

---

## Schema (current + planned)

Current `SCHEMA_VERSION` = **7** (`src/db.rs`). Migrations run on open and are
additive; there is no downgrade path, so take a copy of
`~/.local/share/kalam/catalog.db` before testing a build that bumps it.

```text
books              id, uuid, title, sort_title, authors, series, description,
                   format, file_name, file_hash, cover_name, added_at, progress
                   + last_opened_at, finished_at                  -- v4
                   + rating (0..=10 half-stars)                   -- v5
                   + publisher, published, series_index (REAL)    -- v6
tags / book_tags
reading_progress   book_id, chapter_index, fraction, updated_at   -- P2
annotations        id, book_id, kind, loc, color, body, …         -- P3 (v3)
saved_words        …                                              -- P3 (v3)
dictionaries       id, name, lang, entry_count, …                 -- P3 (v3)
shelves            id, name, kind, description, rules(JSON), position  -- P4 (v4)
shelf_books        shelf_id, book_id, position, added_at             -- P4 (v4)
reading_list       book_id, position, note, added_at                 -- P4 (v4)
reading_events     id, book_id, kind, at, detail                     -- P4 (v4)
reading_sessions   id, book_id, started_at, ended_at, seconds, pct   -- P4 (v4)
reading_goals      year, target_books                                -- v5
app_prefs          key, value
metadata_overrides keyed on file_hash, NOT cascaded from books      -- v7
sources_state / download_jobs                                     -- P6+
```

`metadata_overrides` is deliberately **not** `ON DELETE CASCADE`: surviving a
book's deletion is the entire point, so edits come back when the same file is
re-imported. Its cover lives in `covers/<file_hash>.<ext>`, not in
`library/<uuid>/`, which is removed with the book.

---

## CI plan

| Check | When |
|-------|------|
| `cargo fmt --check` | every push |
| `cargo clippy -D warnings` | every push |
| `cargo build` / `release` | every push |
| GUI smoke | **your Arch machine** at phase end |

Deps include `webkitgtk-6.0` for P2+.

---

## Risk register

| Risk | Mitigation |
|------|------------|
| WebKit RAM on 4 GB | One WebView; chapter-wise load |
| EPUB blue link spam | Aggressive reading CSS; inject at end of body |
| Reader timer after drop | No background progress timer; save on nav/close |
| Comics RAM | Decode only nearby pages (P8) |
| Scope creep | Phase gates; comics design locked, code in P8 |
| Annotation loc vs re-download | P7 best-effort; may reset on spine change |

---

## Post-P5 work (done, between P5 and P6)

### Performance pass ✅
- **Query storm**: ~99 SQL queries per Library click with 50 books → ~8.
  N+1 tag lookups collapsed, `list_books()` no longer loads the whole library
  to draw 6 covers, prepared-statement cache, `library_stats()` memoised
  against SQLite's `total_changes()` so no write path has to remember to
  invalidate. `synchronous=NORMAL`, 64 MB cache, `temp_store=MEMORY`, mmap.
  Indexes on `progress`, `last_opened_at`, `finished_at`.
- **Widget rebuilds**: pages cached in `AppModel.cache` keyed by route.
  Reader/BookPage/ShelfDetail/TagBooks/LibrarySection deliberately excluded
  (they own a WebView or per-book state). Invalidated via `cache_token`.
- **Blocking imports**: `Catalog` moved `Rc` → `Arc`; the import loop runs on
  `spawn_command` and reports per-file progress.

### Hardening pass ✅
1. **Notifications** (`src/notify.rs`) — toast overlay per
   `docs/design/notifications.png`; history panel in Settings.
2. **Library backup** — `VACUUM INTO`, consistent even while running.
3. **Reader cache pruning** — orphaned + 14-day-stale extracts dropped at
   startup.
4. **Poison-safe locks** — 77 `expect("db lock")` → `Catalog::conn()`.
5. **`db.rs` split** — 3,367 lines → 1,702 + 7 focused modules.

### Bug fixes worth remembering
- **Metadata overrides** keyed on `file_hash` (schema v7) so edits survive
  delete → re-import, including the cover, which is stashed in
  `covers/<hash>.<ext>` because `library/<uuid>/` goes with the book.
- **EPUB writeback on single-line OPFs** — `rewrite_opf` filtered the
  metadata block line by line, which only works on pretty-printed files.
  Real EPUBs often put the whole block on one line, leaving the old
  `<dc:title>` beside the new one; readers showed the stale one. Now walks
  elements, not lines.
- **Notifications only fired on failure** — `notify::report` stayed silent on
  `Ok`, so every successful action looked broken. Added
  `notify::outcome`/`outcome_info`.

---

## Immediate next steps

1. **UI overhaul** once the Figma designs are final (see `docs/design/`).
   `library_look.png` shows a two-column dashboard; the app is currently a
   single vertical stack. Known divergence, deliberately deferred.
2. **P6 — Downloads hub** (unified queue + folder watch; prerequisite for P7).
3. Keep refining text-reader polish only if you file specific UX bugs (note editing UI, CFI, dict HTML rendering)
4. **P8** when you want comics for real (UI target already specified above)  

---

## Changelog of plan decisions

| Date | Decision |
|------|----------|
| 2026-07-24 | Name: Kalam; Linux only; Relm4+GTK4; no Z-Library |
| 2026-07-24 | EPUB engine: WebKit; chapter-wise scroll |
| 2026-07-24 | Custom UI; slim sidebar IA |
| 2026-07-24 | Shelves: grid → detail → book page or float |
| 2026-07-24 | CI compile; Arch at phase boundaries |
| 2026-07-24 | Roadmap P0–P11 |
| 2026-07-25 | P0 signed off; P1 library core |
| 2026-07-25 | Cover cards: 1.6:1 cover, title/author below; click=float, Ctrl+click=page |
| 2026-07-25 | Float: Suwayomi-style panel; later no open/close morph animations |
| 2026-07-26 | P2 EPUB reader; restyle to tablet-book floating chrome |
| 2026-07-26 | Two readers locked: text vs comics |
| 2026-07-26 | Text reader look: sepia default, no blue body/links, no underlines |
| 2026-07-26 | Comics reader look: Moku-like black stage, top meta + bottom scrub (P8) |
| 2026-07-26 | P0–P2 treated complete; **next = P3** |
| 2026-07-27 | P5 shipped: metadata editor, cover replacement, Open Library lookup (staged for review, never auto-applied) |
| 2026-07-27 | Ratings (half-star), yearly reading goals and quote notes added from the reference designs; social elements deliberately skipped |
| 2026-07-27 | P4 shipped: shelves engine (manual + flat-rule smart shelves), reading list, event-log history, reading-time sessions, tags browse, analytics with streaks |
| 2026-07-27 | Smart shelves locked as flat rules + All/Any; nested groups deferred and kept JSON-compatible |
| 2026-07-26 | P3 annotations & dictionary shipped: highlights (5 colors), quotes, offline dict packs (StarDict/SQLite/TSV), Saved quotes/words real data, export Markdown, annotations list, dictionary popup, Settings import |
| 2026-07-28 | Performance pass shipped: query batching + stats memoisation, page cache, imports off the UI thread |
| 2026-07-28 | Hardening pass shipped: toast notifications, `VACUUM INTO` backup, reader-cache pruning, poison-safe locks, `db.rs` split |
| 2026-07-28 | Metadata overrides keyed on `file_hash` (schema v7) so edits and covers survive delete → re-import |
| 2026-07-28 | EPUB writeback fixed for single-line OPFs; toast accent restyled to the reference; every user action now confirms |
| 2026-07-28 | P5.5 opened: UI overhaul, one window at a time, mockup before code. Colour system + 13 dark themes + slim sidebar shipped |
