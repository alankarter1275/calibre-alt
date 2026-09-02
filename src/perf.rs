//! A0 measurement harness (headless — no display needed, safe in CI).
//!
//! Two probes, both `#[ignore]`d so the normal `cargo test` stays fast:
//!
//! - **Data layer** (`perf_data_layer_report`): the cost of the queries that
//!   feed every list page, seeded with 2,000 books.
//! - **Cover decode** (`perf_cover_decode`): the A0 step 3 win — decoding a
//!   full cover vs the persistent thumbnail for a grid slot.
//!
//! Run them on the Arch machine for a baseline / regression check:
//!
//! ```text
//! cargo test --release perf -- --ignored --nocapture
//! ```
//!
//! The other half of A0 (cold start, book open, chapter turn, grid scroll) is
//! measured in-app via `KALAM_TIMING=1` (see `src/timing.rs`).

#![cfg(test)]

use crate::db::{Catalog, SortKey};
use crate::models::BookFormat;
use std::time::Instant;

const N: usize = 2000;

/// Seed `n` varied books (unique uuid/hash; author, series and tags rotate so
/// every sort order is deterministic-but-non-trivial).
fn seed(cat: &Catalog, n: usize) {
    for i in 0..n {
        let title = format!("Book {i:04} — The Something of the Elsewhere");
        let uuid = format!("perf-uuid-{i:04}");
        let hash = format!("perf-hash-{i:04}");
        let author = format!("Author {}", i % 100);
        let series = (i % 3 == 0).then(|| format!("Series {}", i % 20));
        let tags = vec!["scifi".to_string(), format!("tag{}", i % 50)];
        cat.insert_book(
            &uuid,
            &title,
            &author,
            series.as_deref(),
            "",
            BookFormat::Epub,
            "book.epub",
            &hash,
            None,
            &tags,
        )
        .expect("seed book");
    }
}

/// Time one call, print the label + duration, return just the duration in ms.
macro_rules! timed {
    ($label:expr, $body:expr) => {{
        let start = Instant::now();
        #[allow(clippy::let_unit_value)]
        let _ = $body();
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        println!("{:<40} {:>8.2} ms", $label, ms);
        ms
    }};
}

/// Loose ceilings. The point is to catch an accidental O(n²) regression (a new
/// `for book in books` loop, an added per-row query), not to assert a hard
/// budget — the real numbers come from running this and from the user's Arch
/// machine. Set generously for CI hosts; tighten after a baseline.
const CEILING_MS: f64 = 500.0;

fn assert_under(label: &str, ms: f64) {
    assert!(
        ms < CEILING_MS,
        "perf regression: {label} took {ms:.2} ms (ceiling {CEILING_MS:.2})"
    );
}

#[test]
#[ignore = "run manually: cargo test --release perf -- --ignored --nocapture"]
fn perf_data_layer_report() {
    let cat = Catalog::open_in_memory().unwrap();
    let seed_start = Instant::now();
    seed(&cat, N);
    let seed_ms = seed_start.elapsed().as_secs_f64() * 1000.0;
    println!("== Kalam data-layer probe: {N} books ==");
    println!("{:<40} {:>8.2} ms", "seed", seed_ms);

    // Whole-library listing (the cost behind an "All books" grid refresh).
    let ms = timed!("list_books(Title, \"\")", || {
        cat.list_books(SortKey::Title, "").unwrap()
    });
    assert_under("list_books(Title, \"\")", ms);

    let ms = timed!("list_books(Author, \"\")", || {
        cat.list_books(SortKey::Author, "").unwrap()
    });
    assert_under("list_books(Author, \"\")", ms);

    let ms = timed!("list_books(Added, \"\")", || {
        cat.list_books(SortKey::Added, "").unwrap()
    });
    assert_under("list_books(Added, \"\")", ms);

    // Search (LIKE + ESCAPE) over the whole table.
    let ms = timed!("list_books(Title, \"something\")", || {
        cat.list_books(SortKey::Title, "something").unwrap()
    });
    assert_under("list_books(Title, search)", ms);

    // Small capped pulls (dashboard cards).
    let ms = timed!("recent_books(30)", || cat.recent_books(30).unwrap());
    assert_under("recent_books(30)", ms);

    // Aggregates.
    let ms = timed!("library_stats()", || cat.library_stats().unwrap());
    assert_under("library_stats()", ms);

    let ms = timed!("list_tags_with_counts()", || {
        cat.list_tags_with_counts().unwrap()
    });
    assert_under("list_tags_with_counts()", ms);

    let ms = timed!("books_with_tag(scifi)", || {
        cat.books_with_tag("scifi", SortKey::Title).unwrap()
    });
    assert_under("books_with_tag(scifi)", ms);

    println!("\nAll data-layer probes under {CEILING_MS:.0} ms — no O(n²) regression visible.");
}

/// A0 step 3 — cover decode: full cover vs the persistent thumbnail. This is
/// the cost behind every grid card on a cold launch. Creates a real cover image
/// in a scratch dir and times both decode paths (gdk-pixbuf).
#[test]
#[ignore = "run manually: cargo test --release perf -- --ignored --nocapture"]
fn perf_cover_decode() {
    use gdk_pixbuf::{InterpType, Pixbuf};

    let scratch = std::env::temp_dir().join(format!("kalam-perf-cover-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap();
    let cover = scratch.join("cover.png");
    let thumb = scratch.join("thumb.png");

    // A believable full cover (600×900) and the 256×408 thumbnail.
    let w = 600u32;
    let h = 900u32;
    let buf: Vec<u8> = (0..(w * h)).flat_map(|_| [40u8, 70, 130]).collect();
    image::save_buffer(&cover, &buf, w, h, image::ExtendedColorType::Rgb8).unwrap();
    assert!(crate::thumbs::generate_thumbnail(&cover, &thumb));

    println!("== Cover decode: full cover vs thumbnail (grid slot 128×204) ==");

    let ms = timed!("gtk decode FULL cover → 128×204", || {
        let p = Pixbuf::from_file_at_scale(&cover, 128, 204, false).unwrap();
        let _ = p.scale_simple(128, 204, InterpType::Bilinear).unwrap();
    });
    println!("{:<40} {:>8.2} ms", "(full cover decode)", ms);

    let ms = timed!("gtk decode THUMB → 128×204", || {
        let p = Pixbuf::from_file_at_scale(&thumb, 128, 204, false).unwrap();
        let _ = p.scale_simple(128, 204, InterpType::Bilinear).unwrap();
    });
    println!("{:<40} {:>8.2} ms", "(thumbnail decode)", ms);

    println!("\nGrid-slot decode of the thumbnail is the hot path after import; the full-cover decode is what a cold launch pays per card today.");

    let _ = std::fs::remove_dir_all(&scratch);
}
