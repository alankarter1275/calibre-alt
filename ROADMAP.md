# Kalam — Detailed Roadmap

Living plan. Phases are **sequential gates**: we do not start phase N+1 until
phase N compiles on CI **and** you have run it on Arch and signed off (or filed
change requests).

This file is updated whenever product/UX decisions change.

---

## ⚠️ Read this first — every chat, every agent

If you are an AI agent starting work on this repo, read ALL of this before
touching code:

1. **`README.md`** — what the app is, current status table.
2. **`ROADMAP.md` (this file)** — the plan, the order, the gates, the locked
   decisions. The **"Current trajectory"** section below is the single summary
   of where we are and what comes next.
3. **`docs/conversation.md`** — the living design-conversation log. Every
   accepted / rejected idea, with reasons. **Do not re-litigate settled
   decisions** — if you think one is wrong, raise it in chat first.

**Hard rules for every change you make:**

- **Keep the docs current — always, in the same commit as the code.** When you
  finish a phase / feature / decision, update: README (status table),
  ROADMAP (phase status, changelog table, next steps), and
  `docs/conversation.md` (if the work changes a decision). A change that
  leaves the roadmap stale is **not done**.
- **Never skip the changelog.** ROADMAP ends with a "Changelog of plan
  decisions" table — append a dated row for every phase shipped or decision
  locked. This is how a new chat catches up in one glance.
- **CI is the gate.** No local Rust toolchain in the sandbox: push and watch
  GitHub Actions (`gh run list`). Never force-push. The App token cannot push
  `.github/workflows/` — workflow changes go to `docs/ci/github-actions-ci.yml`
  and the user installs them.
