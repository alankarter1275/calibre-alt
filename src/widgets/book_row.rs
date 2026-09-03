//! Cover cards for library grids (click → float, Ctrl+click → full page).
//!
//! Every card is the **same pixel width** so long titles cannot break the grid.
//! Cover is 1.6:1 portrait; title + author sit below and ellipsize inside that width.

use crate::models::Book;
use gtk::gdk::ModifierType;
use gtk::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

thread_local! {
    /// Decoded cover textures, keyed by path + requested size.
    ///
    /// Without this every navigation re-reads and re-scales each cover PNG from
    /// disk, which is what made scrolling and page switches feel heavy. GDK
    /// textures are reference-counted and live on the GPU, so re-using them is
    /// both faster and lighter than holding pixbufs.
    static COVER_CACHE: RefCell<HashMap<(String, i32, i32), gtk::gdk::Texture>> =
        RefCell::new(HashMap::new());
}

/// A cover frame showing a placeholder, waiting for its texture.
struct PendingFrame {
    key: (String, i32, i32),
    /// Weak: the page can be destroyed before the decode finishes.
    frame: gtk::glib::WeakRef<gtk::Box>,
}

thread_local! {
    /// Frames waiting on a decode. UI-owned and only ever touched on the main
    /// thread, which is exactly what `thread_local!` is for
    /// (`docs/pitfalls.md` §4e).
    static PENDING_FRAMES: RefCell<Vec<PendingFrame>> = const { RefCell::new(Vec::new()) };
}

/// Drop cached textures for a cover that changed or was deleted.
pub fn invalidate_cover_cache(path: &Path) {
    let key = path.to_string_lossy().to_string();
    COVER_CACHE.with(|c| {
        c.borrow_mut().retain(|(p, _, _), _| *p != key);
    });
}

/// Whether a cover is already decoded at this size.
///
/// Lets the preloader (A0 step 5) skip work the grid has already done, so
/// re-running it after a small scroll costs almost nothing.
pub fn is_cover_cached(path: &Path, w: i32, h: i32) -> bool {
    let key = (path.to_string_lossy().to_string(), w, h);
    COVER_CACHE.with(|c| c.borrow().contains_key(&key))
}

/// Store a cover decoded off the UI thread by the preloader.
///
/// The worker cannot build the texture — `gdk::Texture` is a GObject and
/// belongs to the main thread — so it sends raw RGBA and the wrap happens
/// here. That wrap is a pointer copy, not a decode, which is the whole point:
/// the expensive part already happened on the worker.
///
/// Ignores anything already cached so a preload can never replace a texture
/// the grid is currently showing.
pub fn cache_decoded_cover(decoded: &crate::preload::DecodedCover) {
    let (w, h) = (decoded.width, decoded.height);
    let expected = (w as usize) * (h as usize) * 4;
    // A mismatch here would be a garbled image or a crash inside GDK rather
    // than a visible bug, so refuse instead of trusting the buffer.
    if w <= 0 || h <= 0 || decoded.rgba.len() != expected {
        return;
    }
    let key = (decoded.cover.to_string_lossy().to_string(), w, h);
    let inserted = COVER_CACHE.with(|c| {
        let mut cache = c.borrow_mut();
        if cache.contains_key(&key) {
            return None;
        }
        if cache.len() > 400 {
            cache.clear();
        }
        let bytes = gtk::glib::Bytes::from(&decoded.rgba[..]);
        let texture: gtk::gdk::Texture = gtk::gdk::MemoryTexture::new(
            w,
            h,
            gtk::gdk::MemoryFormat::R8g8b8a8,
            &bytes,
            (w as usize) * 4,
        )
        .upcast();
        cache.insert(key.clone(), texture.clone());
        Some((key, texture))
    });

    // Outside the cache borrow: the swap re-enters nothing, but holding a
    // RefCell borrow across widget work is how re-entrancy panics start.
    if let Some((key, texture)) = inserted {
        swap_in_cover(&key, &texture);
    }
}

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
    card.set_halign(gtk::Align::Start);
    card.set_valign(gtk::Align::Start);
    // Lock both axes so long titles never widen the cell.
    card.set_size_request(CARD_W, CARD_H);
    card.set_overflow(gtk::Overflow::Hidden);

    // Deferred: a grid builds hundreds of these at once, so decoding here
    // would be the stall the preloader exists to remove.
    let cover = cover_widget_deferred(book.cover_path.as_deref(), COVER_W, COVER_H);
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
    grid.set_hexpand(true);
    grid.set_vexpand(false);
    grid.add_css_class("kalam-book-grid");

    // Shell keeps the whole grid from being stretched by the parent but allows fill.
    let shell = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    shell.set_halign(gtk::Align::Fill);
    shell.set_valign(gtk::Align::Start);
    shell.set_hexpand(true);
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
        cell.set_halign(gtk::Align::Start);
        cell.append(&card);

        let col = (i as i32) % GRID_COLS;
        let row = (i as i32) / GRID_COLS;
        grid.attach(&cell, col, row, 1, 1);
    }

    shell.append(&grid);

    // Frames from pages that have since been destroyed. Covers that never
    // decode -- a missing or corrupt file -- are never swapped, so without
    // this their entries would accumulate for the life of the process.
    drop_dead_pending_frames();

    // Every card above is showing a placeholder. Start decoding, nearest
    // first, so the top of the grid fills in while the user is still looking
    // at it. `warm_covers` calls back into `cache_decoded_cover`, which swaps
    // each image into its frame as it lands.
    crate::preload::warm_covers(
        crate::preload::ahead_of(books, 0, COVER_W, COVER_H),
        COVER_W,
        COVER_H,
    );

    shell
}

