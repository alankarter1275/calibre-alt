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
- [x] Selection toolbar: compact themed icon actions with tooltips (P3)

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
- [x] Dictionary popup inside WebView: shows up to five matching entries near the selection rect, with separate Save word / Copy actions for each result
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

**Status: planned; not started.** The dictionary features below are deliberately
scheduled for later. They are recorded here as the implementation brief for a
future isolated reader-improvement phase.

### Implementation brief: Kalam dictionary overhaul

#### Context for the implementing AI

Kalam is a Rust + GTK4 + Relm4 + WebKitGTK ebook reader. Work on branch
`arena/01a0487b-calibre-alt`. The dictionary spans three areas:

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

#### Phase 1 — Precomputed headword index

**Goal:** exact/lemma lookups hit an index instead of `LIKE` scans; kill the
`LENGTH(word)` tiebreak proxy.

**Do:**

- In `db.rs` `migrate()`: `ALTER TABLE dict_entries ADD COLUMN key TEXT`
  (guarded — check `PRAGMA table_info`). Add
  `CREATE INDEX IF NOT EXISTS idx_dict_entries_key ON dict_entries(key COLLATE NOCASE)`.
  Bump `SCHEMA_VERSION`.

- Add a normalization fn `fold_key(word) -> String`: lowercase, strip diacritics
  (NFD + drop combining marks), collapse whitespace, trim surrounding
  non-alphanumerics per token. Reuse/extend `normalize_dictionary_term`.

- On import (all three importers in `dict.rs`) populate `key = fold_key(word)`
  when inserting.

- Backfill: after the migration, if any `dict_entries.key IS NULL`, run a
  one-time `UPDATE` computing key for existing rows (batch in a transaction).
  Guard so it runs once.

- Rewrite `search_dict_exact_or_prefix` to match on `key = ?` (exact) then
  `key LIKE ?||'%'` (prefix), ordering exact-first.

**Acceptance:** looking up `Run`, `run`, `rún` all resolve to `run`; explain-plan
uses `idx_dict_entries_key`; existing databases upgrade without reimport.

#### Phase 2 — Real lemmatization from WordNet data

**Goal:** irregulars resolve (`went→go`, `mice→mouse`, `better→good`), with suffix
rules as fallback only.

**Do:**

- Ship WordNet's morphological exception lists (`noun.exc`, `verb.exc`,
  `adj.exc`, `adv.exc`) as a gzipped resource under
  `resources/dictionaries/`, embedded via `include_bytes!` like the existing
  WordNet TSV.

- Load them once into a `HashMap<String, Vec<String>>` (surface → lemmas),
  lazily (`OnceLock`).

- In `dictionary_query_variants`, consult the exception map before
  `simple_inflection_variants`; keep the suffix rules as fallback. Preserve
  existing dedup via `push_dictionary_variant`.

- Extend the existing `#[cfg(test)]` tests with irregular cases.

**Acceptance:** `went→go`, `mice→mouse`, `better→good`, `running→run` all return
a headword; regular cases still work.

#### Phase 3 — Phrase decomposition

**Goal:** `odd mixture` yields something useful instead of a dead end.

**Do:**

- Add `search_phrase(phrase, limit) -> PhraseLookup` in
  `db/dictionaries.rs`. Strategy: (a) try full phrase via `search_dict`; (b) try
  the longest contained sub-phrase that is a headword (slide window from
  longest to shortest — catches `run a risk`); (c) if neither, return per-token
  results: for each token, run the single-word `search_dict`.

- Define a return type distinguishing `Phrase(entries)` vs
  `Breakdown(Vec<(token, entries)>)` vs `Empty`.

- Wire reader.rs `dict-lookup` handler to call `search_phrase` when the query
  has `>1` token.

**Acceptance:** `odd mixture` (no headword) returns a breakdown for `odd` and
`mixture`; `run a risk` (if present) returns the phrase entry; single words
unchanged.

#### Phase 4 — Source-aware results & ranking (monolingual-first)

**Goal:** results carry their dictionary; ranked with the bundled monolingual
WordNet first.

**Do:**

- `search_dict` currently returns `DictEntry` (no dict name). Change it (or add
  a sibling returning a richer struct) to `JOIN dictionaries` and include
  `dict_id + dict_name`. Update `DictEntry` or introduce
  `DictHit { entry, dict_id, dict_name, tier }`.

- Add per-dictionary priority: `ALTER TABLE dictionaries ADD COLUMN priority
  INTEGER NOT NULL DEFAULT 100` (guarded; bump `SCHEMA_VERSION`). Default the
  bundled WordNet to a higher priority (lower number = shown first) than
  imported dicts, since the primary user wants monolingual first. Expose reorder
  in Settings later (not required this phase).

- Rank results by tuple: (match tier: exact > lemma > phrase > prefix > substring)
  then (dictionary priority) then (word length). Gate the definition-substring
  branch so it never outranks a headword hit.

