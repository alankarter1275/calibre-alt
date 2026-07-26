# Kalam

**Kalam** is a lightweight, personal, all-in-one ebook manager and reader for Linux.
Built with **Rust**, **GTK4**, and **Relm4**. Designed to stay fast on modest hardware.

> Phase 3 — annotations & offline dictionary on top of the EPUB reader.

## Working agreement

- **CI (GitHub Actions)** compiles every push — you don’t need to build between commits.
- **Your Arch machine** is only needed at **phase boundaries** (smoke-test + design feedback).
- Full plan: [`ROADMAP.md`](./ROADMAP.md) · architecture notes: [`ARCH.md`](./ARCH.md)

## What works now (P3)

- Slim sidebar shell + cover-card library grid
- **SQLite catalog** at `~/.local/share/kalam/catalog.db`
- **Import EPUB** (My Library → All books → “+ Import EPUB”)
- **EPUB reader** (WebKitGTK): chapter-wise scroll, TOC, themes, font size, progress restore
- **Highlights & quotes**: select text → floating chip (yellow/green/blue/pink/orange), save quote (❝), copy
- **Dictionary**: offline packs (StarDict .ifo/.idx/.dict[.dz], SQLite .db, TSV), lookup via chip or `D` shortcut, definition popup near selection, save word
- **Annotations list**: reader bottom pill ✎ shows highlights/quotes for current book, jump & delete
- **Library hub**: My Library → Saved quotes (real data) → export to Markdown (`~/Quotes.md`), Saved words (real data)
- **Settings**: dictionary packs import (+ Import dictionary), list & remove, data paths
- Float detail panel (Suwayomi-style); Read opens the viewer
- Shelves / AO3 / comics / etc. still placeholders

## Phase overview

| Phase | Feature |
|-------|---------|
| P0 | Shell + nav ✅ |
| P1 | SQLite library, EPUB import, covers ✅ |
| P2 | EPUB reader ✅ |
| **P3** | **Highlights, quotes, offline dictionary** ← current ✅ |
| P4 | Home / shelves engine / lists (real data) |
| P5 | Metadata edit + Open Library fetch |
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

## Click-through demo (P3)

1. **Library → All books → + Import EPUB** → import a book
2. **Open** book → **Read** → reader opens (sepia default, no blue links)
3. **Select text** in reader → floating chip appears (colors / ❝ quote / Aa dictionary / copy)
4. Highlight yellow → annotation saved, reappears on reload
5. **D** key or chip **Aa** → dictionary lookup (if packs imported in Settings), popup near word, **Save word**
6. Bottom pill **✎** → list highlights/quotes for book, Jump / Delete
7. **Library → Saved quotes** → search, delete, **Export Markdown** → `~/Quotes.md`
8. **Library → Saved words** → search, delete
9. **Settings** → Offline dictionaries → + Import dictionary (StarDict .ifo or SQLite .db or TSV) → list & remove
10. **Shelves** still placeholder (P4)

## Project layout

```text
src/
  main.rs          entry + dark preference
  app.rs           shell, sidebar, routing
  db.rs            SQLite catalog + annotations + dict
  dict.rs          StarDict / SQLite / TSV import & search
  epub.rs          EPUB OPF metadata + cover extract
  epub_book.rs     spine, TOC, chapter HTML + reading CSS/JS (highlights, chip, dict)
  models.rs        routes + books/shelves
  style.rs         global CSS (including P3 badges/rows)
  pages/           Home, Library, Shelves, Book, Reader (P3), SavedQuotes, SavedWords, Settings
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
