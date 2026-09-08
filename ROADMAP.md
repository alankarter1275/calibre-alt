# Kalam — Master Roadmap

Kalam is a pure, robust offline reading sanctuary for EPUBs, CBZs, and PDFs, built with Rust and GTK4. 

This document outlines the current state of the architecture and the strict sequence of future phases.

---

## ⚠️ For AI Agents
1. **Never litigate old decisions:** If you want to see the 2,600-line history of past features (P0-P7), read `docs/archive/historical-roadmap.md`. 
2. **Current Trajectory:** We have intentionally ripped out all hardcoded online scrapers (AO3, MangaDex, etc.) to focus exclusively on polishing the offline reading engines and library management.
3. **Documentation Rule:** All architectural plans, discussions, and pitfalls MUST be written directly to the `docs/` folder. Do not use temporary Gemini artifacts.

---

## Current State: The Foundation
*   **Library Core (`src/db/`):** SQLite-backed catalog managing books, tags, and custom shelves.
*   **EPUB Engine (`src/pages/reader/`):** WebKit-based rendering with custom dictionary lookups (`src/dict.rs`), highlighting, and annotations.
*   **Comic Engine (`src/pages/comics_reader/`):** Local offline CBZ/ZIP reading.
*   **Online UI:** Stripped. Kalam is currently a completely "hollow" local library.

---

## Active Track: Pillar 1 & 2 (Offline Polish)
We are currently executing the **Offline Library Architecture** track. No online or plugin work may begin until these pillars are complete.

### 1. Reading Engines
*   **PDF Engine:** Integrate `pdfium-render` to provide Zathura-style smart cropping (auto-detecting ink bounding boxes) and a heuristic reflow toggle for text extraction.
*   **EPUB Inline Editing:** Build a Rust-proxy sidecar patch system (`kalam.json`). When WebKit requests a chapter, Rust applies XPath text-replacement patches to fix typos non-destructively.
*   **Comic Engine:** Add right-to-left manga mode, background image preloading, and a permanent "Remaster" tool to upscale low-res CBZs using CPU-heavy Lanczos3 interpolation.

### 2. Library Organization & UI
*   **Inline Editing (`book.rs`):** Replace static GTK labels with `gtk::Stack` to allow frictionless, click-to-type inline metadata editing for Titles and Authors directly on the book page.
*   **Cover Stacks & Smart Shelves:** Visually collapse books in a series into a single "stack" in the grid. Implement dynamic Smart Shelves based on search queries.
*   **Dedicated Metadata Fetcher:** Elevate the current Open Library `in_app_dialog` to a full dedicated route.

### 3. File Ingestion
*   **The Sanitizer Pipeline:** A background worker that intercepts imported EPUBs, strips hardcoded CSS, fixes broken XML, and repacks them cleanly before adding them to the library.
*   **Deep Search:** Implement a default `tantivy` indexer to provide 20ms full-text search across the actual contents of the entire library.

---

## Future Track: Phase 9 (Wasm Plugin System)
Once the offline sanctuary is flawless, we will reintroduce online reading via isolated Wasm components.
*   **Architecture:** Use `wasmtime` and the Wasm Component Model (`wit/kalam.wit`).
*   **The Rule:** Kalam (the Rust host) handles all HTTP requests to bypass Cloudflare. Wasm plugins receive raw HTML and parse it into structured metadata.
*   **Bespoke UI Husks:** Major plugins (like AO3) will pair with hardcoded, bespoke GTK4 UIs in the main app to maintain premium polish, rather than relying on generic unified filters.
