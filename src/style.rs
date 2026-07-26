//! Temporary visual system for P0.
//! Replace / extend once the real Kalam design lands.

pub const APP_CSS: &str = r#"
/* ── window ─────────────────────────────────────────── */
window.kalam-window {
    background: @kalam_bg;
}

/* ── palette (dark-first draft) ─────────────────────── */
@define-color kalam_bg           #12141a;
@define-color kalam_surface      #1a1d27;
@define-color kalam_surface_2    #222633;
@define-color kalam_border       #2e3345;
@define-color kalam_text         #e8eaf0;
@define-color kalam_text_dim     #9aa3b5;
@define-color kalam_accent       #7c9cff;
@define-color kalam_accent_dim   alpha(#7c9cff, 0.18);
@define-color kalam_danger       #ff7c8a;
@define-color kalam_sidebar      #0e1016;

/* ── slim sidebar ───────────────────────────────────── */
.kalam-sidebar {
    background: @kalam_sidebar;
    border-right: 1px solid @kalam_border;
    min-width: 64px;
    padding: 10px 0;
}

.kalam-sidebar-inner {
    padding: 0 8px;
}

.kalam-brand {
    font-weight: 700;
    font-size: 0.95rem;
    letter-spacing: 0.04em;
    color: @kalam_accent;
    padding: 8px 0 14px 0;
}

.kalam-nav-btn {
    padding: 10px 6px;
    border-radius: 10px;
    margin: 2px 0;
    background: transparent;
    border: none;
    color: @kalam_text_dim;
    font-size: 0.72rem;
    font-weight: 500;
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
    margin-bottom: 2px;
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
}

/* ── breadcrumbs / back ─────────────────────────────── */
.kalam-back-btn {
    padding: 6px 10px;
    border-radius: 8px;
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
    border-radius: 12px;
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

.kalam-hub-tile-meta {
    font-size: 0.75rem;
    color: @kalam_text_dim;
}

/* ── book row in shelf list ─────────────────────────── */
.kalam-cover-frame {
    border-radius: 6px;
    background: @kalam_surface_2;
    /* no extra padding — grey side bars came from empty frame chrome */
    padding: 0;
    margin: 0;
    border: none;
    box-shadow: 0 2px 10px alpha(#000, 0.35);
}

.kalam-cover-placeholder {
    background: linear-gradient(160deg, #2a3148 0%, #1a1f30 100%);
    border-radius: 6px;
}

.kalam-cover-img {
    border-radius: 6px;
}

.kalam-book-grid-shell {
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
    color: #0e1016;
    font-weight: 700;
    border-radius: 10px;
    padding: 10px 18px;
    border: none;
}

.kalam-primary-btn:hover {
    filter: brightness(1.08);
}

.kalam-secondary-btn {
    background: @kalam_surface_2;
    color: @kalam_text;
    border-radius: 10px;
    padding: 10px 16px;
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
    /* parent must not stretch this */
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
    /* force label to respect allocation width */
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

/* Left column: fixed-width cover rail */
.kalam-float-cover-col {
    background: #0c0e14;
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
    border-radius: 8px;
    color: @kalam_text_dim;
    padding: 8px 6px;
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
    border-radius: 8px;
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
    color: #0e1016;
    background: #7cffc3;
    border-radius: 6px;
    padding: 3px 8px;
}

.kalam-badge-unread {
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    color: #1a1208;
    background: #ffc37c;
    border-radius: 6px;
    padding: 3px 8px;
}

.kalam-badge-progress {
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    color: #0e1016;
    background: @kalam_accent;
    border-radius: 6px;
    padding: 3px 8px;
}

.kalam-badge-done {
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    color: #e8eaf0;
    background: #3a4560;
    border-radius: 6px;
    padding: 3px 8px;
}

/* ── reader (immersive tablet-book) ─────────────────── */
.kalam-content > .kalam-reader {
    margin: -20px -24px;
}

.kalam-reader {
    background: #0a0a0b;
}

.kalam-reader-stage {
    background: #0a0a0b;
    min-height: 100%;
}

.kalam-reader-top-float {
    background: alpha(#1c1917, 0.72);
    border: 1px solid alpha(#fff, 0.08);
    border-radius: 999px;
    padding: 4px 10px 4px 4px;
    box-shadow: 0 8px 28px alpha(#000, 0.35);
}

.kalam-reader-crumb {
    font-size: 0.8rem;
    font-weight: 500;
    color: alpha(#fafaf9, 0.88);
    padding-right: 6px;
}

/* Bottom floating control pill */
.kalam-reader-pill {
    background: alpha(#1c1917, 0.88);
    border: 1px solid alpha(#fff, 0.10);
    border-radius: 999px;
    padding: 6px 10px;
    box-shadow: 0 12px 40px alpha(#000, 0.45);
}

.kalam-reader-pill-btn {
    min-width: 36px;
    min-height: 36px;
    padding: 0 10px;
    border-radius: 999px;
    border: none;
    background: transparent;
    color: #f5f5f4;
    font-weight: 600;
    font-size: 0.95rem;
}

.kalam-reader-pill-btn:hover {
    background: alpha(#fff, 0.12);
}

.kalam-reader-pill-meta {
    font-size: 0.78rem;
    font-weight: 600;
    color: alpha(#e7e5e4, 0.75);
    padding: 0 6px;
    min-width: 3.5rem;
}

.kalam-reader-popover {
    padding: 4px;
}

.kalam-reader-popover-title {
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    color: @kalam_text_dim;
}

.kalam-reader-theme-btn {
    border-radius: 8px;
    padding: 8px 12px;
    font-size: 0.85rem;
}

.kalam-toc-item {
    background: transparent;
    border: none;
    border-radius: 8px;
    color: @kalam_text;
    padding: 8px 10px;
    font-size: 0.88rem;
}

.kalam-toc-item:hover {
    background: @kalam_surface_2;
}
"#;