- **You cannot see the screen.** The user is the QA loop for anything visual:
  ask for error text (not screenshots — you can't view them), and have the
  user run the app on Arch at phase boundaries.
- **Branch:** each Arena session is pinned to its own `arena/<id>-calibre-alt`
  branch. Work only on the branch the current session names, and push only to
  it. Do not copy the branch id out of this file — it changes every session.

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

### Documentation discipline (non-negotiable)

- **Every phase ships with its docs.** README status table, ROADMAP phase
  status + changelog row + next-steps update, and (if a decision changed)
  `docs/conversation.md` — in the **same commit** as the code, never "later".
- **The changelog table is the memory.** A new chat must be able to catch up
  by reading: README status, ROADMAP "Current trajectory" + changelog tail,
  and `docs/conversation.md` §6 (standing decisions).
- **A fresh chat must be able to continue without asking the user what the
  plan is.** If that is not true after your change, the change is not done.
- **Decisions change only through conversation.** Mark reversals in
  `docs/conversation.md` with the old position struck through and the new one
  recorded with the date.

### Non-goals (whole project)

- Z-Library / unauthorized shadow libraries  
- Calibre multi-app suite, content server, fetch news  
- ~~Plugin API~~ — **reversed 2026-09-02**: plugins are wanted; see
  "Architecture & performance track" below and `docs/conversation.md` §5
- Windows / macOS  

---

## North star

> One native Linux app: fast personal library for a few thousand books, excellent
> EPUB reading & annotation, honest metadata, modular sources (AO3, fanfic,
> comics) later — no bloat.

**Stack:** Rust · GTK4 · Relm4 · custom CSS · SQLite · WebKitGTK (EPUB) · image
pipeline (comics) · MuPDF later (PDF) · cosmic-text (custom renderer, A0)

---

## Current trajectory (locked 2026-09-02)

**Where we are:** P0–P5 shipped and CI-green; dictionary track Phases 1–10
shipped and CI-green (173 unit tests). The product is now a **content
platform**: fiction (AO3 / FFN / webnovels) and manga sources with native tag
search, downloads, offline reading, auto-updates — on a fast, Yazi-style
architecture.

**The agreed order — do not reorder without asking:**

1. **A0 — Architecture & performance track** (◀ NEXT, detailed below):
   measure → `LibraryService` behind `Catalog` → thumbnails at import +
   async cover decode → task manager → preloaders → grid virtualization
   *only if the numbers say so* → perf-budget CI test → **plugin-host seam
   design** (the `Source` adapter API that P7/P9 depend on).
2. **P6 — Downloads hub** (queue + folder watch; prerequisite for P7).
4. **P7 — Fiction platform** (AO3 first, then FFN / Royal Road / ScribbleHub /
   Webnovel) via **Lua source plugins**; native tag search (fandom, tags,
   characters, ships, rating, status); download + offline reading; follow +
   **auto-updater** (background scheduler; FFN-app-class).
5. **Renderer vertical slice** — starts *alongside* P7, not after: custom
   renderer on **cosmic-text** for fiction content (clean plugin output).
   This is the **calibration milestone**: 2–4 weeks of sessions; if it takes
   longer, stop and reassess before sinking months in.
6. **P8 — Comics local** (image pager — decode + paint, no engine) and
   **P9 — Manga platform** (Lua source plugins; MangaDex official API first,
   then Komga/Kavita/OPDS clients, scraped sites later).
6. **EPUB path** → custom renderer takes EPUBs: either a normalization
   pipeline (lol_html + rules) or **stylo** (Firefox's CSS engine, via
   chapbook — the approach chapbook proves); WebKit demoted to fallback for
   exotic EPUBs (may be cut later). **Adopt quote-anchored locators
   (LayeredLocator, chapbook's model) for annotations regardless** — it
   fixes the auto-updater anchor risk (§11).
7. **P10 — PDF** (MuPDF) · **P11 — Tools** · **P12 — Lua plugin system**
   matures into a user-facing plugin surface.

**Locked decisions (full reasoning in `docs/conversation.md`):**

- **Stay Rust.** No language rewrite — performance is architecture, not
  language (§1–2).
- **No browse mode.** Kalam never renders arbitrary websites; sources return
  structured data via plugins; search UI is native (§8).
- **Custom renderer is the endgame for ALL reflowable text** (cosmic-text
  based). WebKit = EPUB fallback only, may be cut. crengine rejected as the
  base: GPL-2/AGPL license mismatch with our GPL-3.0, C++ FFI burden, partial
  CSS 2.1; at most a separate dynamically-linked fallback bridge (§8, §10).
  **chapbook (ophymx, Apache-2.0) is a candidate foundation** — it is this
  exact architecture, already built (stylo + cosmic-text + tiny-skia/vello,
  no webview, GTK4 viewer, quote-anchored locators). Re-evaluate at
  vertical-slice time (§11).
- **Manga = Tachiyomi-shaped `Source` adapter API, Lua plugins we write.**
  No Kotlin extension bridge (Android APKs — wrong shape); no Suwayomi server
  rewrite; optional Suwayomi-server *client* adapter later (§7–8).
- **PDF = MuPDF** (fixed-layout, AGPL — acceptable; Poppler/GPL the
  alternative; **hayro** — Apache-2.0 pure-Rust — also on the P10 shortlist,
  §11). **Comics = image decode + GTK pager** — no engine (§8).
- **Perf order:** measure → thumbnails/async decode → virtualize if numbers
  say so (§2).
- **Renderer effort estimate:** 2–4 wk vertical slice; 6–12 months total for
  "no WebKit for text". **The user is the QA loop** — the agent has no
  display (§9).
- **Borrow list** (license-compatible with GPL-3.0): FanFicFare (fiction
  adapters), Tachiyomi extensions (pattern), MangaDex API, Komga/Kavita,
  KOReader + crengine (reference), cosmic-text/swash/fontdb/vello (Rust text
  stack), lol_html/ammonia (sanitizing), Yazi, Foliate (§8).

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

**Renderer trajectory (2026-09, supersedes the engine column above):** the
custom renderer (cosmic-text) is the endgame for ALL reflowable text — source
fiction first, EPUB after a normalization pipeline. WebKit becomes the EPUB
fallback only (exotic EPUBs), then may be cut. The table above remains the
*current* engine map; see "Current trajectory" and `docs/conversation.md` §8.

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
P7  Fiction platform ── AO3 first → FFN/RoyalRoad/etc.; Lua source
                        plugins; tag search; downloads; auto-updater
P8  Comics local ────── CBZ/CBR + Moku-style comics reader (image pager)
P9  Manga platform ──── Suwayomi-class sources via Lua plugins
                        (MangaDex API first; legal/self-hosted)
P10 PDF ─────────────── MuPDF in text-reader family + basic marks
P11 Tools ───────────── convert (external), polish, Calibre import
A0  Architecture track ─ service layer + task manager + preloaders +
                        thumbnails + virtualization + plugin-host seam
P12 Plugins ──────────── Lua plugin system (source adapters first;
                        matures after A0)
```

**Order lock:** P0→P5 stay sequential. P6–P11 may reorder after P3 if priorities shift.
Comics UI design is frozen in this doc now; **implementation is P8**.

**Architecture track (A0)** is the performance/async work discussed in
`docs/conversation.md` §§1–3 — the **next big work item** (detailed in the
"A0 — Architecture & performance track" section below). It is a track, not
a phase: it may interleave with P6–P11. The **renderer decision** (WebKit
vs custom text engine) belongs to it — resolved direction: **no browse
mode**; **custom renderer is the endgame for ALL reflowable text**
(cosmic-text based; fiction first, EPUB after a normalization pipeline);
WebKit = fallback only (exotic EPUBs), may be cut later; PDF = MuPDF;
comics = image pager (see `docs/conversation.md` §8).

> **Track phases are separate from P0–P11.** The dictionary overhaul uses its
> own numbering (Phases 1–10, in the "Dictionary overhaul" section below) —
> those are *not* P8/P9/P10. Global P8/P9/P10 remain Comics local / Comics
> sources / PDF.

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
- [x] Selection toolbar: compact themed icon actions with tooltips (P3)

### Arch check

Read long EPUB, change theme/font, TOC jump, quit, reopen mid-book; no crash on leave; text not blue/underlined.

### Exit criteria

Daily-driver EPUB reading without annotations — **met for P2 scope**.

---

## Reader chrome restyle (P2.1)  ✅ done

**Decision (2026-07-26):** the reader is a tablet-book, not a browser: no heavy
top toolbar. Restyled while P2 was still open.

- [x] Top-left close + crumb (book → chapter); no reader top bar
- [x] Bottom floating pill: `‹ ☰ ch Aa ›` (prev / TOC / chapter label / font+theme)
- [x] Immersive mode: app sidebar + topbar hidden while reading
- [x] Reading CSS: body/links forced to ink color (never browser-blue), no
      underlines on body text; selection tint reserved for P3
- [x] Chapter reload on theme/font change (accepted; instant CSS-var swap deferred)

**Look target:** immersive "tablet book" — cream/sepia page, floating chrome.

---

## P3 — Annotations & dictionary  ✅ done

**Goal:** “Editor in the viewer” on the text-reader surface.

### Shipped

- [x] Selection in WebView → floating chip: **Highlight** (yellow/green/blue/pink/orange) / **Save quote** (❝) / **Dictionary** (Aa) / copy
- [x] Shortcut **`d`** → dictionary popover near word (via JS + GTK popover search)
- [x] Offline dict packs: **StarDict** (.ifo/.idx/.dict[.dz]), **SQLite** .db with entries(word,definition), **TSV** (word<TAB>def)
- [x] Bundled English WordNet 2025 starter pack (about 127k headwords, about 4.2 MB compressed), enabled on first run with attribution and licence notices
- [x] Bundled English Idioms and Expressions pack (1,024 phrase-to-meaning entries, about 16 KB compressed), kept separate and enabled on first run with source-quality and licence notices
- [x] Import via Settings → Offline dictionaries → + Import dictionary; list & remove
- [x] Persist annotations: chapter_index + DOM path (nodePath) + offsets, color, text_excerpt, note, kind
- [x] Reinject highlights on chapter load (`kalamInjectHighlights` + `wrapRangeByPaths`)
- [x] Annotations list: reader bottom pill **✎** shows highlights/quotes for current book, Jump & Delete
- [x] Reader selection toolbar uses compact themed icon actions with useful tooltips; default WebKit context menus are suppressed without changing text selection or the automatic selection-actions toolbar
- [x] Annotation workflow: exact cross-chapter jumps, near-top positioning, temporary focus emphasis, and annotation-ID-first restoration
- [x] Annotation cards: dark rounded cards with subtle pastel tints, colored left edges, saved note previews, expandable multiline note editing, and autosave without Save/Cancel controls
- [x] My Library → **Saved quotes** (real data, search, delete, Export Markdown → `~/Quotes.md`)
- [x] My Library → **Saved words** (real data, search, delete, saved from dict lookup with context)
- [x] Export quotes → Markdown with book title, chapter, color, timestamp, quote block
- [x] Selection chip UI: semi-transparent dark pill above selection, color dots + ❝ Aa ⧉
- [x] Dictionary popup inside WebView: single merged entry with numbered senses and POS, bookmark + copy in the sticky header, synonym/antonym chips and idiom cards, did-you-mean suggestions, "Show N more" for long entries
- [x] Reader typography popover now includes dictionary search (prefix → substring fallback) + Save/Clear
- [x] Highlight storage: SQLite `annotations` table, `saved_words`, `dictionaries`, `dict_entries`
- [x] CSS: soft highlight tints (yellow 0.62, green, blue, pink 0.70, orange), chip & dict popup styling, badge colors for annotation list, P3 GTK rows

### Look

- Selection UI inspired by tablet readers (compact chip above selection) — implemented as `#kalam-chip` inside WebView
- Highlight colors soft (incl. pink/rose option like reference photo) — `kalam-hl-pink` rgba(251,207,232,0.70)
- Must not reintroduce blue underlines on body text — preserved via reading CSS `!important`

### Known issue (deferred)

- [ ] Triple-click paragraph selection can still render an italic run in a different temporary selection text colour from the preceding roman text. Revisit the WebKit selection rendering later.

### Out (deferred)

- Full EPUB HTML editing, sync, and collaborative notes
- CFI spec (using path+offset for now; excerpt fallback is tracked in the reader-improvements section below)
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

## Deferred features from the reference designs  (tracked, not scheduled)

Pulled from the collected design references so nothing is lost. These are
**deferred** — they do not block any phase, and are recorded here so the plan
stays honest about what the reference imagery showed that Kalam does not yet
have.

| Feature | From | Status |
|---------|------|--------|
| Reading goal progress ring/bar on Home | Home/dashboard reference | deferred — goal pref + count exist (`finished_this_year`); no ring UI yet |
| "Up next" / Continue shelf on Home | Home reference | partially shipped (Continue row); shelf-style "Up next" peek deferred |
| Half-star rating in book grid rows | Library reference | deferred — half-stars shipped on book page only |
| Similar-from-your-shelf on book page | Book page reference | deferred to its own UI round (P5.5 next list) |
| Annotation design polish (cards, colors) | Annotation reference | deferred until reader feature work is complete (milestone order) |
| Series card on book page | Book page mockup | deliberately **not** a card — series lives in the hero + float (decision) |
| Social features (sharing, activity feed) | Analytics reference | explicitly out of scope (decision 2026-07-27) |

**Rule:** a reference image is direction, not specification. When a reference
feature is requested, the mockup-first workflow applies (see P5.5 working
method) before any Rust.

---

## Reader improvements — annotation workflow  ◀ current track

This track follows the shipped P2/P3 text reader. It keeps the work in the
order we agreed: finish the annotation workflow first, then improve anchoring,
annotation controls, and dictionary behaviour. Larger reader architecture
changes come last.

### Milestone 1 — annotation workflow  ✅ complete

A user can:

1. [x] Click an annotation in the right panel.
2. [x] Load its chapter when necessary.
3. [x] Restore the exact saved text location.
4. [x] Scroll that location into view near the top.
5. [x] Briefly emphasize the location without changing the permanent highlight.
6. [x] Edit the annotation note in an expandable multiline editor with autosave.

This uses the existing chapter index, DOM paths, start/end offsets, text
excerpt, note field, and `update_annotation_note`. The current restoration
prefers an injected annotation ID when available, then validates the existing
DOM path/offset range against the saved excerpt. If that anchor is missing or
points to different text, it falls back to matching the saved text excerpt.

Related reader work already completed:

- [x] Compact themed selection toolbar with tooltips, rounded ends, and no
      decorative pointer.
- [x] Temporary selection handles are draggable for pointer/touch input while
      preserving native selection, copy, and annotation actions.
- [x] Temporary selection bands update live while a fresh mouse/touch selection
      is being extended; handles and actions wait until pointer-up.
- [x] Default WebKit context menu suppressed without affecting text selection
      or the automatic selection-actions toolbar.
- [x] Dark rounded annotation cards with subtle pastel tints, colored left
      edges, no color dot, and improved quote presentation.
- [x] Saved note previews as note indicators.
- [x] Annotation filtering by color/type.
- [x] Existing highlights can be recolored from each card using the current
      pastel palette.
- [x] Annotation hover styling fixed so quote buttons do not add a second light
      highlight.
- [x] CI green for the current reader changes: rustfmt, Clippy with `-D
      warnings`, debug build, and release build.
- [x] Arch UX sign-off for this completed milestone.

### After milestone 1 — agreed order

1. **Hybrid anchoring**  ✅ complete
   - [x] Try the existing DOM path and start/end offsets first.
   - [x] Validate that path result against the saved text excerpt.
   - [x] Fall back to matching the saved text excerpt when that location is
         missing or points to different text.
   - [x] Keep full EPUB CFI for later; the current system was not replaced.

2. **Improve annotation controls**  ◀ current reader work
   - [x] Edit notes.
   - [x] Recolor existing highlights.
   - [x] Show note indicators through saved note previews.
   - [x] Add text search across saved highlight text and notes; keep the
         existing color/type filters.
   - [x] Improve quote/highlight presentation with the approved dark card design.

   The next isolated reader change is dictionary behavior. Annotation-control
   design polish remains deferred until the feature work is complete.

3. **Improve dictionary behavior**
   - [x] Better phrase selection: dictionary lookup preserves the selected
         phrase instead of reducing it to the first word.
   - [x] Punctuation and simple inflection handling for lookup terms.
   - [x] Multiple results, with up to five entries and separate save/copy
         actions.
   - [x] Safe formatting for dictionary text shown in the WebView popup.
   - [x] Separate bundled English idiom and expression entries, while keeping
         ordinary phrase lookup and future phrase-composition policy separate.

   Dictionary feature work is deferred for now. When it resumes, follow the
   planned dictionary overhaul below in phase order; the bundled phrase pack
   still does not make every compositional phrase meaningful automatically.
   **Phases 1–5.5 of that overhaul are shipped (precomputed headword key
   index, WordNet exception-list lemmatization, phrase decomposition,
   merged dictionary store, popup redesign, likely-sense hint; the POS
   pill shipped but POS grouping dividers remain deferred). Offline
   pronunciation (CMU Pronouncing Dictionary → IPA, Phase 5.6) is
   shipped too. The Settings reorder UI remains deferred.**

4. **Only later consider architecture changes**
   - [ ] Multi-chapter buffering.
   - [ ] Book-wide continuous scrolling.
   - [ ] Automatic chapter advance redesign.
   - [ ] Advanced CFI support.

### Reader constraints that remain locked

- Keep temporary text selection separate from permanent saved highlights.
- Do not add multi-chapter buffering until anchoring and progress behaviour are
  settled.
- Do not reintroduce an always-running background progress timer; use
  event-based or debounced persistence instead.

---

## Dictionary overhaul — planned reader-improvement track

**Status: Phases 1–10 shipped (headword key index, WordNet exception
lemmatization, phrase decomposition, merged dictionary store, popup
redesign, likely-sense hint, offline pronunciation, tap-to-look-up +
popup keyboard + find in chapter, vocabulary review + CSV/Anki export,
POS grouping dividers, dictionary priority reorder UI, lookup history).
Phase numbering is track-local — separate from the P0–P11 project
phases.** The phases are recorded here as the implementation brief for
future isolated reader-improvement steps.

### Implementation brief: Kalam dictionary overhaul

#### Context for the implementing AI

Kalam is a Rust + GTK4 + Relm4 + WebKitGTK ebook reader. Work on the branch
your session names (see "Hard rules" above). The dictionary spans three areas:

- `src/db.rs` — schema/migrations. `migrate()` uses `CREATE TABLE IF NOT EXISTS`
  plus guarded `ALTER TABLE ... ADD COLUMN` plus a `SCHEMA_VERSION` constant /
  `schema_version` table. Structs: `DictEntry { id, dict_id, word, definition }`,
  `Dictionary { id, name, lang, entry_count, added_at }`, `SavedWord`.
  `dict_entries(id, dict_id, word, definition)`;
  `saved_words(id, word, definition, dict_name, book_id, chapter_index,
  context_text, created_at)`.

- `src/db/dictionaries.rs` — `search_dict`, `search_dict_exact_or_prefix`,
  `search_dict_substring`, and query-planning helpers
  `dictionary_query_variants`, `normalize_dictionary_term`,
  `dictionary_possessive_base`, `simple_inflection_variants`.

- `src/dict.rs` — importers (`import_stardict`/`import_sqlite_pack`/`import_tsv`),
  `install_bundled_dictionaries` (WordNet + idioms shipped as
  `resources/dictionaries/*.tsv.gz` via `include_bytes!`), and
  `strip_dict_html`.

- `src/epub_book.rs` — reader WebView JS + CSS: `kalamHandleDict`,
  `showDictPopup`/`ensureDictPopup`, `window.kalamShowDict`, and the
  `#kalam-dict-popup` / `.kalam-dict-*` CSS.

- `src/pages/reader.rs` — Relm4 messages `ReaderMsg::DictSearch`,
  `DictSearchSelect`, the `dict-lookup` and `save-word` bridge handlers,
  `show_dict_in_webview(...)`, and fields `dict_lookup_word/def`, `dict_context`.

#### Conventions to follow strictly

- Migrations: add tables with `CREATE TABLE IF NOT EXISTS`, add columns with
  guarded `ALTER TABLE`, bump `SCHEMA_VERSION`, and backfill existing rows
  (users already have imported dicts + 127k-entry WordNet). Never drop/recreate
  `dict_entries`.

- All work is offline, single-process. No network calls, no new services.

- The dictionary popup is app chrome: keep it dark regardless of the reader's
  Light/Sepia/Dark paper theme. Match the existing chip:
  `border-radius: 16px`, `backdrop-filter: blur(22px)`, the current shadow, and
  `@kalam`/`--kalam-*` color variables. Do not introduce literal hex where a
  theme variable exists (see `ARCH.md` note on `style.rs`).

- Add `#[cfg(test)]` unit tests next to new pure functions (the query-planner
  already has tests — extend them).

- Verify with `cargo build` and `cargo test` after each phase.

- Do **not** rewrite unrelated code. Keep diffs scoped.

Build in the phase order below; each phase compiles and is independently useful.

#### Phase 1 — Precomputed headword index  ✅ complete

**Goal:** exact/lemma lookups hit an index instead of `LIKE` scans; kill the
`LENGTH(word)` tiebreak proxy.

- [x] `migrate()` adds `dict_entries.key TEXT` guarded by `PRAGMA table_info`
      and `CREATE INDEX idx_dict_entries_key ON dict_entries(key COLLATE NOCASE)`;
      `SCHEMA_VERSION` bumped to 11.
- [x] `fold_key(word)` in `src/db/dictionaries.rs`: lowercase, NFD + drop
      combining marks, collapse whitespace, trim surrounding non-alphanumerics
      per token — reuses `normalize_dictionary_term`.
- [x] All imports (StarDict / SQLite / TSV / bundled packs) write
      `key = fold_key(word)` at insert time via
      `insert_dict_entry` / `batch_insert_dict_entries`.
- [x] One-time backfill for pre-v11 rows: batched in 2,000-row write
      transactions, guarded by `key IS NULL` so it runs once and no-ops on
      fresh databases.
- [x] `search_dict_exact_or_prefix` matches `key = ?` (exact) then
      `key LIKE ?||'%'` (prefix), exact-first; the `LENGTH(word)` tiebreak is
      gone (prefix follows index order).

**Acceptance:** `Run`, `run`, `rún` all resolve to `run` — unit-tested with an
in-memory catalog, including an `EXPLAIN QUERY PLAN` assertion that the exact
lookup uses `idx_dict_entries_key`; existing databases upgrade in place (no
reimport, no re-download).

#### Phase 2 — Real lemmatization from WordNet data  ✅ complete

**Goal:** irregulars resolve (`went→go`, `mice→mouse`, `better→good`), with suffix
rules as fallback only.

- [x] Princeton WordNet 3.0 exception lists (`noun.exc`, `verb.exc`,
      `adj.exc`, `adv.exc`) shipped gzipped under `resources/dictionaries/`
      as `wordnet-3.0-*.exc.gz`, embedded via `include_bytes!` like the
      existing WordNet TSV. Provenance, SHA-256 checksums and the WordNet
      licence reference live in `resources/dictionaries/wordnet-3.0-exc.NOTICE.txt`.
- [x] Loaded once into a `HashMap<String, Vec<String>>` (surface → lemmas),
      lazily via `OnceLock` in `src/db/dictionaries.rs`.
- [x] `dictionary_query_variants` consults the exception map (lowercased
      surface) before `simple_inflection_variants`; the suffix rules run only
      when the surface is not in the lists. Dedup stays in
      `push_dictionary_variant`.
- [x] Tests extended: irregular variants (`went→go`, `mice→mouse`,
      `better→good`+`well`, `children’s→child`, capitalized `Went`), the
      suffix fallback (`walked→walk`), no junk stems on irregular hits
      (`better` ≠ `bett`), and end-to-end `search_dict` resolution.

**Acceptance:** `went→go`, `mice→mouse`, `better→good`, `running→run` all return
a headword (unit-tested against the shipped lists); regular cases still work.

#### Phase 3 — Phrase decomposition  ✅ complete

**Goal:** `odd mixture` yields something useful instead of a dead end.

- [x] `search_phrase(phrase, limit) -> PhraseLookup` in `db/dictionaries.rs`:
      (a) the whole phrase as a headword via the query variants (exact then
      prefix on the precomputed key); (b) the longest contained multi-word
      headword, sliding a window from longest to shortest with an exact-only
      lookup (catches `run out of steam` inside a longer selection);
      (c) per-token single-word `search_dict` results (lemmas included, so
      `went` inside a phrase still resolves to `go`). Tokens with no hits are
      omitted from the breakdown.
- [x] `PhraseLookup` enum: `Phrase(Vec<DictEntry>)` vs
      `Breakdown(Vec<(token, entries)>)` vs `Empty`.
- [x] Every reader lookup path routes through the shared phrase pipeline
      (`lookup_dict`): the selection popup's `dict-lookup` bridge and the
      sidebar Words search box both call `search_phrase` for queries with
      `>1` token, so a phrase typed in the sidebar never dead-ends either.
      A breakdown renders each token's best hit in the existing popup /
      sidebar list (capped at five / thirty) until Phase 5's popup redesign
      turns it into clickable breakdown chips. Single-word lookups are
      unchanged.
- [x] Unit tests: full-phrase headword, contained phrase headword,
      per-token breakdown, tokens without hits omitted, irregular token
      inside a phrase, and the empty case.

One deliberate deviation from the brief: step (a) uses the headword variants
(exact/prefix) rather than `search_dict`'s definition-substring fallback, so
a phrase lookup can never "match" an entry whose *definition* merely
contains the words. Substring-in-definition results stay single-word-only
until Phase 4 tiers them.

**Acceptance:** `odd mixture` (no headword) returns a breakdown for `odd` and
`mixture`; `run a risk` (if present) returns the phrase entry; single words
unchanged.

#### Phase 4 — Merged dictionary store (one word = one entry)  ✅ complete

**Goal (user decision):** one word appears exactly once, with one dictionary
speaking for it — no duplicate cards, no per-dictionary boxes, no source
clutter in the reader.

**Design (agreed):**

- **Priority, not ranking.** Every dictionary has a `priority` (lower =
  consulted first). The dictionary with the lowest priority that has the
  word provides the whole entry — every sense of *that* dictionary's word.
  Priority wins even when a lower-priority dictionary has more senses; the
  other dictionaries' copies of the word stay in the database but are never
  shown. If no dictionary has the word, the next in priority order speaks.
- **A combined store.** `combined_words` (schema v12) holds one row per
  headword key: the winning dictionary's id plus its senses as a JSON array
  (deduplicated, in entry order). The reader searches only this table, never
  the raw entries.
- **Automatic rebuild.** The store is rebuilt whenever the dictionary set
  changes — bundled install, import, removal — and once at migration for
  existing databases. Rebuild is one pass over the entries ordered by
  priority (first key seen wins) plus one bulk insert, all in one
  transaction.
- Bundled priorities: WordNet 10, Idioms 20, Synonyms 30, Antonyms 40;
  imported packs default to 100. A Settings reorder UI is deferred.

**Acceptance:** a word in both WordNet and an imported dictionary shows the
WordNet entry only; `set` with 15 senses in one dictionary and 15 in another
shows 15, not 30; removing a dictionary drops its words (or falls back to the
next in priority); identical duplicate definitions collapse to one sense.

#### Phase 5 — Popup redesign (app chrome, dark)  ✅ complete

**Goal:** fix the fake result, structure the entry, label sources. All in
`epub_book.rs` (`showDictPopup` + CSS) and the `reader.rs` handler that feeds it.

**Do:**

- Empty state: remove the `No definition found... Total dict entries: N` string
  entirely. When there's no hit, render a distinct empty-state block (not a
  `.kalam-dict-result`): a short `No entry for '{query}'.` plus, for phrases,
  the breakdown chips from Phase 3 (`[odd] [mixture]`, each clickable → re-fires
  `dict-lookup` for that token via `kalamBridge`). Also a `Search in book` action
  (Phase 6).

- Kill `RESULT n` and the fake result: replace the subheading with a quiet
  count line. The Phase 4 merged store means one entry per word, so the old
  per-dictionary tabs idea is obsolete — do not reintroduce tabs.

- Structure the entry: headword once at top. The senses now arrive numbered
  from the merged store; render them as separate lines (the popup body
  already preserves newlines). For imported HTML dicts, sanitize (allowlist
  `b/i/em/strong/br/p/ul/li/span`, drop scripts/handlers) and render instead
  of `strip_dict_html` flattening — add a `sanitize_dict_html` fn.

- One action bar: a single Save / Copy / Highlight-in-book row acting on the
  entry, instead of per-result button pairs.

- Anchor discipline: keep the existing rect-anchored placement + above/below
  flip; add a small caret pointing at the word and ensure it never overlaps
  the selection rect (nudge if it would).

- Theming: keep dark chrome on all paper themes. Reuse chip tokens (radius,
  blur, shadow, `--kalam-*`). Add a subtle border for contrast over light/sepia
  pages.

**Acceptance:** the fake `1 RESULT / No definition / Total dict entries` is
gone; one clean entry per word with numbered senses; phrase misses show
tappable word chips; imported HTML renders formatted; popup stays dark on
sepia.

#### Phase 5.5 — POS grouping + Lesk "likely sense" hint  ✅ complete

**Goal:** make senses easier to scan by grouping on part of speech, and
optionally mark the sense most likely to fit the reader's sentence — without
ever hiding a sense. Fully offline, using WordNet data already shipped in
`resources/dictionaries/`. Applies only to the bundled WordNet entries; imported
dicts render as-is.

**Hard rule for the implementing AI:** this feature may reorder or highlight
senses; it must never remove or collapse them. Every sense that matched stays
visible. A wrong guess must cost at most a misplaced highlight, never a hidden
answer.

**Do:**

- **Expose the context sentence.** The `dict-lookup` bridge already sends
  context (the surrounding text) and `reader.rs` stores it as `dict_context`.
  Ensure the full sentence — not just the selected word — reaches the ranking
  step. If tap-to-lookup (Phase 6) is present, use the sentence the tapped word
  sits in.

- **POS grouping (primary, low-risk).**

  - Parse the WordNet definition text into senses tagged by part of speech.
    WordNet glosses carry POS; if your bundled TSV flattened that, derive POS
    from the WordNet data files instead (extend the resource shipped in Phase 2
    to retain POS per sense).

  - In the popup (`showDictPopup` in `epub_book.rs`), render senses grouped
    under POS dividers — verb, noun, adjective, adverb — in that fixed order,
    numbered within each group. This is the main clarity win and carries
    essentially no risk.

  - Optional light POS preference from context: if the token is preceded by an
    article/adjective ("an odd mixture") lean noun; if by "to"/a subject
    pronoun, lean verb. Use this only to decide which POS group is shown first,
    never to drop groups. Keep the heuristic in a small pure fn with
    `#[cfg(test)]` cases.

- **Simplified Lesk soft-highlight (optional, transparent).**

  - Add a pure fn `likely_sense(context_sentence, senses) -> Option<sense_index>`
    in `db/dictionaries.rs` (or a new `wsd.rs`): lowercase + tokenize the
    context sentence into a set; for each sense, build a bag of words from its
    gloss and examples; score by set overlap (ignore stopwords — ship a tiny
    stopword list). Return the top-scoring sense index, or `None` if the best
    overlap is zero (no evidence → no hint).

  - In the popup, mark that sense with a subtle "likely here" badge/accent (use
    `--kalam-*` accent, app-chrome dark, consistent with the chip). Do not move
    it out of its POS group and do not restyle the others into looking disabled.

  - Ties or zero-overlap: show no hint rather than an arbitrary one.

- **Settings toggle.** Add a pref (via the existing `app_prefs` table / prefs
  module) `dict_sense_hint` defaulting to on, so the user can disable the Lesk
  highlight while keeping POS grouping. POS grouping itself is always on.

- **Tests.** Unit-test `likely_sense` on 2–3 hand-built cases (a sentence that
  clearly favors one gloss returns that index; a neutral sentence returns
  `None`). Unit-test the POS-preference heuristic.

**Acceptance:**

- WordNet lookups show senses grouped under POS headers, numbered within each
  group; all senses remain visible.

- With a context sentence that clearly matches one gloss, that sense gets a
  "likely here" marker and its POS group sorts first; a neutral/empty context
  produces no marker and no sense is hidden.

- Turning off `dict_sense_hint` removes the highlight but keeps POS grouping.

- No model files, no network, no new runtime dependency; `cargo build` and
  `cargo test` pass.

**Out of scope for this phase:** embedding/transformer WSD, knowledge-graph
(UKB) methods, and anything that picks a single sense and hides the rest.

### Done

- **POS pill (primary item, shipped earlier)** — the header shows the
  entry's parts of speech from the WordNet pack groups; senses are never
  collapsed.
- **Lesk "likely here" hint** — `likely_sense_index` in `db/dictionaries.rs`
  (pure fn, unit-tested): stopword-filtered content-token overlap between the
  context sentence and each sense's gloss + example. Returns `None` on zero
  overlap and on ties, so a neutral sentence shows no marker and a wrong
  guess costs at most a misplaced highlight. Two refinements keep real-world
  hints honest: the looked-up headword itself is excluded from the context
  bag (it appears in many of its own glosses and would manufacture ties —
  "deposit" is what decides the financial sense of "bank"), and both sides
  are lightly stemmed by a tiny rule-based stemmer so inflected words match
  gloss base forms ("deposits" ≈ "deposit", "running" ≈ "run").
- **Full sentence context** — the bridge already carried `context`; the popup
  now sends the whole sentence around the selection
  (`getContextSentence` in `epub_book.rs`), not just the selected word, to
  the ranking step.
- **Badge** — the hinted sense gets a small accent "likely here" pill
  (app-chrome `--kalam-*` tokens, dark on all paper themes), stays inside its
  list position, and no other sense is restyled.
- **Toggle** — reader settings → Dictionary → "Sense hint" switch persists
  `dict_sense_hint` in `app_prefs` (default on). Off removes the highlight;
  POS grouping stays always on.
- **Scope guard** — the hint is only computed for WordNet entries (senses
  carry POS) and only when a context sentence exists (sidebar searches get
  none).

#### Phase 5.6 — Offline pronunciation  ✅ done

**Goal:** the popup shows how a word is pronounced, fully offline, with no
new runtime dependency.

- [x] **Data** — CMU Pronouncing Dictionary 0.7a (BSD-style redistribution
      licence) packed as `resources/dictionaries/cmudict-0.7a.tsv.gz`:
      133,737 `word\tARPABET` rows (123,455 unique words), variants in
      counter order, compiled into the binary via `include_bytes!`.
      Provenance, licence text and the SHA-256 checksum live in
      `resources/dictionaries/cmudict-0.7a.NOTICE.txt`.
- [x] **`src/db/pronunciation.rs`** — lazy `OnceLock` parse (same pattern as
      the WordNet `.exc` lists) into a `fold_key`-keyed map; ARPABET →
      compact IPA (`B AE1 NG K` → `ˈbæŋk`, stress marks included); lookups
      fall back through `dictionary_query_variants` so inflected forms
      resolve. Unit tests cover the mapping, stress marks, case/
      diacritic-insensitive lookup and the packed table's coverage.
- [x] **Popup** — the `#kalam-pronunciation` slot (monospace, dim, hidden
      while `:empty`) is now filled by the reader payload, e.g. `bank` →
      `/ˈbæŋk/`; words absent from cmudict simply show no transcription and
      the popup layout is unchanged.
- [x] **Docs** — README feature list + this roadmap entry updated; popup
      preview regenerated with a pronunciation line.

**Acceptance:** fully offline (no network, no new crate); `bank` →
`/ˈbæŋk/`, `run` → `/ˈrʌn/`; unknown words render without a pronunciation
line; `cargo build` and `cargo test` pass.

**Deferred:** multi-pronunciation variants (first variant is shown), the
Settings reorder UI, and anything involving network pronunciation services.

#### Phase 6 — Interaction  ✅ done

**Goal:** tap-to-look-up and keyboard parity.

- [x] **Tap-a-word** — a plain click on book content (no drag-select, no
      link/image/UI node) resolves the word under the caret via
      `caretFromPoint` + `wordFromCaret` (letters, digits, apostrophes,
      hyphens; expanded to word boundaries) and fires `dict-lookup` with
      that word, its surrounding sentence (`sentenceAroundText`, shared
      with selection lookups) and a rect for anchoring. A short tap delay
      (240 ms) keeps double-click-to-select working (the pending tap is
      cancelled on the second pointer press and on `dblclick`). Tapping a
      word while the popup is open swaps the entry instead of closing it;
      tapping empty space closes it. Drag-select → phrase is unchanged.
      The `D` key with no selection re-looks-up the last tapped word,
      falling back to the sidebar dict-shortcut when there was no tap.
- [x] **Keyboard** — Esc closes the popup (existing); ↑/↓ move a sense
      focus ring (`k-def-focus` accent card, wraps around, reveals hidden
      "Show N more" senses when focus lands there); Enter saves the word
      with the *focused* sense's definition (header Save still uses the
      first sense). **Deviations from the brief:** ←/→ are deliberately
      not bound — the Phase 4 merged store removed dictionary tabs, and
      GTK reserves ←/→ for chapter navigation.
