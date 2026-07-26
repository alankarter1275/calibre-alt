//! Cover cards for library grids (click → float, Ctrl+click → full page).
//!
//! Every card is the **same pixel width** so long titles cannot break the grid.
//! Cover is 1.6:1 portrait; title + author sit below and ellipsize inside that width.

use crate::models::Book;
use gtk::gdk::ModifierType;
use gtk::prelude::*;
use std::path::Path;

/// Cover width (px). Height = width × 1.6 (standard ebook portrait).
pub const COVER_W: i32 = 128;
pub const COVER_ASPECT: f64 = 1.6;
pub const COVER_H: i32 = ((COVER_W as f64) * COVER_ASPECT) as i32; // 204

/// Full card width — locked; labels cannot grow past this.
pub const CARD_W: i32 = COVER_W;
/// Space reserved under the cover for title (2 lines) + author.
const TITLE_AREA_H: i32 = 36;
const AUTHOR_AREA_H: i32 = 18;
const GAP: i32 = 6;
pub const CARD_H: i32 = COVER_H + GAP + TITLE_AREA_H + AUTHOR_AREA_H;

/// Columns in the library grid (uniform cells).
const GRID_COLS: i32 = 6;
const COL_SPACING: u32 = 16;
const ROW_SPACING: u32 = 20;

/// One bookshelf card: fixed cover + title + author underneath.
pub fn build_book_card(
    book: &Book,
    on_full: impl Fn() + 'static,
    on_float: impl Fn() + 'static,
) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, GAP);
    card.add_css_class("kalam-book-card");
    card.set_hexpand(false);
    card.set_vexpand(false);
    card.set_halign(gtk::Align::Center);
    card.set_valign(gtk::Align::Start);
    // Lock both axes so long titles never widen the cell.
    card.set_size_request(CARD_W, CARD_H);
    card.set_overflow(gtk::Overflow::Hidden);

    let cover = cover_widget(book.cover_path.as_deref(), COVER_W, COVER_H);
    cover.add_css_class("kalam-book-card-cover");
    cover.set_halign(gtk::Align::Center);

    let click = gtk::GestureClick::new();
    click.set_button(1);
    click.connect_released(move |gesture, _n, _x, _y| {
        let state = gesture.current_event_state();
        if state.contains(ModifierType::CONTROL_MASK) {
            on_full();
        } else {
            on_float();
        }
    });
    card.add_controller(click);
    card.set_cursor_from_name(Some("pointer"));
    // Full title/author available on hover even when ellipsized.
    card.set_tooltip_text(Some(&format!(
        "{}\n{}\n\nClick: float · Ctrl+click: full page",
        book.title,
        book.authors_display()
    )));

    // Text column clamped to COVER_W — labels ellipsize inside, never expand card.
    let text_col = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text_col.set_size_request(COVER_W, TITLE_AREA_H + AUTHOR_AREA_H);
    text_col.set_hexpand(false);
    text_col.set_vexpand(false);
    text_col.set_halign(gtk::Align::Center);
    text_col.set_overflow(gtk::Overflow::Hidden);

    let title = gtk::Label::new(Some(&book.title));
    title.add_css_class("kalam-book-card-title");
    title.set_halign(gtk::Align::Center);
    title.set_justify(gtk::Justification::Center);
    title.set_xalign(0.5);
    // Single-line ellipsis is the most reliable way to keep fixed width in GTK.
    // Two lines with wrap still often grow the parent; we use 2 lines capped in pixels.
    title.set_wrap(true);
    title.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    title.set_lines(2);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    title.set_hexpand(false);
    title.set_vexpand(false);
    title.set_size_request(COVER_W, TITLE_AREA_H);
    // Critical: natural width must not exceed cover width.
    title.set_width_chars(1);
    title.set_max_width_chars(1);

    let author = gtk::Label::new(Some(book.authors_display()));
    author.add_css_class("kalam-book-card-author");
    author.set_halign(gtk::Align::Center);
    author.set_xalign(0.5);
    author.set_ellipsize(gtk::pango::EllipsizeMode::End);
    author.set_single_line_mode(true);
    author.set_hexpand(false);
    author.set_vexpand(false);
    author.set_size_request(COVER_W, AUTHOR_AREA_H);
    author.set_width_chars(1);
    author.set_max_width_chars(1);

    text_col.append(&title);
    text_col.append(&author);

    card.append(&cover);
    card.append(&text_col);
    card
}

