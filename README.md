# Kalam

**Kalam** is a lightweight, personal, all-in-one ebook manager and reader for Linux.
Built with **Rust**, **GTK4**, and **Relm4**. Designed to stay fast on modest hardware.

> Reader-improvements track complete — merged dictionary store, popup redesign,
> POS + likely-sense hint, offline IPA pronunciation, tap-to-look-up, find in
> chapter, vocabulary review + CSV/Anki export.

## Working agreement

- **CI (GitHub Actions)** compiles, clippys, runs the unit tests and builds
  debug+release on every push — you don’t need to build between commits.
- **Backend review pass done** — full line-by-line sweep of the data layer:
  importers hardened (read-only SQLite packs, identifier quoting, rollback,
  catalog.db self-import guard), one latent bug fixed (reading-list column
  offsets), 9 new unit tests. No other defects.
- **CI now runs the 154 unit tests on every push** (the `cargo test` step is
  live in `.github/workflows/ci.yml`). The first real run caught one failing
  test (a bad escape in the `quote_ident` test literal) — fixed, all green.
  On failure the diagnostics are published to `ci-logs/test-latest.txt`.
- Workflow changes are made by you with your own account (the App cannot push
  `.github/workflows/`); the canonical copy lives at
  `docs/ci/github-actions-ci.yml` — see `docs/ci/README.md`.
- **Your Arch machine** is only needed at **phase boundaries** (smoke-test + design feedback).
- Full plan: [`ROADMAP.md`](./ROADMAP.md) · architecture notes: [`ARCH.md`](./ARCH.md)

## What works now

- Slim sidebar shell + cover-card library grid
- **SQLite catalog** at `~/.local/share/kalam/catalog.db`
- **Import EPUB** (My Library → All books → “+ Import EPUB”)
- **EPUB reader** (WebKitGTK): chapter-wise scroll, TOC, themes, font size, progress restore
- **Highlights & quotes**: select text → floating chip (yellow/green/blue/pink/orange), save quote (❝), copy
- **Dictionary**: offline packs (StarDict .ifo/.idx/.dict[.dz], SQLite .db, TSV), lookup via chip, tap, or `D` shortcut, a popup with numbered senses + POS, synonym/antonym chips, idiom cards, bookmark & copy, offline IPA pronunciation (`bank` → `/ˈbæŋk/`) from the bundled CMU Pronouncing Dictionary, keyboard support (↑/↓ focus a sense, Enter saves it), and **Find in chapter**
- **Annotations list**: reader bottom pill ✎ shows highlights/quotes for current book, jump & delete
- **Library hub**: My Library → Saved quotes (real data) → export to Markdown (`~/Quotes.md`), Saved words (real data) → vocabulary review (mark known / to review, All/To review/Known filter) → export CSV (`~/SavedWords.csv`) or Anki TSV (`~/SavedWords-Anki.txt`)
- **Settings**: dictionary packs import (+ Import dictionary), list & remove, data paths
- **Shelves**: manual collections + **smart shelves** with a rule builder
  (tag / author / series / format / progress / title / added · is · is not ·
  contains · date windows · All-or-Any), live match count, 2-column grid
- **Reading list**: ordered TBR with ↑/↓ reorder and a bulk picker
- **History**: every open / finish / import, grouped by day and filterable
- **Reading time**: sessions recorded per reader visit (clamped at 6h)
- **Tags**: usage-weighted tag cloud → per-tag book grid
- **Analytics**: counts, time read (7d/30d/all), streaks, 14-day bar chart,
  books-added-per-month, most read, top tags & authors
- **Book page**: add to reading list, Mark finished / unread, shelf checklist,
  half-star rating, **Edit metadata**
- **Metadata editor**: title/authors/series/tags/description, replace cover
  from disk, and **Open Library** search that stages results for review
- **Ratings & goals**: half-star ratings, yearly reading goal, daily streak
- Float detail panel (Suwayomi-style); Read opens the viewer
- AO3 / comics / downloads still placeholders

## Phase overview

