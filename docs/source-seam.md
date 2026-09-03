# The source seam — A0 step 8

**Status:** design, not code. Nothing here is implemented yet.
**Why it exists:** P7 (fiction sources) and P9 (manga sources) both need this
interface. Designing it once, before either is built, is the whole point of the
step — two phases inventing their own shape would mean rewriting one of them.

**Scope note (2026-09-03, revised twice — read §9a):** this is a **single-user
app, not an ecosystem** (§0), but that does **not** rule out a scripting
runtime. Lua is **in the plan** for content sources and add-on metadata
providers, because those break when websites change and a rebuild of this crate
is expensive (fat LTO, `codegen-units = 1`). It is **sequenced after** AO3
proves the interface natively — an API designed against zero implementations is
a guess.

**Acceptance test for this document** (from the A0 acceptance criteria): *a
fresh chat can add a source plugin from the documented API alone.* If you are
that fresh chat and something below is ambiguous, that is a bug in this file.

---

## 0. Scale: personal use, no ecosystem — but yes, a scripting runtime

> **The short answer, because it is the question people ask first:**
> **Yes — Lua, for the surfaces that break.** Content sources (AO3, FFN,
> scraped manga) and *additional* metadata providers are Lua plugins. The
> built-ins that ship with the app — Open Library, Google Books, themes,
> export formats — stay compiled-in Rust. The dividing line is **does this
> break when a website changes its HTML**, not "is this a source". §9a has the
> reasoning; §12a lists the surfaces.

The user settled the audience question:

> *"sources and metadata and maybe a few more, not an ecosystem, because it's
> for personal use. plugin system makes sense if there is a community, which
> isn't the case here."*

One consequence, and one **non**-consequence that I got wrong on the first
pass.

**Consequence — surfaces are a short, deliberate list.** Sources (P7/P9) and
metadata providers, plus possibly export formats, dictionaries and themes
later. Not UI extension, not reader/renderer hooks, not anything that writes to
the library. Each is added when it has a real consumer, never speculatively.
**No plugin marketplace, no versioning story, no compatibility guarantees** —
those are ecosystem problems and this is not an ecosystem.

**NOT a consequence — "no ecosystem" does not mean "no scripting."** I claimed
it did, and that was wrong. Those are two separate questions:

| Question | Answer |
| --- | --- |
| Should we court third-party plugin authors, run a repo, promise API stability? | **No** — no community, so no ecosystem |
| Should *we* be able to fix a broken scraper without recompiling 44k lines under fat LTO? | **Yes** — and that is what a runtime buys |

The second question is about **iteration speed on code that breaks often**, and
it applies just as much to a single user as to a thousand. Conflating the two
is what produced the wrong call; see §9a.

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

§9a decides **whether** to use Lua (yes, for the surfaces that break). This
section is the **how**, and it is load-bearing: the `SourceFactory` / `Source`
split below is what makes a Lua-backed source possible at all.

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

## 9a. Lua: yes, for the surfaces that break (decided 2026-09-03)

This section was written twice with the wrong answer. The record of why is kept
because the mistake is instructive.

### What I got wrong

The user said this is for personal use, with no community. I concluded "no
community ⇒ no plugin system" and removed Lua from the plan.

**That conflates two questions.** "Should we run an ecosystem?" and "should we
be able to change a scraper without recompiling?" are unrelated. The first is
about *other people*; the second is about *iteration speed on fragile code*,
which matters to a solo user just as much.

The user's counter-example settled it:

> *"have you seen how metadata plugins in Calibre work? there are many, many
> plugins in Calibre just for metadata sources. I'd say, Open Library and
> Google Books should be built in, but we can have option to add more sources
> later with Lua."*

That is exactly right, and it is checkable. Calibre's plugin index carries
**~20+ third-party metadata-source plugins**: Goodreads, Amazon, Kobo,
StoryGraph, FictionDB, ISFDB, Douban, DNB, Baen, Fantastic Fiction, Barnes &
Noble, noosfere, Moly.hu, databazeknih.cz, Skoob, Bookline, Lira, Alexandra,
Biblioman, Kitapyurdu, SF-Leihbuch… Calibre ships a handful built in and lets
everything else be a plugin. The long tail is regional (Hungarian, Czech,
Brazilian, German, Chinese, Turkish) and niche (science fiction, romance),
which is precisely the tail a built-in list can never serve.

