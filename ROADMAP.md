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
P3  Annotations ─────── highlights, quotes, offline dictionary    ← next
P4  Library depth ───── shelves engine, lists, tags, analytics
P5  Metadata ────────── edit metadata, cover pick, Open Library
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

## P3 — Annotations & dictionary  ← **next**

**Goal:** “Editor in the viewer” on the text-reader surface.

### Scope

- Selection in WebView → floating chip: **Highlight** (colors) / **Save quote** / **Dictionary** / copy  
- Shortcut **`d`** → dictionary popover near word  
- Offline dict packs (import StarDict or prebuilt SQLite); none shipped by default  
- Persist annotations: loc (CFI or chapter + offsets), color, note text  
- Reinject highlights on chapter load  
- Annotations list (jump to)  
- My Library → Saved quotes / Saved words (real data)  
- Export quotes → Markdown  

### Look

- Selection UI inspired by tablet readers (compact chip above selection)  
- Highlight colors soft (incl. pink/rose option like reference photo)  
- Must not reintroduce blue underlines on body text  

### Out

Full EPUB HTML editing, sync, collaborative notes.

### Arch check

Highlight, quit, reopen; offline dictionary; export quotes.

### Exit criteria

Annotations trustworthy enough you stop using another app for EPUB markup.

---

## P4 — Library depth

**Goal:** Home, shelves, lists feel like *your* library.

### Scope

- Home polish: continue, recent, reading-list peek, counts  
- Reading list (ordered TBR)  
- History (opened/finished)  
- **Shelves engine**
  - Manual shelf (book ids)  
  - Smart shelf (rules: tag, author, format, progress, series, and/or)  
  - Shelves grid: 2 columns, live counts (as designed in P0)  
- Tags browse  
- Light analytics  
- Book page / float wired fully to DB  

### Out

Online sources.

---

## P5 — Metadata

**Goal:** Fix messy imports without leaving Kalam.

### Scope

- Edit metadata dialog  
- Replace cover  
- Fetch from **Open Library** (user-triggered, confirm before overwrite)  
- Keep uuid paths by default  

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

```text
books              id, uuid, title, sort_title, authors, series, description,
                   format, file_name, file_hash, cover_name, added_at, progress
tags / book_tags
reading_progress   book_id, chapter_index, fraction, updated_at   -- P2
annotations        id, book_id, kind, loc, color, body, …         -- P3
saved_words        …                                              -- P3
shelves / shelf_books / reading_list                              -- P4
sources_state / download_jobs                                     -- P6+
```

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

## Immediate next steps

1. **P3 — Annotations & dictionary** on the text reader  
2. Keep refining text-reader polish only if you file specific UX bugs  
3. **P8** when you want comics for real (UI target already specified above)  

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
