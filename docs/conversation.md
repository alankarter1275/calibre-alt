# Kalam — Design Conversation Log

This file is a **living record of design discussions**. It is edited as ideas
are accepted, refined, or rejected. Rejected ideas are marked with `~~strike~~`
and a one-line reason, so we remember *why* — not just *what* we decided.

Status emoji: ✅ accepted · 🔶 under discussion · ~~struck~~ rejected.

---

## 1. The performance conversation

**Question asked (2026-09):** "The app is Rust only but not fast/snappy enough.
Should we rewrite parts in another language for speed, polish in Rust, or
something else?"

### Accepted: language is not the bottleneck — architecture is

**Verdict: do NOT rewrite in another language. Polish in Rust, but the polish
is architecture, not syntax.**

Why: Kalam is "Rust only" in our code, but the runtime doing almost all the
work is already C (GTK4), C++ (WebKitGTK), and C (SQLite). Rewriting our code
in C++/Zig/Go moves the same bytes across the same FFI boundaries into the
same libraries — months of work, a worse safety story, identical perceived
speed.

Reference implementations prove the point:

| App | Stack | Snappy? |
|---|---|---|
| Foliate | JS + GTK + WebKitGTK | Same profile as ours |
| Calibre | Python + Qt + Chromium | Heavier |
| KOReader | C++ + MuPDF + custom renderer | Genuinely snappy |

Lesson: the one snappy reader in the group is the one **without a browser
engine**. The engine, not the language, is the performance ceiling.

Our SQLite layer is already tuned exactly as we'd tune it: WAL,
`synchronous=NORMAL`, 64 MB cache, `temp_store=MEMORY`, 256 MB mmap,
`prepare_cached` everywhere, foreign keys on. The dict rebuild is guarded so
it doesn't run every startup. That layer would have been the tempting rewrite
target, and it's already right.

### Where the time actually goes

- A cold WebKitGTK process is 300–600 ms and 100+ MB before it paints.
- Reader does `webkit6::WebView::new()` + `load_html()` per chapter — a full
  browser engine parse + layout + rasterize on every book open and chapter
  change.
- Dictionary popup, annotations, find-in-chapter all cross the JS bridge
  into that engine.

### How we'd have built it (accepted approach)

Same core stack — Rust + GTK4 + WebKitGTK + SQLite — then attack the render
pipeline:

1. **One webview, created lazily.** Don't touch WebKit until the first book
   opens; prewarm the web process in the background while the user browses
   the library.
2. **Page by CSS multi-column, never re-layout on turn.** Load chapter once,
   let WebKit paginate via fixed-height columns; page turn = pure
   scroll/transform. Settings changes should be CSS-variable injection, not
   `load_html` again.