| Phase | Feature |
|-------|---------|
| P0 | Shell + nav ✅ |
| P1 | SQLite library, EPUB import, covers ✅ |
| P2 | EPUB reader (incl. P2.1 chrome restyle) ✅ |
| P3 | Highlights, quotes, offline dictionary ✅ |
| P4 | Shelves engine, lists, history, tags, analytics ✅ |
| **P5** | **Metadata edit, cover replace, Open Library fetch** ✅ |
| Reader track | Annotation workflow + hybrid anchoring; dictionary overhaul (merged store, popup redesign, likely-sense hint, IPA pronunciation, tap-to-look-up, find in chapter) + vocabulary review (known flag, CSV/Anki export) ✅ · Phases 8–10 planned: POS grouping dividers, dictionary priority reorder UI, lookup history |
| Backend review | Full sweep of `db.rs` + `db/*`: importers hardened, reading-list column bug fixed, 9 new tests ✅ |
| P6–P11 | Downloads, AO3/FF, comics, PDF, tools — see ROADMAP |
| UI overhaul (P5.5) | Colour system, 13 themes, Settings v2, book page, series float — **in progress** (Home/Library/Reader chrome next) |

## Requirements (Arch Linux)

```bash
sudo pacman -S --needed rust gtk4 libadwaita webkitgtk-6.0 base-devel pkgconf
```

## Build & run

```bash
cd calibre-alt   # or your clone path
cargo run
```

Release build (what you’ll use day to day):

```bash
cargo run --release
```

## Test online with GitHub Codespaces

If running Kalam on your own machine is a pain, you can test it in GitHub Codespaces.

### First time setup

1. Open this repo on GitHub.
2. Click **Code**.
3. Open the **Codespaces** tab.
4. Click **Create codespace** on the branch you want to test.
5. Wait for the setup to finish.

### Start Kalam in the browser

In the Codespaces terminal, run:

```bash
./scripts/run-kalam-codespace.sh
```

Then:

1. Open the **Ports** tab in Codespaces.
2. Find port **6080**.
3. Open it in the browser.
4. You should see a simple Linux desktop.
5. Kalam should open there.

### If you want to start it by hand

```bash
./scripts/codespaces-desktop.sh
source ~/.cache/kalam-codespace/env.sh
cargo run
```

### Notes

- The first start can take a few minutes.
- Port **6080** is the browser view for the app.
- If you only see a file list in the browser, run `./scripts/codespaces-desktop.sh` again and refresh the page.
- If the desktop opens but Kalam is not running yet, go back to the terminal and run `cargo run`.
- You can also use `cargo run --release` if you want the faster build.

## Click-through demo (P4)

1. **Shelves → + Smart shelf** → name it, add rules (e.g. `Tag is fantasy`
   **and** `Progress is Unread`) → watch the **live match count** → Create
2. Click the card → the shelf lists exactly those books
3. **Shelves → + Shelf** (manual) → open it → **+ Add books** → tick a few →
   expand **Manage shelf order** → reorder with ↑/↓ or remove
4. **Book page → Shelves…** → tick/untick manual shelves → chips update
5. **Book page → + Reading list** → **Library → Reading list** → reorder, Read
6. **Read** a book for a minute, leave → **Library → History** shows "Opened"
7. **Library → Analytics** → time read, streaks, 14-day chart, top tags
8. **Library → Tags** → click a tag → grid of books with that tag
9. **Book page → Mark finished** → drops off the reading list, logged in History
10. **Home** → counts strip, Continue row, Up next peek

## Project layout

```text
src/
  main.rs          entry + dark preference
  app.rs           shell, sidebar, routing
  db.rs            SQLite catalog + annotations + dict + P4 shelves/lists/stats
  dict.rs          StarDict / SQLite / TSV import & search
  shelf_rules.rs   smart-shelf rule documents → SQL
  epub.rs          EPUB OPF metadata + cover extract/replace
  openlibrary.rs   Open Library search / description / cover
  epub_book.rs     spine, TOC, chapter HTML + reading CSS/JS (highlights, chip, dict)
  models.rs        routes + books/shelves
  style.rs         global CSS (including P3 badges/rows)
  pages/           Home, Library, Shelves (+ editor/detail), ReadingList,
                   History, Tags, Analytics, Book, Reader, SavedQuotes,
                   SavedWords, Settings
  db/              db.rs split: annotations, authors, dictionaries, history,
                   metadata, prefs, pronunciation, series, shelves, stats
  widgets/         book row, shelf card
```

