//! Cover cards for library grids (click → float, Ctrl+click → full page).
//!
//! Covers are fixed-size so FlowBox cannot stretch them full-width.

use crate::models::Book;
use gtk::gdk::ModifierType;
use gtk::prelude::*;
use std::path::Path;

/// Standard library card cover size (portrait ebook).
pub const CARD_COVER_W: i32 = 120;
pub const CARD_COVER_H: i32 = 180;
pub const CARD_WIDTH: i32 = 132;

/// Build a cover-first card.
///
/// * plain click → `on_float` (floating panel)
/// * Ctrl+click → `on_full` (full book page)
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
    card.set_size_request(CARD_WIDTH, -1);

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
    title.set_max_width_chars(14);
    title.set_width_chars(14);
    title.set_lines(2);
    title.set_wrap(true);
    title.set_hexpand(false);

    let author = gtk::Label::new(Some(book.authors_display()));
    author.add_css_class("kalam-book-card-author");
    author.set_halign(gtk::Align::Center);
    author.set_ellipsize(gtk::pango::EllipsizeMode::End);
    author.set_max_width_chars(14);
    author.set_width_chars(14);
    author.set_hexpand(false);

    card.append(&cover);
    card.append(&title);
    card.append(&author);
    card
}

/// FlowBox of fixed-size cover cards (does not stretch children).
pub fn build_book_grid(
    books: &[Book],
    on_full: impl Fn(i64) + Clone + 'static,
    on_float: impl Fn(i64) + Clone + 'static,
) -> gtk::FlowBox {
    let grid = gtk::FlowBox::new();
    grid.set_valign(gtk::Align::Start);
    grid.set_halign(gtk::Align::Start);
    grid.set_max_children_per_line(12);
    grid.set_min_children_per_line(1);
    grid.set_selection_mode(gtk::SelectionMode::None);
    // Critical: homogeneous stretches a single child to full width.
    grid.set_homogeneous(false);
    grid.set_column_spacing(14);
    grid.set_row_spacing(16);
    grid.set_hexpand(true);
    grid.set_vexpand(false);
    grid.add_css_class("kalam-book-grid");

    for book in books {
        let id = book.id;
        let f1 = on_full.clone();
        let f2 = on_float.clone();
        let card = build_book_card(book, move || f1(id), move || f2(id));
        grid.insert(&card, -1);
        // FlowBoxChild defaults can expand a lone child across the row — pin it.
        if let Some(child) = grid.last_child() {
            child.set_halign(gtk::Align::Start);
            child.set_valign(gtk::Align::Start);
            child.set_hexpand(false);
            child.set_vexpand(false);
        }
    }
    grid
}

/// Fixed-size cover image (or placeholder). Never expands past `w`×`h`.
pub fn cover_widget(path: Option<&Path>, w: i32, h: i32) -> gtk::Widget {
    // Outer frame clamps size; Picture fills the frame without growing the layout.
    let frame = gtk::Box::new(gtk::Orientation::Vertical, 0);
    frame.set_size_request(w, h);
    frame.set_hexpand(false);
    frame.set_vexpand(false);
    frame.set_halign(gtk::Align::Center);
    frame.set_valign(gtk::Align::Center);
    frame.set_overflow(gtk::Overflow::Hidden);
    frame.add_css_class("kalam-cover-frame");

    if let Some(path) = path {
        if path.is_file() {
            let picture = gtk::Picture::for_filename(path);
            // Contain keeps whole cover visible; Cover crops to fill the frame.
            picture.set_content_fit(gtk::ContentFit::Cover);
            picture.set_can_shrink(true);
            picture.set_hexpand(true);
            picture.set_vexpand(true);
            picture.set_halign(gtk::Align::Fill);
            picture.set_valign(gtk::Align::Fill);
            picture.add_css_class("kalam-cover-img");
            frame.append(&picture);
            return frame.upcast();
        }
    }

    let placeholder = gtk::Box::new(gtk::Orientation::Vertical, 0);
    placeholder.add_css_class("kalam-cover-placeholder");
    placeholder.set_hexpand(true);
    placeholder.set_vexpand(true);
    frame.append(&placeholder);
    frame.upcast()
}
