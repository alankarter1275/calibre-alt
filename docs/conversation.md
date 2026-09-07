# Kalam Vision & Design Discussions

## The Core Identity (Established Sept 2026)
- **Target Audience:** Avid readers who want offline ebook management, a stellar reader, an editor (planned), and access to online content.
- **The Vibe:** Calm, zen, tranquil. "Like sitting and reading in a park at the evening, or sitting and reading in a library near a window with natural sunlight coming in."
- **First Impression:** Pure reading. The app should feel like a place for reading first and foremost, not a busy management suite or a noisy online hub.
- **Separation of Concerns:** Clear boundary between the Offline (Library/Reading) and Online (Browsing/Downloading) experiences. Online features should lack noisy elements like comments to maintain the tranquil environment.

## Current Roadmap Redesign (In Progress)
- Addressing the cluttered UI/navigation.
- Redesigning the home screen to prioritize the current reading experience.
- Structuring the app to hide management/online tools when the user just wants to read.

### Decision 1: Top-Level Navigation
- **Outcome:** The "Two Worlds" Toggle. The app opens purely to the offline library/current reads. A distinct button (e.g., a "Globe" or "Discover" icon) switches the entire app into a separate "Online" mode. You flip back to the peaceful offline library when done hunting.

### Decision 2: The Default Screen (Offline)
- **Outcome:** The specific UI design of the home screen is deferred to the final Phase 5.5 (UI Overhaul). However, the established principle is that it will prioritize a tranquil "Current Reading" experience over an overwhelming management/library grid.

### Decision 3: Roadmap Sequence
- **Outcome:** The roadmap is reprioritized to focus on the core "Offline Reading Engines" first before expanding the online/scraping features. 
- **New Sequence:**
  1. Polish existing readers (specifically the Comic Book Reader needs attention).
  2. PDF Reader (P10).
  3. EPUB Polish & Offline Editor/Tools (P11).
  4. Resume remaining online scrapers (Literotica, FFN) and the Download Queue Hub.
  5. The final UI Overhaul (P5.5) to implement the "Two Worlds" Zen design.

### Area for Improvement: Comic Book Reader
The current Comic Book Reader requires a significant overhaul before the final UI phase:
1. **Performance & Stability:** Currently struggles with large images, jankiness, or high memory/CPU usage. Needs a buttery-smooth, optimized rendering pipeline.
2. **UI & Navigation:** Top/bottom bars and sliders are intrusive. Needs to be stripped down to a minimalist, distraction-free "tranquil" reading mode.
3. **Format Support:** Lacks vertical Webtoon/Long-strip scrolling support.

### Area for Improvement: EPUB Reader & Editor
- Currently discussing how to balance a powerful offline editor with the minimalist, zen reading experience.
- **Editor Integration:** Metadata editing will remain a completely separate UI screen (to be redesigned later). However, **Text Editing / Typos** will be inline. You should be able to fix the actual book text seamlessly while reading.
- **Text Editing Mechanics:**
  - **Quick Fixes:** Highlight a word/sentence, click "Edit" from the context menu, and fix the typo directly inline.
  - **Deep Editor:** Deferred for now. Will be brainstormed later.
  - **Inline Editing Architecture (The "Patch" System):**
    - **Non-destructive by default:** When a user fixes a typo inline, the original EPUB is NOT modified. The fix is saved as a "patch" or "delta" in the `kalam.json` sidecar file, which the reader applies on the fly.
    - **Baking changes:** Later, when the Deep Editor (Studio) is built, it will include an option to "commit" or "bake" these pending patches permanently into the EPUB file. This keeps reading fast/safe and defers heavy file I/O.

### Area for Improvement: PDF Reader (P10)
- Currently discussing how to handle rigid, fixed-layout PDFs within a "tranquil" reading environment.
- **PDF Handling Strategy:** 
  - **Proposed Hybrid:** Reflow Engine as default, falling back to Smart Crop Viewer.
  - **Pending Technical Validation:** Assessing if a full Reflow Engine adds too much bloat/weight to the app.
- **PDF Architecture Decision:**
  - **Core:** We will build a lightweight, Zathura-style viewer with Smart Crop and Color Inversion as the default.
  - **Reflow:** We will build the Reflow Engine, but it will be toggled OFF by default (and potentially architected as an optional modular plugin) to keep the baseline app fast and lightweight.