### Backend layout (post split)

The 3,400-line `db.rs` was split so each area is navigable. All methods live
on the same `Catalog`:

| File | Covers |
|------|--------|
| `db.rs` | schema/migrations, book CRUD, progress, row mappers, helpers (`escape_like`, `chrono_like_now`, streaks) |
| `db/dictionaries.rs` | merged store, search/lookup chain, sense parsing |
| `db/annotations.rs` | highlights, quotes, saved words, reading bookmarks |
| `db/history.rs` | event log, reading sessions |
| `db/metadata.rs` | metadata edits, overrides/restore, covers, ratings, goals |
| `db/shelves.rs` | shelves, reading list |
| `db/stats.rs` | analytics, backup |
| `db/authors.rs` / `db/series.rs` | author profiles, series cache |
| `db/prefs.rs` / `db/pronunciation.rs` | app prefs, IPA pronunciation |

## Data

```text
~/.local/share/kalam/
  catalog.db
  library/<uuid>/
  dictionaries/        (imported packs meta only; entries in catalog.db)
  cache/reader/<uuid>/
  override-covers/     (stashed covers for metadata restore)
  series-covers/       (cached series float covers)
~/.config/kalam/       (future)
~/Quotes.md            (export target — saved quotes, Markdown)
~/SavedWords.csv       (export target — vocabulary, RFC-4180)
~/SavedWords-Anki.txt  (export target — vocabulary, Anki TSV)
```

## Offline dictionaries

Kalam does not fetch dictionary data from the internet. You add a pack once,
then lookups work offline.

### Import a pack

1. Download a dictionary in one of the supported formats.
2. Open **Settings → Dictionaries → Import dictionary**.
3. For a StarDict pack, select its `.ifo`, `.idx`, or `.dict`/`.dict.dz`
   file. Keep all three files together; Kalam finds the matching files beside
   the one you select.
4. For a SQLite pack, select the `.db` file. It should have a table with
   `word` and `definition` columns.
5. For a text pack, select a `.tsv` or `.txt` file with one entry per line:
   `word<TAB>definition`.

The imported entries are copied into Kalam's catalog database, so the original
pack can be moved afterwards. Imported packs appear in the same Settings page
and can be removed there. On Linux, the dictionary data directory is
`~/.local/share/kalam/dictionaries`.

In the reader, select a word or complete phrase and choose **Dictionary** (or
press `D`). Kalam keeps the phrase, removes surrounding punctuation, and
lemmatizes the term before looking it up: WordNet's irregular exception lists
resolve `went` → `go`, `mice` → `mouse`, `better` → `good` and `running` →
`run`, with regular suffix rules as the fallback. Lookups are case- and
diacritic-insensitive — `Run`, `RUN` and `rún` all find `run` — because every
headword is indexed under a normalized key. Selecting a phrase looks for the
phrase itself first (so `run out of steam` resolves to the idiom entry), then
falls back to per-word results (`odd mixture` yields `odd` and `mixture`).

All installed dictionaries are combined into one merged store: each word
appears exactly once, provided by the highest-priority dictionary that has it
(WordNet first, then the other bundled packs, then any dictionary you
import), with all of that dictionary's senses listed. A word shared by two
dictionaries is never shown twice, and the other dictionary's version stays
hidden. The store rebuilds automatically whenever you import or remove a
dictionary. The popup shows up to five matching words; each has **Save word**
and **Copy** buttons.

The popup also shows the word's pronunciation as a compact IPA
transcription — `bank` → `/ˈbæŋk/`, `run` → `/ˈrʌn/` — from the bundled CMU
Pronouncing Dictionary 0.7a (BSD-style licence; provenance and checksums in
`resources/dictionaries/cmudict-0.7a.NOTICE.txt`). It is fully offline: the
packed dictionary is compiled into the binary, ARPABET phonemes are
converted to IPA with stress marks, and lookups resolve through the same
lemmatization as definitions, so `running` finds `run`. Words absent from
the dictionary (and multi-word phrases) simply show no transcription.