- [x] **Find in chapter** — the popup header gains a magnifier button
      (between Save and Copy). The reader has no in-book search yet, so
      per the brief this is scoped to highlighting every occurrence of the
      headword in the current chapter: `kalamSearchInBook` wraps each
      match in a temporary accent `kalam-search-hit` span (case-
      insensitive, skipping annotations/links/UI), scrolls to the first,
      and reports the count back for a toast ("N matches in this
      chapter"). Esc or the next lookup clears the hits.
- [x] **Tests** — the popup preview harness
      (`docs/files/test_kalam_dict_preview.js`, jsdom) grew to 55 checks:
      word-from-caret expansion, case-insensitive sentence context, the
      tap→bridge flow (word + sentence + rect), popup keyboard (focus
      movement, wrap, reveal-hidden, Enter-save with the focused sense),
      and find-in-chapter (wrapping, count, no double-wrap, annotation
      skip, clearing).

**Acceptance:** single tap on a word opens the popup with its sentence
context; Esc closes it; ↑/↓ + Enter work; Find in chapter highlights all
occurrences and reports the count.

#### Phase 7 — Vocabulary tools  ✅ done

**Goal:** make Saved Words more than a list.

- [x] **Schema v13** — `saved_words.known INTEGER NOT NULL DEFAULT 0` via the
      guarded `add_column_if_missing` ALTER (existing rows default to
      to-review; no backfill needed). `SCHEMA_VERSION` bumped to 13.
- [x] **Review view** — the Saved Words page
      (`src/pages/saved_words.rs`) gains a review scope: **All / To review /
      Known** radio toggles (same `group_toggles` pattern as History),
      each word row gets a **mark known** check button (accent-filled when
      known; click again to move back to review), and known rows recede
      visually (dimmed title/definition). The status line reports counts
      ("3 to review · 12 words saved · 9 known"). The reader-sidebar
      vocabulary list is unchanged.
- [x] **DB layer** — `list_saved_words(query, known: Option<bool>)` filters
      by review status (combined with search); `set_saved_word_known(id,
      known)` toggles the flag. Unit test covers the flag round-trip, both
      filter directions, toggling back, and search+filter combination.
- [x] **Exports** — **Export CSV** writes `~/SavedWords.csv`
      (`word,definition,context,dictionary,known,created_at`, RFC-4180
      quoting, definition newlines collapsed) and **Export Anki** writes
      `~/SavedWords-Anki.txt` (tab-separated `word / definition / context`
      with `#separator:tab` header — Anki's default import format), both
      mirroring the `~/Quotes.md` pattern (fixed home path + toast with
      count and path). The pure string builders are unit-tested for
      escaping and column counts.

**Acceptance:** saved words can be marked known (and back), filtered by
review status, and exported to CSV and to Anki's TSV format.

#### Phase 8 — POS grouping dividers (finishes P5.5)  ✅ complete

**Goal:** group the senses in the dictionary popup under part-of-speech
dividers instead of one flat numbered list.

**Already in place — do not rebuild:** `Sense { number, pos: Option<String>,
def, example }` and `EntryData.pos: Vec<String>` are populated by
`parse_pos_blob`/`distinct_pos` in `db/dictionaries.rs`, and `pos` already
reaches the popup as `payload.pos` (per-entry at reader.rs:1927, per-sense at
1930). This is a **rendering-only** change in `showDictPopup`
(`src/epub_book.rs`, around the `defItem` helper) plus CSS.

### Shipped

- [x] In `showDictPopup`, senses are grouped by `s.pos` with a divider row
      before each group: **noun, verb, adjective, adverb**, then remaining
      POS in first-appearance order; senses with `pos == null` go in a final
      unlabelled group with no divider. A group straddling the "Show N more"
      fold keeps a single divider at its true start.
- [x] **Flat-index invariant held:** `Sense.number` stays sequential across
      the whole entry and the flat senses array is untouched, so
      `payload.hint` and the ↑/↓ keyboard walk still index the same senses.
- [x] `.k-pos-divider` CSS rule added in the popup block (small uppercase
      label + hairline rule, `--kalam-*` variables).
- [x] The header `.k-pos` chip is dropped when there are 2+ groups and kept
      for a single group.

**Acceptance:** a multi-POS word (e.g. `run`) shows `noun` / `verb` dividers
with senses grouped beneath; sense numbers remain continuous 1..n across the
whole entry; the "likely here" badge still lands on the same sense as before;
↑/↓ still walks every sense in flat order; a single-POS word looks unchanged
apart from one divider or the retained header chip.

#### Phase 9 — Settings: dictionary priority reorder UI  ✅ complete

**Goal:** let the user order their dictionaries, so the merged store's
"which dictionary speaks for this word" is user-controllable.

**Context (verified against the code):** schema v12 added
`dictionaries.priority` (lower = consulted first, imports default 100).
`set_dictionary_priority()` and `rebuild_combined_dictionary()` exist and
work. But **`priority` is currently write-only** — it is missing from the
`Dictionary` struct (`db.rs:170` — only id, name, lang, entry_count,
added_at) and from `list_dictionaries()` (`db/dictionaries.rs:122`, which
selects no priority and orders by `name ASC`). The plumbing must be added,
not just buttons. Verified already wired: `rebuild_combined_dictionary()`
runs after import (`dict.rs:219`), bundled install (`dict.rs:95`) and
removal (`db/dictionaries.rs:170`) — so step 4 below is about *priority
changes*, not the other paths.

### Shipped

- [x] `Dictionary` gains `pub priority: i64`; `list_dictionaries()` selects it
      and orders `ORDER BY priority ASC, name ASC` — list order *is*
      effective order.
- [x] `SettingsTab::Dictionaries` rows get **↑ / ↓** buttons (reading-list
      reorder pattern). On reorder: all dictionaries are **renumbered
      compactly** (0, 1, 2, …) via `set_dictionary_priority()`, then
      **`rebuild_combined_dictionary()`** runs, then the list refreshes; ↑ is
      disabled on the first row and ↓ on the last.
- [x] The top row carries a quiet **"speaks first"** chip so the order's
      meaning is self-explanatory.

**Acceptance:** the Dictionaries tab lists packs in priority order; moving a
pack to the top makes its definition the one the reader popup shows for a
word both packs contain; the change survives a restart; importing a new pack
lands it last, not first.

#### Phase 10 — Lookup history (optional, lower priority)  ✅ complete

**Goal:** an append-only record of every dictionary lookup, separate from the
deliberate saves in Saved Words. Enables "what was that word?", surfaces
repeat lookups, and can propose study sets for the Phase 7 vocabulary tools.

### Shipped

- [x] **Schema v14** `dict_lookups(id, word, book_id → books ON DELETE
      SET NULL, chapter_index, context_text, found INTEGER NOT NULL, at
      TEXT)` + `idx_dict_lookups_word` (NOCASE) and `idx_dict_lookups_at`.
- [x] **Misses and hits are both logged** (`found = 0` when the entry has no
      senses). Logging lives in `lookup_dict` in `src/pages/reader.rs` — the
      funnel both the `"dict-lookup"` selection/tap path and the sidebar
      search go through — so sidebar lookups are recorded too (a literal
      `lookup_dict`-only pin was rejected: it silently skips sidebar
      searches). Per-keystroke search prefixes are **not** logged.
- [x] **Repeat collapse:** the insert is skipped when the same word (NOCASE)
      + same `IFNULL(book_id, -1)` was logged within the same ISO hour — the
      `reading_events` pattern, so flipping back to a word doesn't flood the
      log.
- [x] Pref `dict_history_enabled`, default `1`, wired like `dict_sense_hint`
      with a **Lookup history toggle in reader Settings**.
- [x] **Lookup History page** (Library section): day-grouped list, search
      filter, "not found" chip, **Clear history** button (with outcome
      notification), 500-row read limit.
- [x] `repeat_lookup_words(limit)` — words looked up more than once, most
      frequent first — feeds **"Suggest from history"** on the Saved Words
      page (word ×N chips) and a **"Last 3 lookups" dashboard card** on
      Library with a miss badge.

**Privacy requirement (shipped with the feature):** the reader Settings
toggle stops all writes, and Clear empties the table — both landed in the
same phase, not after.

**Acceptance (CI-verified, 156 unit tests green):** looking up a word writes
one row; looking it up again immediately writes none; looking up a word with
no definition writes a row with `found = 0`; disabling the pref stops all
writes; clear empties the table; the day-grouped list renders and the
repeat-lookups query returns sensible counts.


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
- **Settings (`src/pages/settings.rs`)** — redesigned into a two-column layout: a 220px navigation rail with 6 categories (Appearance, Storage & Backup, Dictionaries, Book Files, Metadata Sources, Notifications) and rounded cards (`kalam-card`) grouping individual setting rows.
- **Settings v2** — rebuilt again in the P5.5 design language from the user's
  `settings.html` reference, reconciled against shipped features: grouped nav
  rail (Appearance / Library / Sources / App overlines), section cards
  (accent glyph + title + description + divider + rows), hairline-separated
  setting rows, Material-You-style pill switches for every real pref
  (EPUB writeback, Open Library, Google Books), theme picker grouped by family
  with per-variant cards (mini UI preview + swatch strip + ✓ seal on the
  active theme), dictionary rows with icon blocks, notification history with
  colour-correct badges, and an Export quotes card sharing the saved-quotes
  Markdown exporter. Nothing faked: every control backs a real mechanism.
- **Book detail page (`src/pages/book.rs`)** — full-page rewrite to the
  `docs/files/book_detail.html` mockup. Fixed chrome row (back pill left,
  metadata pencil right), hero with 3D cover + progress + Format/Series/
  Publisher meta, title/author/half-stars/inline tag chips (add + remove),
  action row (Read / reading list / shelves / finished / Remove), serif
  description. Card grid: reading stats (4 tiles: time, estimate, minutes per
  chapter, sessions) + 7-day bars + timeline (recent sessions, finished,
  first opened); highlights (≤4, View-all dialog with delete); author
  (avatar, birth year only when known, bio, other owned books clickable);
  reading journey (✓/●/○ chapters from the EPUB spine, +N more expand);
  book file (name/size/imported/hash, show in file manager). The app's back
  chip hides on this page — the page owns its own. Series is **not** a card.
- **Series float (`src/pages/series_float.rs`, schema v10)** — the series name
  in the hero opens a floating window with the full listing. Fetched once from
  Open Library (series field, quoted-query fallback), cached in
  `series_cache` keyed by normalised `series|first_author`; covers cached in
  `series-covers/`. Works you own are title-matched and get live badges
  (Read / N% read / Not started) and open their book page; the rest read
  "Not in library". Footer says where and when it was fetched; ⟳ is the only
  re-fetch. An empty OL result is shown but not cached.


### Next
1. Home / dashboard — the two-column layout the design references imply.
2. Library, Reader chrome, dialogs.
3. "Similar from your shelf" on the book page — deferred from the mockup to
   its own round.

### Hard-won GTK/CSS rules

These are written up properly in the module header of `src/style.rs`. Read that
before touching the scrollbar block — the summary:

1. **This stylesheet already outranks Adwaita.** `relm4::set_global_css` loads
   at `APPLICATION` (600), the theme at `THEME` (200), and priority beats
   specificity. Long `:not()` chains are unnecessary, and GTK drops an entire
   comma-separated rule when one selector in the group fails to parse.
2. **Never use `opacity` below 1 on a widget that can collapse.** It forces an
   offscreen surface; a collapsed overlay scrollbar's is zero-sized and pixman
   rejects it. Hide with a transparent `background-color` instead.
3. **`margin`/`border`/`padding` are subtracted from the allocation.** Adwaita's
   slider carries 16px of them, so resetting only the border still leaves 8px
   and every `min-width` comes out negative. Zero all three, then set the size.
4. **`scrolledwindow:hover` matches the whole content area**, not the scrollbar.
   Use `scrollbar:hover` / `scrollbar.hovering`.
5. **`KALAM_NO_CSS=1` runs with no custom stylesheet** — use it to confirm a
   warning is even ours before theorising. `GTK_DEBUG=interactive` shows which
   rule actually wins on a node.

Cost of learning this the wrong way: about a dozen rounds on one scrollbar.
Each fix was plausible, none was verified against the toolkit's actual
behaviour first. When a warning carries a number, do the arithmetic across
runs — the constant that keeps appearing is the answer.

### Notes
- Reader *page* theming (Light/Sepia/Dark paper) stays separate from app
  chrome: a sepia page inside a dark app is a legitimate combination.
- The images in `docs/design/` are **inspiration the user collected**, not
  their own designs. Treat them as direction, not specification.

---

## A0 — Architecture & performance track  ◀ NEXT

**Status:** decided 2026-09-02 (design in `docs/conversation.md` §§1–3);
**steps 1 and 3 are done; CI green.** A track, not a phase — interleaves with
P6–P11.
- **Step 1 (measure) — done.** Data layer (headless `src/perf.rs`) confirmed all
  list-page queries < ~20 ms for 2,000 books; GUI (`src/timing.rs`,
  `KALAM_TIMING=1`) confirmed cold start ~0.9 s warm, book open 3.5 ms revisit,
  chapter turn ~50 ms warm, ~400 ms after a WebView re-spawn. The DB and warm
  reader are not the bottleneck; the cost is first-open + WebKit re-spawn +
  per-card decode of full covers.
- **Measured on the user's Arch machine, 2026-09-02 (post-WebView-pool).** The
  steady state is good: chapter turns settle at **37–48 ms**, repeat book opens
  at **2.8–215 ms**, and **no ~400 ms WebKit re-spawn appears after the first
  book** — the pool works. First book of a session still pays WebKit process
  startup (`chapter_load` 2772.9 then 697.2 ms); unavoidable without pre-warming.
- **Cold start, resolved 2026-09-02 — there was no regression.** The alarming
  8018.7 ms was a *first-run-after-build* artefact. Six consecutive runs:
  6290 → 1596 → 965 → 852 → 945 → 831 ms, i.e. **~900 ms steady, exactly the
  documented baseline**. The decay is the OS page cache warming on a
  freshly-linked binary (and its GTK/WebKit/ICU shared libraries), not app work.
  **My stated suspect — the bundled-dictionary import — was wrong**:
  `startup_dicts` measured **0.1–0.2 ms on every run including the first**, so
  the pref early-out was already doing its job. Splitting the span is what
  disproved it; the guess would have sent a fix at the wrong code.
- **What the breakdown does show.** Of a steady ~898 ms cold start, our
  instrumented work is **~138 ms (16%)**: `startup_db_open` ~6 ms (nothing to
  win), `startup_dicts` ~0.1 ms (nothing to win), `startup_first_page`
  ~137 ms — the only app-side target, and the one A0 can actually move.
  The remaining **~754 ms (84%) is un-instrumented**: GTK/libadwaita init,
  WebKit process setup, CSS parsing and GTK's first layout/realize, most of it
  before `AppModel::init` runs. So cold start is **not** a data-layer or
  page-construction problem, and further service-layer work will not touch it.
  A0 step 5 should treat ~750 ms of toolkit startup as the floor unless the
  first paint is decoupled from full initialisation.
- **Step 3 (thumbnails) — done.** `src/thumbs.rs` generates a persistent
  256×408 thumbnail (`cache/thumbs/<uuid>.png`) at import and on cover
  replacement; the grid decodes that instead of the full cover when the slot is
  small enough (never upscales it). New dep: `image` (default features off; only
  png/jpeg/gif/webp). **Existing books** are backfilled on a background thread at
  startup so nothing needs re-importing. **Next on this front:** the async
  *swap-in* (placeholder → texture on a worker) belongs to the task manager
  (step 4), where it architecturally lives.
- **Step 2 (LibraryService) — started; the seam exists, 3 pages converted.**
  `src/service.rs` answers a page's whole data question in **one call
  returning one owned snapshot** (`service.home()`), instead of a page making
  four direct `Catalog` reads and swallowing each error. Snapshots are plain
  owned `Send` structs *on purpose*: that is what lets the same call move to a
  worker thread later without touching the page, and a compile-time
  `snapshots_are_send()` test stops a future edit from breaking the property.
  The error policy now lives in one place — a failed read degrades to empty
  **and records the reason**, which pages surface as a toast (the service does
  not call `notify` itself: it must stay worker-callable, and `notify` is
  UI-thread-only). Converted: **Home** (4 reads → 1), **Analytics** (4 → 1),
  **Tags** cloud + tag-books, **Reading list**, **Shelves**, **All books**,
  **History**, **Lookup History**, **My Library** (8 reads → 1, the worst
  offender: it swallowed all eight), **Saved quotes** (N+1 removed),
  **Vocabulary**, **Book float**, **Series float** (N+1 removed),
  **Shelf detail**, **Book page**, **Reader**, plus the error-reporting pass
  over **Settings**, **Metadata editor** and **Shelf editor**. **Step 2 is
  complete: no page swallows a database read any more.** Home's
  "continue reading"
  fallback chain moved
  into the service and is unit-tested. **Remaining pages still hold an
  `Arc<Catalog>` and that is fine** — `LibraryService` borrows the same `Arc`,
  so both styles coexist; converting the next page is: add a snapshot method,
  swap the field, delete its `unwrap_or_default()`s. Writes (import) still go
  straight to the catalog — they belong to step 4.
- **Step 4 (task manager), step 5 (preloaders)** not started. **Step 6 (grid
  virtualization) is not planned** — the data layer is <20 ms and there is no
  measured grid lag, so it would add risk for no win.
- **WebView reuse — done.** The cheap win the timing surfaced: the reader used
  to call `webkit6::WebView::new()` in `init()`, so every book open spawned a
  WebKit process (~400 ms). `src/webview_pool.rs` now parks exactly one view
  between readers; the reader acquires it in `init()` and releases it in
  `shutdown()`. The reader's own lifecycle is unchanged (session start/end and
  progress save still run on every entry/exit) — only the expensive object is
  pooled, deliberately *not* the whole page, which would keep a reading session
  counting while the user browsed the library. Handlers that capture the
  component's `Sender` are recorded as `SignalHandlerId`s and disconnected
  before parking; sizing, context-menu suppression and the `"kalam"`
  script-message *registration* are permanent and live in the pool (WebKit
  rejects a second registration of that name). Escape hatch:
  `KALAM_NO_WEBVIEW_POOL=1` restores the old spawn-per-open behaviour for A/B
  measurement with `KALAM_TIMING=1`. Cost: the WebKit process (~100–200 MB)
  stays resident after the first book instead of being released on leave; the
  page is blanked on release so the book's DOM is still freed.
  **Needs an Arch smoke-test:** open book A → leave → open book B → return to
  A, checking highlights, dictionary popup, tap-to-look-up and progress restore
  all still work on the second and third opens (that is what a stale handler or
  a missed re-registration would break).

**Goal:** make Kalam feel instant (Yazi philosophy: *"don't make the UI
fast — make it never wait"*) and lay the seams the source platform needs.

### Scope (in order)

1. **Measure first.** `perf` + sysprof + GTK inspector on: cold start, book
   open, chapter turn, dictionary lookup, library scroll. Record the numbers
   — they decide what gets fixed (asserted bottlenecks get measured before
   being trusted).
2. **`LibraryService` behind `Catalog`.** ✅ seam built (`src/service.rs`),
   Home / Analytics / Tags converted; other pages migrate incrementally.
   Pages stop calling the DB directly and *ask* the service, which answers in
   one call with one owned `Send` snapshot. Moving queries off the UI thread
   then becomes a change in one place. (`Catalog.conn` is already
   `Mutex`-wrapped — feasible without a rewrite.)
3. **Thumbnails at import + async cover decode.** ~200px thumbnails into
   `cache/thumbs/<uuid>.png` at import time; the grid decodes tiny files that
   survive restarts; cards show a placeholder and swap in the texture when a
   worker finishes decoding. (The in-memory `COVER_CACHE` dies every launch.)
4. **Task manager (`src/tasks.rs`).** Import, dictionary rebuild, metadata
   fetch, downloads → background tasks with progress + cancellation
   (`thread::spawn` + `async-channel` + `glib::idle_add` — **no tokio**;
   relm4 is already the actor framework).
5. **Preloaders.** Cover preloader (rows 1–30 visible → decode 31–60 in the
   background); chapter preloader in the reader (preload the next chapter
   while reading the current one).
6. **Grid virtualization** — *only if the numbers earn it*: `GtkGridView` +
   `GListModel` replacing the 400-widget `build_book_grid` and the
   teardown-and-rebuild `rebuild_list`.
7. **Perf-budget CI test.** Seed 2,000 books; assert grid build under N ms.
   Catches regressions like a new `for book in books` loop.
8. **Plugin-host seam design** (the dependency for P7/P9): define the
   `Source` adapter API shape (search / details / chapters / content — text
   and image flavors) and the Lua plugin host interface, even if the first
   real plugins ship in P7.

**Acceptance:** library grid stays smooth with 2,000+ books; book open and
page turns feel instant; all slow work is off the UI thread; pages are thin
(no DB calls, no decoding); a fresh chat can add a source plugin from the
documented API alone.

**DoD:** CI green; README / ROADMAP / `docs/conversation.md` updated; user
runs it on Arch.

---

## P6 — Downloads hub

**Goal:** One place for inbound files/jobs.

### Scope

- Queue: queued / active / done / failed  
- Sidebar Downloads UI  
- Folder-watch import  
- Hooks for AO3/comics jobs  

---

## P7 — Fiction platform (AO3 first, then more)

**Goal:** A FanFiction.net-app-class fiction platform inside Kalam: search
across sources with tag filters, download fics, read offline, and
**auto-update** downloaded fics as new chapters release.

### Scope

- `FictionSource` trait → implemented by **source plugins (P12, Lua)**
- Sources: AO3 first, then FFN, Royal Road, Webnovel, Scribble Hub, … each
  exposes search / detail / chapter list / chapter content (sanitized)
- **Structured search UI** (native): fandom, tags, characters, ships,
  rating, status — fed by plugin-parsed results (AO3 has no public API;
  plugins parse the site, Tachiyomi-style)
- Download into library with `source` + `remote_id`; offline reading
- **Follow + automatic updater:** background scheduler (A0 task manager +
  glib timers) polls followed fics, downloads new chapters, notifies
  (replaces "Manual Check updates")
- Rate limits / clear errors / polite polling (respect sites)
- Caveat (existing risk): annotations anchor to spine — re-downloaded fics
  may lose anchors; best-effort

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

## P9 — Manga platform (Suwayomi-class)

**Goal:** Browse/download manga into library → open in the P8 comics viewer.

### Scope

- `MangaSource` trait → **source plugins (P12, Lua)**; same adapter shape as
  Tachiyomi/Suwayomi extensions: search, popular, chapter list, page fetch  
- **No Suwayomi server rewrite:** Suwayomi's value is its Kotlin extension
  ecosystem, which can't run in Rust; we reimplement the *adapter concept*
  natively (porting an extension's scraping logic is hours — they are simple
  scrapers). Optional later: a "Suwayomi server" adapter so Kalam can talk
  to a user's existing Suwayomi instance via its API — the cheapest bridge
  to the whole ecosystem  
- Sources: MangaDex, Komga, Kavita, own archive, OPDS, … (legal /
  self-hosted first)  
- Downloads hub integration; per-source rate limits  
- Reader is an **image pager** (P8) — no WebKit involved  

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

## P12 — Lua plugin system

**Status:** wanted (reversed 2026-09-02 from "no plugin API"); design TBD.
**Dependency:** the A0 plugin-host seam (the `Source` adapter API + Lua host
interface) must exist first.

**Goal:** a user-facing plugin surface. First-class consumers: **fiction
source plugins** (P7) and **manga source plugins** (P9) — one Lua plugin per
site, written by us (and later by users).

### Scope (design points, refine in conversation)

- **Language: Lua** via `mlua` — tiny, embeddable, battle-tested (Yazi,
  Neovim, AwesomeWM). (WebAssembly and compiled-in Rust traits were
  considered and set aside: wasm = heavy tooling, compiled-in = no
  user-authored scripts.)
- Plugin API surface: `search(query, filters) → results`, `details(url)`,
  `chapters(url) → list`, `content(chapter) → clean text` (fiction) or
  `pages(chapter) → image URLs` (manga).
- **No Kotlin-extension bridge** (Tachiyomi extensions are Android APKs —
  wrong shape for desktop). **No Suwayomi server rewrite** — optional later:
  a "Suwayomi server" *client* adapter plugin that talks to a user's existing
  instance via its API.
- Sandboxing / rate limits / polite polling: per-source schedules
  (user-controlled, default daily).
- **Borrow:** FanFicFare adapter logic (AO3/FFN/RoyalRoad/…) ports to Lua;
  MangaDex official API needs no scraping.

### Out

Running Tachiyomi/Suwayomi Kotlin extensions. A plugin marketplace (later,
if ever).

## Schema (current + planned)

Current `SCHEMA_VERSION` = **14** (`src/db.rs`). Migrations run on open and are
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

| Check | When | Status |
|-------|------|--------|
| `cargo fmt --check` (auto-fix + push fix commit when it differs) | every push | active |
| `cargo clippy -D warnings` | every push | active |
| `cargo test --all-targets` — compiles **and runs** the unit tests (in-memory SQLite, headless); failures publish to `ci-logs/test-latest.txt` | every push | **active — installed by the user (`bc6473f`), first run green (`1aae8f1`)** |
| `cargo build` / `release` | every push | active |
| GUI smoke | **your Arch machine** at phase end | user |

Deps include `webkitgtk-6.0` for P2+.

**Workflow files:** the App cannot push `.github/workflows/` (GitHub App lacks
`workflows` permission). The canonical workflow with the `cargo test` step
lives at `docs/ci/github-actions-ci.yml`; install it by copying over
`.github/workflows/ci.yml` and pushing from your own account. See
`docs/ci/README.md` for the exact commands.

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

## A1 — In-app dialogs (requested 2026-09-02, scheduled after A0 step 2)

**Why:** on a tiling compositor (the user runs Sway) a `gtk::Window` is a real
top-level window. Sway will happily send it to another workspace, tile it
beside the main window, or leave it behind when you switch — none of which is
what a modal dialog means. The app already agrees with this in principle:
`app.rs` has an in-app float layer (`float_host` + `float_scrim` over a
`gtk::Overlay`) and a comment on `open_annotations_floating` stating floats
live there "never a separate window — so the compositor can't move it to
another workspace". **The conversion was simply never finished.**

**Current state (audited 2026-09-02):**

*Already in-app (5):* book float, series float, annotations panel, shelves
panel, tags panel. **Audited and brought in line 2026-09-03** — see "Making
the older dialogs match" below; they were not consistent with each other, let
alone with the five new ones.

*Still separate `gtk::Window`s: **none**. All five converted 2026-09-03.*

| Where | What it is | Exit kind |
|---|---|---|
| `metadata_editor.rs` | Edit metadata (the big one — scrolling content) | `UnsavedInput` |
| `shelf_editor.rs` | New / edit shelf (manual **and** smart-rule builder) | `UnsavedInput` |
| `reading_list.rs` | "Add to reading list" book picker | `OwnButtons` |
| `shelf_detail.rs` | "Add books to shelf" book picker | `OwnButtons` |
| `shelves_grid.rs` | "Delete shelf" confirmation | `OwnButtons` |

All five now go through `src/widgets/in_app_dialog.rs`. `gtk::Window::builder`
no longer appears anywhere in `src/`.

*Deliberately staying native (4):* every `gtk::FileDialog` (`all_books.rs`,
`home.rs`, `metadata_editor.rs`, `settings.rs` ×2). These are portal-backed
file pickers — the compositor and the desktop portal own them, the user
expects their normal file manager, and re-implementing a file browser in-app
would be strictly worse.

**Requirement (revised by the user 2026-09-03).** The earlier rule was
"every in-app dialog needs a visible ✕". The user corrected it: **the dialogs
exist for different reasons, so they should not all be dismissed the same
way.** The affordance must match what the dialog *is*:

| Dialog is… | Affordance | Why |
|---|---|---|
| A **detour** you came to from somewhere (book float, series float) | **‹ Back** | You are returning to where you were, not discarding something. Back says that. |
| A **transient panel** layered on the current context (annotations, shelves, tags) | **✕** | Nothing to return to; you are dismissing an overlay. |
| A **form with unsaved input** (metadata editor, shelf editor) | **Cancel** + explicit Save | "✕" is ambiguous next to unsaved edits: does it discard? Cancel is unambiguous. |
| A **confirmation** (delete shelf) | **Cancel / Delete** | Two named outcomes; a ✕ is a third, vaguer one. |

**Universal, on top of the above: clicking the dimmed backdrop closes the
dialog, and Esc closes it.** Backdrop-click is now implemented (see the
changelog entry for 2026-09-03) — the scrim already intercepted those clicks
so they could not reach the page behind it, but its handler was empty, which
made the dim look interactive and do nothing.

**Open question the user raised:** with backdrop-click and Esc both working,
can the explicit button be dropped entirely? **Decision: no, not for all of
them.** Backdrop-click and Esc are both *invisible* affordances — nothing on
screen advertises them, so a dialog whose only exits are invisible is still a
trap for anyone who does not already know the trick. Keep one visible control
per dialog, but let it be the *right* one from the table above rather than a
reflexive ✕. Forms and confirmations especially must keep a named button,
because for those the question is not only "how do I leave" but "what happens
to my edits when I do". The one place a bare ✕ can go is where the visible
control would be pure duplication of an already obvious action.

**Order (cheapest and safest first):** delete-shelf confirmation → the two
book pickers (they are near-identical, so one helper serves both) → shelf
editor → metadata editor last, because it is the largest and has its own
scrolling/sizing logic tuned for short laptop screens.

**Risk to watch:** the float layer is a `gtk::Box` in an overlay, not a
window, so it has no built-in focus containment. The pickers contain long
scrollable lists and the metadata editor contains many entries — Tab order and
initial focus need checking on each conversion, and the scrim already blocks
click-through.

### What the conversion actually needed (2026-09-03)

`src/widgets/in_app_dialog.rs` — one helper, `present(anchor, title, exit,
content) -> Option<InAppDialog>`. It walks up from any widget to the app's
root `gtk::Overlay`, then adds its own scrim + centred panel. It deliberately
does **not** reuse the `AppMsg` float layer: these dialogs are plain functions
taking an `on_confirm: impl Fn()` closure, and a closure cannot travel through
a `#[derive(Debug)]` message enum.

`DialogExit` has two variants rather than the table's four, because the two
kinds that were real windows both bring their own buttons:

* `OwnButtons` — content supplies the named buttons (Done, or Cancel/Delete).
  Backdrop-click dismisses; nothing is lost.
* `UnsavedInput` — a form. Backdrop-click is **disabled**: silently discarding
  a half-typed description because a click landed slightly off target is a bad
  trade. Esc still works and Cancel is right there.

The ‹ Back and ✕ rows of the table describe the book/series floats, which
already live in `app.rs` with their own headers; they join `DialogExit` when
those headers are revisited.

**Three things a `gtk::Window` had been doing for free**, each of which had to
be replaced by hand:

1. **Teardown.** `window.close()` destroys the widget tree, which breaks the
   reference loop between a widget and the callback that captures it. Removing
   an overlay child does not, so `teardown()` also empties the host — without
   it every dialog ever opened would stay in memory.
2. **Height bounds.** A window has a default height; a panel centred in an
   overlay is sized by its content. The metadata form and the smart-shelf rule
   list both got `max_content_height` + `propagate_natural_height`, or a long
   description / twenty rules would push Save off a 768px screen.
3. **A real window handle** for the cover `gtk::FileDialog`, which is
   portal-backed and needs a genuine top-level parent. Resolved from the
   anchor's root instead of from the (now non-existent) dialog window.

Also removed: three copies of a `window_of()` helper that existed only to find
a parent window for these dialogs, and `shelf_editor`'s and `metadata_editor`'s
hand-rolled Esc handlers, now that the helper provides Esc for all of them.

### Making the older dialogs match (2026-09-03)

The user asked whether the five dialogs that were *already* in-app matched the
five new ones. They did not, and they did not match each other either. Four
differences found, all fixed:

1. **Two floats taught different shortcuts for the same keys.** The book
   float's ✕ was tooltipped "Close (Q)", the series float's "Close (Esc)" —
   both keys worked on both.
2. **`q` closed a float while you were typing in it.** The handler in
   `app.rs` fired on `q`/`Q`/Esc whenever a float was visible, without asking
   whether a text box had focus. The tags panel has an entry, so typing the
   letter `q` into it dismissed the panel. Esc now always closes; `q` is
   ignored while a `gtk::Entry`, `SearchEntry` or `Text` has focus.
3. **The two floats had a ✕ that the user did not want.** Decision
   (2026-09-03): **remove it, and do not replace it with ‹ Back.** Both floats
   are read-only detours, so a misclick on the backdrop costs nothing — "I
   won't lose anything if I accidentally misclicked on the outside". This is
   the one case where the "keep one visible control" rule is waived, and
   deliberately: the rule exists to protect against *losing something*, and
   there is nothing here to lose. `SeriesFloatMsg::Close` and
   `SeriesFloatOut::Close` became unreachable and were deleted with it.
4. **Three panels had no title, and a different shell.** The annotations,
   shelves and tags panels rendered with no heading at all, and with their own
   14px-corner CSS (three byte-identical copies) against the dialogs' 20px.
   They now use a shared `panel_title()` helper with the same markup and CSS
   class as the A1 dialog header, and one merged CSS rule.

Note the corrected finding: the series float opens from the **book page**
(`book.rs:695`), not from the book float — the book float's series line is a
plain label. So closing it to the page was always right; an earlier reading
that it "skipped a step" was wrong.

## Immediate next steps

**Next, in order (locked 2026-09-02):**

1. **A0 — Architecture & performance track** (the section above). Start with
   measurement, then `LibraryService` + thumbnails/async decode; design the
   plugin-host seam. This is the foundation for everything after.
2. ~~**A1 — In-app dialogs**~~ **done 2026-09-03.** All five remaining
   `gtk::Window` dialogs now draw inside the main window via
   `src/widgets/in_app_dialog.rs`. Remaining A1 polish: focus containment and
   Tab order inside a float (no window means no built-in focus scope), and
   folding the book/series float headers into `DialogExit`.
3. **P6 — Downloads hub** (unified queue + folder watch).
3. **P7 — Fiction platform** (AO3 first) via Lua source plugins; native tag
   search; download; follow + auto-updater.
4. **Renderer vertical slice** (alongside P7) — cosmic-text fiction renderer;
   the 2–4 week calibration milestone.
5. **P8 — Comics local** → **P9 — Manga platform** → **P10 — PDF (MuPDF)** →
   **P11 — Tools** → **P12 — Lua plugin system matures.**

**Recently completed (do not redo):** dictionary track Phases 8–10 (shipped,
CI-green: POS dividers `be11c07`, priority reorder `d12c905`, lookup history
`7ce8bfb`+fixes); CI workflow with the `cargo test` step installed and green
(173 unit tests, failures publish to `ci-logs/test-latest.txt`); backend
review done (one latent bug fixed, dict importers hardened, 9 new tests);
reader milestones 1–3 shipped (annotation workflow, hybrid anchoring, dict
multi-result popup).

**Deferred (revisit only after the above is underway):** annotation design
polish (without changing saved-highlight anchoring); multi-chapter
buffering; continuous book-wide scrolling; chapter auto-advance redesign;
advanced CFI; UI-overhaul screen work (`library_look.png` two-column
dashboard — the app is currently a single vertical stack).

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
| 2026-07-28 | P5.5 Settings window redesigned: 220px 6-tab navigation rail + rounded cards layout |
| 2026-08-26 | P5.5 button hierarchy + chip + serif-title classes adopted from user style pass (`d545d28`) |
| 2026-08-26 | Settings v2 shipped: grouped nav, section cards, family theme picker, pill switches, export card — mockup-first, CI green |
| 2026-08-30 | Reader-improvements track recorded: annotation workflow and hybrid anchoring are complete; recoloring existing highlights is shipped, with text search next, followed by dictionary improvements and only later reader architecture changes |
| 2026-08-31 | Reader annotation search shipped across saved highlight text and notes; color/type filters remain available, and annotation design polish is deferred until feature work is complete |
| 2026-08-31 | Dictionary lookup keeps the selected phrase intact and normalizes surrounding punctuation plus common simple inflections |
| 2026-08-31 | Dictionary popup displays up to five matching results with separate save/copy actions; the clearly licensed English WordNet 2025 starter pack is bundled and enabled on first run |
| 2026-08-31 | Bundled the separate English Idioms and Expressions pack (1,024 phrase-to-meaning entries) with its upstream Unlicense notice, source revision, checksum, and first-run removal marker |
| 2026-08-31 | Dictionary overhaul plan extended with deferred Phase 5.5 POS grouping and transparent, optional Lesk sense hints; all matched senses remain visible |
| 2026-08-31 | Temporary selection handles now support pointer/touch dragging without changing native selection or saving annotations implicitly; triple-click rendering remains deferred |
| 2026-08-31 | Fresh text selections now paint their custom selection bands live during mouse/touch drag; the toolbar and handles still wait for release |
| 2026-09-01 | Dictionary overhaul Phase 1 shipped: precomputed `fold_key` headword index (schema v11). Exact/prefix lookups use `idx_dict_entries_key`; `Run`/`run`/`rún` all resolve to `run`; existing databases backfill in place with no reimport |
| 2026-09-01 | Dictionary overhaul Phase 2 shipped: real lemmatization from the bundled Princeton WordNet 3.0 exception lists (`noun.exc`/`verb.exc`/`adj.exc`/`adv.exc`, gzipped, with NOTICE + checksums). `went→go`, `mice→mouse`, `better→good`, `running→run` resolve to headwords; suffix rules remain the fallback |
| 2026-09-01 | Fixed dormant test failures/warnings found by the first real `cargo test` run: `series_key` now strips leading series articles (The/A/An) so article variants share one series-cache key, while author names are never article-stripped; the series ordering test helper now stores the series index it was passed, making the indexed-vs-unindexed ordering assertion real instead of vacuous |
| 2026-09-01 | Fixed the flaky cover-override tests: all three cover tests seeded the same book title, so in parallel `cargo test` runs they raced on the same real filesystem paths (shared stashed cover `covers/hash-A.png` and book dir). Each test now seeds a unique title, isolating its uuid/hash paths |
| 2026-09-01 | Dictionary overhaul Phase 3 shipped: `search_phrase` in the catalog (full phrase → longest contained phrase headword via sliding window → per-token breakdown with lemmatization), `PhraseLookup` enum, and the reader's dict-lookup bridge routes multi-token queries through it. `odd mixture` now yields cards for `odd` and `mixture`; `run out of steam today` resolves to the `run out of steam` entry |
| 2026-09-01 | Phase 3 follow-up: the sidebar Words search box still used the single-word path, so phrases typed there dead-ended with "No matches". All lookup paths now share one phrase-aware pipeline (`lookup_dict`) — sidebar search and selection popup behave identically |
| 2026-09-01 | Two more bundled English packs, on by default: **English Synonyms (WordNet 3.0)** (110,365 words, synset companions) and **English Antonyms (WordNet 3.0)** (6,621 antonym pairs), both derived from the already-licensed Princeton WordNet 3.0 data (NOTICE + checksums beside the packs). New packs auto-install on next launch via their own first-run prefs, so existing installs gain them without re-import |
| 2026-09-01 | Dictionary overhaul Phase 4 shipped (user-designed): merged dictionary store. One word = one entry, from the highest-priority dictionary that has it (WordNet 10, Idioms 20, Synonyms 30, Antonyms 40, imports 100) — priority wins even when another dictionary has more senses, and the losing dictionaries' copies are never shown. Schema v12: `dictionaries.priority` + `combined_words` (one row per headword key, senses as deduped JSON). Auto-rebuilt on import/remove/bundled install and once at migration. The old Phase 4/5 plan of per-dictionary tabs is obsolete and removed |
| 2026-09-01 | Dictionary overhaul Phase 5 shipped: popup redesign — the fake "1 RESULT / No definition / Total dict entries" state is gone, one clean entry per word with numbered senses, phrase misses render clickable breakdown chips, imported HTML definitions render formatted instead of flattened, a single action bar (Save/Copy/Search-in-book) replaces per-result button pairs, and the popup stays dark on every paper theme | 
| 2026-09-01 | Dictionary overhaul Phase 5.5 shipped (partial): Lesk "likely here" hint — `likely_sense_index` (pure fn, unit-tested: stopword-filtered overlap, headword excluded, tiny stemmer) marks the best-matching sense with an accent pill, the bridge now sends the full sentence context, a `dict_sense_hint` pref toggles the hint only, and senses are never hidden. The header POS pill shipped earlier; **POS grouping dividers remain deferred** |
| 2026-09-01 | Dictionary overhaul Phase 6 shipped: tap-to-look-up (240 ms tap delay, caret-word resolution, sentence context) + popup keyboard (↑/↓ focus ring with wrap, Enter saves the focused sense; ←/→ deliberately unbound) + **Find in chapter** (`kalamSearchInBook`, temporary accent hits, count toast, Esc clears). jsdom harness at `docs/files/test_kalam_dict_preview.js` grew to 55 checks |
| 2026-09-01 | Dictionary overhaul Phase 7 shipped: vocabulary review — schema v13 `saved_words.known`, Saved Words page gains All/To review/Known filter + mark-known check buttons, and exports **CSV** (`~/SavedWords.csv`, RFC-4180) and **Anki TSV** (`~/SavedWords-Anki.txt`, `#separator:tab`). The dictionary overhaul is complete |
| 2026-09-01 | Backend review pass (focused + full sweep) shipped: dict importers hardened — SQLite packs are opened read-only (never modify the pack, no `-wal`/`-journal` sidecars), pack table/column identifiers are double-quoted against crafted names, failed imports roll back the dictionary row instead of leaving a half-import, and importing Kalam's own `catalog.db` as a pack is rejected via canonical path comparison. Fixed a latent bug: `list_reading_list` read `progress/rating/publisher` as `position/note/added_at` (column offset vs the 16-column `BOOK_COLUMNS`). 9 new unit tests; the rest of the sweep (shelves, history, metadata, stats, authors, series, annotations, prefs, pronunciation, `shelf_rules.rs`, schema/FKs) found no defects |
| 2026-09-01 | CI workflow gains a `cargo test` step: compiles and runs all unit tests headless (in-memory SQLite), publishes failures to `ci-logs/test-latest.txt` and fails the run. The full workflow is staged at `docs/ci/github-actions-ci.yml` — the Arena App cannot push `.github/workflows/` changes, so the user installs it manually with their own account |
| 2026-09-01 | The first real `cargo test` run on CI (user installed the workflow, `bc6473f`) caught exactly one failure: `quote_ident_escapes_embedded_quotes` — my test's expected string had one extra escaped quote (three quotes after the word instead of the correct two that SQLite identifier quoting produces). Function correct, test literal wrong; fixed (`1aae8f1`). All 154 unit tests now pass on every push |
| 2026-09-01 | Dictionary track Phases 8–10 recorded in the roadmap (user-authored): Phase 8 — POS grouping dividers (finishes P5.5, rendering-only, flat-index invariant for hint + keyboard nav); Phase 9 — Settings dictionary priority reorder UI; Phase 10 — lookup history (optional, schema v14, logs misses, privacy toggle + clear ship with the feature). Verified against the code before recording: `priority` is write-only today (missing from `Dictionary` and `list_dictionaries()`, which also orders by name — plumbing required); import/removal already call `rebuild_combined_dictionary()`; `lookup_dict` (reader.rs:1880) is the single lookup funnel to log in; `pos` already reaches the popup per-sense. Track numbering is local, distinct from global P8/P9/P10 (comics/PDF) |
| 2026-09-01 | Dictionary track Phase 8 shipped: POS grouping dividers in the dictionary popup (`be11c07`) — senses grouped under noun/verb/adjective/adverb dividers (remaining POS first-appearance order; unlabelled senses last with no divider). Rendering-only over the flat senses array: `Sense.number`, the Lesk hint index and the ↑/↓ keyboard walk keep indexing the flat list; a group straddling the "Show N more" fold keeps one divider at its true start; the header POS chip is dropped when 2+ groups exist. CI green |
| 2026-09-01 | Dictionary track Phase 9 shipped: Settings dictionary priority reorder UI (`d12c905`) — the v12 `priority` column was write-only; `Dictionary` now carries it, `list_dictionaries()` selects it and orders `priority ASC, name ASC`, and Dictionaries rows get ↑/↓ buttons that renumber all packs compactly (0, 1, 2, …), trigger `rebuild_combined_dictionary()`, and refresh; the top row shows a quiet "speaks first" chip; ↑/↓ disabled at the ends; re-imports keep user-set priority (upsert doesn't touch the column). CI green |
| 2026-09-01 | Dictionary track Phase 10 shipped: lookup history — schema v14 `dict_lookups` (FK to books `ON DELETE SET NULL`, NOCASE word index), logging in the `lookup_dict` funnel (both the popup selection/tap path and sidebar lookups; per-keystroke search prefixes not logged), misses logged with `found = 0`, same-hour same-book collapse, pref `dict_history_enabled` (default 1) with a reader Settings toggle, Lookup History page (day-grouped, searchable, miss chip, Clear), `repeat_lookup_words()` feeding "Suggest from history" chips on Saved Words plus a "Last 3 lookups" dashboard card on Library. Three CI iterations fixed a relm4 5-arg `update_with_view` + `#[watch]` label lifetime (`68373fa`), a FK violation in the new test (seed real books, `6b4e528`), and the repeat-set expectation (miss word logged across two books, `0b188a3`). 156 unit tests green |
| 2026-09-02 | Design conversation recorded in `docs/conversation.md`: (1) performance — stay Rust, architecture is the bottleneck (no language rewrite); (2) second AI's analysis reviewed and verified (grid rebuild, sync decode, in-memory cover cache, 3,638-line CSS — all real; fix order adjusted: thumbnails → async decode → virtualize if numbers say so); (3) Yazi philosophy adopted — service layer + task manager + preloaders + thin UI (relm4 + async-channel already give half the skeleton); (4) roadmap scope reviewed — P6–P11 stand; custom text-renderer question deferred to a dedicated discussion; (5) **Plugin API non-goal reversed** — plugins wanted, leaning Lua, design TBD after the architecture track. Roadmap: schema version corrected to 14, phase map gains A0 (architecture track) + P12 (plugins), non-goals updated, next-steps gains the A0 entry |
| 2026-09-02 | Scope confirmed as a **content platform** (user): fiction sources (AO3/FFN/Webnovel/Royal Road…) with tag search, downloads, offline reading, follow + **auto-updater** (FFN-app-class) and manga sources (Suwayomi-class browse/read). Roadmap: P7 expanded (fiction platform, auto-updater replaces manual check), P9 expanded (manga platform; **no Suwayomi server rewrite** — adapter concept natively, optional Suwayomi-server client adapter later), P12 = source-adapter plugins (Lua). Renderer question framed in `docs/conversation.md` §7: WebKit = full browser engine (power vs weight); custom renderer = we draw text (1–3 person-years, but sources feed it clean content so it never needs to be a browser); **leaning hybrid** — custom for reading, WebKit for browse/fallback; sequencing: product (sources) first on WebKit, renderer as A0 crown after |
| 2026-09-02 | Scope sharpened: **no browse mode** — Kalam never renders arbitrary websites; sources return structured data via plugins. Renderer decision resolved direction: WebKit = EPUB engine only (lazy), custom renderer for source fiction + comics; EPUB normalization (crengine-style) is Path B crown. Tachiyomi/Suwayomi fully explained: extension = 200–500-line Kotlin adapter for one site; Kotlin/JVM can't run in Rust; we reimplement the adapter pattern natively (MangaDex/Komga/OPDS need no scraping); Suwayomi-server bridge is a cheap optional plugin. PDF not forgotten — P10 MuPDF, fixed-layout, never WebKit (AGPL license note). Borrow list recorded in `docs/conversation.md` §8: FanFicFare (fiction adapters), Tachiyomi extensions (pattern), MangaDex API, Komga/Kavita, KOReader + crengine (renderer), cosmic-text/swash/vello (Rust text stack), lol_html/ammonia (sanitizing), Yazi, Foliate — all license-compatible with GPL-3.0-or-later |
| 2026-09-02 | Renderer direction sharpened (user push-back): custom renderer is the **endgame for all reflowable text** — cosmic-text based (Rust text layout, NOT an EPUB engine; we build normalization/pagination/painting), fiction first (clean content), EPUB via a lol_html normalization pipeline after; WebKit demoted to fallback for exotic EPUBs (may be cut). crengine (C++ EPUB engine) kept as a legitimate shortcut if EPUB-before-custom-engine is wanted, at the cost of C++ in the stack + less dict/theme/annotation control. Manga architecture confirmed: Tachiyomi-shaped `Source` adapter API with **Lua plugins we write**; **no Kotlin-extension bridge** (they are Android APKs — wrong shape for desktop; pattern + scraping logic port instead; MangaDex/Komga/OPDS need no scraping). PDF stays MuPDF (P10); comics = image decode + GTK pager (no engine). Engine map recorded in `docs/conversation.md` §8 |
| 2026-09-02 | Renderer timing decided: **not now, not at the end — start right after sources, grow alongside.** Order: A0 architecture → P7 fiction sources → renderer vertical slice (alongside) → EPUB normalization → PDF/comics. crengine deep-dive: GPL-2.0 (KOReader fork AGPL-3.0) vs our GPL-3.0-or-later — license mismatch; C++ codebase with no Rust bindings (FFI wrapper burden); partial CSS 2.1 (no float/border/etc. — what real EPUBs use); dict/annotation integration is the same fight against a foreign engine. Verdict: crengine only as a separate dynamically-linked fallback bridge; **cosmic-text + our own normalizer remains the recommendation** (hard part is ours either way) — recorded in `docs/conversation.md` §10 |
| 2026-09-02 | Roadmap restructured as the single handoff document for future chats: new "⚠️ Read this first" block (agent instructions: read README/ROADMAP/conversation.md, keep docs current in the same commit, never skip the changelog, CI is the gate, user is the QA loop); new "Documentation discipline" rules in the Working agreement; new **"Current trajectory (locked)"** section — agreed order A0 → P6 → P7 → renderer vertical slice (alongside P7) → P8/P9 → EPUB normalization → P10/P11/P12, plus all locked decisions in one place; detailed **A0 section** (measure → LibraryService → thumbnails/async decode → task manager → preloaders → virtualization-if-numbers-earn-it → perf-budget CI test → plugin-host seam); detailed **P12 section** (Lua via mlua, source-adapter API, no Kotlin bridge, no Suwayomi rewrite); phase map + next steps rewritten to match |
| 2026-09-02 | **chapbook discovered** (`ophymx/chapbook`, Apache-2.0): a week-old project that IS our custom-renderer plan — stylo (Firefox's CSS engine) + cosmic-text + tiny-skia/vello, no webview, pagination-first, GTK4 viewer, quote-anchored `LayeredLocator` positions, PDF via hayro. Recorded in `docs/conversation.md` §11: added as a **candidate renderer foundation** (re-evaluate at vertical-slice time, not a dependency yet), **quote-anchored locators adopted for annotations** (fixes the P7 auto-updater anchor risk), **hayro added to the P10 shortlist** (AGPL-free PDF), stylo noted as the preferred EPUB-cascade option vs hand-rolled normalization. WebKit fallback kept (chapbook's own stated limit: a subset of publisher EPUBs) |\n| 2026-09-02 | A0 step 1 (measure first) started: headless data-layer measurement harness added as `src/perf.rs` (`#[ignore]`d, seeds 2,000 books; times `list_books` ×3 sorts, search, `recent_books`, `library_stats`, tags). Run with `cargo test --release perf -- --ignored --nocapture`. CI compiles it but skips it, so CI stays fast. No behaviour change |
| 2026-09-02 | A0 step 1, GUI half: in-app timing harness added as `src/timing.rs`, gated behind `KALAM_TIMING=1`. Prints cold start (`window_shown` via window `realize`), `book_open` (EPUB parse), `chapter_load`→`chapter_done` (WebKit render), and `dict_lookup` milliseconds to the terminal. No-op when the env var is absent, so zero overhead in normal use. `src/perf.rs` counts timings. The data-layer baseline is in: all list-page queries stay under ~20 ms for 2,000 books, confirming the DB layer is well-tuned and the UI side (sync cover decode + full grid rebuild + ephemeral cover cache) is where A0's later steps focus. Run with `KALAM_TIMING=1 cargo run --release` |
| 2026-09-02 | **Held A0 step 6 (grid virtualization).** Measured evidence: the data layer is <20 ms for 2,000 books and there is no measured grid lag, so virtualization would add risk for no measured win. Recorded in the A0 status block. |
| 2026-09-02 | Home gains an **"+ Add books"** button (header, right of the title) that opens the same EPUB picker and background import as **My Library → All books** — parse, hash, copy, per-file progress, and an "Import done — N added / M already in library / K failed" summary. It disables itself while importing and refreshes Home in place on completion so the counts, Continue and Recently-added cards update. Home was converted from a relm4 `SimpleComponent` to a full `Component` so the import worker can report progress and the button/label state can update; it reuses `ImportProgress`/`ImportTally` from All Books. The My Library dashboard keeps its own path, which heads to All books for import |
| 2026-09-02 | A0 step 3 (thumbnails) shipped and CI-green: `src/thumbs.rs` generates a persistent 256×408 thumbnail (`cache/thumbs/<uuid>.png`) at import and on cover replacement; the grid prefers it when the slot is small enough (never upscales it); covered by 5 headless unit tests. New dep `image` (default-features off; only png/jpeg/gif/webp) because gdk-pixbuf in this toolchain cannot encode PNG. Thumbnails are removed on book delete. `src/perf.rs` gains a cover-decode probe (full cover vs thumbnail). **Existing books:** `backfill_missing` runs on a background thread at startup so a library imported before this change gains thumbnails without re-importing (only missing files are generated). Several CI iterations fixed the real bugs — image's `std` feature does not exist (default-features off is fine; `image::open`/`save_buffer` need no feature), `DynamicImage` has `width()/height()` not `dimensions()`, `paths.rs` needed `Path` imported, a temporary `Option<PathBuf>` was dropped while borrowed, and `thumbs.rs`/tests used `PathBuf` without importing it and passed a `String` to a `&str` parameter. The true async *swap-in* is deferred to the task manager (step 4) where it architecturally belongs — the thumbnail-decode win is already captured synchronously. Part of A; see the A0 later steps |
| 2026-09-02 | **WebView reuse shipped** (the cheap A0 win step 1 measured): the reader called `webkit6::WebView::new()` in `init()`, so every book open spawned a WebKit process (~400 ms, vs ~3.5 ms to revisit a warm one). `src/webview_pool.rs` parks exactly one view between readers — `acquire()` in `init()`, `release()` in `shutdown()`. Only the widget is pooled, deliberately **not** the whole reader page: caching the page would also keep the reading session counting while the user browsed the library and defer the progress write, so the reader's lifecycle is unchanged. Handler discipline is the subtle part — a recycled view still carries the previous reader's handlers, each holding a dropped component's `Sender`, so sizing / context-menu suppression / the `"kalam"` script-message *registration* are permanent and live in the pool (WebKit rejects a second registration of that name on one manager), while every handler capturing a `ComponentSender` is recorded as a `SignalHandlerId` and disconnected in `shutdown()` before parking. Cost: the WebKit process (~100–200 MB) stays resident after the first book instead of being released on leave; the page is blanked on release so the book's DOM is still freed. `KALAM_NO_WEBVIEW_POOL=1` restores the old behaviour for A/B measurement with `KALAM_TIMING=1`. **Needs an Arch smoke-test:** book A → leave → book B → back to A, checking highlights, dictionary popup, tap-to-look-up and progress restore on the 2nd/3rd open |
| 2026-09-02 | **A0 step 2 started: `LibraryService`** (`src/service.rs`) — the seam between pages and the database. Pages made several direct `Catalog` calls each and swallowed the errors individually (26 `unwrap_or_default()`, 16 `.ok().flatten()` across `pages/`), so a broken database rendered as an empty library, there was nowhere to put caching, and moving queries off the UI thread meant editing every page. The service answers a page's whole data question in **one call returning one owned snapshot**. Snapshots rather than one-for-one wrapped getters is the substance of the step: a snapshot is a plain owned `Send` struct, so the same call can later run on a worker and be handed back to the UI without touching the page — asserted at compile time by `snapshots_are_send()`. Error policy now lives in one place (degrade to empty **and** record the reason; pages surface it as a toast — the service never calls `notify`, which is UI-thread-only, so it stays worker-callable). Converted Home (4 reads → 1), Analytics (4 → 1), Tags cloud + tag-books; Home's "continue reading" fallback chain moved into the service and gained tests, including the previously untested rule that a 100%-finished book is not offered as "continue". Other pages keep their `Arc<Catalog>` and migrate incrementally — the service borrows the same `Arc`. Writes (import) still go straight to the catalog: those belong to step 4. 9 new unit tests |
| 2026-09-02 | Docs accuracy pass: README file map listed `openlibrary.rs` at the top level (moved to `src/metadata/`) and omitted `author`/`notify`/`thumbs`/`perf`/`timing`/`icons`/`paths`; README + ARCH data-dir blocks said `override-covers/` where the code creates `covers/` (`paths.rs:43`) and omitted `cache/thumbs/` and `authors/`; ARCH phase table said "P4 ← you are here", three phases stale; ROADMAP pinned agents to a branch id from two sessions ago in two places (now states the per-session rule instead of naming one) and carried a verbatim duplicate of the "Reader chrome restyle (P2.1)" section; test count 156 → 163 (165 `#[test]`s, 2 `#[ignore]`d perf probes) in both files; `ci-logs/` held failures from runs that were since fixed, which reads as if the branch is red — cleared with a note, CI overwrites them on the next real failure |
| 2026-09-02 | Home gains an **"All books"** button (header, left of "+ Add books") routing to the existing `Route::LibrarySection(AllBooks)` — the full grid was previously reachable only from the My Library dashboard's empty-library placeholder — see the correction row below, this claim was wrong. Browsing is secondary-styled, importing keeps the primary emphasis. Also fixed a real defect found while wiring it: when a search matched nothing, All books rendered "Your library is empty. Click + Import EPUB" — wrong for anyone with books, and it hid the actual fix. `rebuild_list` now takes the query and distinguishes the two empty cases, naming the failed term and pointing at clearing the search. Added `docs/testing-a0.md`: the Arch smoke-test recipe (the 5 checks that catch a stale WebView handler on the 2nd/3rd book open), what each `KALAM_TIMING=1` label measures, and the A/B procedure against `KALAM_NO_WEBVIEW_POOL=1` that quantifies the ~400 ms saving. Corrected `timing.rs`'s own module docs, which advertised a `chapter_done` line that is never printed — `span_end` prints under the opening label, so `chapter_load` is a single line covering load→rendered. Test count corrected again: README/ROADMAP claimed 163, the tree now has 175 `#[test]`s minus the 2 `#[ignore]`d perf probes = **173** that CI runs (the `service.rs`, `webview_pool.rs` and `thumbs.rs` tests landed after the last recount) |
| 2026-09-02 | **Correction + dead-UI fix, prompted by the user disputing the previous row.** I had written that All books was "reachable through the My Library dashboard"; that was wrong. `library.rs` linked `AllBooks` from exactly one place — line 114, inside the `if stats.total_books == 0 { … return; }` placeholder — so the only moment the full grid was reachable was while the library was empty, and importing your first book removed the link. Auditing the other sections found worse: `ReadingList`, `Tags` and `Analytics` have complete pages, `PageSlot` variants and `Route::LibrarySection` arms in `app.rs`, but **nothing anywhere in the UI ever emitted those routes** — three finished pages that could not be opened at all. (`LibrarySection::ALL`/`icon()` are `#[allow(dead_code)]`, a leftover of the tile grid that the v5 dashboard replaced; the dashboard routes via content sections, and sections only render when they have content, so pages with no section were orphaned.) Added a `quick_links` row under the My Library title — All books / Reading list / Tags / Analytics — and dropped the now-duplicate "ALL BOOKS" section from the empty branch. Lesson recorded: "a route exists in `app.rs`" is not evidence the user can get there; reachability means grepping for who *emits* the route |
| 2026-09-02 | First real `KALAM_TIMING=1` run on the user's library (Arch, release build). **The WebView pool is confirmed working**: chapter turns settle at 37–48 ms and later book opens at 2.8–215 ms, with no ~400 ms WebKit re-spawn after the first book — the tail the pool was built to remove. The first book of a session still pays WebKit startup (`chapter_load` 2772.9 then 697.2 ms), which is expected and unavoidable without pre-warming. **But cold start came back at 8018.7 ms against a ~0.9 s baseline**, which nothing in A0 explains. Rather than guess, split `window_shown` into `startup_db_open` / `startup_dicts` / `startup_first_page` so the next run attributes it; prime suspect is the first-run bundled-dictionary import, which decompresses and inserts ~6.8 MB of gzipped TSV **on the UI thread** before first paint (`install_bundled_dictionaries`, `app.rs` init) and early-outs on a pref afterwards — i.e. probably once-per-install, not per-launch. Also removed `LibraryService::change_token`, a passthrough I added in step 2 that nothing ever called (the app cache uses `self.catalog.change_token()` directly): it was `pub`, so only the binary's `dead_code` warning caught it, and **CI could not have** — the clippy step has `continue-on-error: true` and no `-D warnings`, so warnings never fail a run. Noted as a gap in the CI gate |
| 2026-09-02 | Cold-start scare **resolved: there was no regression, and my diagnosis was wrong.** Six consecutive `KALAM_TIMING=1` runs decayed 6290 → 1596 → 965 → 852 → 945 → 831 ms, settling at **~898 ms — the documented ~0.9 s baseline**. The 8 s was a first-run-after-build artefact (OS page cache warming on a freshly-linked binary and its GTK/WebKit/ICU libraries), not app work. I had named the bundled-dictionary import as prime suspect; the spans measured `startup_dicts` at **0.1–0.2 ms on every run including the first**, so the pref early-out was already working and the suspect was innocent — had I "fixed" it on the hypothesis I would have rewritten correct code and left the real distribution unmeasured. Useful result from the breakdown: of a steady ~898 ms, only **~138 ms (16%)** is instrumented app work, and **~754 ms (84%) is toolkit startup** (GTK/libadwaita/WebKit/CSS/first layout) before or around `AppModel::init`. `startup_first_page` (~137 ms) is the only app-side target worth attacking; the DB open (~6 ms) and dictionary check (~0.1 ms) have nothing left in them. Recorded as the floor for A0 step 5. **CI gate tightened** in the staged workflow: clippy now runs `-- -D warnings`, so a `dead_code` warning like the unused `LibraryService::change_token` fails the run instead of passing green (handed to the user to install — the agent cannot push `.github/workflows/`) |
| 2026-09-02 | `-D warnings` installed by the user (`163c56c`) and **it immediately paid for itself**: the first run failed with **30 warnings that had been invisible**, one of them a real bug. `src/pages/series_float.rs` used `let _ = tx.send(result)` on an `async_channel::Sender` from a plain worker thread — `send()` there returns a *future*, nothing polled it, so the fetched series listing was silently dropped and the receiver only woke when `tx` fell out of scope, reporting "Fetch worker ended unexpectedly" on every **successful** fetch. Fixed with `send_blocking` (the pattern `metadata_editor.rs` already used correctly); it was the only `async_channel` send in the tree. Also fixed: a `HomeOut` variant-prefix regression I introduced (renamed `OpenBook`/`OpenBookDialog`/`OpenAllBooks` → `Book`/`BookDialog`/`AllBooks`, matching the convention `LibraryOut` documents), an unused test helper in `thumbs.rs`, `epub_write.rs` had 7 real functions sitting *after* its `#[cfg(test)] mod tests` (moved the module to EOF), plus needless borrows, redundant closures, `contains` over `iter().any`, `sort_by_key`, an unnecessary `to_string`, redundant `i32` casts and two redundant re-bindings. Three judgement calls kept the code and justified the suppression in a comment instead: the Lesk stemmer's identical-bodied suffix rules (distinct linguistic rules that will diverge), `build_reader_settings_panel`'s 8 arguments (a struct existing only for one call site), and `AuthorPageMsg::Fetched` was **boxed** rather than suppressed since ~376 bytes on every message including the frequent `Refresh` is a genuine cost. `Rc<RefCell<Option<Rc<dyn Fn()>>>>` appeared in three pages and is now the `pages::SelfRebuild` alias |
| 2026-09-02 | A0 step 2 continues: **Reading list converted** (4th page). It read `list_reading_list().unwrap_or_default()` in two places, so a failed database read rendered as the friendly *"Books you plan to read next"* placeholder — the same empty-state lie as the All books search bug, and the same class as the series-fetch bug the strict clippy gate just exposed: **code written never to complain**. Now `service.reading_list()` returns one owned snapshot and the page reports failures as a toast. Writes (reorder, remove, bulk add) still go through `service.catalog()`, the deliberate escape hatch — they belong to step 4. 2 new tests, including one asserting an empty queue raises **no** error, so the fix cannot regress into a toast on every visit |
| 2026-09-02 | A0 step 2 continues: **Shelves grid converted** (5th page) — `list_shelves().unwrap_or_default()` in both `init()` and `Refresh`, so a failed read rendered as "no shelves yet". Now one `service.shelves()` snapshot with the failure surfaced as a toast; the writes (create, edit, delete) keep going through `service.catalog()` until step 4. 1 new test covering both halves: a real shelf comes back, and an empty grid produces **no** error. **Saved quotes was examined and deliberately left alone** — it already does a full `match` on `list_all_quotes` and shows `"DB error: {e}"`, so it does not have the defect step 2 removes; converting it would be churn for its own sake. Being on the list of unconverted pages is not the same as being broken, and the count is not the goal |
| 2026-09-02 | A0 step 2 continues: **All books converted** (6th page), and it turned out to be *half* honest already — `reload()` matched on `list_books` and showed `"Database error: .."`, but `init()` used `unwrap_or_default()`, so **the first paint of the page claimed an empty library where a refresh on the same broken database would have told the truth**. Exactly the inconsistency a single seam removes. Now both paths go through `service.all_books(sort, query)`; the error still lands in the status line rather than a toast, because this page has always reported failures there and changing that would be a UI change smuggled into a refactor. The import-summary line still outranks the generic count. 1 new test pinning the three cases the page's own copy distinguishes: all books, a matching search, and a search matching nothing (which is **not** an error) |
| 2026-09-02 | **A1 (in-app dialogs) logged and scheduled** after the user reported that dialogs open as separate windows, which a tiling compositor (Sway) treats as ordinary top-levels — movable to another workspace, tiled beside the app, left behind on a workspace switch. Audit found the app already half-converted: an in-app float layer exists in `app.rs` (`float_host` + `float_scrim` on a `gtk::Overlay`) with a comment explicitly stating floats must never be separate windows "so the compositor can't move it to another workspace", and **5 dialogs already use it** (book float, series float, annotations, shelves, tags). **5 remain as `gtk::Window`**: metadata editor, shelf editor, the two book pickers, delete-shelf confirm. The 4 `FileDialog`s stay native on purpose — they are portal-backed and the user wants their real file manager. User's hard requirement recorded: **every in-app dialog needs a visible close/✕ button**, since without a title bar there is no compositor-provided escape; the existing five already comply (✕ on the floats, Done on the panels, global Esc/Q in `app.rs`). Ordered cheapest-first, metadata editor last |
| 2026-09-02 | A0 step 2 continues: **History and Lookup History converted** (7th and 8th pages). Both called `unwrap_or_default()` on every read — `history.rs` in `init()` and `reload()`, `lookup_history.rs` in `init()` and *both* refresh paths — so a failed read rendered as "Opened and finished books show up here as you read", i.e. indistinguishable from a fresh install. Both now take one snapshot per read and toast the failure; the clear-history writes still go through `service.catalog()` until step 4. 2 new tests: rows come back for a seeded event and lookup, and an empty log produces **no** error. **`saved_words.rs` was examined and deliberately skipped** — like `saved_quotes.rs` it already `match`es its main read and surfaces the error, so there is nothing for step 2 to remove there. Third page now audited-and-left-alone; the unconverted list is not a to-do list, it is a list of pages that have not been *checked* |
| 2026-09-02 | A0 step 2 continues: **My Library converted** (9th page) and it was the worst one — the dashboard made **eight** independent catalog reads and `unwrap_or_default()`d **every single one**, so a broken database rendered as a cheerful, fully laid-out, completely empty dashboard: no stats, no continue-reading strip, no quotes, no vocabulary, no history, and a reading goal of zero. The landing page of the app was the least honest page in it. All eight now come from one `service.dashboard(FEED_LIMIT)` snapshot and failures are toasted. Two structural fixes came with it: the free functions `build_dashboard`/`goal_card`/`history_feed` took `&Arc<Catalog>` and read the database themselves, which is exactly what blocks step 4 — they now take the snapshot instead; and `history_feed` held **an N+1**, calling `get_reading_progress` or `get_book` once per event, which is the same defect found in `saved_quotes.rs`. `LibrarySession` had to be exported from `crate::db` because no caller had ever been able to name the type. `reading_goal()`/`finished_this_year()` return bare `i64` and cannot fail, so they add no error rows. 2 new tests: a seeded dashboard reports no errors, and an empty one is **not** an error |
| 2026-09-02 | **Correction to the two rows above.** They record `saved_quotes.rs` and `saved_words.rs` as "examined and deliberately left alone" because each already `match`es its main read. When challenged to justify that, the reasoning did not survive: checking only the headline query is not a sufficient screening test. `saved_quotes.rs` calls `get_book` **per quote**, and `get_book` internally runs a second tags query — about **2N+1 queries on every page open and every keystroke in the search box** (500 quotes is roughly 1,001 queries). That is the same class of bug this project already fixed once for the Library dashboard (~99 reads down to ~8) and then reintroduced elsewhere unnoticed; there is still no batch `books_by_ids` in `src/db*` to fix it with. `saved_words.rs` swallows two *counting* queries with `.unwrap_or(0)`, so a failure silently reports **zero known words**. Both also keep scattered DB calls in UI code, which is the thing step 4 cannot move off the UI thread. The screening test is therefore: main read, **secondary/enriching reads, N+1 patterns, and off-thread readiness**. Both pages are back on the list. "It reports its main error" is not a reason to skip a page |
| 2026-09-02 | A0 step 2 continues: **Saved quotes converted** (10th page) — the page named in the correction row above. Its main read was always honest, but it called `get_book` **once per quote** and `get_book` runs a second query for tags, so a 500-quote library did about **1,001 round trips on every page open and every keystroke in the search box**. Added `Catalog::books_by_ids`, which does the whole batch in **two** queries (books, then all their tags at once) regardless of how many quotes there are, de-duplicates ids because one book usually owns many quotes, and chunks at 500 to stay under SQLite's host-parameter cap. Missing ids are simply absent from the map, so "this book was deleted" stays distinguishable from "the read failed" and the page keeps rendering *Unknown Book*. The same N+1 in the shared `export_all_quotes_markdown` (used by Settings → Export too) is fixed the same way. The page keeps reporting errors in its status line, since that is what it has always done. 4 new tests (187 total): the batch agrees with `get_book` including tags and file paths, duplicate and unknown ids are tolerated, an empty id list is not an error, and search hits, misses and an empty library are all error-free |
| 2026-09-02 | A0 step 2 continues: **Vocabulary (saved words) converted** (11th page), the second page named in the correction row. Its list read was honest, but the two header counts were fetched by calling `list_saved_words` twice more and taking `.len()` — loading up to 500 full rows, definitions and all, purely to count them — and both were swallowed with `.unwrap_or(...)`, so a failed read reported **zero known words** as though it were a fact. Added `Catalog::saved_word_counts()`, one `COUNT(*)` + `SUM(CASE WHEN known ...)` query, and routed the page through `service.words(query, filter)`. Two side benefits: the page went from **three** queries per reload to **two**, and the header counts now cover the whole table instead of stopping at the 500-row display cap, so they are correct for large vocabularies. Errors stay in the status line, matching the page's existing behaviour. 2 new tests (189 total): counts are read rather than guessed and survive filtering, and an empty vocabulary is **not** an error |
| 2026-09-02 | A0 step 2 continues: **Book float and Series float converted** (12th and 13th pages). The book float made the same three reads twice — once in `init()`, once in `reload_state()` — and swallowed all six, so a database failure rendered as **"this book was deleted"**: the panel could not tell a missing book from a broken read. Now one `service.book_detail(id)` snapshot serves both paths, and `book: None` means genuinely gone while a real failure goes to `errors` and a toast. The series float held **another N+1**: `book_finished_at` once per row, so a 20-book series meant 20 extra queries every time the panel opened. Added `Catalog::finished_book_ids(&[i64])` — one query, chunked at 500 — and the panel now computes the whole set before the row loop. Its `books_in_series` read was also swallowed, which would have quietly claimed **you own none of the series** on a failure; it now reports. That is the **third** N+1 found since the screening test was tightened (saved quotes, the dashboard's history feed, and this one), which is the strongest evidence yet that "its main read is honest" was never a safe way to skip a page. 3 new tests (192 total) |
| 2026-09-02 | A0 step 2 continues: **Shelf detail converted** (14th page), and a stale-header bug fell out of it. `Refresh` re-read the shelf row and then called `reload()`, which re-read only the books — two separate reads of the same thing, both swallowed, and the header could end up describing a shelf that had already been renamed. One `service.shelf_detail(id, sort, query)` snapshot now returns the shelf and its books together, so they cannot disagree, and a missing shelf skips the books read entirely instead of producing two vague outcomes. Failures toast. The book-picker at the bottom of the file still takes a raw `Arc<Catalog>` **on purpose** — it is a separate `gtk::Window` and therefore an **A1** target; converting its plumbing now would only have to be redone when it becomes an in-app dialog. 2 new tests (194 total). **A test I wrote was wrong and CI caught it**: it asserted a book could be both finished and on the reading list, but `set_book_finished(true)` deliberately drops the book off the list. The test now pins that real coupling instead of my assumption about it |
| 2026-09-02 | A0 step 2 continues: **Book page converted** (15th page) — the second-largest file on the list, 31 swallow sites. Two clusters did the damage. The header repeated the same three reads across **four** call sites, and two of those re-read the book row *and then* called `reload_state()`, which re-read the other two — so opening the page and pressing anything did overlapping work, all of it swallowed, and a failure looked like a deleted book. It now reuses the very same `service.book_detail(id)` snapshot the book float uses; one seam, two pages. The stats/timeline card made **six** more swallowed reads scattered through a 150-line function (total time, session count, reading position, seconds-by-day, recent sessions, finished date, first opened) and drew "you have never read this book" on any failure; those are now one `service.book_stats(id, days, limit)` call at the top of the function. `SessionRow` had to be exported from `crate::db`. The remaining `unwrap_or`s in this file are string and slice defaults (`.get(..10).unwrap_or("")`), not swallowed DB errors. 2 new tests (196 total) |
| 2026-09-02 | A0 step 2 continues: **Reader converted** (16th page) — the largest file in the project. Its raw count of 35 "swallows" was misleading: most are `get_pref_i64(key, default)`, which returns a default **by design** and is step-4 work, not error swallowing. The real defect was four reads at book-open — the book row, its highlights, its bookmarks and your saved words — each `unwrap_or_default()`d, so **opening a book against a broken database silently showed none of your work**: the reader looked completely normal and simply had no highlights. Those are now one `service.reader(book_id)` snapshot. The three `reload_*` helpers also swallowed, and worse, they *overwrote* the live lists with empty ones on failure; they now report and **keep what is on screen**, because a blank list reads as "you never highlighted anything". Also removed a redundant `get_book` at session start that re-read the book row the snapshot had just fetched. Fixes the clippy failure from `a4ebfd2`: two **multi-line** `model\n.catalog` chains in `book.rs` that a single-line grep missed — the sweep is now a regex over `(self|model)\s*\n\s*\.catalog` and the whole repo is clean. 2 new tests (198 total) |
| 2026-09-02 | **A0 step 2 complete.** Final pass over the last four files, none of which needed a snapshot but three of which were lying anyway. **Metadata editor** and **Shelf editor** both did `let Ok(Some(x)) = ... else { return }` / `_ => return`, so on a failed read the dialog **simply never appeared** — no window, no message, nothing to click; they now say whether the thing was deleted or the read failed. Shelf editor's duplicate-name check was `.unwrap_or(false)`, i.e. a failed check assumed the name was free. **Settings** rendered a failed `list_dictionaries()` as "no dictionaries installed", which is exactly what a successful uninstall looks like. The reader's dictionary search made a broken index indistinguishable from "that word isn't in the dictionary". **`author.rs` genuinely needed nothing** — it makes no database reads at all, it only passes the `Arc` to its children, and its lone `unwrap_or_default()` is on a local helper; that is what a real skip looks like, stated precisely, versus the earlier hand-waving. Remaining `unwrap_or`s across the pages are now only `get_pref(key, default)` calls (defaults by design, step 4) and string/slice defaults like `.get(..10).unwrap_or("")`. **Score for the whole step: 16 pages converted, 5 N+1 query storms removed, 3 new batch queries (`books_by_ids`, `saved_word_counts`, `finished_book_ids`), 198 tests** |
| 2026-09-02 | **A real bug the step-2 pass uncovered, not just a refactor.** The new `book_stats` tests failed in CI, and the cause was a genuine defect in `Catalog::book_first_opened`: `SELECT MIN(at) ...` over zero rows still returns **one row containing NULL**, so `.optional()` does not help — the *value* has to be nullable. Reading it as a plain `String` made "this book has never been opened" a **hard error**. It went unnoticed for as long as it existed precisely because every caller wrote `.ok().flatten()`,which turned the error into `None` and produced the right screen by accident. This is the clearest possible demonstration of why step 2 was worth doing: the swallow was not just hiding hypothetical future failures, it was hiding a live bug in the query underneath it. Fixed by reading into `Option<String>`; audited the other aggregates and they all already use `IFNULL`. 1 regression test (199 total) |
| 2026-09-02 | Wrote `docs/testing-a0-step2.md` answering "do I need to test anything before A1?". Short answer recorded: **no new parameters, no required testing** — CI covers compile, clippy `-D warnings` and 199 tests. Two things CI genuinely cannot check are written down: (1) the **corrupt-database test**, which is the only way to see what step 2 actually fixed — `XDG_DATA_HOME=/tmp/kalam-test` gives a throwaway library so the real one is never touched, then overwrite `catalog.db` with garbage and confirm the app now *says* something instead of drawing a cheerful empty library; and (2) a two-minute pass opening each page to confirm the happy path still looks right, since CI has no display. Also noted the three user-visible effects of step 2 on a healthy database: the `book_first_opened` bug fix, Saved quotes no longer running ~1,001 queries per keystroke, and failures now toasting. Added a pre-A1 question for the user: the five already-in-app dialogs are the pattern A1 will copy five more times, so it is worth deciding now whether they are the standard to match |
| 2026-09-03 | **Crash fixed: a corrupt catalog aborted the process with a core dump.** The user ran the corrupt-database test from `docs/testing-a0-step2.md` and it did not toast — it died. Cause was a fallback in `AppModel::init` that "handled" a failed `Catalog::open()` by **calling the same function again and `.expect()`ing it**, which is a guaranteed panic; and because `init()` runs inside a GTK signal callback, that panic **cannot unwind**, so it escalated to `panic in a function that cannot unwind` → abort → core dump, printing a raw backtrace instead of saying what was wrong. Fixed in two places: `main()` now opens the catalog **before** `app.run()` and, on failure, prints the error, the database path, and the exact `mv` command to move the broken file aside (noting book files live elsewhere and are safe), then exits 1; and the `init()` arm no longer retries — it reports and exits cleanly, since reaching it means the database broke between the pre-flight check and startup. **This is the second real bug the step-2 pass has surfaced**, and again the pattern is the same: the error path had never been executed, so nobody noticed it was nonsense |
| 2026-09-03 | **Backdrop-click now closes in-app dialogs, and the A1 close-button rule is revised.** The user asked for click-outside-to-close and questioned whether the ✕ could then go away. Implementation turned out to be two lines: the scrim already had a `GestureClick` whose handler was **empty** — it existed only to stop clicks reaching the page behind it, so the dimmed area looked interactive and did nothing. It now sends `CloseBookDialog`, the same message Esc sends. Z-order was already correct (scrim added to the overlay before `float_host`), so clicks *inside* the dialog are unaffected. On the button question the user made the sharper point that **the dialogs exist for different reasons and should not all be dismissed identically**; the roadmap now carries a table mapping dialog *kind* to affordance — **‹ Back** for detours you navigated into, **✕** for transient overlays, **Cancel + Save** for forms with unsaved input, **Cancel / Delete** for confirmations. Recorded decision on dropping the button entirely: **no** — backdrop-click and Esc are both invisible affordances, so a dialog whose only exits are invisible is still a trap; keep one visible control, but the right one rather than a reflexive ✕ |
| 2026-09-03 | **A1 done: all five remaining `gtk::Window` dialogs now draw inside the app.** `gtk::Window::builder` no longer appears anywhere in `src/`. One new helper, `src/widgets/in_app_dialog.rs`, walks up from any widget to the app's root `gtk::Overlay` and adds its own scrim + centred panel; it deliberately does **not** reuse the `AppMsg` float layer, because these dialogs are plain functions taking an `on_confirm: impl Fn()` closure and a closure cannot travel through a `#[derive(Debug)]` message enum. `DialogExit` ended up with two variants, not the table's four: both converted kinds already bring their own named buttons, so the header adds none — `OwnButtons` (pickers, delete confirmation) closes on a backdrop click, `UnsavedInput` (metadata editor, shelf editor) **does not**, because silently discarding a half-typed description over a slightly-off click is a bad trade. The ‹ Back and ✕ rows still describe the book/series floats, which have their own headers in `app.rs`. The interesting part was **what a `gtk::Window` had quietly been doing for free**: (1) `close()` destroys the widget tree and so breaks the reference loop between a widget and the callback capturing it — removing an overlay child does not, so `teardown()` empties the host too, otherwise every dialog ever opened would leak; (2) a window has a default height, while a panel centred in an overlay is sized by its content, so the metadata form and the smart-shelf rule list needed `max_content_height` + `propagate_natural_height` or a long description would push Save off a 768px screen; (3) the cover `gtk::FileDialog` is portal-backed and needs a genuine top-level parent, now resolved from the anchor's root. Also deleted three copies of a `window_of()` helper that existed only to parent these dialogs, and two hand-rolled Esc handlers now that the helper gives Esc to all of them. 1 new test |
| 2026-09-03 | **Audited the five dialogs that were already in-app; they did not match the five new ones, or each other.** Four fixes. (1) The book float's ✕ was tooltipped "Close (Q)" and the series float's "Close (Esc)" — same layer, same keys, two different lessons. (2) A real bug: the global float key handler in `app.rs` closed on `q` whenever a float was visible **without checking whether a text box had focus**, so typing the letter `q` into the tags panel's entry dismissed the panel instead of typing. Esc now always closes; `q` is ignored while an entry has focus. (3) On the user's instruction the ✕ came off **both** floats with **no ‹ Back replacement** — they are read-only detours, so a stray backdrop click costs nothing ("I won't lose anything if I accidentally misclicked"). This deliberately waives the "keep one visible control" rule for exactly the case the rule was never meant to cover: there is nothing to lose. Removing the button made `SeriesFloatMsg::Close` and `SeriesFloatOut::Close` unreachable, so they were deleted, along with four now-dead `.kalam-float-close` CSS rules. (4) The annotations, shelves and tags panels had **no title bar at all** while the new dialogs do, and used three byte-identical copies of a 14px-corner shell against the dialogs' 20px; they now share a `panel_title()` helper using the dialog header's own markup and CSS class, and one merged CSS rule. Also corrected an earlier misreading of my own: the series float opens from the **book page**, not the book float, so closing it to the page was always correct |
| 2026-09-03 | Follow-up: the `q`-while-typing guard failed clippy with `E0034: multiple applicable items in scope — multiple \`focus\` found`. Both `WidgetExt` and `GtkWindowExt` define `focus`, and with `gtk::prelude::*` in scope on a `gtk::Window` a bare `.focus()` is ambiguous. Fixed by naming the trait: `gtk::prelude::GtkWindowExt::focus(&key_root)`. Also reordered the focused-widget test to check `gtk::Text` first, since that is the inner widget of a `gtk::Entry` and the one that actually holds focus |
| 2026-09-03 | **Book float: fixed the panel changing size from book to book.** The user reported the floating book details resizing and guessed the tags were behind it — correct, and there were three more causes of the same fault. (1) **Tags** were a `gtk::FlowBox` with `max_children_per_line: 8` holding up to 12 chips, so a book with 9+ tags **wrapped to a second row** and made the panel taller. Per the user's instruction they are now a **single horizontally-sliding line** — a `ScrolledWindow` with `hscrollbar_policy: Automatic`, `vscrollbar_policy: Never` and a fixed 34px height — never two lines. The `.take(12)` cap went with it: the row scrolls, so every tag can be shown. (2) **Title and series** labels had `set_wrap: true`, so a long title took two or three lines; both are now one ellipsised line with the full text in a tooltip. (3) **Authors** are filled by the shared `replace_author_links`, which builds a wrapping FlowBox — right for the book page and reader, wrong here, so the host's height is pinned at the float's own call site rather than changing the helper for everyone. (4) The **biggest** one: the description section's "no Read more" branch left the section completely unbounded (`height_request(-1)`, natural height, no max), so a short blurb gave a short panel and a nearly-long-enough one gave a tall panel; it now reserves the same `DESC_SECTION_HEIGHT` as every other branch. Root cause behind all four: `set_size_request(720, 420)` is a **floor**, not a size — GTK grows a widget past its request whenever content needs the room, so any unbounded child could resize the panel |
| 2026-09-03 | Book float, two follow-ups from the user, both caused by the previous fix. (1) **The action buttons shifted up on a book with no tags.** `fill()` hid the tag scroller when `book.tags` was empty, and a hidden widget occupies no space, so the buttons moved depending on whether the book happened to be tagged. The row is now shown **unconditionally** — it is empty and invisible either way, and its fixed height is precisely what holds the buttons still. (2) **Removed the visible scrollbar from the tag row.** `hscrollbar_policy` changed from `Automatic` to **`External`**: the row still slides by wheel, touchpad and drag, but GTK draws and allocates no bar. `Automatic` was also its own small version of the original bug — it reserved bar space only for heavily-tagged books. `TAGS_ROW_H` dropped 34px → 26px now that no bar has to fit. A belt-and-braces CSS rule hides any scrollbar a theme might still paint, and it does so by making the slider's **background transparent, not with `opacity: 0`** — the warning block at the top of `style.rs` records that opacity on a collapsed scrollbar renders through a zero-sized offscreen surface and trips `pixman_region32_init_rect: Invalid rectangle`. Nearly repeated that exact bug |
