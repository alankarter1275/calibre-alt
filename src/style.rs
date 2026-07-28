//! The shape of the interface: spacing, radii, type scale, borders.
//!
//! Colours are **not** defined here. `theme.rs` prepends an `@define-color`
//! block at runtime, so every rule below must refer to colours by `@kalam_*`
//! name. A literal hex here is a bug unless it is deliberately theme-independent
//! (the highlight marker palette, the reader's paper swatches, and the reader
//! stage) — those are commented where they appear.

pub const APP_CSS: &str = r#"
/* ── window ─────────────────────────────────────────── */
window.kalam-window {
    background: @kalam_bg;
}

/* ── scrollbars ─────────────────────────────────────── */
/* Deliberately minimal.
 *
 * Five attempts at a custom thin overlay scrollbar each produced
 *     *** BUG *** In pixman_region32_init_rect: Invalid rectangle passed
 * in varying numbers. GTK4 allocates a collapsed overlay scrollbar only a few
 * pixels wide, and anything that shrinks the slider further — margins, padding,
 * a smaller min-width — risks a negative allocation, which is what pixman
 * rejects. Restyling geometry here is not worth a stream of runtime errors.
 *
 * So: colour only. No margin, no padding, no size overrides. GTK's own
 * geometry is left completely alone. */
scrollbar slider {
    background: alpha(@kalam_text_dim, 0.4);
    border-radius: 999px;
}

scrollbar slider:hover {
    background: alpha(@kalam_text_dim, 0.7);
}

scrollbar slider:active {
    background: @kalam_accent;
}

scrollbar trough {
    background: transparent;
    border: none;
}

/* ── slim sidebar ───────────────────────────────────── */
.kalam-sidebar {
    background: @kalam_sidebar;
    border-right: 1px solid @kalam_border;
    /* Icon-only rail: 34px disc + 7px either side. */
    min-width: 48px;
    padding: 8px 0;
}

.kalam-sidebar-inner {
    padding: 0 6px;
}

.kalam-brand {
    padding: 2px 0 10px 0;
}

/* No size here on purpose. The exact size is set in code with
   gtk::Image::set_pixel_size; a CSS min-width would only act as a floor and
   could push the mark larger again. */

.kalam-nav-btn {
    padding: 0;
    /* A circle, not a rounded square: width and height match, so a fully
       round radius reads as a disc behind the glyph. */
    border-radius: 999px;
    margin-top: 4px;
    margin-bottom: 4px;
    background: transparent;
    border: none;
    color: @kalam_text_dim;
    min-width: 34px;
    min-height: 34px;
}

.kalam-nav-btn:hover {
    background: @kalam_surface_2;
    color: @kalam_text;
}

.kalam-nav-btn.active {
    background: @kalam_accent_dim;
    color: @kalam_accent;
    font-weight: 600;
}

.kalam-nav-icon {
    font-size: 1.25rem;
}

.kalam-nav-spacer {
    min-height: 0;
}

/* ── main column ────────────────────────────────────── */
.kalam-main {
    background: @kalam_bg;
}

.kalam-topbar {
    background: @kalam_surface;
    border-bottom: 1px solid @kalam_border;
    padding: 10px 18px;
    min-height: 52px;
}

.kalam-topbar-title {
    font-size: 1.05rem;
    font-weight: 600;
    color: @kalam_text;
}

.kalam-topbar-subtitle {
    font-size: 0.8rem;
    color: @kalam_text_dim;
}

.kalam-content {
    padding: 20px 24px;
    background: @kalam_bg;
    min-width: 0;
}

/* ── breadcrumbs / back ─────────────────────────────── */
.kalam-back-btn {
    padding: 6px 14px;
    border-radius: 999px;
    background: @kalam_surface_2;
    color: @kalam_text;
    border: 1px solid @kalam_border;
    font-size: 0.85rem;
}

.kalam-back-btn:hover {
    background: @kalam_border;
}

/* ── page headings ──────────────────────────────────── */
.kalam-page-title {
    font-size: 1.6rem;
    font-weight: 700;
    color: @kalam_text;
    margin-bottom: 4px;
}

.kalam-page-sub {
    font-size: 0.9rem;
    color: @kalam_text_dim;
    margin-bottom: 18px;
}

