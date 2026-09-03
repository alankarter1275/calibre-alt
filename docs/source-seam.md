# The source seam — A0 step 8

**Status:** design, not code. Nothing here is implemented yet.
**Why it exists:** P7 (fiction sources) and P9 (manga sources) both need this
interface. Designing it once, before either is built, is the whole point of the
step — two phases inventing their own shape would mean rewriting one of them.

**Scope note (2026-09-03):** this is a **single-user app, not an ecosystem**
(§0). Sources are implemented as compiled-in Rust; the Lua runtime is
**deferred, probably indefinitely** (§9a). The trait is designed so Lua can be
added later without changing it, which is why deferring costs nothing.

**Acceptance test for this document** (from the A0 acceptance criteria): *a
fresh chat can add a source plugin from the documented API alone.* If you are
that fresh chat and something below is ambiguous, that is a bug in this file.

---

## 0. Scale: personal use, no ecosystem (decided 2026-09-03)

> **The short answer, because it is the question people ask first:**
> **No, extension surfaces are NOT done through Lua.** There is no scripting
> runtime and none is planned. A new source, metadata provider, export format
> or theme is a **Rust file compiled into the app**, exactly like the two
> metadata providers that ship today. §9a has the full reasoning; §12a lists
> the surfaces.

The user settled the audience question:

> *"sources and metadata and maybe a few more, not an ecosystem, because it's
> for personal use. plugin system makes sense if there is a community, which
> isn't the case here."*

Two consequences, and the second one contradicts what §9–§12 below assume.

**Surfaces are a short, deliberate list.** Sources (P7/P9) and metadata
providers, plus possibly export formats, dictionaries and themes later. Not UI
extension, not reader/renderer hooks, not anything that writes to the library.
Each is added when it has a real consumer, never speculatively.

**The case for Lua is now much weaker — see §9a.** A scripting runtime exists
so that people who cannot compile the app can extend it. If the only authors
are the user and the agent, and both can compile, the runtime is cost without
its benefit.

---

## 1. What a "source" is

A source is a place books come from that is not the user's disk: Archive of Our
Own, FanFiction.net, Royal Road, MangaDex, a Komga server.

Every one of them answers the same four questions:

1. **Search** — "what do you have matching this?"
2. **Detail** — "tell me more about this one."
3. **Chapters** — "what parts does it have?"
4. **Content** — "give me part 3."

That list is not invented here. It is the shape Tachiyomi settled on after
years and hundreds of extensions (`fetchSearchManga`, `fetchMangaDetails`,
`fetchChapterList`, `fetchPageList`), and the shape this repo already uses for
metadata providers in `src/metadata/mod.rs`. Two independent arrivals at the
same four verbs is the strongest evidence available that they are the right
four.

---

## 2. The decision that shapes everything else

**One trait, not two.**

The roadmap currently names `FictionSource` (P7) and `MangaSource` (P9) as
separate traits. They should be one.

Fiction and manga differ in exactly one place — the last step. Fiction returns
text; manga returns image URLs. Everything else is identical: searching,
paginating, listing chapters, rate limits, the download queue, the follow
scheduler, error handling, the plugin loader, the enable/disable preference.

Two traits means writing all of that twice and having it drift. One trait with
a two-variant content type means writing it once:

```rust
pub enum Content {
    /// Fiction: sanitized HTML for one chapter.
    Text(String),
    /// Manga: the images that make up one chapter, in order.
    Images(Vec<ImageRef>),
}
```

A source declares which flavour it is up front (`ContentKind`), so the UI knows
whether to open the EPUB-style reader or the comics pager before it fetches
anything.

**Cost of being wrong:** if a third flavour appears (audio, say), the enum gains
a variant and every `match` on it fails to compile until handled — which is the
good failure. Two separate traits would silently grow a third copy of the queue
and the scheduler.

---

## 3. The types

Deliberately plain owned data. No `Rc`, no borrowed lifetimes, no GTK. Same
rule as `LibraryService`'s snapshots (`src/service.rs`) and for the same reason:
these have to cross a thread boundary.

