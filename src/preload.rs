//! A0 step 5 — preloaders.
//!
//! # Cover preloader
//!
//! The grid decodes a cover the first time a card needs one, on the UI thread.
//! Step 3 made each decode cheap (a 256×408 thumbnail instead of a 1000×1500+
//! cover), but cheap times four hundred is still a stall, and it lands at the
//! worst moment: the frame where the user just scrolled or switched pages.
//!
//! So decode on a worker instead. Cards go up with a placeholder immediately
//! and each image is swapped in as it becomes ready, which means building a
//! grid costs no decodes at all.
//!
//! # Why this is not just `tasks::spawn`
//!
//! Two reasons.
//!
//! **A texture cannot cross a thread.** `gdk::Texture` is a GObject and lives
//! on the main thread; `tasks::spawn` refuses to let a worker return one,
//! which is the seam doing its job. What *can* cross is the decoded pixel
//! buffer — a plain `Vec<u8>` — so the worker does the expensive part (read
//! the PNG, decode it, resize it) and the main thread does the cheap part
//! (wrap the bytes in a `MemoryTexture`).
//!
//! **The results arrive one at a time.** A preloader that reported only when
//! all twenty covers were done would be useless, so this uses
//! [`crate::tasks::spawn_stream`] and each cover is cached the moment it is
//! ready.
//!
//! # What it deliberately does not do
//!
//! It only ever replaces a **placeholder**. `book_row` refuses to overwrite a
//! texture that is already cached, so a preload can never change an image the
//! user is currently looking at — the failure mode that makes async image
//! loading feel glitchy.

use std::path::{Path, PathBuf};

/// How many covers to decode ahead. The grid shows six columns, so this is
/// roughly the next three to four rows — enough to stay ahead of a scroll
/// without spending the session decoding a 2,000-book library nobody scrolls.
pub const PRELOAD_AHEAD: usize = 24;

/// A decoded cover on its way back to the main thread.
///
/// Deliberately plain data: `Vec<u8>` crosses threads, `gdk::Texture` does not.
pub struct DecodedCover {
    /// The *original* cover path — the cache is keyed on it, so that
    /// invalidation on a cover change keeps working.
    pub cover: PathBuf,
    pub width: i32,
    pub height: i32,
    /// Tightly packed RGBA, `width * height * 4` bytes.
    pub rgba: Vec<u8>,
}

/// Decode one cover to raw RGBA at `w`×`h`.
///
/// Headless and dependency-free — no GTK, no GDK — which is what makes it
/// testable in CI and legal to call from a worker thread. Returns `None`
/// rather than an error for anything unreadable: a missing cover is a
/// placeholder, not a failure worth reporting.
pub fn decode_rgba(path: &Path, w: i32, h: i32) -> Option<DecodedCover> {
    if w <= 0 || h <= 0 || !path.is_file() {
        return None;
    }
    let img = image::open(path).ok()?;
    let resized = img.resize_exact(w as u32, h as u32, image::imageops::FilterType::Triangle);
    Some(DecodedCover {
        cover: path.to_path_buf(),
        width: w,
        height: h,
        rgba: resized.to_rgba8().into_raw(),
    })
}

/// Which file to decode for a cover slot: the thumbnail when it exists and is
/// big enough, else the original.
///
/// Mirrors the same choice `book_row::scaled_cover_picture` makes, so the
/// preloader and the on-demand path always decode the same source and the
/// warmed entry is a real hit rather than a near miss.
fn source_for(cover: &Path, w: i32, h: i32) -> PathBuf {
    if w <= crate::thumbs::THUMB_W as i32 && h <= crate::thumbs::THUMB_H as i32 {
        if let Some(thumb) = crate::paths::thumbnail_for_cover(cover) {
            if thumb.is_file() {
                return thumb;
            }
        }
    }
    cover.to_path_buf()
}

