# Kalam — Detailed Roadmap

Living plan. Phases are **sequential gates**: we do not start phase N+1 until
phase N compiles on CI **and** you have run it on Arch and signed off (or filed
change requests).

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
3. README / ARCH updated if behaviour changed  
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

**Stack:** Rust · GTK4 · Relm4 · custom CSS · SQLite · WebKitGTK (EPUB) · MuPDF later (PDF)

---

## Phase map (overview)

```text
P0  Shell ───────────── sidebar, routing, sample shelves/books     ← current
P1  Library core ────── SQLite, import EPUB, covers, all-books UI
P2  Reader ──────────── WebKit, chapter-wise scroll, fonts, progress
P3  Annotations ─────── highlights, quotes, offline dictionary
P4  Library depth ───── home real data, shelves engine, lists, tags
P5  Metadata ────────── edit metadata, cover pick, Open Library fetch
P6  Downloads hub ───── unified queue UI + local folder watch
P7  Fiction sources ─── AO3 first, then other fanfic adapters
P8  Comics local ────── CBZ/CBR reader (webtoon + page modes)
P9  Comics sources ──── browse/download module (careful, legal-first)
P10 PDF ─────────────── view + basic highlights
P11 Tools ───────────── convert (external), polish, extras
```

Phases P6–P11 may be reordered once P3 is solid; **P0→P5 stay in order**.

---

## P0 — Application shell  ✅ in progress

**Goal:** A real window you can click while the design evolves. No real files.

### Scope

- [x] Cargo project `kalam`, app id `app.kalam.Kalam`
- [x] Slim sidebar: Home, Library, Shelves, Downloads, Comics, AO3, Fanfic, Settings
- [x] Settings pinned to bottom
- [x] Home placeholder with demo continue/recent
- [x] My Library hub tiles → section pages (sample lists where relevant)
- [x] Shelves: 2-column grid → shelf detail → book page
- [x] Book page (metadata layout) + **Float** window option
- [x] Back stack within a module
- [x] Draft dark CSS (`src/style.rs`)
- [x] CI workflow
- [x] ROADMAP + ARCH docs
- [ ] CI green (fix after first Actions run)
- [ ] Your Arch smoke-test sign-off

### Explicitly out

Real DB, import, reader, network.

### Arch check (you)

```bash
sudo pacman -S --needed rust gtk4 libadwaita base-devel pkgconf
cargo run
```

Click: Shelves → shelf → Open / Float → Back → Library hub → Settings.

### Exit criteria

CI green + you OK with nav model (change requests allowed before P1).

---

## P1 — Library core

**Goal:** Your real EPUBs live in Kalam.

### Scope

- SQLite catalog at `~/.local/share/kalam/catalog.db`
- Schema v1: `books`, `authors`, `book_authors`, `tags`, `book_tags`, `covers` path, file hash, format, added_at
- Import: file picker + drag-drop (EPUB first)
- Copy book into `library/<uuid>/book.epub`, extract cover → `cover.jpg`
- My Library → All books: grid + list toggle (virtualized `GridView`/`ListView`)
- Search box (title/author `LIKE`; FTS5 optional same phase if easy)
- Sort: title, author, added
- Remove book (DB + files)
- Empty states
- Drop sample-only models for library views (keep tiny demo seed behind `cfg` or delete)

### Out

Reader, shelves rules engine, network metadata.

### Arch check

Import 5–10 EPUBs, search, delete one, restart app — data persists.

### Exit criteria

Persistent library feels usable as a dumb catalog.

---

## P2 — EPUB reader

**Goal:** Read comfortably; position restores.

### Scope

- WebKitGTK 6 view inside app (immersive: collapse/hide slim rail)
- Open from book page **Read**
- Parse EPUB spine/nav; chapter-wise continuous scroll
- Prefetch next chapter ~85–90%; keep ≤3 chapters mounted
- Seamless boundary (no full reload flash when prefetch wins)
- Inject CSS: font family/size, line-height, margins, width, theme (light/sepia/dark)
- Instant theme/font updates (CSS variables, no reload)
- Progress: chapter id + % ; debounce write to SQLite; restore on open
- TOC sidebar / popover
- Keyboard: scroll, next/prev chapter, Esc back, fullscreen
- Typography popover (Aa)

### Out

Highlights, dictionary, PDF.

### Arch check

Long EPUB, scroll across chapters, change font, quit, reopen mid-book.

### Exit criteria

Daily-driver reading for EPUB without annotations.

---

## P3 — Annotations & dictionary

**Goal:** “Editor in the viewer” for personal use.

### Scope

- Selection → Highlight (colors) / Save quote / Dictionary
- Shortcut `d` → dictionary popover near word
- Offline dict packs (import StarDict or prebuilt SQLite); none shipped by default
- Persist annotations: loc (CFI or chapter+offsets), color, note text
- Reinject highlights on chapter load
- Annotations list UI (jump to)
- Saved quotes page under My Library (real data)
- Saved words from dict popover
- Export quotes → Markdown file

### Out

Full EPUB HTML editing, collaborative sync.

### Arch check

Highlight, quit, reopen; dictionary offline; export quotes.

### Exit criteria

Annotations trustworthy enough you stop using another reader for EPUB.

---

## P4 — Library depth

**Goal:** Home, shelves, lists feel like *your* library.

### Scope

- Home: continue, recent, reading-list peek, counts
- Reading list (ordered TBR)
- History (opened/finished)
- **Shelves engine**
  - Manual shelf (book ids)
  - Smart shelf (rules: tag, author, format, progress range, series, and/or)
  - Shelves grid shows live counts