**Acceptance:** a word in both WordNet and an imported dict shows WordNet first;
results expose their source name; substring-in-definition matches sink to the
bottom.

#### Phase 5 — Popup redesign (app chrome, dark)

**Goal:** fix the fake result, structure the entry, label sources. All in
`epub_book.rs` (`showDictPopup` + CSS) and the `reader.rs` handler that feeds it.

**Do:**

- Empty state: remove the `No definition found... Total dict entries: N` string
  entirely. When there's no hit, render a distinct empty-state block (not a
  `.kalam-dict-result`): a short `No entry for '{query}'.` plus, for phrases,
  the breakdown chips from Phase 3 (`[odd] [mixture]`, each clickable → re-fires
  `dict-lookup` for that token via `kalamBridge`). Also a `Search in book` action
  (Phase 6).

- Kill `RESULT n`: replace that subheading with the dictionary name. When
  results span multiple dictionaries, render a segmented control / tabs at the
  top switching source; single source → quiet subheading.

- Structure the entry: headword once at top (drop the duplicate). If the
  definition text carries POS/sense structure, render numbered senses with
  italic examples. For imported HTML dicts, sanitize (allowlist
  `b/i/em/strong/br/p/ul/li/span`, drop scripts/handlers) and render instead of
  `strip_dict_html` flattening — add a `sanitize_dict_html` fn.

- One action bar: a single Save / Copy / Highlight-in-book row acting on the
  focused sense, instead of per-result button pairs.

- Anchor discipline: keep the existing rect-anchored placement + above/below
  flip; add a small caret pointing at the word and ensure it never overlaps the
  selection rect (nudge if it would).

- Theming: keep dark chrome on all paper themes. Reuse chip tokens (radius,
  blur, shadow, `--kalam-*`). Add a subtle border for contrast over light/sepia
  pages.

**Acceptance:** the screenshot's fake `1 RESULT / No definition / Total dict
entries` is gone; a real multi-source lookup shows tabs with dictionary names;
phrase misses show tappable word chips; imported HTML renders formatted; popup
stays dark on sepia.

#### Phase 5.5 — POS grouping + Lesk "likely sense" hint

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

#### Phase 6 — Interaction

**Goal:** tap-to-look-up and keyboard parity.

**Do:**

- Tap-a-word: in `epub_book.rs`, on a plain click with no selection, resolve the
  word under the caret (use `caretRangeFromPoint`/`caretPositionFromPoint`, expand
  to word boundaries) and fire `dict-lookup` with that word + surrounding sentence
  as context. Keep drag-select → phrase. (There's already a dict-shortcut bridge
  path to model this on.)

- Keyboard: Esc closes the popup; ←/→ switch dictionary tabs; ↑/↓ move senses;
  Enter saves the focused sense. Wire in the popup JS.

- Search-in-book action: from the popup, trigger the reader's existing in-book
  search for the headword (reuse whatever find/search path `reader.rs` has; if
  none, scope this to "highlight all occurrences in current chapter").

**Acceptance:** single tap on a word opens the popup; Esc/arrows/Enter work;
search-in-book jumps to occurrences.

#### Phase 7 — Vocabulary tools (lower priority)

**Goal:** make Saved Words more than a list. Only after 1–6.

**Do:**

- Saved Words already stores `word`, `definition`, `book_id`, `chapter_index`,
  `context_text`. Add a Saved Words review view (in `src/pages/saved_words.rs`)
  with "mark as known" (guarded `ALTER TABLE saved_words ADD COLUMN known INTEGER
  DEFAULT 0`).

- Add export: CSV (`word,definition,context`) and optionally Anki-importable
  format, writing to a user-chosen path (mirror the existing `~/Quotes.md`
  export pattern).

**Acceptance:** saved words can be marked known and exported to CSV.


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

1. **Reader milestone 1 validation:** ✅ completed on Arch; the current
   annotation workflow was tested and works as expected.
2. **Reader milestone 2 validation:** test hybrid anchoring with an EPUB whose
   chapter HTML has changed, then record your sign-off or change requests.
3. **Reader milestone 3 validation:** test phrase preservation,
   punctuation/inflection normalization, and multiple dictionary results
   together; record your sign-off or change requests.
4. **Later dictionary work:** when it resumes, follow the planned dictionary
   overhaul below in phase order, starting with Phase 1. Keep the existing
   offline import flow for all other packs and do not infer definitions for
   arbitrary compositional phrases before the planned phase addresses them.
5. After the reader feature work is complete, return to the deferred annotation
   design polish without changing saved-highlight anchoring or temporary
   emphasis.
6. Only after those reader milestones, consider multi-chapter buffering,
   continuous book-wide scrolling, chapter auto-advance redesign, or advanced
   CFI.
7. **UI overhaul:** after the reader track is signed off, continue the
   mockup-first screen work. `library_look.png` shows a two-column dashboard;
   the app is currently a single vertical stack.
8. **P6 — Downloads hub** (unified queue + folder watch; prerequisite for P7).
9. **P8** when you want comics for real (UI target already specified above).

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