/// Decode the covers in `covers` on a worker thread and warm the texture cache
/// as each one lands.
///
/// `covers` should already be trimmed to what is worth preloading — see
/// [`ahead_of`]. Call this from the main thread.
pub fn warm_covers(covers: Vec<PathBuf>, w: i32, h: i32) {
    if covers.is_empty() {
        return;
    }
    crate::tasks::spawn_stream(
        move |reporter, emit| {
            for cover in covers {
                // Cheap to check and worth checking: closing the page should
                // not leave a thread decoding covers nobody will see.
                if reporter.cancelled() {
                    return;
                }
                let src = source_for(&cover, w, h);
                let Some(mut decoded) = decode_rgba(&src, w, h) else {
                    continue;
                };
                // Report the *cover* path even when a thumbnail was decoded,
                // so the cache key matches what the grid will ask for.
                decoded.cover = cover;
                // A closed channel means the UI is gone; stop rather than
                // decode the rest into nothing.
                if !emit.send(decoded) {
                    return;
                }
            }
        },
        |decoded| {
            crate::widgets::book_row::cache_decoded_cover(&decoded);
        },
    );
}

/// The covers worth preloading, given what is already on screen.
///
/// Skips books with no cover and anything already cached, so a second call
/// after a small scroll costs almost nothing.
///
/// Pure and headless apart from the cache probe, which is why the interesting
/// half — the windowing — is tested below.
pub fn ahead_of(
    books: &[crate::models::Book],
    visible_from: usize,
    w: i32,
    h: i32,
) -> Vec<PathBuf> {
    let start = visible_from.min(books.len());
    books
        .iter()
        .skip(start)
        .take(PRELOAD_AHEAD)
        .filter_map(|b| b.cover_path.clone())
        .filter(|p| !crate::widgets::book_row::is_cover_cached(p, w, h))
        .collect()
}

// ---------------------------------------------------------------------------
// Chapter preloader
// ---------------------------------------------------------------------------

/// The next chapter's file, read and warmed in the OS page cache.
///
/// # What this can and cannot do
///
/// A chapter turn is two costs: reading and assembling the HTML (ours) and
/// WebKit parsing plus laying it out (not ours). Only the first is
/// preloadable — a `WebView` belongs to the main thread, and rendering into a
/// second hidden one would cost more memory than the turn costs time.
///
/// So this warms the read. `chapter_html` does a `read_to_string` of a file
/// that, on a cold chapter, is not yet in the page cache; doing that read
/// ahead of time on a worker means the real one is served from RAM.
///
/// It is deliberately *only* a read. Building the HTML needs the current CSS,
/// which changes with theme and font settings, so a cached string would be
/// stale the moment the user changed anything — and a wrong chapter rendered
/// is far worse than a slow one.
pub fn warm_chapter_file(path: PathBuf) {
    if !path.is_file() {
        return;
    }
    crate::tasks::spawn(
        move |_reporter| {
            // The result is discarded on purpose: the point is the side effect
            // on the OS page cache, not the bytes.
            let _ = std::fs::read(&path);
        },
        |_update| {},
        |_done| {},
    );
}