3. **Thin the bridge.** Every `postMessage` is an async hop across process
   boundaries. Batch where possible; dict popup should be pure DOM/CSS
   in-page — one Rust call per lookup. (Phase 10 already did the right
   version of this: don't log per-keystroke.)
4. **Keep chapters small.** `load_html` with a giant string = WebKit parsing
   + laying out a huge document on the main thread. Split or lazy-load
   sections for 100k+ word chapters.
5. **Profile for two days, fix the top three.** `perf` + sysprof + GTK
   inspector: cold start → window, book open, chapter turn, dict lookup.

### The one honest rewrite — and why even that is Rust

**~~Throw out the browser engine and render the book ourselves~~** — 🔶
**under discussion** (user: "we will talk about this in detail").

If we made the fastest possible ebook reader on Linux, the ceiling-move is
the KOReader play: a purpose-built text engine (skia/cosmic-text or pango)
where pagination, selection, and dict lookup are in-process operations
measured in microseconds instead of bridge round-trips. But that's a 1–3
person-year project reimplementing a slice of CSS paged media, and we'd lose
WebKit's accessibility, IME, and CSS completeness for free. **Even in that
extreme case we'd write it in Rust.** There is no scenario where the answer
is another language.

**To discuss in detail:** what we keep (anchoring, selection, dictionary
integration), what we lose (CSS, a11y, IME), what we'd write first.

---

## 2. Second opinion reviewed (another AI's analysis)

**Question:** another AI's answer claimed the bottleneck is the update model,
not Rust, with three specific costs:

1. `build_book_grid` at `src/widgets/book_row.rs:136` — 400 books = 400
   fully-realized card trees; GTK4 has `GtkGridView` + `GListModel` for
   ~30 widgets regardless of library size.
2. `rebuild_list` at `src/pages/all_books.rs:395` — teardown and rebuild the
   whole grid on every sort/filter/refresh.
3. Nothing off the main thread except network calls — every SQLite query and
   image decode runs on the GTK main loop; first paint = N synchronous JPEG
   decode-and-scale ops.
   Plus: cover cache is thread-local + in-memory (dies every launch, decode
   still synchronous on first paint); `style.rs` is 3,638 lines of CSS,
   `set_global_css` re-parses the whole sheet; `backdrop-filter: blur` on the
   dictionary popup.

**Verified against the code — all accurate.** `build_book_grid` really is at
book_row.rs:136, `rebuild_list` really does the remove-and-rebuild dance at
all_books.rs:395, the cover cache really is in-memory with a 400-entry bound,
the CSS really is 3,638 lines, `async-channel` really is in Cargo.toml. The
blur is 16px (not 22px) and is on the popup chip, not the whole popup —
minor, doesn't change the point.

### Where we agree

The problem is architecture, not language — same conclusion reached
independently.

### Where we push back

1. **"Single biggest cost" is asserted, not measured.** If "not snappy"
   means the library page lagging on scroll/search: yes, grid + decodes. If
   it means opening a book or turning pages: that's WebKit, and their list
   does nothing for it. 30 minutes of `perf` data before crowning a
   "single biggest cost."
2. **Their fix order puts the hardest thing first.** `GListModel` behind
   `GtkGridView` is the right end-state but real work. The cheaper fixes —
   **persist ~200px thumbnails at import time** (decode 30 tiny files instead
   of 400 big ones, cache survives restarts) and **async cover binding**
   (card shows placeholder, texture swaps in) — probably capture 80% of the
   perceived win at 20% of the effort. Order: thumbnails → async decode →
   measure again → virtualize only if the numbers say so.
3. **DB-off-main-thread nuance.** `Catalog` already wraps the connection in a
   `Mutex` — moving queries to a worker thread is feasible without rebuilding
   everything. But for a few thousand books most SQLite queries are
   sub-millisecond; the pain is image decode, LIKE-scans across the
   dictionary (no FTS index), and widget construction. The practical rule:
   "nothing *slow* on the UI thread" — decode is the slow one.
4. **Small nits.** Split-CSS is lower value than implied — re-parsing 3,600
   lines on a theme switch is a one-time few-ms cost; per-widget style
   resolution shrinks when the widget count shrinks. Fix the widget count and
   the CSS problem mostly fixes itself.

### What we'd actually do (accepted plan)

1. **Measure for a day.** `perf`, sysprof, GTK inspector → numbered list:
   cold start, book open, chapter turn, dict lookup, library scroll.
2. **Thumbnails at import + async cover decode.** Big, simple, survives
   restarts.
3. **Virtualize the grid if the numbers earn it.**
4. **The reader** — the big lever; the "throw out the browser engine"
   conversation plugs in here.

---

## 3. The Yazi philosophy conversation

**Question:** "Let's make Kalam with the same philosophy as Yazi file manager —
blazing fast, async tasks, clean architecture."

### What Yazi actually is

Four ideas:

1. **The UI never does work.** Render layer draws and sends commands; copying,
   renaming, previewing, filtering are all requests answered asynchronously.
2. **A task system.** Heavy operations become background tasks with progress,
   cancellation, queue. UI shows progress bars, never blocks.
3. **Preloading.** Hover a file → preview loads before you need it. It feels
   instant because it's *early*, not fast.
4. **Clean layering.** `yazi-core` is pure logic with no UI; UI is a thin
   shell. Testable, stays clean.

Philosophy in one sentence: **"don't make the UI fast — make it never wait."**

### The good news: Kalam already has the skeleton

- **relm4 is already an actor framework.** `AppMsg`, `ComponentSender`,
  `PageSlot` = Yazi's event bus, type-safe, GTK-integrated. Yazi built its
  own event system because it had to; we already have one.
- **`async-channel` is already in Cargo.toml** — the same hand-off pattern
  Yazi uses, already proven in the metadata fetch.
- **`src/db/` is already separate from the UI** (headless tests) — that's our
  `yazi-core`; it just needs a *service layer* on top.

So: not adopting Yazi's architecture from scratch — finishing the job already
started.

### What Yazi's shape looks like in Kalam

| Yazi | Kalam equivalent | Status |
|---|---|---|
| `yazi-core` (pure logic, no UI) | `src/db/` | ✅ done |
| Event bus | relm4 components | ✅ done |
| Task system (progress + cancel) | **`src/tasks.rs` TaskManager** | ❌ missing |
| Preloaders (preview cache) | **cover preloader + chapter preloader** | ❌ missing |
| Thin UI that only renders state | **pages still do their own DB calls** | ❌ missing |

The missing pieces:

1. **A service layer.** One `LibraryService` owns the `Catalog` and exposes
   requests: "books matching filter", "load cover". Pages stop calling the DB
   directly and start *asking*. Moving queries off the UI thread becomes a
   change in one place, not in every page. Most Yazi-like thing we can do;
   makes everything after it easy.
2. **A task manager.** Import, dictionary rebuild, metadata fetch, EPUB
   conversion → tasks with progress + cancellation. Metadata fetch already
   half-does this; extend to everything slow.
3. **Preloaders.** Cover preloader: rows 1–30 visible → worker decodes
   31–60 in background; scroll down and everything's already there.
   Reader: preload the *next chapter* while reading the current one —
   kills most of "book open feels slow" even before the renderer
   conversation.
4. **Thin pages.** After 1–3, pages become what Yazi's UI is: render the
   state given, send commands on click. No DB calls, no decoding, no loops
   that build 400 widgets.

### What does NOT translate

- **~~Tokio~~** — rejected. Yazi runs on tokio because a TUI has one tiny
  render loop; GTK has its own main loop and its own async. We don't need
  tokio; we need `thread::spawn` + `async-channel` + `glib::idle_add` back to
  the UI — the pattern we already have. Adding tokio to a GTK app is a cost,
  not a feature.
- **~~The plugin system~~** — rejected (initially). Yazi's Lua plugins work
  because a file manager's verbs are tiny; an ebook reader's verbs are huge.
  We wrote "no plugin API" in the roadmap. **User has since reversed this —
  see section 5. We now want plugins, design TBD.**
- **The "blazing fast" bar itself.** A TUI renders text in microseconds and
  reads a directory listing. We render books. Even with perfect architecture,
  WebKitGTK dominates. That's why the renderer conversation is the other
  half: **Yazi's architecture makes the app feel instant around the engine;
  the custom renderer replaces the engine. Do both, architecture first.**

### What we'd add, beyond what was asked

1. **Startup prewarm.** Kalam will never start in milliseconds (GTK+WebKit),
   but it can *look* like it: don't build the WebView until the first book
   opens, prewarm in background while browsing, warm the dictionary at idle.
2. **A perf budget as a habit.** CI runs 156 tests every push — add a rule
   like "library with 2,000 seeded books must build its grid in under N ms"
   as a test. It won't be precise, but it catches regressions — the day
   someone adds a `for book in books` loop again, the test screams.
3. **Live config reload.** Themes live in `theme.rs`; a "reload config" that
   re-applies without restart is small and makes the app feel alive.
   (Caveat: `set_global_css` re-parses 3,600 lines; a few ms, fine for a
   manual action.)
4. **The testability dividend.** Headless db tests already caught real bugs
   (the `quote_ident` escape, the Phase 10 FK violation). A clean service
   layer makes more of the app testable the same way.

### Where we'd start on Monday

1. `LibraryService` behind the existing `Catalog` — pages ask, service
   answers. (Weeks, mostly mechanical.)
2. Cover preloader + thumbnail persistence — fastest visible win.
3. Task manager for import/rebuild/metadata.
4. Grid virtualization — now *easy*, because the service layer feeds it.
5. Then the renderer conversation.

**Closing thought:** Yazi's real secret isn't speed — it's that **the
architecture makes speed inevitable**. Every decision pushes work away from
the render path. That fits Kalam perfectly, and unlike the language question,
it requires throwing nothing away.

---

## 4. Scope conversation — the roadmap's future phases

**Question:** "go through the Roadmap once and then we will discuss."

### What's currently in the roadmap

- **P6 — Downloads hub:** unified queue (queued/active/done/failed),
  sidebar Downloads UI, folder-watch import, hooks for AO3/comics jobs.
- **P7 — Fiction sources (AO3 first):** `FictionSource` trait, AO3
  search/detail/download EPUB into library, `source` + `remote_id`, manual
  "Check updates", rate limits / clear errors, open in text reader.
  Out: piracy sources.
- **P8 — Comics local + Moku-style reader:** import CBZ/CBR into same
  catalog, black immersive stage, top bar (close/title/page/zoom), bottom
  scrubber, page LTR/RTL + webtoon long-strip, fit width/height, tap-center
  toggle chrome, memory-safe decode (viewport ± neighbors only), progress per
  book, Read routes by format. Out: remote catalogues.
- **P9 — Comics sources:** source framework, self-hosted/legitimate backends
  first (OPDS, Komga, Kavita, own archive), Downloads hub integration,
  Suwayomi-like module depth only as needed.
- **P10 — PDF (text-reader family):** MuPDF (or Poppler) in the text-reader
  chrome family (not comics shell), continuous or page mode, basic
  highlight/underline stored like EPUB annotations, same library entry model.
- **P11 — Tools:** convert via external `ebook-convert`/`pandoc` if present,
  EPUB polish, batch metadata/cover refresh, optional Calibre `metadata.db`
  one-shot import.

### Non-goals currently locked

- ~~Z-Library / unauthorized shadow libraries~~ (still a hard non-goal — this
  is legal/safety, not a phase)
- Calibre multi-app suite, content server, fetch news (still non-goal)
- **~~Plugin API~~** — **being reversed, see section 5**
- Windows / macOS (still non-goal)

### What's worth a closer look (open discussion points)

1. **The "two readers" lock.** The roadmap locks a separate WebKit text
   reader and a separate image-based comics reader. That's fine, but the
   architecture work (service layer, task manager) applies to both — worth
   deciding whether the custom renderer conversation changes the comics
   reader plan too (decode-only is simpler than a text engine).
2. **P7 dependency on P6.** "Downloads hub is a prerequisite for P7" — fair,
   but the folder-watch import could ship standalone; the queue is the
   dependency, not the watch.
3. **P10 PDF via MuPDF/Poppler** — this is a *third* rendering engine. With
   the renderer conversation pending, PDF is the one place a purpose-built
   renderer (not WebKit) is already the plan.
4. **P11 Tools scope is healthy** — external tools if present, no bundling.
   Keep it that way.
5. **The performance/architecture track is not yet in the roadmap as a
   phase.** It's discussed above but not scheduled. Needs a home — probably
   an "architecture track" alongside P6–P11.
6. **"Check updates" for fanfic (P7)** — the roadmap says manual only. With a
   task manager, scheduled updates become cheap. Worth revisiting.
7. **Annotations in comics (P8) and PDF (P10)** — roadmap says "basic marks"
   for PDF, nothing for comics. Worth deciding scope early so the service
   layer supports it.

---

## 5. Plugin system — decision reversed

**Status: 🔶 under discussion → leaning Lua, design TBD.**

The roadmap listed **Plugin API** as a non-goal. The user has **changed their
mind**: plugins are wanted — "it will help in the future phases" — and the
user defers to deeper discussion before final design.

### Initial rejection (context)

Yazi's Lua plugins work because a file manager's verbs are tiny ("copy",
"rename", "filter"). An ebook reader's verbs are huge ("render this chapter",
"annotate this selection"). Plugin systems are also a massive compatibility
surface — they become a second API you must keep stable forever.

