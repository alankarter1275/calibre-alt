# Kalam Offline Library Roadmap

This document outlines the architecture, design decisions, and minute implementation details for making Kalam the ultimate offline reading sanctuary. It covers the three core pillars: Reading Engines, Library Organization & UI, and File Ingestion.

## Pillar 1: The Reading Engines
The core philosophy is that the offline reading experience must be flawless, performant, and deeply customizable without modifying the original source files destructively.

### 1. PDF Engine (Zathura-style)
- **Backend Selection:** `pdfium-render` (Google's PDFium wrapper for Rust). Chosen for its extreme speed, robust parsing, and perfect bounding-box detection.
- **Smart Crop Feature:** Unlike standard PDF readers that center pages with massive white margins, Kalam will aggressively detect the actual ink bounds (bounding boxes of text and images) using PDFium. The viewport will automatically crop and zoom to these exact bounds, maximizing screen real estate (similar to Zathura's crop mode).
- **Reflow Toggle:**
  - PDFs lack structural text markup. When "Reflow" is toggled, Kalam uses PDFium to extract the raw text layer.
  - A heuristic algorithm runs over the text to guess paragraph boundaries based on line spacing and indentation.
  - The extracted, reflowed text is instantly fed into our existing EPUB WebKit engine, allowing users to read text-heavy PDFs with custom fonts, colors, and font-sizes just like an EPUB. Complex layouts (math, double columns) will degrade gracefully.

### 2. EPUB Engine (Inline Editing & Non-Destructive Patches)
- **The Philosophy:** EPUBs are zipped archives. Direct modification breaks file hashes and signatures. Kalam will never touch the original `.epub` file.
- **Sidecar Patch Architecture (Rust Proxy):** 
  - All typo fixes and edits are saved as XPath replacement rules in the `kalam.json` sidecar (or SQLite DB).
  - When the WebKit WebView requests an HTML chapter (via the `kalam://` custom URI scheme), the Rust host intercepts the request.
  - Rust parses the HTML, applies the XPath patches natively using a fast DOM library (like `scraper` or `tl`), and serves the patched HTML string to the WebView.
  - **Result:** Zero layout shift, invisible to the user, pristine original EPUB file.
- **Interaction Paradigms:**
  - **Annotation Toolbar:** Highlighting text spawns a sleek floating toolbar (Medium-style). Clicking `[Fix Typo]` opens an inline popover with a text box, saving the patch effortlessly without breaking reading flow.
  - **Edit Mode (Power User):** A pencil toggle in the top bar. When active, hovering over paragraphs highlights them. Clicking a paragraph transforms it into an inline editor for heavy proofreading sessions.

### 3. Offline Comic/CBZ Reader
- **Engine Features:** Right-to-Left (Manga) reading direction, Double-Page Spreads (landscape view of two pages side-by-side), and Background Image Preloading (loading the next 3 pages into memory to guarantee zero-latency page turns).
- **Real-Time Scaling vs. "Remastering":**
  - **Real-Time:** Uses GTK4's built-in GPU acceleration for fast, battery-efficient reading.
  - **The "Remaster" Tool:** Many old CBZs have terrible 600px scans. The Library will feature a heavy "Enhance Comic" tool. It spins up a background thread, unpacks the CBZ, and runs every image through an ultra-high-quality CPU scaler (Lanczos3 via Rust's `image` crate) to preserve crisp black ink while smoothing screentones. It then repacks it into a new, permanently remastered high-res CBZ.

## Pillar 2: Library Organization & UI
Calibre-level power wrapped in a Zen, un-bloated Moku design. No massive spreadsheets or intimidating data-entry forms.

### 1. Inline Metadata Editing (`book.rs`)
- The Book Details page will be interactive. The Title, Author, and Tags will not be static `gtk::Label`s.
- **Implementation:** Using `relm4`, we wrap labels in a `gtk::Stack`. Clicking the massive beautiful Title text instantly swaps it to a `gtk::Entry` box overlapping perfectly. You type the correction, hit Enter, and it swaps back to a label, instantly saving to `kalam.json`.
- **Tags:** A permanent `[ + ]` pill exists at the end of the Tag FlowBox. Clicking it opens a tiny inline entry to quickly append tags.

### 2. Dedicated Metadata Fetcher
- The current Open Library fetcher exists as an `in_app_dialog`. 
- Fetching covers, resolving authors, and picking the right edition is heavy work. This dialog will be elevated into a full, dedicated Page/Route so the user has maximum screen space to compare covers and metadata before hitting "Save".

### 3. Series Management ("Cover Stacks")
- **The Problem:** 15 *Wheel of Time* books clutter the grid.
- **The Solution:** Books belonging to the same series visually collapse into a single "Cover Stack" in the main Library grid (showing Book 1's cover with a subtle stacked-paper effect behind it).
- **Interaction:** Clicking the stack elegantly expands an overlay or horizontal row showing all books in strict reading order.

### 4. Custom Shelves (Manual + Smart)
- **Manual Shelves:** Traditional drag-and-drop playlists for curated collections (e.g., "Summer Reading").
- **Smart Shelves:** Driven by a powerful universal search bar. You type `tag:Fantasy status:Unread rating:>4`. The grid filters instantly. You click "Save to Sidebar", and it pins as a dynamic Smart Shelf that automatically updates as your library grows.

## Pillar 3: File Ingestion & Parsing
The goal is to hide the reality of terrible file formatting from the user.

### 1. The "Sanitizer" Pipeline
- **Problem:** Downloaded EPUBs (especially from fanfic exporters) have broken XML, no TOC, and hardcoded garbage CSS (e.g., `font-size: 8px`).
- **Solution:** When a user drags-and-drops a file into Kalam, it doesn't just copy it. It runs a quiet background worker that:
  1. Unzips the EPUB.
  2. Strips absolute CSS rules (fonts, line-heights, margins).
  3. Fixes broken XML tags.
  4. Auto-generates a TOC if one is missing (by scanning for `<h1>` and `<h2>`).
  5. Pre-extracts the cover for the DB.
  6. Repacks the file.
- The user only ever reads a perfectly formatted book.

### 2. Deep Content Search (Tantivy FTS)
- **Architecture:** Powered by Rust's `tantivy` (a blazing fast Lucene alternative).
- **Implementation:** By default, Kalam will run a background indexer not just on metadata, but on the *actual text contents* of every imported book.
- **Resource Footprint:** 
  - An index for 10,000 books takes ~2GB to 3GB of disk space.
  - Initial indexing will peg the CPU for 5-10 minutes.
- **Payoff:** The ability to search across a 10,000-book library for a specific character name or quote in 10-20 milliseconds. The ultimate killer feature for power readers.
