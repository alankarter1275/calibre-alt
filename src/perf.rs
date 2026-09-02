//! A0 step 1 — measurement harness for the **data layer**.
//!
//! These run against an in-memory catalog, so they need no display and are safe
//! in CI. They are `#[ignore]`d by default so the normal `cargo test` run stays
//! fast; run them when you want a baseline or a regression check:
//!
//! ```text
//! cargo test --release perf -- --ignored --nocapture
//! ```
//!
//! What this measures: the cost of the queries that feed every list page. The
//! other half of A0 — cold start, book open, chapter turn, grid scroll — is
//! GUI-side and needs the user's Arch machine (`perf`, sysprof, GTK inspector);
//! see A0 in `ROADMAP.md` and `docs/conversation.md` §§1–3.
//!
//! Sizing: 2,000 books is the A0 acceptance target, so that's what we seed.

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