/* ── cards (shelves grid, book cards, hub tiles) ─────── */
.kalam-card {
    background: @kalam_surface;
    border: 1px solid @kalam_border;
    border-radius: 14px;
    padding: 16px;
    min-height: 96px;
}

.kalam-card:hover {
    border-color: @kalam_accent;
    background: @kalam_surface_2;
}

.kalam-card-title {
    font-size: 1.05rem;
    font-weight: 600;
    color: @kalam_text;
}

.kalam-card-meta {
    font-size: 0.82rem;
    color: @kalam_text_dim;
    margin-top: 4px;
}

.kalam-card-badge {
    font-size: 0.75rem;
    font-weight: 600;
    color: @kalam_accent;
    background: @kalam_accent_dim;
    border-radius: 999px;
    padding: 2px 8px;
}

/* ── hub tiles inside My Library ────────────────────── */
.kalam-hub-grid {
    margin-top: 8px;
}

.kalam-hub-tile {
    background: @kalam_surface;
    border: 1px solid @kalam_border;
    border-radius: 14px;
    padding: 18px 14px;
    min-width: 120px;
}

.kalam-hub-tile:hover {
    border-color: @kalam_accent;
}

.kalam-hub-tile-icon {
    font-size: 1.4rem;
    margin-bottom: 6px;
}

.kalam-hub-tile-label {
    font-size: 0.88rem;
    font-weight: 600;
    color: @kalam_text;
}

.kalam-hub-tile-count {
    font-size: 1.25rem;
    font-weight: 700;
    color: @kalam_accent;
    margin-bottom: 2px;
}

.kalam-hub-tile-meta {
    font-size: 0.75rem;
    color: @kalam_text_dim;
}

