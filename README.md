# Kalam

**Kalam** is a lightweight, personal, all-in-one ebook manager and reader for Linux.
Built with **Rust**, **GTK4**, and **Relm4**. Designed to stay fast on modest hardware.

> Phase 0 — application shell. Sample data only; no real library yet.

## Working agreement

- **CI (GitHub Actions)** compiles every push — you don’t need to build between commits.
- **Your Arch machine** is only needed at **phase boundaries** (smoke-test + design feedback).
- Full plan: [`ROADMAP.md`](./ROADMAP.md) · architecture notes: [`ARCH.md`](./ARCH.md)

## What works now (P2)

- Slim sidebar shell + cover-card library grid
- **SQLite catalog** at `~/.local/share/kalam/catalog.db`
- **Import EPUB** (My Library → All books → “+ Import EPUB”)
- **EPUB reader** (WebKitGTK): chapter-wise scroll, TOC, themes, font size, progress restore
- Float detail panel (Suwayomi-style); Read opens the viewer
- Shelves / AO3 / comics / etc. still placeholders

## Phase overview

| Phase | Feature |
|-------|---------|
| P0 | Shell + nav |
| P1 | SQLite library, EPUB import, covers |
| **P2** | EPUB reader ← current |
| P3 | Highlights, quotes, offline dictionary |
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

## Click-through demo

1. **Shelves** (sidebar) → grid of 6 demo shelves  
2. Open **Currently reading** → list of books  
3. **Open** on a book → full book page (cover, tags, Read button)  
4. **← Back** returns through the stack  
5. **Float** opens the same book page in a floating window  
6. **Library** → **All books** → same book rows  

## Project layout

```text
src/
  main.rs          entry + dark preference
  app.rs           shell, sidebar, routing
  models.rs        routes + sample books/shelves
  style.rs         global CSS
  pages/           Home, Library, Shelves, Book, placeholders
  widgets/         book row, shelf card
```

## Data (future)

```text
~/.local/share/kalam/     library DB, books, dict packs
~/.config/kalam/          config.toml
```

## License

GPL-3.0-or-later (aligned with typical GTK app norms; adjust if you prefer).
