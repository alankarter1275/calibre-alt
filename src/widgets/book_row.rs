//! Cover cards for library grids (click → float, Ctrl+click → full page).
//!
//! Cover image area is always a fixed portrait box (height:width = 1.6:1).
//! Title and author sit *below* that box — not inside the cover ratio.
//!
//! Covers are scaled to exact pixel size at load so GTK never expands them
//! to the image's full natural resolution (the CONTINUE banner bug).

use crate::models::Book;
use gtk::gdk::ModifierType;
use gtk::prelude::*;
use std::path::Path;

/// Cover width (px). Height = width × 1.6 (standard ebook portrait).
pub const COVER_W: i32 = 128;
pub const COVER_ASPECT: f64 = 1.6;
pub const COVER_H: i32 = ((COVER_W as f64) * COVER_ASPECT) as i32; // 204

/// Full card width (cover only; text is same width).
pub const CARD_W: i32 = COVER_W;
/// Cover + gap + ~2 title lines + author.
const TEXT_H: i32 = 48;
pub const CARD_H: i32 = COVER_H + 8 + TEXT_H;

const GRID_COLS: i32 = 6;

/// One bookshelf card: fixed cover + title + author underneath.
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
    // Minimum = maximum intent: natural size forced by children at COVER_W.
    card.set_size_request(CARD_W, -1);

    let cover = cover_widget(book.cover_path.as_deref(), COVER_W, COVER_H);
    cover.add_css_class("kalam-book-card-cover");

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
    title.set_wrap(true);
    title.set_lines(2);
    title.set_max_width_chars(15);
    title.set_width_chars(15);
    title.set_hexpand(false);
    title.set_size_request(COVER_W, -1);

    let author = gtk::Label::new(Some(book.authors_display()));
    author.add_css_class("kalam-book-card-author");
    author.set_halign(gtk::Align::Center);
    author.set_ellipsize(gtk::pango::EllipsizeMode::End);
    author.set_max_width_chars(15);
    author.set_width_chars(15);
    author.set_hexpand(false);
    author.set_size_request(COVER_W, -1);

    card.append(&cover);
    card.append(&title);
    card.append(&author);
    card
}

/// Row/grid of fixed-size cards. Never expands to full window width.
pub fn build_book_grid(
    books: &[Book],
    on_full: impl Fn(i64) + Clone + 'static,
    on_float: impl Fn(i64) + Clone + 'static,
) -> gtk::Box {
    // Horizontal wrapping via nested rows of fixed cards — no FlowBox, no stretch.
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 18);
    outer.set_halign(gtk::Align::Start);
    outer.set_valign(gtk::Align::Start);
    outer.set_hexpand(false);
    outer.set_vexpand(false);
    outer.add_css_class("kalam-book-grid");

    let mut row: Option<gtk::Box> = None;
    for (i, book) in books.iter().enumerate() {
        if i as i32 % GRID_COLS == 0 {
            let r = gtk::Box::new(gtk::Orientation::Horizontal, 16);
            r.set_halign(gtk::Align::Start);
            r.set_hexpand(false);
            r.set_vexpand(false);
            outer.append(&r);
            row = Some(r);
        }
        let id = book.id;
        let f1 = on_full.clone();
        let f2 = on_float.clone();
        let card = build_book_card(book, move || f1(id), move || f2(id));
        if let Some(ref r) = row {
            r.append(&card);
        }
    }
    outer
}

/// Fixed `w`×`h` cover (always the same size — with or without an image).
///
/// Thumbnails are decoded **scaled to w×h** so the widget's natural size is
/// exactly the cover box. No AspectFrame (that caused grey side bars).
pub fn cover_widget(path: Option<&Path>, w: i32, h: i32) -> gtk::Widget {
    // Outer clamp box: reports min=nat=w×h and never expands.
    let frame = gtk::Box::new(gtk::Orientation::Vertical, 0);
    frame.add_css_class("kalam-cover-frame");
    frame.set_size_request(w, h);
    frame.set_hexpand(false);
    frame.set_vexpand(false);
    frame.set_halign(gtk::Align::Start);
    frame.set_valign(gtk::Align::Start);
    // Overflow hidden clips anything that still tries to paint outside.
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

/// Load image scaled to exactly w×h so natural size cannot exceed the card.
fn scaled_cover_picture(path: &Path, w: i32, h: i32) -> Option<gtk::Picture> {
    use gdk_pixbuf::{InterpType, Pixbuf};

    // preserve_aspect_ratio=false → exact w×h paintable (fills the cover box).
    let pixbuf = Pixbuf::from_file_at_scale(path, w, h, false).ok()?;
    let pixbuf = if pixbuf.width() != w || pixbuf.height() != h {
        pixbuf.scale_simple(w, h, InterpType::Bilinear)?
    } else {
        pixbuf
    };

    // Texture natural size = pixbuf size = w×h.
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