```rust
/// Which source. A string, not an enum, because plugins are an open set.
///
/// `src/metadata/mod.rs` has `enum SourceId { OpenLibrary, GoogleBooks }`,
/// which is right for two compiled-in providers and wrong the moment a user
/// can drop in a script. Stable, lowercase, no spaces: "ao3", "mangadex".
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SourceId(pub String);

pub enum ContentKind { Text, Images }

pub struct SourceInfo {
    pub id: SourceId,
    pub name: String,        // "Archive of Our Own"
    pub lang: String,        // BCP-47: "en"
    pub kind: ContentKind,
    pub base_url: String,
    /// Bumped when a source's parsing changes incompatibly.
    pub version: u32,
}

/// One page of results. `has_more` is not optional — see §4.
pub struct ResultPage {
    pub items: Vec<WorkRef>,
    pub has_more: bool,
}

/// A work as it appears in a result list.
///
/// Everything a search-result card draws must be in here. See §5.
pub struct WorkRef {
    /// Stable, source-scoped. See §6 — this one is load-bearing.
    pub remote_id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub cover_url: Option<String>,
    /// Short blurb for the card. Full text lives in `WorkDetail`.
    pub summary: String,
    pub tags: Vec<String>,
    pub status: WorkStatus,
    pub chapter_count: Option<u32>,
    /// Unix seconds. Drives the auto-updater's "is there anything new?".
    pub updated_at: Option<i64>,
}

pub enum WorkStatus { Ongoing, Complete, Hiatus, Abandoned, Unknown }

pub struct WorkDetail {
    pub work: WorkRef,
    pub description: String,
    pub series: Option<String>,
    pub language: Option<String>,
}

pub struct ChapterRef {
    pub remote_id: String,
    pub title: String,
    /// Float, not integer: manga really does have chapter 12.5.
    pub number: Option<f32>,
    pub published_at: Option<i64>,
    /// Manga only — two groups may translate the same chapter.
    pub scanlator: Option<String>,
}

pub struct ImageRef {
    pub url: String,
    /// Some sites 403 an image request without the page it came from.
    pub referer: Option<String>,
}
```

### Errors

Reuse the shape already proven in `src/metadata/mod.rs`, plus two:

```rust
pub enum SourceError {
    Network(String),
    Parse(String),
    /// Rate limited, quota exhausted, or blocked. Distinct from Network
    /// because the response is "wait", not "retry now" — the scheduler in §7
    /// needs to tell those apart.
    Limited(String),
    /// The user closed the window mid-fetch. Not a failure; show nothing.
    Cancelled,
    /// This source does not offer that (e.g. no search).
    Unsupported,
}
```

`Cancelled` being an error variant rather than a silent empty result is
deliberate: a caller that forgets to handle it shows an empty list where the
user expects results, which reads as "this source is broken".

---

## 4. Pagination is not optional

`ResultPage.has_more` must exist from day one.

Without it a search returns the first 20 hits and the user can never reach the
21st. Adding pagination later changes the return type of the single most-used
method, so every source and every call site is rewritten.

This is the cheapest thing in the whole document to get right now and one of
the most expensive to retrofit. Tachiyomi carries the same field
(`MangasPage.hasNextPage`) for the same reason.

---

## 5. The search result must be complete enough to draw

`WorkRef` carries title, authors, cover, tags, status and chapter count — not
just an id and a title.

This is not padding. Tachiyomi's own contributor documentation warns:

> *"You should set `thumbnail_url` if it is available, if not,
> `fetchMangaDetails` will be **immediately** called (this will increase
> network calls heavily and should be avoided)."*

That is an N+1 query, over the network, one HTTP request per row. It is the
same bug this repo has now fixed three times in the database layer — the saved
quotes page issuing ~1,001 queries per keystroke, the series float, and the
thumbnail backfill reading 21 columns to use 2 (pitfalls §16, §18).

**Rule:** if the search-results card draws it, `WorkRef` carries it. A source
that genuinely cannot supply a field leaves it empty rather than making the UI
fetch it per row.

---

## 6. `remote_id` is the load-bearing field

