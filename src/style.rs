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
.kalam-book-row {
    background: @kalam_surface;
    border: 1px solid @kalam_border;
    border-radius: 12px;
    padding: 12px 14px;
    margin-bottom: 8px;
}

.kalam-book-row:hover {
    border-color: @kalam_accent;
    background: @kalam_surface_2;
}

.kalam-cover-placeholder {
    background: linear-gradient(145deg, #2a3148, #1a1f30);
    border-radius: 6px;
    min-width: 48px;
    min-height: 72px;
    border: 1px solid @kalam_border;
}

.kalam-book-title {
    font-weight: 600;
    font-size: 0.98rem;
    color: @kalam_text;
}

.kalam-book-author {
    font-size: 0.84rem;
    color: @kalam_text_dim;
}

.kalam-progress {
    font-size: 0.75rem;
    color: @kalam_accent;
}

/* ── book detail page ───────────────────────────────── */
.kalam-detail-cover {
    background: linear-gradient(145deg, #3a4570, #1c2238);
    border-radius: 10px;
    min-width: 160px;
    min-height: 240px;
    border: 1px solid @kalam_border;
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
"#;
