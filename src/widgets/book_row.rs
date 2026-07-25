//! Cover cards for library grids (click → float, Ctrl+click → full page).
//!
//! Cover image area is always a fixed portrait box (height:width = 1.6:1).
//! Title and author sit *below* that box and are not part of the cover ratio.

use crate::models::Book;
use gtk::gdk::ModifierType;
use gtk::prelude::*;
use std::path::Path;

/// Cover width in CSS px. Height = width × COVER_ASPECT (1.6:1 portrait).
pub const COVER_W: i32 = 128;
/// height / width — standard ebook cover proportion.
pub const COVER_ASPECT: f64 = 1.6;
pub const COVER_H: i32 = (COVER_W as f64 * COVER_ASPECT) as i32; // 204

/// Card width (cover + small padding). Card height is cover + text below.
pub const CARD_W: i32 = COVER_W + 8;
/// Approximate text block under the cover (title 2 lines + author).
const TEXT_BLOCK_H: i32 = 52;
pub const CARD_H: i32 = COVER_H + TEXT_BLOCK_H + 12;

const GRID_COLS: i32 = 6;

/// Build a cover-first card.
///
/// Layout:
/// ```text
/// ┌──────────┐  ← cover only, fixed 1.6:1
/// │  cover   │
/// └──────────┘
///   Title       ← outside the cover ratio
///   Author
/// ```
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
    card.set_size_request(CARD_W, CARD_H);

    // Cover slot — always COVER_W × COVER_H, even with no image / no metadata.
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
    title.set_size_request(COVER_W, -1);

    let author = gtk::Label::new(Some(book.authors_display()));
    author.add_css_class("kalam-book-card-author");
    author.set_halign(gtk::Align::Center);
    author.set_ellipsize(gtk::pango::EllipsizeMode::End);
    author.set_max_width_chars(14);
    author.set_width_chars(14);
    author.set_hexpand(false);
    author.set_size_request(COVER_W, -1);

    card.append(&cover);
    card.append(&title);
    card.append(&author);
    card
}

/// Horizontal grid of fixed-size cards inside a non-expanding shell.
pub fn build_book_grid(
    books: &[Book],
    on_full: impl Fn(i64) + Clone + 'static,
    on_float: impl Fn(i64) + Clone + 'static,
) -> gtk::Box {
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

    // Outer shell stops parents from stretching the card grid full-width.
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
        let col = (i as i32) % GRID_COLS;
        let row = (i as i32) / GRID_COLS;
        grid.attach(&card, col, row, 1, 1);
    }
    shell.append(&grid);
    shell
}

/// Cover slot: always exactly `w`×`h` (portrait 1.6:1 for library cards).
///
/// Images are **scaled down at load** so GTK never sees a huge natural size.
/// Missing covers get the same empty frame — same dimensions always.
pub fn cover_widget(path: Option<&Path>, w: i32, h: i32) -> gtk::Widget {
    // AspectFrame with obey_child=false forces the geometric ratio and
    // ignores the child's natural size (the root cause of the banner bug).
    let ratio = w as f32 / h as f32; // width/height; for 128×204 ≈ 0.627
    let aspect = gtk::AspectFrame::new(0.5, 0.5, ratio, false);
    aspect.set_size_request(w, h);
    aspect.set_hexpand(false);
    aspect.set_vexpand(false);
    aspect.set_halign(gtk::Align::Center);
    aspect.set_valign(gtk::Align::Center);
    aspect.add_css_class("kalam-cover-frame");
    aspect.set_overflow(gtk::Overflow::Hidden);

    if let Some(path) = path {
        if path.is_file() {
            if let Some(picture) = load_scaled_picture(path, w, h) {
                aspect.set_child(Some(&picture));
                return aspect.upcast();
            }
        }
    }

    let placeholder = gtk::Box::new(gtk::Orientation::Vertical, 0);
    placeholder.add_css_class("kalam-cover-placeholder");
    placeholder.set_hexpand(true);
    placeholder.set_vexpand(true);
    aspect.set_child(Some(&placeholder));
    aspect.upcast()
}

/// Decode image and scale to exactly w×h so natural size cannot blow the layout.
fn load_scaled_picture(path: &Path, w: i32, h: i32) -> Option<gtk::Picture> {
    use gtk::gdk_pixbuf::InterpType;
    use gtk::gdk_pixbuf::Pixbuf;

    // Scale to fill the box (cover crop): load larger side then we'll clip via AspectFrame.
    // from_file_at_scale with preserve_aspect_ratio=false stretches to exact w×h —
    // good enough for a thumbnail and guarantees natural size = w×h.
    let pixbuf = Pixbuf::from_file_at_scale(path, w, h, false).ok()?;
    // Ensure exact size even if loader rounded.
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
    picture.add_css_class("kalam-cover-img");
    Some(picture)
}