/// The file to warm when the reader is showing `chapter`, if there is a next
/// one.
///
/// Split out from [`warm_chapter_file`] so the "which file" decision — the
/// part with an off-by-one in it — is testable without a reader or a display.
pub fn next_chapter_file(spine: &[crate::epub_book::SpineItem], chapter: usize) -> Option<PathBuf> {
    spine.get(chapter + 1).map(|item| item.path.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unique scratch dir, cleaned up on drop. Same shape as `thumbs.rs`:
    /// tests run in parallel threads of one process, so the pid alone is not
    /// unique enough.
    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "kalam-preload-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.subsec_nanos())
                    .unwrap_or(0)
            ));
            std::fs::create_dir_all(&p).expect("scratch dir");
            Scratch(p)
        }
        fn join(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// `Book` has no `Default`, so spell one out once.
    fn book_with_cover(id: i64, cover: Option<PathBuf>) -> crate::models::Book {
        crate::models::Book {
            id,
            uuid: String::new(),
            title: String::new(),
            authors: String::new(),
            series: None,
            description: String::new(),
            format: crate::models::BookFormat::Epub,
            file_name: String::new(),
            file_hash: String::new(),
            cover_name: None,
            added_at: String::new(),
            progress: 0,
            rating: 0,
            publisher: String::new(),
            published: String::new(),
            series_index: 0.0,
            tags: Vec::new(),
            cover_path: cover,
            file_path: PathBuf::new(),
        }
    }

    /// Same shape as `thumbs.rs`'s test helper — `save_buffer` is the API
    /// known to work under this crate's cut-down `image` features.
    fn write_png(path: &Path, w: u32, h: u32) {
        let buf = vec![90u8; (w * h * 3) as usize];
        image::save_buffer(path, &buf, w, h, image::ExtendedColorType::Rgb8).expect("write png");
    }

    #[test]
    fn decoding_gives_back_exactly_the_requested_size() {
        let dir = Scratch::new();
        let src = dir.join("cover.png");
        write_png(&src, 300, 500);

        let got = decode_rgba(&src, 128, 204).expect("a readable png decodes");
        assert_eq!((got.width, got.height), (128, 204));
        // The buffer must match the dimensions exactly, because the texture is
        // built with a stride computed from them -- a mismatch is a crash or
        // a garbled image, not a wrong-looking one.
        assert_eq!(got.rgba.len(), 128 * 204 * 4);
        assert_eq!(got.cover, src);
    }

    #[test]
    fn unreadable_sources_are_skipped_not_fatal() {
        // A preloader runs unattended over whatever is in the library, so
        // every one of these has to be a quiet `None`.
        let dir = Scratch::new();

        let missing = dir.join("nope.png");
        assert!(decode_rgba(&missing, 10, 10).is_none(), "missing file");

        let garbage = dir.join("garbage.png");
        std::fs::write(&garbage, b"this is not a png").unwrap();
        assert!(decode_rgba(&garbage, 10, 10).is_none(), "not an image");

        let real = dir.join("real.png");
        write_png(&real, 20, 20);
        assert!(decode_rgba(&real, 0, 10).is_none(), "zero width");
        assert!(decode_rgba(&real, 10, -1).is_none(), "negative height");
    }

    #[test]
    fn the_window_starts_after_what_is_visible_and_is_bounded() {
        let books: Vec<crate::models::Book> = (0..100)
            .map(|i| book_with_cover(i, Some(PathBuf::from(format!("/covers/{i}.png")))))
            .collect();

        // Nothing is in the cache in a headless test, so every candidate
        // survives the filter and this measures the windowing alone.
        let got = ahead_of(&books, 30, 128, 204);
        assert_eq!(got.len(), PRELOAD_AHEAD, "capped at PRELOAD_AHEAD");
        assert_eq!(
            got[0],
            PathBuf::from("/covers/30.png"),
            "starts at the mark"
        );
        assert_eq!(got[PRELOAD_AHEAD - 1], PathBuf::from("/covers/53.png"));
    }

    #[test]
    fn a_mark_past_the_end_asks_for_nothing() {
        // Reachable: the grid can shrink under a search while a scroll
        // position from the longer list is still around.
        let books: Vec<crate::models::Book> = (0..3)
            .map(|i| book_with_cover(i, Some(PathBuf::from(format!("/covers/{i}.png")))))
            .collect();
        assert!(ahead_of(&books, 99, 128, 204).is_empty());
    }

    fn spine_item(path: &str) -> crate::epub_book::SpineItem {
        crate::epub_book::SpineItem {
            id: String::new(),
            href: String::new(),
            path: PathBuf::from(path),
            title: String::new(),
        }
    }

    #[test]
    fn the_next_chapter_is_the_one_after_the_current() {
        let spine = vec![spine_item("/a.xhtml"), spine_item("/b.xhtml")];
        assert_eq!(
            next_chapter_file(&spine, 0),
            Some(PathBuf::from("/b.xhtml"))
        );
    }

    #[test]
    fn the_last_chapter_has_nothing_to_preload() {
        // The off-by-one that matters: reading the final chapter must not
        // index past the spine.
        let spine = vec![spine_item("/a.xhtml"), spine_item("/b.xhtml")];
        assert_eq!(next_chapter_file(&spine, 1), None);
        assert_eq!(next_chapter_file(&spine, 99), None, "out of range");
        assert_eq!(next_chapter_file(&[], 0), None, "empty spine");
    }

    #[test]
    fn books_without_a_cover_are_not_queued() {
        let books: Vec<crate::models::Book> = (0..6)
            .map(|i| {
                book_with_cover(
                    i,
                    (i % 2 == 0).then(|| PathBuf::from(format!("/covers/{i}.png"))),
                )
            })
            .collect();
        let got = ahead_of(&books, 0, 128, 204);
        assert_eq!(got.len(), 3, "only the three with covers");
    }
}