- Tags browse
- Light analytics (minutes approx, finished count) — simple
- Book page wired to real DB fields

### Out

Online sources.

### Arch check

Create smart shelf “unread EPUB”, pin manual favorites, Home correct.

### Exit criteria

No need for Calibre virtual libraries for daily browsing.

---

## P5 — Metadata

**Goal:** Fix messy imports without leaving Kalam.

### Scope

- Edit metadata dialog: title, authors, tags, series, series_index, identifiers
- Replace / crop cover (basic)
- Fetch metadata from **Open Library** (and maybe one backup) — user-triggered
- Apply fetch with confirm (don’t silently overwrite)
- Filename / folder optional rename policy (default: keep uuid paths)

### Out

Calibre plugin ecosystem, bulk magic beyond multi-select edit if time.

### Arch check

Fetch one book’s metadata, edit tags, cover swap.

---

## P6 — Downloads hub

**Goal:** One place for “stuff coming in.”

### Scope

- Queue model: queued / active / done / failed
- UI under sidebar Downloads
- Local **folder watch** import (drop EPUB in folder → queue → library)
- Hook points for future AO3/comics jobs
- Persist queue state lightly

### Out

Actual AO3/comics fetch (next phases).

---

## P7 — Fiction sources (AO3 first)

**Goal:** Search/read/track fanfiction without a browser.

### Scope

- `FictionSource` adapter trait
- AO3: search, work detail, download EPUB into library, store `source` + `remote_id`
- Track updates: manual “Check updates” → re-download if new chapters
- Respect rate limits / ToS; clear errors
- Optional second adapter (e.g. another FF site you name) only after AO3 is solid

### Out

Piracy sources.

---

## P8 — Comics local

**Goal:** CBZ/CBR you already have.

### Scope

- Import CBZ/CBR into library
- Reader: page mode LTR/RTL, webtoon long-strip, progress, double-page optional
- Memory-safe image pipeline (decode viewport only)
- Book page format-aware actions

### Out

Remote catalogues.

---

## P9 — Comics sources

**Goal:** Suwayomi-*like* **module**, not a full extension store on day one.

### Scope

- Source framework + 1–2 **legitimate / self-hosted** backends first  
  (e.g. OPDS, Komga, Kavita, or user-owned archive)
- Browse → download → library → open in comics reader
- Downloads hub integration

### Policy

Only sources you’re allowed to use. No assistance for unauthorized manga scrapers.

---

## P10 — PDF

**Goal:** Read and lightly mark PDFs.

### Scope

- MuPDF (or Poppler) view
- Continuous or page mode
- Basic highlight/underline (browser-like), store in DB
- Share library entry with EPUB flow

### Out

Full PDF editor, forms, heavy annotation suite.

---

## P11 — Tools

**Goal:** Occasional Calibre jobs you still need.

### Scope

- Convert via external tool if present (`ebook-convert` / `pandoc`) — not reimplemented
- Polish: strip junk CSS, simple cleanup passes on EPUB
- Batch metadata / cover refresh
- Optional Calibre `metadata.db` one-shot import

---

## Schema preview (P1+, expands later)

```text
books            id, uuid, title, sort_title, path, format, hash, added_at, …
authors          id, name, sort_name
book_authors     book_id, author_id, role, position
tags             id, name
book_tags        book_id, tag_id
progress         book_id, chapter_id, fraction, updated_at
annotations      id, book_id, kind, loc, color, body, created_at
saved_words      id, word, snapshot, book_id, created_at
shelves          id, name, kind (manual|smart), rules_json, …
shelf_books      shelf_id, book_id, position   -- manual only
reading_list     book_id, position
sources_state    book_id, source, remote_id, last_check, …
download_jobs    id, kind, state, payload_json, …
```

Exact SQL lands in P1 migration files.

---

## CI plan

| Check | When |
|-------|------|
| `cargo fmt --check` | every push |
| `cargo clippy -D warnings` | every push |
| `cargo build` | every push |
| `cargo build --release` | primary job |
| GUI / WebKit smoke | **your Arch machine** at phase end |

Workflow: `.github/workflows/ci.yml`  
Two jobs: gtk-rs container + plain Ubuntu apt (belt and suspenders).

---

## Risk register

| Risk | Mitigation |
|------|------------|
| WebKit RAM on 4 GB | One WebView; chapter-wise mount; immersive unload library heavies |
| HDD lag on chapter turn | Prefetch early; decode off UI thread |
| Scope creep (comics+AO3+reader) | Hard phase gates; P0–P5 before sources |
| Custom UI thrash | Shell first; skin CSS without rewriting routes |
| EPUB edge cases | Prefer WebKit fidelity; quarantine broken files with error UI |
| Annotation loc breaks on re-download | P7: best-effort; may reset progress on major spine change |

---

## Immediate next steps

1. Push branch → watch Actions  
2. Fix CI failures until green (**P0 complete on CI**)  
3. You run P0 on Arch once → feedback list  
4. Apply P0 polish from your list  
5. Start **P1** only after you say go  

---

## Changelog of plan decisions

| Date | Decision |
|------|----------|
| 2026-07-24 | Name: Kalam; Linux only; Relm4+GTK4; no Z-Library |
| 2026-07-24 | EPUB engine: WebKit; chapter-wise seamless scroll |
| 2026-07-24 | Custom UI; slim sidebar IA agreed |
| 2026-07-24 | Shelves: grid → detail → book page or float |
| 2026-07-24 | CI for compile; Arch only at phase boundaries |
| 2026-07-24 | This roadmap P0–P11 |
