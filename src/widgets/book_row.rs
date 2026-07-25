//! A single book row used in shelf lists and library sections.

use crate::models::Book;
use gtk::prelude::*;

/// Build a clickable book row.
///
/// * `on_open` — open the full book page in the main column
/// * `on_dialog` — open the floating book window
pub fn build_book_row(
    book: &Book,
    on_open: impl Fn() + 'static,
    on_dialog: impl Fn() + 'static,
) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    row.add_css_class("kalam-book-row");
    row.set_hexpand(true);

    let cover = gtk::Box::new(gtk::Orientation::Vertical, 0);
    cover.add_css_class("kalam-cover-placeholder");
    cover.set_valign(gtk::Align::Center);
    row.append(&cover);

    let text = gtk::Box::new(gtk::Orientation::Vertical, 3);
    text.set_hexpand(true);
    text.set_valign(gtk::Align::Center);

    let title = gtk::Label::new(Some(&book.title));
    title.add_css_class("kalam-book-title");
    title.set_halign(gtk::Align::Start);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let author = gtk::Label::new(Some(&book.authors.join(", ")));
    author.add_css_class("kalam-book-author");
    author.set_halign(gtk::Align::Start);

    let meta = gtk::Label::new(Some(&format!(
        "{} · {}%",
        book.format, book.progress
    )));
    meta.add_css_class("kalam-progress");
    meta.set_halign(gtk::Align::Start);

    text.append(&title);
    text.append(&author);
    text.append(&meta);
    row.append(&text);

    let actions = gtk::Box::new(gtk::Orientation::Vertical, 6);
    actions.set_valign(gtk::Align::Center);

    let open_btn = gtk::Button::with_label("Open");
    open_btn.add_css_class("kalam-secondary-btn");
    open_btn.connect_clicked(move |_| on_open());

    let float_btn = gtk::Button::with_label("Float");
    float_btn.add_css_class("kalam-secondary-btn");
    float_btn.set_tooltip_text(Some("Open in a floating window"));
    float_btn.connect_clicked(move |_| on_dialog());

    actions.append(&open_btn);
    actions.append(&float_btn);
    row.append(&actions);

    // Whole row (except action buttons) also opens the page on click.
    // Buttons stop propagation by being separate targets.
    row
}
