//! Cover cards for library grids (click → float, Ctrl+click → full page).
//!
//! Uses a plain Grid of fixed-size cards — FlowBox was stretching children
//! full-width when only one/few books were present.

use crate::models::Book;
use gtk::gdk::ModifierType;
use gtk::prelude::*;
use std::path::Path;

/// Portrait ebook cover on cards (~2:3).
pub const CARD_COVER_W: i32 = 120;
pub const CARD_COVER_H: i32 = 180;
/// Total card width including padding (must match cover + a little margin).
pub const CARD_WIDTH: i32 = 128;
/// How many cards per row in library grids.
const GRID_COLS: i32 = 6;

/// Build a cover-first card at fixed size.
///
/// * plain click → `on_float`
/// * Ctrl+click → `on_full`
pub fn build_book_card(
    book: &Book,
    on_full: impl Fn() + 'static,
    on_float: impl Fn() + 'static,
) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    card.add_css_class("kalam-book-card");
    card.set_hexpand(false);
    card.set_vexpand(false);
    card.set_halign(gtk::Align::Start);
    card.set_valign(gtk::Align::Start);
    // Hard clamp — parent cannot grow this widget past CARD_WIDTH.
    card.set_size_request(CARD_WIDTH, CARD_COVER_H + 56);

    let cover = cover_widget(book.cover_path.as_deref(), CARD_COVER_W, CARD_COVER_H);
    cover.add_css_class("kalam-book-card-cover");
    cover.set_halign(gtk::Align::Center);
    cover.set_hexpand(false);
    cover.set_vexpand(false);

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
    card.set_tooltip_text(Some("Click: float · Ctrl+click: full page"));

    let title = gtk::Label::new(Some(&book.title));
    title.add_css_class("kalam-book-card-title");
    title.set_halign(gtk::Align::Center);
    title.set_justify(gtk::Justification::Center);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    title.set_max_width_chars(13);
    title.set_width_chars(13);
    title.set_lines(2);
    title.set_wrap(true);
    title.set_hexpand(false);

    let author = gtk::Label::new(Some(book.authors_display()));
    author.add_css_class("kalam-book-card-author");
    author.set_halign(gtk::Align::Center);
    author.set_ellipsize(gtk::pango::EllipsizeMode::End);
    author.set_max_width_chars(13);
    author.set_width_chars(13);
    author.set_hexpand(false);

    card.append(&cover);
    card.append(&title);
    card.append(&author);
    card
}

/// Fixed-size cover grid (no FlowBox — avoids full-width stretch).
pub fn build_book_grid(
    books: &[Book],
    on_full: impl Fn(i64) + Clone + 'static,
    on_float: impl Fn(i64) + Clone + 'static,
) -> gtk::Grid {
    let grid = gtk::Grid::new();
    grid.set_column_spacing(16);
    grid.set_row_spacing(18);
    grid.set_column_homogeneous(false);
    grid.set_row_homogeneous(false);
    grid.set_halign(gtk::Align::Start);
    grid.set_valign(gtk::Align::Start);
    grid.set_hexpand(false);
    grid.set_vexpand(false);
    grid.add_css_class("kalam-book-grid");

    for (i, book) in books.iter().enumerate() {
        let id = book.id;
        let f1 = on_full.clone();
        let f2 = on_float.clone();
        let card = build_book_card(book, move || f1(id), move || f2(id));
        let col = (i as i32) % GRID_COLS;
        let row = (i as i32) / GRID_COLS;
        grid.attach(&card, col, row, 1, 1);
    }
    grid
}

/// Fixed-size cover (or placeholder). Allocation is hard-capped at `w`×`h`.
pub fn cover_widget(path: Option<&Path>, w: i32, h: i32) -> gtk::Widget {
    let frame = gtk::Frame::new(None);
    frame.add_css_class("kalam-cover-frame");
    frame.set_size_request(w, h);
    frame.set_hexpand(false);
    frame.set_vexpand(false);
    frame.set_halign(gtk::Align::Center);
    frame.set_valign(gtk::Align::Center);
    // Frame won't grow with parent when hexpand is false + size_request set.

    if let Some(path) = path {
        if path.is_file() {
            // GdkTexture + Picture: keep paintable, clamp allocation via Frame size.
            if let Ok(texture) = gtk::gdk::Texture::from_filename(path) {
                let picture = gtk::Picture::for_paintable(&texture);
                picture.set_content_fit(gtk::ContentFit::Cover);
                picture.set_can_shrink(true);
                picture.set_size_request(w, h);
                picture.set_hexpand(false);
                picture.set_vexpand(false);
                picture.add_css_class("kalam-cover-img");
                frame.set_child(Some(&picture));
                return frame.upcast();
            }
            // Fallback if texture load fails
            let picture = gtk::Picture::for_filename(path);
            picture.set_content_fit(gtk::ContentFit::Cover);
            picture.set_can_shrink(true);
            picture.set_size_request(w, h);
            picture.set_hexpand(false);
            picture.set_vexpand(false);
            picture.add_css_class("kalam-cover-img");
            frame.set_child(Some(&picture));
            return frame.upcast();
        }
    }

    let placeholder = gtk::Box::new(gtk::Orientation::Vertical, 0);
    placeholder.add_css_class("kalam-cover-placeholder");
    placeholder.set_size_request(w, h);
    frame.set_child(Some(&placeholder));
    frame.upcast()
}