And critically: **almost all of those are scrapers.** Goodreads, Amazon and
Barnes & Noble have no public metadata API — the plugins parse HTML, and they
carry changelogs full of "fixed for site change".

### The real dividing line

Not "source vs. everything else". It is:

> **Does this break when somebody else changes their website?**

| | Breaks? | Why | Implementation |
| --- | --- | --- | --- |
| Open Library | rarely | documented JSON API (`openlibrary.org/search.json`) | **built-in Rust** |
| Google Books | rarely | documented JSON API (`googleapis.com/books/v1`) | **built-in Rust** |
| MangaDex | rarely | official versioned API | **built-in Rust** |
| Goodreads, Amazon, StoryGraph… | **often** | HTML scraping, no API | **Lua** |
| AO3, FFN, Royal Road, Webnovel | **often** | HTML scraping, no API | **Lua** |
| Scraped manga sites | **often** | HTML scraping | **Lua** |
| Themes, export formats, dictionaries | never | pure data transforms, no remote dependency | **built-in Rust** |

Tachiyomi's ecosystem says the same thing in one line:

> *"Extensions are not just sources. They are **parsers**. If a website changes
> its structure, the extension breaks. The core app stays stable. **Extensions
> change constantly.**"*

### Why the rebuild cost is not trivial here

I previously waved this away as "a few minutes". It is worse than that, and the
reason is in this repo's own `Cargo.toml`:

```toml
[profile.release]
lto = true            # fat LTO: whole-program, all IR in memory at once
codegen-units = 1     # no codegen parallelism
```

That is the slowest possible rebuild configuration, over 44k lines and 36
dependencies. **A one-character change to a CSS selector re-links the entire
binary.** On a 4 GB machine, fat LTO is also the configuration most likely to
be OOM-killed (documented Rust issue; see `conversation.md`).

So the honest comparison for fixing a broken scraper is:

- **Lua:** edit a selector, restart the app. Seconds.
- **Rust:** edit a selector, full fat-LTO rebuild, restart. Minutes, with a
  memory-pressure risk.

Multiply by "sites change often" and the difference stops being cosmetic.

### What Lua still costs, honestly

None of this disappears; it is simply outweighed for the breaking surfaces.

| Cost | Mitigation |
| --- | --- |
| A second language in the debugging path | Confined to scrapers — parse-and-return, the simplest code in the app |
| No compile-time checking | The host validates returned tables and reports a clear error; a bad plugin fails one source, not the app |
| Sandbox surface | Small: HTTP through the host agent, an HTML selector, JSON. No filesystem, no catalog (§10) |
| A frozen host API | Real, and the reason Lua is **sequenced after** AO3 native (§11) — freeze it once it is known to fit |
| `mlua` vendors a C interpreter | ~200 KB of C; a rounding error next to WebKitGTK |
| The `!Send` dance | Already designed and paid for (§9) |

### The decision

**Lua is in the plan, for content sources and add-on metadata providers.**
Built-in Rust for Open Library, Google Books, MangaDex, themes, export formats
and dictionaries — the things with stable APIs or no remote dependency at all.

**Sequenced, not skipped:** AO3 native first (§11). An API designed against
zero implementations is a guess; an API extracted from two working ones is a
description.

## 10. What a plugin is allowed to touch

These rules govern Lua plugins, and they describe the discipline a built-in
Rust source should follow voluntarily too — a Rust source that reaches into the
catalog is just as wrong, it simply cannot be stopped by a sandbox.

The host gives a plugin exactly four things:

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
4. **The Lua host**, with AO3 ported to Lua as the proof. If the ported plugin
   behaves identically to the native one, the API is right. This is the step
   that unlocks add-on metadata providers too, since they share the host,
   the sandbox and the loader.
5. **Move scrapers out to Lua as they prove annoying.** AO3 can stay native and
   act as the reference implementation; the ones that break most often are the
   ones worth converting first.

This matches `docs/conversation.md` §5's lean — *"define the seams as stable
APIs now, add scripting later"* — with "later" now having a concrete trigger:
once two native sources exist.

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

## 12a. The surface list