### Area for Improvement: Online Scrapers (Literotica & FFN)
- Moving to discuss how to handle the remaining web sources, particularly addressing their unique technical hurdles (Cloudflare on FFN, multi-page chapters and authentication on Literotica).
- **Scraper Roadmap Restructuring:**
  - Fanfiction sites are fundamentally different from manga sites. They require bespoke UI (tags, author pages, complex metadata).
  - AO3 is currently incomplete (missing tags, author pages, etc.).
  - **Decision:** Each major site gets its own dedicated development phase.
  - **Fiction Priorities:** 1. AO3, 2. Literotica (skip login for now), 3. FFN, 4. RoyalRoad.
  - **Comics Priorities:** MangaDex and WeebCentral need dedicated polish phases. Explore adding Manhwa and Western Comics (Marvel, DC) sources.
  - **New Sources:** Add a Manhwa source phase now. Add a Western Comics source phase for later.
  - **The Convergence Principle:** While the "Discover/Browse" UI must be highly customized for each source (to support unique tags, authors, filters), the destination is always unified. All text sources funnel into the unified EPUB Reader, and all image sources funnel into the unified Comic Reader.

### Area for Improvement: Multiple Books/Readers Open
- Recalling a previously parked topic: The ability to have multiple books or readers open at the same time.
- **Multiple Readers (The "Bubble" Architecture):**
  - **No visible tabs or OS windows.** To maintain tranquility, only one book is visible at a time.
  - **Instant Switching (Bubbles):** Within the reader, a hidden left sidebar allows you to search your library/online sources. Opening a new book replaces the current one on screen, but the previous book is kept alive in a background "bubble" (in memory, maintaining its exact DOM/scroll state).
  - **Result:** You can have multiple books open simultaneously and switch between them instantly from the sidebar, without cluttering the reading interface with browser tabs.
  - **The Floating Bubble (Outside Reader):** When the main reader is minimized (e.g. you are browsing the library), the active books collapse into a literal floating "bubble" UI (like Android chat heads). Clicking the bubble opens a mini, floating, read-only window with your active books lined up at the top for instant switching.
  - **Memory Optimization:** To support multiple open books without massive RAM usage, all EPUB books will share a single WebKit process (either via a shared `WebContext` or a single `WebView` using DOM swapping).

## Final Cleanup
- **Save vs Download:** Re-confirmed prior decision: Merged into a single "Add to Library" action. No separate screens.
- **Account / Logins:** Confirmed not needed for now. Pushed out of active scope.
- **Session Concluded:** The /grill-me session successfully re-architected the Master Roadmap and UI flow.

## Technical Safeguards & Suggestions (Sept 8)
- **The Wayland Caveat (Floating Bubbles):** Pure OS-level floating widgets are heavily restricted on Linux Wayland. The "Floating Bubble" will be implemented as an in-app overlay using `gtk::Overlay` so it floats freely *inside* the Kalam window without fighting the OS.
- **The Locator Problem (Inline Editing):** Saving patches as "page 14" is fragile. Patches will use **Text-Quote Anchors** or **EPUB CFI** (Canonical Fragment Identifiers) to anchor edits to the actual text content, ensuring edits survive font-size changes or resizing.
- **PDF Reflow Engine:** Will rely on battle-tested external logic (like `k2pdfopt`) rather than attempting to write complex layout-analysis algorithms from scratch in Rust.

## Plugin Architecture Decision (Sept 8)
- **WebAssembly (Wasm) replaces Lua:** To solve "scraper rot" and support extensible metadata (like Calibre), Kalam will implement a Wasm-based plugin system.
- **Scope:** This Wasm architecture will power both the **Online Scrapers** (Phase 7) and the **Metadata Providers** (Phase 11).
- **Host-Delegated Networking:** Wasm plugins will run in a secure sandbox. They will ask the Host (Kalam) to perform HTTPS requests. Kalam handles the networking (and Cloudflare/cookies), while the tiny Wasm plugin handles the HTML/JSON parsing. This ensures ultimate security, near-native speed, and very small plugin file sizes.
- **Language:** Plugins can be written in pure Rust (compiled to `wasm32`), maintaining our strong-typing preference while allowing OTA updates without recompiling the main app.