/// Fixed `w`×`h` cover (always the same size — with or without an image).
///
/// Decodes on the spot. Right for the one-or-two covers on a detail page,
/// where a placeholder that fills in a moment later would just look like a
/// flicker. Grids want [`cover_widget_deferred`] instead.
pub fn cover_widget(path: Option<&Path>, w: i32, h: i32) -> gtk::Widget {
    let frame = new_cover_frame(w, h);

    if let Some(path) = path {
        if path.is_file() {
            if let Some(picture) = scaled_cover_picture(path, w, h) {
                frame.append(&picture);
                return frame.upcast();
            }
        }
    }

    frame.append(&placeholder_for(w, h));
    frame.upcast()
}

/// Like [`cover_widget`], but never decodes on the UI thread.
///
/// A0 step 5, and the half of step 3 that was deferred to here: an uncached
/// cover gets a placeholder **immediately** and the frame is recorded, so
/// [`swap_in_cover`] can fill it once a worker has decoded the image.
///
/// This is what makes a big grid cheap. A 400-book page used to decode 400
/// covers on the UI thread before it could show anything; now it shows
/// straight away and the images arrive as they are ready.
pub fn cover_widget_deferred(path: Option<&Path>, w: i32, h: i32) -> gtk::Widget {
    let frame = new_cover_frame(w, h);

    if let Some(path) = path {
        if path.is_file() {
            let key = (path.to_string_lossy().to_string(), w, h);
            // Already decoded: use it now, no placeholder flash.
            if let Some(texture) = COVER_CACHE.with(|c| c.borrow().get(&key).cloned()) {
                frame.append(&build_picture(&texture, w, h));
                return frame.upcast();
            }
            frame.append(&placeholder_for(w, h));
            PENDING_FRAMES.with(|p| {
                p.borrow_mut().push(PendingFrame {
                    key,
                    frame: frame.downgrade(),
                });
            });
            return frame.upcast();
        }
    }

    frame.append(&placeholder_for(w, h));
    frame.upcast()
}

/// Forget frames whose widgets have been destroyed.
fn drop_dead_pending_frames() {
    PENDING_FRAMES.with(|p| {
        p.borrow_mut().retain(|entry| entry.frame.upgrade().is_some());
    });
}

fn new_cover_frame(w: i32, h: i32) -> gtk::Box {
    let frame = gtk::Box::new(gtk::Orientation::Vertical, 0);
    frame.add_css_class("kalam-cover-frame");
    frame.set_size_request(w, h);
    frame.set_hexpand(false);
    frame.set_vexpand(false);
    frame.set_halign(gtk::Align::Center);
    frame.set_valign(gtk::Align::Start);
    frame.set_overflow(gtk::Overflow::Hidden);
    frame
}