/* ── book row in shelf list ─────────────────────────── */
.kalam-cover-frame {
    border-radius: 6px;
    background: @kalam_surface_2;
    padding: 0;
    margin: 0;
    border: none;
    box-shadow: 0 2px 10px alpha(#000, 0.35);
}

.kalam-cover-placeholder {
    background: linear-gradient(160deg, @kalam_surface_2 0%, @kalam_surface 100%);
    border-radius: 6px;
}

.kalam-cover-img {
    border-radius: 6px;
}

.kalam-progress {
    font-size: 0.75rem;
    color: @kalam_accent;
}

/* ── book detail page ───────────────────────────────── */
.kalam-detail-cover {
    border-radius: 10px;
}

.kalam-detail-title {
    font-size: 1.7rem;
    font-weight: 700;
    color: @kalam_text;
}

.kalam-detail-author {
    font-size: 1.05rem;
    color: @kalam_accent;
    margin-bottom: 8px;
}

.kalam-detail-section-title {
    font-size: 0.78rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    color: @kalam_text_dim;
    margin-top: 14px;
    margin-bottom: 4px;
}

.kalam-chip {
    background: @kalam_surface_2;
    color: @kalam_text;
    border-radius: 999px;
    padding: 4px 10px;
    font-size: 0.78rem;
    border: 1px solid @kalam_border;
}

.kalam-primary-btn {
    background: @kalam_accent;
    color: @kalam_bg;
    font-weight: 700;
    border-radius: 999px;
    padding: 10px 22px;
    border: none;
}

.kalam-primary-btn:hover {
    filter: brightness(1.08);
}

.kalam-secondary-btn {
    background: @kalam_surface_2;
    color: @kalam_text;
    border-radius: 999px;
    padding: 10px 18px;
    border: 1px solid @kalam_border;
}

.kalam-secondary-btn:hover {
    border-color: @kalam_accent;
}

/* ── empty / placeholder ────────────────────────────── */
.kalam-placeholder {
    color: @kalam_text_dim;
    font-size: 0.95rem;
    padding: 24px;
    border: 1px dashed @kalam_border;
    border-radius: 12px;
    background: alpha(@kalam_surface, 0.5);
}

.kalam-muted {
    color: @kalam_text_dim;
}

.kalam-home-continue {
    background: @kalam_surface;
    border: 1px solid @kalam_border;
    border-radius: 16px;
    padding: 20px;
}

.kalam-section-label {
    font-size: 0.75rem;
    font-weight: 700;
    letter-spacing: 0.07em;
    color: @kalam_text_dim;
    margin-bottom: 10px;
    margin-top: 8px;
}

/* ── cover cards (Goodreads-style bookshelf grid) ───── */
.kalam-book-grid {
    margin-top: 4px;
}

.kalam-book-grid-shell {
}

.kalam-book-card {
    padding: 0;
    border-radius: 0;
    background: transparent;
}

.kalam-book-card:hover .kalam-cover-frame {
    box-shadow: 0 6px 18px alpha(#000, 0.5);
}

.kalam-book-card-title {
    font-weight: 600;
    font-size: 0.78rem;
    color: @kalam_text;
    min-width: 0;
}

.kalam-book-card-author {
    font-size: 0.7rem;
    color: @kalam_text_dim;
    min-width: 0;
}

/* ── floating book panel (Suwayomi-style) ───────────── */
window.kalam-float-window {
    background: @kalam_surface;
    border-radius: 16px;
    border: 1px solid @kalam_border;
    box-shadow: 0 24px 64px alpha(#000, 0.55);
}

.kalam-float {
    background: @kalam_surface;
    padding: 0;
    min-width: 820px;
    min-height: 480px;
}

.kalam-float-cover-col {
    background: @kalam_sidebar;
    border-right: 1px solid @kalam_border;
    padding: 16px 14px 12px 14px;
}

.kalam-float-cover-host {
    border-radius: 10px;
    min-height: 0;
}

.kalam-float-cover {
    border-radius: 10px;
}

.kalam-float-side-actions {
    margin-top: 10px;
}

.kalam-float-side-btn {
    background: transparent;
    border: none;
    border-radius: 999px;
    color: @kalam_text_dim;
    padding: 8px 10px;
    font-size: 0.84rem;
}

.kalam-float-side-btn:hover {
    background: @kalam_surface_2;
    color: @kalam_text;
}

.kalam-float-right {
    background: @kalam_surface;
}

.kalam-float-header {
    padding: 16px 18px 12px 20px;
}

.kalam-float-title {
    font-size: 1.45rem;
    font-weight: 700;
    color: @kalam_text;
}

.kalam-float-close {
    min-width: 34px;
    min-height: 34px;
    padding: 0;
    border-radius: 999px;
    background: transparent;
    border: none;
    color: @kalam_text_dim;
    font-size: 1.05rem;
}

.kalam-float-close:hover {
    background: @kalam_surface_2;
    color: @kalam_text;
}

.kalam-float-body {
    padding: 4px 22px 12px 20px;
}

.kalam-float-footer {
    padding: 10px 20px 16px 20px;
    border-top: 1px solid @kalam_border;
}

.kalam-float-read {
    padding: 11px 26px;
    font-size: 0.98rem;
    min-width: 120px;
}

.kalam-float-desc {
    font-size: 0.9rem;
    color: @kalam_text_dim;
    line-height: 1.5;
}

.kalam-float-meta-key {
    font-size: 0.7rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    color: @kalam_text_dim;
}

.kalam-float-meta-val {
    font-size: 0.9rem;
    color: @kalam_text;
}

.kalam-badge-format {
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    color: @kalam_bg;
    background: @kalam_success;
    border-radius: 999px;
    padding: 3px 8px;
}

.kalam-badge-unread {
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    color: @kalam_bg;
    background: @kalam_warning;
    border-radius: 999px;
    padding: 3px 8px;
}

.kalam-badge-progress {
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    color: @kalam_bg;
    background: @kalam_accent;
    border-radius: 999px;
    padding: 3px 8px;
}

.kalam-badge-done {
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    color: @kalam_text;
    background: @kalam_surface_2;
    border-radius: 999px;
    padding: 3px 8px;
}

/* ── reader (immersive tablet-book) ─────────────────── */
.kalam-content.kalam-content-reader {
    padding: 0;
}

/* The reader stage stays near-black in every theme on purpose: it is the
   letterbox around the page, and tinting it would fight whichever paper
   colour (light / sepia / dark) the reader itself is set to. */
.kalam-reader {
    background: #0a0a0b;
}

.kalam-reader-stage {
    background: #0a0a0b;
}

.kalam-reader-top-float {
    background: rgba(28, 25, 23, 0.72);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 999px;
    padding: 4px 10px 4px 4px;
    box-shadow: 0 8px 28px rgba(0, 0, 0, 0.35);
}

.kalam-reader-crumb {
    font-size: 0.8rem;
    font-weight: 500;
    color: rgba(250, 250, 249, 0.88);
    padding-right: 6px;
}

.kalam-reader-pill {
    background: rgba(28, 25, 23, 0.88);
    border: 1px solid rgba(255, 255, 255, 0.10);
    border-radius: 999px;
    padding: 6px 10px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.45);
}

.kalam-reader-pill-btn {
    min-width: 36px;
    min-height: 36px;
    padding: 0 12px;
    border-radius: 999px;
    border: none;
    background: transparent;
    color: #f5f5f4;
    font-weight: 600;
    font-size: 0.95rem;
}

.kalam-reader-pill-btn:hover {
    background: rgba(255, 255, 255, 0.12);
}

/* ensure MenuButton inner button is also capsule and hover is tight — same size as button */
.kalam-reader-pill menubutton.kalam-reader-pill-btn {
    min-width: 0;
    min-height: 0;
    padding: 0;
    border-radius: 999px;
    background: transparent;
}

.kalam-reader-pill menubutton.kalam-reader-pill-btn > button {
    min-width: 36px;
    min-height: 36px;
    padding: 0 12px;
    border-radius: 999px;
}

.kalam-reader-pill menubutton.kalam-reader-pill-btn:hover {
    background: transparent;
}

.kalam-reader-pill menubutton.kalam-reader-pill-btn > button:hover {
    background: rgba(255, 255, 255, 0.12);
}

.kalam-reader-pill-meta {
    font-size: 0.78rem;
    font-weight: 600;
    color: rgba(231, 229, 228, 0.75);
    padding: 0 6px;
    min-width: 3.5rem;
}

/* Popovers — no thick borders */
.kalam-reader-popover {
    padding: 4px;
    background: @kalam_surface;
    border: none;
    outline: none;
    border-radius: 14px;
    box-shadow: 0 16px 40px alpha(#000, 0.45);
}

popover.kalam-reader-popover {
    border: none;
    box-shadow: none;
    background: transparent;
    padding: 0;
}

popover.kalam-reader-popover > contents {
    border: none;
    border-radius: 14px;
    box-shadow: 0 16px 40px alpha(#000, 0.55);
    background: @kalam_surface;
    padding: 4px;
}

.kalam-reader-popover-title {
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    color: @kalam_text_dim;
}

.kalam-reader-theme-btn {
    border-radius: 10px;
    padding: 10px 14px;
    font-size: 0.9rem;
    border: 1px solid @kalam_border;
}

.kalam-reader-theme-btn:hover {
    border-color: @kalam_accent;
}

/* The tick keeps its space even when inactive so the row never reflows.
   Hidden with a transparent colour, not `opacity`, which would force GTK to
   render through an offscreen surface (see the scrollbar note above). */
.kalam-theme-tick {
    font-weight: 700;
}

.kalam-theme-tick-off {
    color: transparent;
}

/* Theme swatches — these MUST match ReadingTheme::swatch() in epub_book.rs so
   the button previews the page it produces. */
.kalam-theme-light {
    background: #faf8f5;
    color: #1c1917;
}

.kalam-theme-light label {
    color: #1c1917;
}

.kalam-theme-sepia {
    background: #f4ecd8;
    color: #3e3226;
}

.kalam-theme-sepia label {
    color: #3e3226;
}

.kalam-theme-dark {
    background: #1a1b1e;
    color: #e7e5e4;
}

.kalam-theme-dark label {
    color: #e7e5e4;
}

.kalam-reader-size-value {
    font-size: 0.9rem;
    font-weight: 600;
    color: @kalam_text;
}

.kalam-toc-item {
    background: transparent;
    border: none;
    border-radius: 999px;
    color: @kalam_text;
    padding: 8px 14px;
    font-size: 0.88rem;
}

.kalam-toc-item:hover {
    background: @kalam_surface_2;
}

/* ── P3: annotations, quotes, words, dict ──────────── */
.kalam-quote-row, .kalam-word-row, .kalam-dict-row {
    background: @kalam_surface;
    border: 1px solid @kalam_border;
    border-radius: 12px;
    padding: 12px 14px;
}

.kalam-quote-text {
    font-size: 0.95rem;
    line-height: 1.5;
    color: @kalam_text;
    font-style: italic;
}

.kalam-word-title {
    font-size: 1.05rem;
    font-weight: 700;
    color: @kalam_text;
}

.kalam-badge-yellow {
    background: #fef08a;
    color: #1a1a12;
}
.kalam-badge-green {
    background: #bbf7d0;
    color: #0e1a12;
}
.kalam-badge-blue {
    background: #bfdbfe;
    color: #0e141e;
}
.kalam-badge-pink {
    background: #fbcfe8;
    color: #1e1216;
}
.kalam-badge-orange {
    background: #fed7aa;
    color: #1e1410;
}

/* ══════════════════════════════════════════════════════
   P4 — shelves, lists, history, tags, analytics
   ══════════════════════════════════════════════════════ */

/* ── shelf cards ────────────────────────────────────── */
.kalam-shelf-card {
    min-height: 120px;
}

.kalam-badge-smart {
    background: alpha(@kalam_accent, 0.18);
    color: @kalam_accent;
}

.kalam-badge-manual {
    background: @kalam_surface_2;
    color: @kalam_text_dim;
}

/* ── compact row buttons (reorder, remove, read) ────── */
.kalam-mini-btn {
    padding: 4px 10px;
    font-size: 0.78rem;
    border-radius: 999px;
    background: @kalam_surface_2;
    color: @kalam_text;
    border: 1px solid @kalam_border;
    min-height: 0;
}

.kalam-mini-btn:hover {
    border-color: @kalam_accent;
}

.kalam-mini-btn:disabled {
    opacity: 0.35;
}

.kalam-mini-btn-danger:hover {
    border-color: @kalam_danger;
    color: @kalam_danger;
}

/* ── generic list rows (reading list, history) ──────── */
.kalam-list-row {
    background: @kalam_surface;
    border: 1px solid @kalam_border;
    border-radius: 12px;
    padding: 10px 14px;
}

.kalam-list-row:hover {
    border-color: @kalam_accent;
    background: @kalam_surface_2;
}

.kalam-list-ordinal {
    font-size: 0.95rem;
    font-weight: 700;
    color: @kalam_text_dim;
}

/* ── manual shelf management strip ──────────────────── */
.kalam-manage-expander {
    margin-top: 14px;
    color: @kalam_text_dim;
    font-size: 0.85rem;
}

.kalam-manage-row {
    padding: 6px 10px;
    border-radius: 8px;
    background: alpha(@kalam_surface, 0.6);
}

/* ── shelf editor / rule builder ────────────────────── */
.kalam-shelf-editor entry {
    border-radius: 8px;
    min-height: 32px;
}

.kalam-rule-row {
    background: @kalam_surface;
    border: 1px solid @kalam_border;
    border-radius: 10px;
    padding: 8px 10px;
}

.kalam-rule-remove {
    padding: 2px 8px;
    border-radius: 999px;
    background: transparent;
    color: @kalam_text_dim;
    border: none;
    min-height: 0;
}

.kalam-rule-remove:hover {
    color: @kalam_danger;
}

.kalam-rule-count {
    font-size: 0.82rem;
    font-weight: 600;
    color: @kalam_accent;
}

.kalam-error-text {
    color: @kalam_danger;
    font-size: 0.85rem;
}

.kalam-picker-row {
    padding: 6px 8px;
    border-radius: 8px;
}

.kalam-picker-row:hover {
    background: @kalam_surface_2;
}

/* ── history event icons ────────────────────────────── */
.kalam-event-icon {
    font-size: 0.95rem;
    font-weight: 700;
}

.kalam-event-finished {
    color: @kalam_success;
}

.kalam-event-opened {
    color: @kalam_accent;
}

.kalam-event-imported {
    color: @kalam_warning;
}

/* ── tag cloud ──────────────────────────────────────── */
.kalam-tag-chip {
    border-radius: 999px;
    background: @kalam_surface;
    border: 1px solid @kalam_border;
    padding: 6px 14px;
    color: @kalam_text;
}

.kalam-tag-chip:hover {
    border-color: @kalam_accent;
    background: @kalam_surface_2;
}

/* Weight classes scale the chip with how many books carry the tag. */
.kalam-tag-w1 { font-size: 0.8rem; }
.kalam-tag-w2 { font-size: 0.9rem; }
.kalam-tag-w3 { font-size: 1.0rem; font-weight: 600; }
.kalam-tag-w4 {
    font-size: 1.1rem;
    font-weight: 700;
    border-color: alpha(@kalam_accent, 0.5);
}

.kalam-tag-count {
    font-size: 0.72rem;
    color: @kalam_text_dim;
    background: @kalam_surface_2;
    border-radius: 999px;
    padding: 1px 7px;
}

/* ── analytics ──────────────────────────────────────── */
/* ── library dashboard ──────────────────────────────── */
.kalam-section-header {
    padding: 2px 0;
}

.kalam-section-header:hover .kalam-section-arrow,
.kalam-section-header:hover .kalam-section-meta {
    color: @kalam_accent;
}

.kalam-section-meta {
    font-size: 0.78rem;
    color: @kalam_text_dim;
}

.kalam-section-arrow {
    font-size: 1.1rem;
    color: @kalam_text_dim;
}

.kalam-shelf-mini {
    background: @kalam_surface;
    border: 1px solid @kalam_border;
    border-radius: 12px;
    padding: 12px 14px;
    min-height: 84px;
}

.kalam-quote-card {
    background: @kalam_surface;
    border: 1px solid @kalam_border;
    border-left: 3px solid @kalam_accent;
    border-radius: 12px;
    padding: 14px 16px;
}

.kalam-quote-text {
    font-size: 0.88rem;
    color: @kalam_text;
}

.kalam-quote-source {
    font-size: 0.76rem;
    color: @kalam_text_dim;
}

.kalam-progress-cell {
    padding: 0;
}

.kalam-mini-progress {
    min-height: 4px;
}

.kalam-mini-progress trough {
    min-height: 4px;
    border-radius: 999px;
    background: @kalam_border;
}

/* `min-width` matters here. At fraction 0 the progress node is allocated zero
   width, and because the rounded corners make GTK clip it, that becomes a
   zero-area clip rectangle which pixman refuses:
       *** BUG *** In pixman_region32_init_rect: Invalid rectangle passed
   Giving it a floor means an unread book shows a small dot instead of nothing,
   which also reads better than an invisible bar. */
.kalam-mini-progress progress {
    min-height: 4px;
    min-width: 4px;
    border-radius: 999px;
    background: @kalam_accent;
}

/* ── toasts (docs/design/notifications.png) ─────────── */
.kalam-toast-host {
    margin: 0 18px 18px 0;
}

.kalam-toast {
    background: @kalam_surface;
    border: 1px solid @kalam_border;
    border-radius: 12px;
    box-shadow: 0 12px 32px alpha(#000, 0.55);
    min-width: 340px;
}

.kalam-toast-body {
    padding: 14px 18px 14px 14px;
}

/* Highlighting and quoting recur constantly while reading, so those get a
   slimmer, quieter card rather than a full-sized toast over the page. */
.kalam-toast-compact {
    min-width: 210px;
}

.kalam-toast-compact .kalam-toast-body {
    padding: 7px 14px 7px 10px;
}

.kalam-toast-compact .kalam-toast-title {
    font-size: 0.82rem;
    font-weight: 500;
}

.kalam-toast-compact .kalam-toast-icon {
    font-size: 0.9rem;
}

.kalam-toast-accent-compact {
    margin-top: 6px;
    margin-bottom: 6px;
    min-height: 12px;
}

.kalam-toast-title {
    font-size: 0.92rem;
    font-weight: 600;
    color: @kalam_text;
}

.kalam-toast-detail {
    font-size: 0.8rem;
    color: @kalam_text_dim;
}

.kalam-toast-icon {
    font-size: 1.05rem;
}

/* The accent stripe carries the severity. Colours sampled straight out of
   docs/design/notifications.png: the reference uses muted, desaturated tones,
   not the bright mint/salmon this first shipped with. A rounded pill inset
   from the card edge, not a full-height bar. */
.kalam-toast-accent {
    border-radius: 999px;
    /* Tucked in close to the card's left edge, but never touching it. */
    margin-left: 6px;
    /* Vertical inset: leaves the bar at ~3/5 of the card height, centred,
       and lets it grow with the card instead of being pinned to a pixel
       height that a taller toast would break. */
    margin-top: 13px;
    margin-bottom: 13px;
    min-width: 5px;
    /* Floor, so a short title-only toast still shows a real bar rather than
       a speck once the insets are taken out. */
    min-height: 18px;
}

.kalam-toast-success .kalam-toast-accent {
    background: @kalam_success;
}

.kalam-toast-success .kalam-toast-icon {
    color: @kalam_success;
}

.kalam-toast-error .kalam-toast-accent {
    background: @kalam_danger;
}

.kalam-toast-error .kalam-toast-icon {
    color: @kalam_danger;
}

.kalam-toast-info .kalam-toast-accent {
    background: @kalam_border;
}

.kalam-toast-info .kalam-toast-icon {
    color: @kalam_text_dim;
}

.kalam-toast-progress .kalam-toast-accent {
    background: @kalam_info;
}

.kalam-toast-progress .kalam-toast-icon {
    color: @kalam_info;
}

/* ── theme picker ───────────────────────────────────── */
.kalam-theme-btn {
    padding: 0;
    background: transparent;
    border: none;
}

.kalam-theme-card {
    padding: 8px;
    border-radius: 10px;
    border: 1px solid @kalam_border;
    background: @kalam_surface;
}

.kalam-theme-card:hover {
    border-color: @kalam_text_dim;
}

.kalam-theme-card.active {
    border-color: @kalam_accent;
    background: @kalam_accent_dim;
}

/* The swatch strip previews sidebar / surface / raised / accent. It is clipped
   to its rounded corners, so it needs a non-zero floor: a clip rectangle with
   no area is what pixman rejects. */
.kalam-theme-strip {
    border-radius: 6px;
    border: 1px solid @kalam_border;
    min-width: 24px;
    min-height: 18px;
}

.kalam-theme-name {
    font-size: 0.72rem;
    font-weight: 500;
    color: @kalam_text_dim;
}

.kalam-theme-card.active .kalam-theme-name {
    color: @kalam_text;
    font-weight: 600;
}

/* One family per block, with a little air between blocks. */
.kalam-theme-family {
    margin-bottom: 6px;
}

.kalam-theme-family-name {
    font-size: 0.8rem;
    font-weight: 600;
    color: @kalam_text;
}

/* ── metadata editor ────────────────────────────────── */
/* Pinned below the scroller, so Save never scrolls out of reach. */
.kalam-dialog-actions {
    padding: 12px 18px;
    background: @kalam_surface;
    border-top: 1px solid @kalam_border;
}

.kalam-desc-scroll {
    border: 1px solid @kalam_border;
    border-radius: 8px;
    background: @kalam_surface;
}

/* Inline field-search icons (Nerd Font glyph). */
.kalam-icon-btn {
    font-family: "Symbols Nerd Font", "Symbols Nerd Font Mono",
                 "JetBrainsMono Nerd Font", "FiraCode Nerd Font", monospace;
    background: @kalam_surface_2;
    border: 1px solid @kalam_border;
    border-radius: 8px;
    color: @kalam_text_dim;
    padding: 0 10px;
    min-width: 0;
    min-height: 32px;
}

.kalam-icon-btn:hover {
    border-color: @kalam_accent;
    color: @kalam_accent;
}

/* Metadata source badges. */
.kalam-badge-ol {
    background: alpha(@kalam_success, 0.18);
    color: @kalam_success;
}

.kalam-badge-gb {
    background: alpha(@kalam_accent, 0.18);
    color: @kalam_accent;
}

.kalam-cover-choice {
    padding: 4px;
    border-radius: 8px;
    background: @kalam_surface_2;
    border: 1px solid @kalam_border;
    /* Letterboxed art sits on the surface colour rather than bare window. */
    min-width: 0;
    min-height: 0;
}

.kalam-cover-choice:hover {
    border-color: @kalam_accent;
}

/* Slide-out Open Library panel; separated from the form by its own edge. */
.kalam-search-panel {
    background: @kalam_surface;
    border-left: 1px solid @kalam_border;
    padding: 16px;
}

.kalam-metadata-side {
    border-left: 1px solid @kalam_border;
    padding-left: 16px;
}

.kalam-desc-view {
    background: transparent;
    padding: 8px;
    font-size: 0.86rem;
}

.kalam-desc-view text {
    background: transparent;
    color: @kalam_text;
}

/* ── ratings ────────────────────────────────────────── */
.kalam-stars {
    color: @kalam_warning;
    font-size: 0.95rem;
}

.kalam-stars-value {
    font-size: 0.76rem;
    color: @kalam_text_dim;
}

/* Glyphs are always drawn, so the control reads at a glance rather than
   only revealing itself on hover. Stars are Labels, not Buttons — see the
   comment in star_picker() for why. */
.kalam-star-glyph {
    font-size: 1.35rem;
    padding: 0 1px;
    color: @kalam_text_dim;
}

.kalam-star-on {
    color: @kalam_warning;
}

/* Applied by an EventControllerMotion, since Labels do not prelight. */
.kalam-star-hover {
    color: @kalam_warning;
}

.kalam-star-clear {
    background: transparent;
    border: none;
    box-shadow: none;
    color: @kalam_text_dim;
    font-size: 0.8rem;
    min-width: 0;
    min-height: 0;
    padding: 0 4px;
    margin-left: 4px;
}

.kalam-star-clear:hover {
    color: @kalam_danger;
}

/* ── streak strip ───────────────────────────────────── */
.kalam-streak-day {
    padding: 6px 2px;
    border-radius: 10px;
}

.kalam-streak-today {
    background: @kalam_surface_2;
    border: 1px solid @kalam_accent;
}

.kalam-streak-label {
    font-size: 0.72rem;
    color: @kalam_text_dim;
}

.kalam-streak-flame {
    font-size: 1rem;
}

/* Inactive days keep their slot but recede. */
.kalam-streak-off {
    opacity: 0.22;
}

.kalam-mini-btn-active {
    border-color: @kalam_accent;
    color: @kalam_accent;
}

.kalam-note-entry {
    font-size: 0.82rem;
    min-height: 28px;
    border-radius: 8px;
}

.kalam-hero-card {
    background: @kalam_surface;
    border: 1px solid @kalam_border;
    border-radius: 16px;
    padding: 14px 16px;
    min-height: 104px;
}

.kalam-hero-label {
    font-size: 0.78rem;
    font-weight: 600;
    color: @kalam_text_dim;
}

.kalam-hero-value {
    font-size: 2rem;
    font-weight: 700;
    color: @kalam_text;
}

.kalam-hero-unit {
    font-size: 0.8rem;
    color: @kalam_text_dim;
    margin-left: 2px;
}

.kalam-hero-blurb {
    font-size: 0.74rem;
    color: @kalam_text_dim;
}

/* Sparkline / line-chart colours come from `color` so the draw handler can
   read them back via widget.color() — keeps palette decisions in CSS. */
.kalam-sparkline {
    color: @kalam_accent;
}

.kalam-spark-green {
    color: @kalam_success;
}

.kalam-spark-red {
    color: @kalam_danger;
}

.kalam-spark-blue {
    color: @kalam_accent;
}

.kalam-chart-card {
    background: @kalam_surface;
    border: 1px solid @kalam_border;
    border-radius: 16px;
    padding: 16px 18px;
}

.kalam-chart-title {
    font-size: 1.4rem;
    font-weight: 700;
    color: @kalam_text;
}

.kalam-chart-sub {
    font-size: 0.8rem;
    color: @kalam_text_dim;
}

.kalam-linechart {
    color: @kalam_accent;
}

.kalam-stat-tile {
    background: @kalam_surface;
    border: 1px solid @kalam_border;
    border-radius: 12px;
    padding: 14px 16px;
}

.kalam-stat-value {
    font-size: 1.5rem;
    font-weight: 700;
    color: @kalam_text;
}

.kalam-stat-label {
    font-size: 0.75rem;
    letter-spacing: 0.05em;
    color: @kalam_text_dim;
}

.kalam-bar-chart {
    background: @kalam_surface;
    border: 1px solid @kalam_border;
    border-radius: 12px;
    padding: 14px 12px 10px 12px;
    min-height: 150px;
}

.kalam-bar {
    background: linear-gradient(180deg, @kalam_accent 0%, alpha(@kalam_accent, 0.55) 100%);
    border-radius: 5px 5px 2px 2px;
    min-width: 14px;
}

.kalam-bar-empty {
    background: @kalam_border;
}

.kalam-bar-value {
    font-size: 0.68rem;
    color: @kalam_text_dim;
}

.kalam-bar-label {
    font-size: 0.7rem;
    color: @kalam_text_dim;
    margin-top: 4px;
}

.kalam-rank-row {
    padding: 7px 12px;
    border-radius: 8px;
    background: @kalam_surface;
    border: 1px solid @kalam_border;
}

.kalam-dict-row {
    padding: 10px 12px;
}
"#;
