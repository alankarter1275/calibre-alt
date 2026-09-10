import re

with open('ROADMAP.md', 'r') as f:
    content = f.read()

# Find the start of the Redux section
split_idx = content.find("## Master Roadmap Redux")

if split_idx != -1:
    new_content = content[:split_idx] + """## Master Roadmap Redux (Updated 2026-09-08 via /grill-me)

**The New "Offline Library First" Sequence:**
We have officially halted all online scraper work (P7/P9) and removed hardcoded scrapers to prioritize the ultimate offline reading sanctuary.

1. **P1.5 - Library Organization & UI:**
   - **Inline Editing (`book.rs`):** `gtk::Stack` to allow click-to-type inline metadata editing for Titles and Authors.
   - **Cover Stacks & Smart Shelves:** Visually collapse books in a series. Implement dynamic Smart Shelves.
   - **Dedicated Metadata Fetcher:** Elevate the Open Library `in_app_dialog` to a full route.
2. **P10 - PDF Engine:** Zathura-style smart crop default (auto-detect ink bounds). Reflow engine toggle using heuristic text extraction via PDFium.
3. **P11 - EPUB Inline Editor:** Build a Rust-proxy sidecar patch system (`kalam.json`). Fix typos non-destructively on-the-fly without altering original `.epub`.
4. **P8.5 - Polish Comics Reader:** Add right-to-left manga mode, background image preloading.
5. **P8.6 - The Remaster Tool:** A heavy offline tool to permanently upscale low-res CBZs using CPU Lanczos3 interpolation.
6. **P3.5 - The Sanitizer Pipeline:** A background worker that intercepts imported EPUBs, strips hardcoded CSS, fixes broken XML, and repacks them cleanly.
7. **P3.6 - Deep Content Search:** Implement a default `tantivy` indexer to provide 20ms full-text search across the actual contents of the entire library.
8. **P6 - Download Queue / Sync Hub.**
9. **P5.5 - UI Overhaul:** "Two Worlds" design (Offline Library vs Online Hub).
10. **P12 - Wasm Plugin Ecosystem:** Build the WebAssembly host architecture to power dynamic scrapers (`wit/kalam.wit`), officially replacing hardcoded Rust scrapers.
11. **P7/P9 - Reintroduce Online Sources:** Write Wasm plugins for AO3, MangaDex, etc., and pair them with bespoke hardcoded GTK UI "Husks" inside Kalam.
"""
    with open('ROADMAP.md', 'w') as f:
        f.write(new_content)
    print("Patched successfully")
else:
    print("Could not find section")
