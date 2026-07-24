//! Shelf card for the 2-column shelves grid.

use crate::models::Shelf;
use gtk::prelude::*;

pub fn build_shelf_card(shelf: &Shelf, on_open: impl Fn() + 'static) -> gtk::Button {
    let inner = gtk::Box::new(gtk::Orientation::Vertical, 8);
    inner.set_halign(gtk::Align::Fill);
    inner.set_hexpand(true);

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);

    let name = gtk::Label::new(Some(&shelf.name));
    name.add_css_class("kalam-card-title");
    name.set_halign(gtk::Align::Start);
    name.set_hexpand(true);
    name.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let badge = gtk::Label::new(Some(&shelf.book_ids.len().to_string()));
    badge.add_css_class("kalam-card-badge");

    header.append(&name);
    header.append(&badge);

    let kind = gtk::Label::new(Some(shelf.kind_label()));
    kind.add_css_class("kalam-card-meta");
    kind.set_halign(gtk::Align::Start);

    let desc = gtk::Label::new(Some(&shelf.description));
    desc.add_css_class("kalam-card-meta");
    desc.set_halign(gtk::Align::Start);
    desc.set_ellipsize(gtk::pango::EllipsizeMode::End);
    desc.set_max_width_chars(40);

    inner.append(&header);
    inner.append(&kind);
    inner.append(&desc);

    let btn = gtk::Button::new();
    btn.set_child(Some(&inner));
    btn.add_css_class("kalam-card");
    btn.set_hexpand(true);
    btn.set_halign(gtk::Align::Fill);
    btn.connect_clicked(move |_| on_open());
    btn
}