/// Uniform grid of fixed-size cards (same cell width for every book).
pub fn build_book_grid(
    books: &[Book],
    on_full: impl Fn(i64) + Clone + 'static,
    on_float: impl Fn(i64) + Clone + 'static,
) -> gtk::Box {
    // GtkGrid with homogeneous columns = true grid view.
    let grid = gtk::Grid::new();
    grid.set_column_spacing(COL_SPACING);
    grid.set_row_spacing(ROW_SPACING);
    grid.set_column_homogeneous(true);
    grid.set_row_homogeneous(false);
    grid.set_halign(gtk::Align::Start);
    grid.set_valign(gtk::Align::Start);
    grid.set_hexpand(false);
    grid.set_vexpand(false);
    grid.add_css_class("kalam-book-grid");

    // Shell keeps the whole grid from being stretched by the parent.
    let shell = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    shell.set_halign(gtk::Align::Start);
    shell.set_valign(gtk::Align::Start);
    shell.set_hexpand(false);
    shell.set_vexpand(false);
    shell.add_css_class("kalam-book-grid-shell");

    for (i, book) in books.iter().enumerate() {
        let id = book.id;
        let f1 = on_full.clone();
        let f2 = on_float.clone();
        let card = build_book_card(book, move || f1(id), move || f2(id));

        // Cell wrapper enforces CARD_W so Grid homogeneous cells stay equal.
        let cell = gtk::Box::new(gtk::Orientation::Vertical, 0);
        cell.set_size_request(CARD_W, CARD_H);
        cell.set_hexpand(false);
        cell.set_vexpand(false);
        cell.set_halign(gtk::Align::Center);
        cell.append(&card);

        let col = (i as i32) % GRID_COLS;
        let row = (i as i32) / GRID_COLS;
        grid.attach(&cell, col, row, 1, 1);
    }

    shell.append(&grid);
    shell
}

/// Fixed `w`×`h` cover (always the same size — with or without an image).
pub fn cover_widget(path: Option<&Path>, w: i32, h: i32) -> gtk::Widget {
    let frame = gtk::Box::new(gtk::Orientation::Vertical, 0);
    frame.add_css_class("kalam-cover-frame");
    frame.set_size_request(w, h);
    frame.set_hexpand(false);
    frame.set_vexpand(false);
    frame.set_halign(gtk::Align::Center);
    frame.set_valign(gtk::Align::Start);
    frame.set_overflow(gtk::Overflow::Hidden);

    if let Some(path) = path {
        if path.is_file() {
            if let Some(picture) = scaled_cover_picture(path, w, h) {
                frame.append(&picture);
                return frame.upcast();
            }
        }
    }

    let placeholder = gtk::Box::new(gtk::Orientation::Vertical, 0);
    placeholder.add_css_class("kalam-cover-placeholder");
    placeholder.set_size_request(w, h);
    placeholder.set_hexpand(false);
    placeholder.set_vexpand(false);
    frame.append(&placeholder);
    frame.upcast()
}

fn scaled_cover_picture(path: &Path, w: i32, h: i32) -> Option<gtk::Picture> {
    use gdk_pixbuf::{InterpType, Pixbuf};

    let pixbuf = Pixbuf::from_file_at_scale(path, w, h, false).ok()?;
    let pixbuf = if pixbuf.width() != w || pixbuf.height() != h {
        pixbuf.scale_simple(w, h, InterpType::Bilinear)?
    } else {
        pixbuf
    };

    let texture = gtk::gdk::Texture::for_pixbuf(&pixbuf);
    let picture = gtk::Picture::for_paintable(&texture);
    picture.set_content_fit(gtk::ContentFit::Fill);
    picture.set_can_shrink(true);
    picture.set_size_request(w, h);
    picture.set_hexpand(false);
    picture.set_vexpand(false);
    picture.set_halign(gtk::Align::Fill);
    picture.set_valign(gtk::Align::Fill);
    picture.add_css_class("kalam-cover-img");
    Some(picture)
}
