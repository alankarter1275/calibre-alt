# Kalam

**Kalam** is a lightweight, personal, all-in-one ebook manager and reader for Linux.
Built with **Rust**, **GTK4**, and **Relm4**. Designed to stay fast on modest hardware.

> Phase 5 — metadata editing, cover replacement and Open Library lookup.

## Working agreement

- **CI (GitHub Actions)** compiles every push — you don’t need to build between commits.
- **Your Arch machine** is only needed at **phase boundaries** (smoke-test + design feedback).
- Full plan: [`ROADMAP.md`](./ROADMAP.md) · architecture notes: [`ARCH.md`](./ARCH.md)

## What works now (P5)

- Slim sidebar shell + cover-card library grid
- **SQLite catalog** at `~/.local/share/kalam/catalog.db`
- **Import EPUB** (My Library → All books → “+ Import EPUB”)
- **EPUB reader** (WebKitGTK): chapter-wise scroll, TOC, themes, font size, progress restore
- **Highlights & quotes**: select text → floating chip (yellow/green/blue/pink/orange), save quote (❝), copy
- **Dictionary**: offline packs (StarDict .ifo/.idx/.dict[.dz], SQLite .db, TSV), lookup via chip or `D` shortcut, definition popup near selection, save word
- **Annotations list**: reader bottom pill ✎ shows highlights/quotes for current book, jump & delete
- **Library hub**: My Library → Saved quotes (real data) → export to Markdown (`~/Quotes.md`), Saved words (real data)
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
| P2 | EPUB reader ✅ |
| P3 | Highlights, quotes, offline dictionary ✅ |
| P4 | Shelves engine, lists, history, tags, analytics ✅ |
| **P5** | **Metadata edit, cover replace, Open Library fetch** ← current ✅ |
| P6–P11 | Downloads, AO3/FF, comics, PDF, tools — see ROADMAP |

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
  widgets/         book row, shelf card
```

## Data

```text
~/.local/share/kalam/
  catalog.db
  library/<uuid>/
  dictionaries/        (imported packs meta only; entries in catalog.db)
  cache/reader/<uuid>/
~/.config/kalam/       (future)
~/Quotes.md            (export target)
```

## License

GPL-3.0-or-later (aligned with typical GTK app norms; adjust if you prefer).