### Why we're now considering it

- Future phases (P6–P11) are source adapters, import/export, and tools —
  exactly the kind of small-verb surface that plugins suit well (a
  `FictionSource` plugin, a metadata-source plugin, an export plugin).
- The architecture track (service layer) is the natural plugin host: if all
  I/O goes through a service, a plugin boundary is a thin seam, not a
  rewrite.
- A clean core is the prerequisite for plugins — plugins become *possible*
  later without being built now.

### Open questions (to discuss)

1. **Lua vs alternatives?**
   - **Lua (mlua)** — the default choice; tiny, embeddable, used by Yazi,
     Neovim, AwesomeWM. Good for config + scripting. Risk: a DSL-shaped
     surface that grows.
   - **WebAssembly (wasmtime)** — sandboxed, language-agnostic (plugins in
     Rust/C/Go), fast, but more tooling complexity and a steeper authoring
     curve.
   - **Rust dyn traits compiled in** — simplest, no runtime, but plugins
     must be compiled with the app (no user-authored scripts).
   - **A tiny JSON/config "plugin" surface first** — not a script runtime at
     all; define the *seams* (source adapters, export formats) as stable
     APIs now, add scripting later. **This is the author's lean** — the
     service layer IS the plugin API; scripts can come later.
2. **Scope:** config scripts? source adapters? export/import formats? UI
   extensions? All of the above?
