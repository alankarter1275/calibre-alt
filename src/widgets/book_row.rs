//! Cover cards for library grids (click → float, Ctrl+click → full page).

use crate::models::Book;
use gtk::gdk::ModifierType;
use gtk::prelude::*;
use std::path::Path;

/// Build a cover-first card.
///
/// * plain click → `on_float` (floating panel)  
/// * Ctrl+click → `on_full` (full book page)
pub fn build_book_card(
    book: &Book,
    on_full: impl Fn() + 'static,
    on_float: impl Fn() + 'static,
) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    card.add_css_class("kalam-book-card");
    card.set_hexpand(false);
    card.set_halign(gtk::Align::Center);

    let cover = cover_widget(book.cover_path.as_deref(), 140, 210);
    cover.add_css_class("kalam-book-card-cover");
    cover.set_halign(gtk::Align::Center);

    // Clickable overlay on the whole card
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
    title.set_max_width_chars(18);
    title.set_lines(2);
    title.set_wrap(true);

    let author = gtk::Label::new(Some(book.authors_display()));
    author.add_css_class("kalam-book-card-author");
    author.set_halign(gtk::Align::Center);
    author.set_ellipsize(gtk::pango::EllipsizeMode::End);
    author.set_max_width_chars(18);

    card.append(&cover);
    card.append(&title);
    card.append(&author);
    card
}

/// FlowBox of cover cards.
pub fn build_book_grid(
    books: &[Book],
    on_full: impl Fn(i64) + Clone + 'static,
    on_float: impl Fn(i64) + Clone + 'static,
) -> gtk::FlowBox {
    let grid = gtk::FlowBox::new();
    grid.set_valign(gtk::Align::Start);
    grid.set_max_children_per_line(8);
    grid.set_min_children_per_line(2);
    grid.set_selection_mode(gtk::SelectionMode::None);
    grid.set_homogeneous(true);
    grid.set_column_spacing(16);
    grid.set_row_spacing(18);
    grid.set_hexpand(true);
    grid.add_css_class("kalam-book-grid");

    for book in books {
        let id = book.id;
        let f1 = on_full.clone();
        let f2 = on_float.clone();
        let card = build_book_card(book, move || f1(id), move || f2(id));
        grid.insert(&card, -1);
    }
    grid
}

pub fn cover_widget(path: Option<&Path>, w: i32, h: i32) -> gtk::Widget {
    if let Some(path) = path {
        if path.is_file() {
            let picture = gtk::Picture::for_filename(path);
            picture.set_content_fit(gtk::ContentFit::Cover);
            picture.set_size_request(w, h);
            picture.set_can_shrink(false);
            picture.add_css_class("kalam-cover-img");
            return picture.upcast();
        }
    }
    let placeholder = gtk::Box::new(gtk::Orientation::Vertical, 0);
    placeholder.add_css_class("kalam-cover-placeholder");
    placeholder.set_size_request(w, h);
    placeholder.upcast()
}