Every downloaded book records which source it came from and its id there
(`source` + `remote_id`, already planned in the roadmap's schema section).

**That id must mean the same thing next month.** Two features depend on it:

- **The auto-updater** re-fetches a followed work by `remote_id`. If the id
  drifts, the updater either loses the work or downloads a duplicate.
- **Annotations** anchor to positions in a downloaded file. Re-downloading a
  fic that gained a chapter must not move the existing chapters, or every
  highlight in the book breaks. (The roadmap already flags this as a known
  risk under P7.)

So: `remote_id` is the site's own permanent identifier — AO3's work number,
MangaDex's UUID. **Never a URL** (sites reorganise paths) and **never a title
slug** (authors rename works).

Chapters need the same treatment: `ChapterRef.remote_id`, not "chapter 3".
Chapter *numbers* are not stable — authors insert a chapter, add an author's
note as a numbered entry, or renumber a whole fic.

---

## 7. Rate limiting belongs to the source

Today there is exactly one rate limit in the codebase: a hardcoded
`thread::sleep(700ms)` inside `src/metadata/google_books.rs`. Fine for one
provider. Wrong the moment a background scheduler is polling twelve sources.

Each source declares its own:

```rust
pub struct RateLimit {
    /// Minimum gap between requests to this source.
    pub min_interval: Duration,
    /// Requests in flight at once. Usually 1.
    pub concurrency: u8,
}
```

The **host** enforces it, not the plugin. A plugin cannot be trusted to sleep —
it might be user-written, and a buggy one that hammers AO3 gets Kalam's
User-Agent blocked for everybody. Politeness has to live where it cannot be
bypassed.

---

## 8. Cancellation and progress come free

Every network method takes a `&Reporter`, the type already used by
`src/tasks.rs`:

```rust
fn chapters(&self, work: &str, r: &Reporter) -> Result<Vec<ChapterRef>, SourceError>;
```

A 2,000-chapter webnovel's chapter list is a long job. `Reporter` already gives
cooperative cancellation and progress updates, and `tasks::spawn` already
routes results back to the UI thread. Nothing new is needed — the seam exists
and is proven by the thumbnail backfill and the dictionary import.

---

## 9. The Lua problem, and how it resolves

**Read §9a first — it argues Lua should not be adopted at all.** This section
is kept because the `SourceFactory` / `Source` split it produced is worth
having regardless, and because if Lua is ever revisited, this is the research
that stops someone re-deriving it.

This was the finding that most affected the original plan.

**`mlua` is `!Send` by default.** The Lua VM holds a raw pointer
(`*mut lua_State`) and cannot move between threads. There is a `send` feature
that makes it `Send + Sync`, but it works by wrapping every VM access in a
reentrant mutex and imposes `Send + Sync` on all userdata — a cost paid on
every call, forever, to solve a problem we do not actually have.

Meanwhile `tasks::spawn` requires `W: Send + 'static`. So a naive
`Box<dyn Source>` holding a Lua VM **cannot be handed to a worker thread.**

The resolution is to move the *recipe* across the thread boundary, not the VM:

```rust
/// Send-able description of a source. Cheap, holds no VM.
pub trait SourceFactory: Send + Sync {
    fn info(&self) -> SourceInfo;
    /// Called ON the worker thread. A Lua-backed factory creates its VM here.
    fn build(&self) -> Result<Box<dyn Source>, SourceError>;
}

/// The live source. Deliberately NOT Send — it never crosses a thread.
pub trait Source {
    fn info(&self) -> SourceInfo;
    fn rate_limit(&self) -> RateLimit;
    fn filters(&self) -> Vec<Filter>;

    fn search(&self, q: &Query, page: u32, r: &Reporter)
        -> Result<ResultPage, SourceError>;
    fn detail(&self, work: &str, r: &Reporter)
        -> Result<WorkDetail, SourceError>;
    fn chapters(&self, work: &str, r: &Reporter)
        -> Result<Vec<ChapterRef>, SourceError>;
    fn content(&self, work: &str, chapter: &str, r: &Reporter)
        -> Result<Content, SourceError>;
}
```

The factory is a path to a script plus a parsed manifest — trivially `Send`.
The worker calls `build()`, gets a VM that was born on that thread and dies on
it, and no mutex is ever taken.

**Consequence: the `send` feature is not needed, and should not be enabled.**
Recorded here because "just turn on the feature" is the obvious wrong answer
and someone will propose it.

---

## 9a. Does Lua earn its place at all? — **NO (decided 2026-09-03)**

§9 solves *how* to host Lua. It never asked *whether* to — and that was the
mistake. The audience answer in §0 settles it: **no scripting runtime.**

**What a scripting runtime buys you:** people who cannot or will not compile
the app can still extend it. That is the entire value proposition. It is why
Yazi, Neovim and Tachiyomi all have one — thousands of users, a handful of
maintainers, and no way for a user to ship a patch quickly.

**Kalam's situation is the opposite.** Two authors, both of whom compile the
app routinely. A new source is a `.rs` file either of us can write, and CI
already type-checks it, lints it under `-D warnings`, and runs its tests.

**What Lua costs, concretely, in this repo:**

| Cost | Detail |
| --- | --- |
| A second language | Every source exists in Lua; debugging means reading Lua *and* the Rust host |
| No type checking | A typo in a Lua field name is a runtime error at 2 a.m., not a compile error in CI |
| Sandbox surface | §10's table has to be *enforced*, and every hole is a security bug |
| A frozen API | The moment a plugin exists, the host API cannot change freely — the exact "second API you must keep stable forever" objection in `conversation.md` §5 |
| Build weight | `mlua` vendors and compiles a C Lua interpreter into every build |
| The `!Send` dance | §9's whole factory/VM split exists *only* to accommodate Lua |

**What a compiled-in Rust source costs:** a `.rs` file and a line in a match
statement. That is genuinely it — `src/metadata/mod.rs` shows the pattern
working today with two providers.

**The counter-argument, stated fairly.** Lua lets a source be fixed without a
rebuild. When AO3 changes its HTML — and it will — a Lua fix is edit-and-
restart, while a Rust fix is edit-and-recompile. On the user's machine that is
a few minutes of `cargo build --release`, not a blocker, but it is a real
difference and it is the strongest argument on the Lua side. It also matters
more for scraped sites (AO3, FFN) than for API-backed ones (MangaDex), because
APIs version and HTML does not.

**Decision: no Lua.** Not "never" — the seam in §9 is designed so
Lua can be added later without changing `Source` at all. A `LuaSource` is just
another implementation of the same trait, and `SourceFactory` already exists to
carry a non-`Send` VM to a worker thread. The decision can be deferred at
almost zero cost, which is the strongest reason to defer it.

**What would change the answer:** a source that breaks often enough that
recompiling becomes annoying, or a second person wanting to add sources. Either
one, and Lua earns its place. Until then it is a runtime, a sandbox and a
frozen API bought for an audience of two people who can both compile.

---

## 10. What a plugin is allowed to touch

These rules apply to any *scripted* plugin, if scripting ever lands (§9a says
probably not). They are recorded because they also describe the discipline a
compiled-in source should follow voluntarily — a Rust source that reaches into
the catalog is just as wrong, it simply cannot be stopped by a sandbox.

The host would give a plugin exactly four things:

| Given | Not given |
| --- | --- |
| `http.get(url, headers)` — through the host's agent, rate-limited, with Kalam's User-Agent | Raw sockets, or any way to bypass the rate limiter |
| `html.select(doc, css)` — parse and query | The filesystem |
| `json.decode(text)` | The catalog / database |
| `log.info(msg)` | Spawning processes, loading native code |

A plugin is a **pure function from a query to structured data**. It never
writes to the library — the host takes what it returns and decides what to
store. That keeps the blast radius of a bad plugin to "wrong search results",
never "corrupted library".

This mirrors the rule that already holds for `LibraryService`: the service
never calls `notify` because it must stay worker-callable. Same discipline, one
layer out.

---

## 11. Build order

**Do not write the Lua host first.** A plugin API designed with zero
implementations is a guess. Two working native sources will shake the shape
out; then the Lua host becomes a mechanical translation of a known-good
interface.

1. **The trait and types** (this document), landing with AO3 so nothing is dead
   code — see §12.
2. **AO3 as a native Rust source.** P7's actual first target. Proves the text
   flavour, HTML parsing, and pagination.
3. **MangaDex as a native Rust source.** Official API, no scraping, so it tests
   the image flavour without also testing a scraper.
4. **Lua host — only if §9a's conditions are met**, with AO3 ported as the
   proof. If the ported plugin behaves identically to the native one, the API
   is right. Currently **not planned**: see §9a. Steps 1–3 are worth doing
   regardless, and none of them depends on this.

This matches the lean already recorded in `docs/conversation.md` §5: *"define
the seams as stable APIs now, add scripting later."* — with §9a noting that
"later" may reasonably be "never" for a single-user app.

---

## 12. Why this is a document and not code yet

Kalam is a **binary crate with no `src/lib.rs`**. Anything unused fails CI
under `-D warnings`. There are already 28 `#[allow(dead_code)]` escapes in the
tree.

A `Source` trait with no implementation and no caller is unused code. Landing
it now means either failing CI or adding another pile of `allow(dead_code)` —
annotations that then sit there indefinitely and stop flagging genuinely dead
code.

So the trait lands **in the same commit as AO3**, its first implementation and
first caller. That is P7's opening move, and this document is what makes it a
half-day of typing instead of a week of design.

---

## 12a. The surface list (what may ever become extensible)

From §0: a short list, each added only when it has a real consumer. Every one
shares the property that makes it safe — **a pure function from data to data**,
touching no widgets, no catalog, no reader state.

| Surface | Status | Existing seam | Notes |
| --- | --- | --- | --- |
| **Content sources** | designing (this doc) | none yet | P7/P9 depend on it. AO3 → MangaDex → more |
| **Metadata providers** | **already exists** | `MetadataSource` in `src/metadata/mod.rs` | Open Library + Google Books ship today. Adding one is a `.rs` file and a match arm |
| **Export formats** | possible later | `export_quotes_markdown` in `src/pages/saved_quotes.rs` | Already a pure `&[Quote] -> String`. Obsidian/Anki/Notion would be siblings |
| **Dictionaries** | possible later | `Catalog::search_dict` / `lookup_entry` | Already clean. A Wiktionary or JMdict provider fits |
| **Themes** | **already exists, no code needed** | `Theme` struct in `src/theme.rs` | One struct of colour literals; 13 themes ship. Arguably a plugin surface with zero runtime |
| UI extension | **out** | — | GTK widget tree; exposing it freezes every layout decision |
| Reader / renderer hooks | **out** | — | The custom renderer is the endgame; scripting it before it exists would freeze a design that does not exist |
| Anything that writes | **out** | — | A bad extension should produce wrong results, never a corrupted library |

**Two of these are already done.** Metadata providers and themes are extensible
today, in the only sense that matters for a single-user app: adding one is a
small, isolated, type-checked change. That is worth stating plainly because it
reframes the question — the app is *already* extensible along the axes the user
named. What is missing is content sources, which is precisely A0 step 8.

### Should `MetadataSource` merge into `Source`?

**No.** They look similar and are not.

`MetadataSource` answers *"what do you know about this book I already own?"* —
it returns `Candidate`s that are proposed edits to existing rows, and the user
picks one. `Source` answers *"what can I download?"* — it returns works that do
not exist locally yet, with chapters and content behind them.

Merging them would mean one trait where half the methods are `Unsupported` for
every implementation. The `Content` split in §2 is justified because fiction
and manga differ in *one* step out of four; metadata and content sources differ
in three out of four.

They should, however, **share the small vocabulary**: `SourceError`,
`RateLimit`, and the host's HTTP agent. That is where the duplication would
actually hurt, and it is worth a small refactor when `Source` lands — moving
`FetchError`'s network-message humanising (already written and tested in
`src/metadata/mod.rs`) somewhere both can use.