3. **When:** after the architecture track (service layer + task manager),
   not before. Plugins are a consumer of clean seams, not a way to create
   them.

---

## 6. Standing decisions & open threads

- **Language:** stay Rust. No rewrite in another language. (✅ locked)
- **Stack:** Rust + GTK4 + Relm4 + SQLite + WebKitGTK (EPUB) + image pipeline
  (comics) + MuPDF later (PDF). (✅ locked)
- **Architecture:** Yazi-style service layer + task manager + preloaders +
  thin UI. (✅ accepted, not yet implemented)
- **Custom text renderer:** 🔶 under discussion → **endgame: custom renderer
  for ALL reflowable text** (cosmic-text; fiction first, EPUB after
  normalization); WebKit = fallback only, may be cut (see §8).
- **Plugin system:** 🔶 under discussion, user wants it, leaning Lua —
  confirmed as the **source-adapter engine** for fiction + manga (see §7).
- **Scope:** confirmed as a **content platform** — fiction sources
  (AO3/FFN/webnovel, tag search, downloads, auto-updates) + manga sources
  (Suwayomi-class) + plugins + fast architecture (see §7).
- **Perf work order:** measure → thumbnails/async decode → virtualize if
  numbers say so → reader. (✅ accepted)
- **Docs discipline:** keep README.md and ROADMAP.md updated as work
  progresses; this file is the design-conversation record. (✅ standing)