fn placeholder_for(w: i32, h: i32) -> gtk::Box {
    let placeholder = gtk::Box::new(gtk::Orientation::Vertical, 0);
    placeholder.add_css_class("kalam-cover-placeholder");
    placeholder.set_size_request(w, h);
    placeholder.set_hexpand(false);
    placeholder.set_vexpand(false);
    placeholder
}

/// Replace the placeholder in every frame waiting on this cover.
///
/// Frames are held **weakly**: a page can be torn down long before its covers
/// finish decoding, and a strong reference here would both leak the widget and
/// let a preload write into a dead page. Dead entries are dropped on the way
/// past, which is what keeps the list from growing across navigations —
/// covers that never decode (a missing or corrupt file) are only reaped this
/// way, since nothing else will ever come back for them.
fn swap_in_cover(key: &(String, i32, i32), texture: &gtk::gdk::Texture) {
    PENDING_FRAMES.with(|p| {
        let mut pending = p.borrow_mut();
        pending.retain(|entry| {
            let Some(frame) = entry.frame.upgrade() else {
                return false; // page is gone
            };
            if &entry.key != key {
                return true; // waiting on a different cover
            }
            while let Some(child) = frame.first_child() {
                frame.remove(&child);
            }
            frame.append(&build_picture(texture, key.1, key.2));
            false
        });
    });
}

fn scaled_cover_picture(path: &Path, w: i32, h: i32) -> Option<gtk::Picture> {
    // Cache key stays the *original* cover path so invalidation on a cover
    // change/replacement keeps working exactly as before.
    let key = (path.to_string_lossy().to_string(), w, h);

    // Serve from cache when we have already decoded this cover at this size.
    if let Some(texture) = COVER_CACHE.with(|c| c.borrow().get(&key).cloned()) {
        return Some(build_picture(&texture, w, h));
    }

    // Once imported, we generate a tiny thumbnail (cache/thumbs/<uuid>.png) so
    // the grid does not decode the full 1000×1500+ cover on the UI thread. Use
    // it whenever it exists and this slot is small enough to fit without
    // upscaling; large slots (book page, author photo) still decode the cover.
    // Keep the thumbnail PathBuf alive for the whole call so the borrow below
    // outlives it (an ephemeral Option<PathBuf> would drop before decode).
    let thumb = thumb_for_slot(path, w, h).filter(|p| p.is_file());
    let decode_path: &Path = thumb.as_deref().unwrap_or(path);

    let texture = decode_cover(decode_path, w, h)?;
    COVER_CACHE.with(|c| {
        let mut cache = c.borrow_mut();
        // Crude bound: a few hundred covers is plenty for one session and keeps
        // memory sane on the 4 GB target machine.
        if cache.len() > 400 {
            cache.clear();
        }
        cache.insert(key, texture.clone());
    });
    Some(build_picture(&texture, w, h))
}

/// The thumbnail path to decode for a cover slot, or `None` when this slot is
/// too large to use the thumbnail (never upscale it).
fn thumb_for_slot(cover: &Path, w: i32, h: i32) -> Option<std::path::PathBuf> {
    if w > crate::thumbs::THUMB_W as i32 || h > crate::thumbs::THUMB_H as i32 {
        return None;
    }
    crate::paths::thumbnail_for_cover(cover)
}

fn decode_cover(path: &Path, w: i32, h: i32) -> Option<gtk::gdk::Texture> {
    use gdk_pixbuf::{InterpType, Pixbuf};

    let pixbuf = Pixbuf::from_file_at_scale(path, w, h, false).ok()?;
    let pixbuf = if pixbuf.width() != w || pixbuf.height() != h {
        pixbuf.scale_simple(w, h, InterpType::Bilinear)?
    } else {
        pixbuf
    };
    Some(gtk::gdk::Texture::for_pixbuf(&pixbuf))
}

fn build_picture(texture: &gtk::gdk::Texture, w: i32, h: i32) -> gtk::Picture {
    let picture = gtk::Picture::for_paintable(texture);
    picture.set_content_fit(gtk::ContentFit::Fill);
    picture.set_can_shrink(true);
    picture.set_size_request(w, h);
    picture.set_hexpand(false);
    picture.set_vexpand(false);
    picture.set_halign(gtk::Align::Fill);
    picture.set_valign(gtk::Align::Fill);
    picture.add_css_class("kalam-cover-img");
    picture
}