From §0 a short list; from §9a each entry is **built-in Rust or Lua depending
on whether it breaks**. Every one shares the property that makes it safe — a
pure function from data to data, touching no widgets, no catalog, no reader
state.

| Surface | How | Status | Existing seam |
| --- | --- | --- | --- |
| **Content sources** — AO3, FFN, Royal Road, scraped manga | **Lua** | designing (this doc) | none yet; P7/P9 depend on it |
| **Content sources** — MangaDex, Komga, Kavita, OPDS | built-in Rust | planned | official APIs, so they do not rot |
| **Metadata — Open Library, Google Books** | built-in Rust | **shipping** | `MetadataSource`, `src/metadata/` |
| **Metadata — add-on providers** (Goodreads, StoryGraph, regional sites…) | **Lua** | after the host lands | same trait, Lua-backed impl |
| **Themes** | built-in Rust | **shipping** | `Theme` in `src/theme.rs`; 13 of them |
| **Export formats** | built-in Rust | possible later | `export_quotes_markdown` is already a pure `&[Quote] -> String` |
| **Dictionaries** | built-in Rust | possible later | `Catalog::search_dict` / `lookup_entry` |
| UI extension | — | **out** | GTK widget tree; exposing it freezes every layout decision |
| Reader / renderer hooks | — | **out** | the custom renderer is the endgame; scripting it now freezes a design that does not exist |
| Anything that writes to the library | — | **out** | a bad extension should give wrong results, never a corrupted catalog |

### Why metadata splits down the middle

This is the Calibre model, and it is the right one.

Open Library and Google Books are **documented JSON APIs** — they version, they
deprecate on notice, they do not rearrange their HTML on a Tuesday. Two
built-in providers cover the common case with compile-time checking, and they
are already written.

Everything past that is a long tail that a built-in list can never serve:
Goodreads, Amazon, StoryGraph, Kobo, plus the regional databases (moly.hu,
databazeknih.cz, Skoob, DNB, Douban) and the niche ones (ISFDB for science
fiction, FictionDB for romance). Calibre's index carries 20+ of these. **Almost
all are HTML scrapers** — Goodreads and Amazon have no public metadata API —
so they break, and they need a fix loop measured in seconds.

Shipping two solid built-ins and letting the tail be plugins is exactly what
Calibre does, and the reason it has the metadata coverage it does.

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
`RateLimit`, the host's HTTP agent — and, once it exists, **the Lua host
itself**. One runtime, one sandbox, one loader, two traits exposed to it. That
is where the duplication would actually hurt, and it is worth a small refactor
when `Source` lands: `FetchError`'s network-message humanising is already
written and tested in `src/metadata/mod.rs` and both should use it.

---

## 12b. What "adding an extension" actually looks like

Two routes, because there are two kinds of extension.

### A built-in (stable APIs): a Rust file and a match arm

This works today — `src/metadata/` has shipped since P5:

1. Write `src/metadata/my_provider.rs` implementing `MetadataSource` — two
   methods, `id()` and `search()`.
2. Add a variant to `SourceId` and an arm to `enabled_sources()`.
3. Rebuild. It appears in Settings, is toggleable, and merges into the results
   list with the others.

Cost: one file, one line, one rebuild. Benefit: the compiler checks it, CI
lints and tests it, no sandbox to get wrong. Correct for Open Library and
Google Books, which do not break.

### A plugin (scrapers): a Lua file, no rebuild

Once the host lands:

1. Drop `~/.local/share/kalam/plugins/goodreads/main.lua` with a small
   manifest (id, name, kind, rate limit).
2. Implement the same four verbs — `search`, `detail`, `chapters`, `content`
   — for a source, or `search` alone for a metadata provider.
3. Restart the app. No rebuild.

When Goodreads moves a `<div>`, the fix is one line and a restart.

### Why the rebuild route is wrong for scrapers

`[profile.release]` in this repo is `lto = true` with `codegen-units = 1` —
whole-program optimisation with no codegen parallelism, over 44k lines and 36
dependencies. A one-character selector change re-links the whole binary, and on
a 4 GB machine fat LTO carries a real OOM risk.

That is a fine price to pay once for a stable API provider. It is the wrong
price to pay every few weeks for a scraper.

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