---

## 12b. What "adding an extension" actually looks like

Because "no plugin system" can sound like "not extensible", here is the whole
cost, using the surface that already works today.

**Adding a metadata provider** (`src/metadata/`, shipping since P5):

1. Write `src/metadata/my_provider.rs` implementing `MetadataSource` — two
   methods, `id()` and `search()`.
2. Add a variant to `SourceId` and an arm to `enabled_sources()`.
3. Done. It appears in Settings, is toggleable, and merges into the results
   list with the others.

That is one file and one match arm. Compare with what a Lua equivalent would
have needed: a manifest format, a loader, a sandbox, host bindings for HTTP and
HTML parsing, an error-reporting path from Lua back to a GTK toast, and a
version-compatibility story. All of that is infrastructure for *distributing*
extensions to people who cannot compile — which is the thing this project does
not need.

**A content source will look the same** once `Source` lands: one file
implementing four methods, one line registering it.

**The trade, stated plainly.** Adding an extension requires a rebuild
(`cargo build --release`, minutes on the user's machine). In exchange: the
compiler checks it, CI lints and tests it, there is no sandbox to get wrong,
and the host API can be changed freely because every caller is in this repo.
For two authors who compile the app anyway, that is a good trade. For a
thousand users who do not, it would be a terrible one — which is exactly why
Yazi and Tachiyomi made the opposite choice, correctly, for their situation.

---

## 13. Open questions

Not decided; needs a conversation.

1. **Search filters.** AO3 has fandom / relationship / character / rating /
   warnings / completion. MangaDex has genre / demographic / status. Tachiyomi
   models these as a list of typed filter widgets the source declares and the
   UI renders generically. That is flexible and makes the search screen
   entirely source-driven — but it is also the single most complex part of the
   Tachiyomi API. A fixed set of common filters would be simpler and would fit
   AO3 badly. **Leaning:** source-declared filters, because AO3's tag search is
   the actual headline feature of P7 and a fixed set cannot express it.
2. **Where downloaded fiction lands.** Convert to EPUB on download and reuse
   the whole existing reader and annotation stack, or store chapters natively
   and teach the reader a second format? EPUB conversion is far less new code;
   native storage makes incremental updates cleaner. **Leaning:** EPUB, because
   the alternative duplicates the reader.
3. **Login-required sources.** AO3 shows some works only to logged-in users.
   Credential storage is a security question this document has not touched.
   Probably out of scope until someone asks.