---

## 7. The content-platform scope + renderer decision

**User (2026-09-02):** "Do you understand the scope now?" — restated:
Kalam is not just a local reader; it is a **content platform**: fiction
sources (AO3, FanFiction.net, Webnovel, Royal Road, …) with rich tag
search, download, offline reading, and **automatic updates** as new
chapters release — FanFiction.net-app-class features — plus **manga
sources** with a Suwayomi-class browse/read experience. Plugins power the
sources.

### Accepted: the scope

- **Fiction:** search (tags, fandom, characters, ships, rating, status) →
  results → read or download → follow → auto-update + notify. Sources
  mostly have no public APIs (AO3 especially) — source plugins parse the
  site and return structured data (Tachiyomi pattern).
- **Manga:** same shape, but content is images — the reader is an image
  pager (P8), not a text engine.
- **Plugins** are confirmed as the source-adapter engine (P12, Lua
  leaning, built on the A0 service layer).

### ~~Rewrite the Suwayomi server in Rust~~ — rejected

Suwayomi = Tachiyomi's engine as a server. Its value is the **Kotlin
extension ecosystem**, which cannot run in Rust. We do not need the
server — Kalam already has (or will have) the DB, download queue (P6),
task manager (A0), and readers. What we need is the **adapter concept**:
a `Source` plugin API (search / popular / chapter list / fetch content).
Porting an existing extension's *logic* is hours (they are simple
scrapers); running its Kotlin is impossible. Optional later: a "Suwayomi
server" adapter that talks to a user's running instance via its API — the
cheapest bridge to the whole ecosystem.

### WebKit vs custom renderer — the framework (under discussion)

