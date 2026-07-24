# Kalam

**Kalam** is a lightweight, personal, all-in-one ebook manager and reader for Linux.
Built with **Rust**, **GTK4**, and **Relm4**. Designed to stay fast on modest hardware.

> Phase 0 — application shell. Sample data only; no real library yet.

## Working agreement

- **CI (GitHub Actions)** compiles every push — you don’t need to build between commits.
- **Your Arch machine** is only needed at **phase boundaries** (smoke-test + design feedback).
- Full plan: [`ROADMAP.md`](./ROADMAP.md) · architecture notes: [`ARCH.md`](./ARCH.md)

## What works in P0

- Slim sidebar: Home, Library, Shelves, Downloads, Comics, AO3, Fanfic, Settings
- **Home** — continue + recent (demo books)
- **My Library** hub — tiles for All books, Reading list, History, Quotes, Words, Tags, Analytics
- **Shelves** — 2-column grid of smart/manual shelves → shelf book list → **book page**
- Each book: **Open** (full page in main column) or **Float** (separate window)
- Custom dark CSS draft (replace with your design anytime)

## Phase overview

| Phase | Feature |
|-------|---------|
| **P0** | Shell + nav + sample data ← current |
| P1 | SQLite library, EPUB import, covers |
| P2 | EPUB reader (WebKitGTK, chapter-wise scroll, fonts) |
| P3 | Highlights, quotes, offline dictionary |
| P4 | Home / shelves engine / lists (real data) |
| P5 | Metadata edit + Open Library fetch |
| P6–P11 | Downloads, AO3/FF, comics, PDF, tools — see ROADMAP |

## Requirements (Arch Linux)

```bash
sudo pacman -S --needed rust gtk4 libadwaita base-devel pkgconf
```

Optional later (reader phase):

```bash
sudo pacman -S webkitgtk-6.0
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