You can also tap any word in the book to look it up — no selection needed.
A plain click resolves the word under the caret and opens the popup for it
(with the surrounding sentence as context); a double-click still selects a
word for highlighting. With the popup open, ↑/↓ move a focus ring across
the senses and Enter saves the word with the focused sense's definition.
The magnifier button in the popup header highlights every occurrence of
the headword in the current chapter (Esc clears the highlights) — a
chapter-scoped stand-in until an in-book search exists.

### Reliable download sources

- [FreeDict downloads](https://freedict.org/downloads/) provides StarDict
  archives, SHA-512 checksums, many language pairs, and a direct link to the
  source dictionary. FreeDict says that each dictionary has its own licence;
  check the licence in the pack before redistributing it.
- [Princeton WordNet downloads](https://wordnet.princeton.edu/download) is the
  official source for the English WordNet database. It requires its licence
  notice and acknowledgement. WordNet normally needs conversion to StarDict,
  SQLite, or TSV before Kalam can import it.
- [Wiktionary dumps](https://dumps.wikimedia.org/) are broad but are released
  under CC BY-SA/GFDL terms. A redistributed extract needs attribution and
  must follow the share-alike and source-copy requirements, so Wiktionary is
  not silently bundled by Kalam.

Avoid download sites that only say “free” without naming the copyright holder
and licence. Oxford, Collins, Longman, and similar commercial dictionaries are
not safe to bundle without a separate redistribution licence.

### Bundled dictionaries

The base app includes four small English-only starter packs. **English WordNet
2025** has about 127,000 headwords and is about 4.2 MB compressed. **English
Idioms and Expressions** adds 1,024 phrase-to-meaning entries and is about
16 KB compressed. **English Synonyms (WordNet 3.0)** covers 110,000+ words
with their synset companions (about 1.7 MB compressed), and **English
Antonyms (WordNet 3.0)** adds 6,600+ antonym pairs (about 50 KB compressed).
All four packs are installed and enabled on the first run (new packs also
appear automatically on the first launch after this update), work without a
download, and appear separately in **Settings → Dictionaries**. If you remove
a pack, Kalam remembers that choice and does not silently add it back.

The WordNet pack is a format conversion of the [Open English Wordnet 2025
Edition](https://github.com/globalwordnet/english-wordnet/releases/tag/2025-edition),
which is derived from Princeton WordNet. Kalam also bundles the Princeton
WordNet 3.0 morphological exception lists (`noun.exc`, `verb.exc`, `adj.exc`,
`adv.exc`, gzipped) to resolve irregular lookup forms such as `went` → `go`,
and the English Synonyms and English Antonyms packs, which are derived from
the Princeton WordNet 3.0 synset and antonym-pointer data; their source,
checksums and licence note are kept beside the packed lists
(`wordnet-3.0-exc.NOTICE.txt`, `wordnet-3.0-synonyms.NOTICE.txt`).
The idiom pack is a format conversion
of [`baiango/english_idioms`](https://github.com/baiango/english_idioms), using
commit `d47bfb40a3f76d0f08ba1867016c383d3c21c596`. Its upstream repository
releases the data under The Unlicense. Its README says the list was collected
with ChatGPT and may contain grammatical, factual, or literal-versus-figurative
errors, so Kalam presents it as a supplemental phrase source rather than an
authoritative dictionary. Selecting a complete phrase such as `break a leg`
or `piece of cake` can now find that phrase in this pack. It does not make every
ordinary word combination meaningful automatically: `odd mixture`, for example,
remains two WordNet word entries unless a dictionary contains that exact phrase.

The bundled source revisions, checksums, attribution, and complete licence
notices are kept beside the generated packs in `resources/dictionaries/`.
Keep the applicable notices with any redistribution. The WordNet files must
remain because that data has both Open English WordNet and underlying Princeton
WordNet terms; the idiom pack has its own upstream Unlicense notice. “Personal
use” does not by itself remove licence obligations when data is committed to a
public repository or shipped in an application. No proprietary or
unclear-licence dictionary data is included.

Additional language packs can be added later after choosing the languages and
checking each pack's licence and size. The existing import flow remains the
way to add those packs now.

## License

GPL-3.0-or-later (aligned with typical GTK app norms; adjust if you prefer).