**What WebKit is:** a full web-browser engine (Safari's). Reading a
chapter today = running a web page: HTML parsing, CSS layout, JS, fonts,
accessibility tree. Power: displays ANY web content perfectly. Cost:
300–600 ms cold start, 100–200 MB RAM, bridge tax (Rust ↔ JS ↔ DOM) for
every feature (dict popup, highlights), and black-box internals we can
only poke with CSS/JS.

**What a custom renderer is:** we draw the text ourselves (like a PDF
reader / KOReader / e-ink reader). We define a clean content format
(paragraphs, headings, images), lay it out with a Rust text engine
(cosmic-text / skia), paginate, and paint. Cost: 1–3 person-years for a
good engine (line breaking, hyphenation, justification, RTL, CJK, font
fallback, selection, **accessibility**). Gain: page turns in ~1 ms, tens
of MB, and every feature (dict lookup, themes, annotations) is native GTK
beside the text — no bridge.

**The unlock:** we control what reaches the renderer. Sources fetch →
**sanitize → convert to clean chapters** (FanFicFare's whole job; AO3
even ships official EPUBs). Manga is images only. So the renderer never
has to be a browser — it only ever sees clean content. Industry proof:
Tachiyomi reads images, FanFicFare converts to EPUB, KOReader renders its
own format.

**Leaning — hybrid, resolved by §8 (no browse mode):**
- Custom renderer for **reading** fiction (clean format) and comics
  (image pager) — the fast, light, integrated reader.
- WebKit stays as the **EPUB engine** (and fallback for exotic EPUBs we
  haven't normalized) — created lazily when an EPUB opens.
- ~~Browse mode~~ — rejected: Kalam never renders arbitrary websites;
  sources return structured data via plugins.
- Sequencing: sources + architecture first (the product), the custom
  renderer as the A0 crown afterward.

### Honest caveats

- **Auto-updates vs annotations:** re-downloading a fic can shift its
  spine; existing annotation anchors may reset (already in the risk
  register). Best-effort.
- **Polling etiquette:** per-source rate limits and user-controlled
  schedules (daily, not per-minute).
- **A11y:** a custom text engine must expose text to screen readers
  (AT-SPI); WebKit gives this free. Budget for it.

---

## 8. Scope sharpened: no browse mode + the Tachiyomi/Suwayomi picture + borrow list

**User (2026-09-02):** "I am not making a web browser right? I just want it
for Epub, and then AO3, fanfiction, etc."

### Accepted: no web-browse mode

- Kalam never renders arbitrary websites. Sources return **structured data**
  (title, author, tags, chapters) via plugins; the search UI is native.
- This **resolves the renderer fork**: WebKit's only remaining job is
  **EPUB rendering** (EPUBs are HTML/CSS internally — the one place web
  content is unavoidable today).

### Renderer decision, sharpened (user pushed back — WebKit days numbered)

**User (2026-09-02):** "WebKit stays as the EPUB engine only — isn't that
the same as now? shouldn't we shift to something better? crengine (C++) or
cosmic-text (Rust)?"

**Conceded: yes, EPUB-on-WebKit is the same cost today.** Lazy creation +
fiction moving off WebKit only helps startup/new-content; EPUB reading
itself stays on WebKit until replaced.

**Sharpened position (accepted):**

- **Custom renderer is the endgame for ALL reflowable text** (fiction AND
  EPUB) — not just fiction. WebKit is a temporary fallback, not a
  foundation, and may be cut entirely later.
- **cosmic-text is NOT an EPUB renderer** — it's a Rust text-layout
  library (System76, MIT). We build parsing/normalization/pagination/
  painting around it. **crengine** is a complete C++ EPUB engine (GPL) —
  a ready-made meal via FFI, but less control over dict-popup/theme/
  annotation integration, and C++ in the stack.
- **Lean: cosmic-text.** The hard part either way is EPUB→clean-content
  normalization (lol_html + rules); that work is ours regardless. Fight
  our own code, not a foreign engine's API. crengine stays a legitimate
  shortcut if we want EPUB rendering before the custom engine matures.
- **WebKit's only honest role: fallback for exotic EPUBs** the normalizer
  can't handle (compatibility mode). Optional — could be cut if we accept
  imperfect rendering of rare weird EPUBs.

**Sequencing (accepted):**

1. Custom renderer v1 on cosmic-text for **fiction first** — content is
   ours (clean plugin output), bounded, proves the engine.
2. **EPUB normalization pipeline** (lol_html + rules) → same renderer.
   WebKit drops to fallback.
3. PDF = MuPDF (P10); comics = image pager (P8) — never in this question.

### Manga architecture confirmed (Tachiyomi shape, Lua plugins, no Kotlin)

**User (2026-09-02):** "We could have a similar architecture… plugins which
we will write. Also, no need for a bridge with the Kotlin extensions —
those are apk… too much work, maybe even impossible."

**Agreed, fully:**

- Manga reader uses the same `Source` adapter shape: `search / popular /
  chapter list / pages`. Plugins are **Lua, written by us** — one per site.
- **No Kotlin extension bridge — confirmed.** Tachiyomi extensions are
  Android APKs calling Android APIs; running them needs an Android runtime
  or JVM emulation — fundamentally wrong shape for a desktop app. We lose
  nothing: MangaDex has an official API (zero scraping), Komga/Kavita/OPDS
  are clean REST, and scraping logic ports from the Apache-2.0 extensions
  (license-compatible).
- **Fiction uses the same architecture** — one plugin system, text flavor
  + image flavor.

### Engine map (final)

| Content | Engine | Notes |
|---|---|---|
| Source fiction | Custom renderer (cosmic-text) | Clean content we define — build first |
| EPUB | Custom renderer + normalizer; WebKit fallback | Normalization is the hard part |
| PDF | **MuPDF** (P10) | Fixed-layout; never WebKit; AGPL (or Poppler/GPL) |
| Comics | Image decode (gdk-pixbuf) + GTK pager | No engine at all — decode and paint; already P8 |

### The complete Tachiyomi / Suwayomi picture

- **Tachiyomi** = Android manga reader. App knows ONE `Source` interface
  (search / popular / details / chapter list / page list). **Extensions**
  = small Kotlin APKs, 200–500 lines each, that implement that interface
  for one site (MangaDex = official JSON API; Komga = REST; random site =
  HTML scrape with Jsoup). Hundreds of sites, one tiny adapter surface.
- **Suwayomi** = Tachiyomi's engine extracted into a JVM **server**; loads
  the same Kotlin extensions, exposes REST + GraphQL; clients are thin.
- **The Kotlin obstacle:** Kotlin compiles to JVM bytecode; a Rust app
  cannot execute it (no JVM inside). The *concepts* are trivial; only the
  runtime is incompatible.
- **Three workarounds:** (1) ship a JVM subprocess — heavy, rejected;
  (2) bridge to a user's existing Suwayomi instance via its API — cheap
  optional plugin later; (3) **reimplement the adapter pattern natively**
  — recommended. The scraping logic is "fetch URL, parse HTML/JSON, return
  fields" — hours per source in Lua, and MangaDex/Komga/Kavita/OPDS need
  **no scraping at all** (official APIs).
- **Fiction side** needs no server concept: same plugin shape, plus
  FanFicFare (below).

### ~~Rewrite the Suwayomi server in Rust~~ — rejected (confirmed)

Nothing to rewrite: we want the adapter *concept*, not the server.
Kalam already has (or will have) the DB, downloads (P6), task manager
(A0), and readers.

### PDF — NOT forgotten (P10)

PDFs are fixed-layout; they were never a WebKit question. P10 uses
**MuPDF** (purpose-built renderer, already on the custom side of the
fence). License note: MuPDF is **AGPL-3.0**; combining with our GPL-3.0
app pulls the app to AGPL (fine for personal open source — KOReader does
it; Poppler/GPL is the alternative if we ever want to avoid AGPL).

### Open-source borrow list (repo is GPL-3.0-or-later — compatible)

| Project | License | What to take |
|---|---|---|
| **FanFicFare** | GPL-3, Python | **P7 already built**: site adapters for AO3, FFN, Royal Road, ScribbleHub, SpaceBattles, Wattpad… port adapter logic to Lua plugins, or shell out to its CLI as a "FanFicFare source" plugin |
| **Tachiyomi extensions** | Apache-2.0 | The adapter pattern + per-site logic to port to Lua |
| **MangaDex API** | public API | First P9 source — zero scraping |
| **Komga / Kavita** | GPL | Self-hosted manga servers; Kalam as a client via their REST APIs |
| **Suwayomi** | MPL-2.0 | Mirror its extension-API shape |
| **KOReader** | AGPL-3.0 | Proof of no-browser reading + Lua plugin architecture to study |
| **crengine** | GPL-family | Ready-made EPUB/HTML rendering engine (Path B) — bind via FFI |
| **cosmic-text** | MIT | Rust text layout/shaping (Path B, if we build our own) |
| **swash / fontdb / ab_glyph** | MIT/Apache | Rust font loading/shaping |
| **vello / skia-safe** | Apache/MIT | 2D/GPU painting for custom engine |
| **lol_html** | Apache/MIT (Cloudflare) | HTML parsing/sanitizing for scrapers + EPUB normalization |
| **ammonia** | MIT | HTML sanitizer for plugin output |
| **Yazi** | MIT | Task system + preloader architecture (already discussed) |
| **Foliate** | GPL | GTK+WebKit reference; CSS pagination tricks |

---

*Last updated: 2026-09-02.*

---

## 9. Renderer effort estimate (2026-09-02)

**User asked:** "for the custom renderer, you have to do all the work. how
much time do you reckon it will take you?"

**Honest answer:**

- Writing code is the fast part; the constraint is that the agent has **no
  display** — the renderer is visual, so the **user is the QA loop** (Arch
  machine GUI smoke tests). The user's testing cadence sets the schedule.
- Layout/pagination/hit-testing are pure functions → unit-testable headless
  (agent-strength); visual polish needs user eyes (agent-weak).

| Milestone | Scope | Estimate (sessions) |
|---|---|---|
| 1. Vertical slice | one clean fiction chapter: content format → cosmic-text → pagination → painted GTK pages → page turn → themes → dict popup hook → position save/restore | **2–4 weeks** |
| 2. Fiction reader complete | all plugin-produced formats, images, footnotes, 100k+ word chapters, selection/copy, perf at scale | +1–2 months |
| 3. EPUB normalization | EPUB HTML/CSS → clean format (lol_html + rules); WebKit fallback when normalization fails; position mapping | +2–4 months |
| 4. Hardening | RTL, CJK, hyphenation, justification, a11y (AT-SPI), incremental layout, edge cases | +2–4 months, ongoing |
| **Total** | "WebKit gone for reflowable text" | **6–12 months** |

- Benchmark: a strong human dev solo ≈ same (6–14 months) — renderer is a
  1–3 person-year thing for anyone.
- **Faster:** user tests in small batches (15 min per slice). **Slower:**
  big-bang testing → visual bugs in bulk, blind debugging.
- **Key risk:** custom engine choking on one weird EPUB feels like a
  regression → WebKit stays as fallback until milestone 3 earns trust.
- **Milestone 1 is the calibration test:** if the slice takes > ~4 weeks,
  we learn the renderer is harder than estimated before sinking months in.

---

*Last updated: 2026-09-02.*

---

## 10. When to build the renderer + the crengine deep-dive (2026-09-02)

**User asked:** when should we do the renderer? and can we just use crengine?

### When: not now, not at the end — start right after sources, grow alongside

- **Not now:** the renderer is a *consumer* — nothing to render until
  sources (P7/P9) and architecture (A0) exist. Building now = building in a
  vacuum.
- **Not at the end:** the renderer is the *riskiest, most uncertain* piece
  (1–3 person-year unknown). Leaving the highest-risk, highest-value work
  to last bets the whole timeline on it landing on schedule.
- **Right after sources (recommended):** sources give it real content, the
  architecture gives it a home, and the fiction-first slice is a bounded
  on-ramp that proves the whole plan is buildable.
- **Order:** A0 architecture → P7 fiction sources → renderer vertical slice
  (alongside) → EPUB normalization → PDF (P10) / comics (P8).

### crengine deep-dive — why "just use it" doesn't work

1. **License mismatch (hard blocker).** crengine is **GPL-2.0**; KOReader's
   fork is **AGPL-3.0**. Our repo is **GPL-3.0-or-later**. GPL-2 and GPL-3
   aren't cleanly compatible; you can't just drop crengine into a GPL-3
   project without relicensing ours or keeping crengine as a separate,
   dynamically-linked component.
2. **C++ codebase, no Rust bindings.** 5,500+ commits, 2 decades old, full
   DOM/XML/CSS engine. Writing + maintaining a C FFI wrapper is a big
   ongoing task. Built for e-ink/embedded (Qt/wxWidgets/XCB) — no GTK
   frontend; KOReader integration is deeply Lua-based.
3. **Partial CSS — exactly the risk we worried about.** crengine's CSS is a
   *subset of CSS 2.1, not CSS 3*. `float`, `clear`, `border`,
   `border-width`, `font-variant`, `text-transform`, `border-collapse` are
   missing/partial — precisely what real publisher EPUBs use. KOReader
   compensates with a huge curated `epub.css` + style-tweaks; its own devs
   admit it "adds strange things when playing with publishers' CSS."
4. **Dict/annotation integration = same fight.** Exposing "word at this
   pixel" / "anchor at this position" through FFI is the same integration
   work as building our own layout — against an API we don't control.
5. **Verdict:** crengine = "EPUB reading this year" shortcut, ONLY as a
   separate dynamically-linked fallback component with a clean interface,
   swappable when our custom engine matures. **Recommendation stands:
   cosmic-text + our own normalizer.** The hard part (messy HTML/CSS →
   clean content) is ours either way; fight our own code, not a foreign
   engine's API with license baggage.

---

*Last updated: 2026-09-02.*
