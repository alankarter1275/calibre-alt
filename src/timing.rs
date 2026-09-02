//! A0 step 1 — in-app GUI timing harness, gated behind `KALAM_TIMING=1`.
//!
//! The data-layer cost was measured by `perf.rs` (headless, automated). The
//! GUI half — cold start, book open, chapter turn, dict lookup — needs a real
//! display, so it is measured by *you* on the Arch machine. This module makes
//! that trivial and repeatable: set `KALAM_TIMING=1`, do the action, and the
//! app prints elapsed milliseconds to the terminal.
//!
//! It is deliberately tiny and thread-safe (a `Mutex<HashMap>` for open spans,
//! an `OnceLock<bool>` for the "enabled" flag). When the env var is absent every
//! call returns immediately and prints nothing, so there is zero overhead in
//! normal use.
//!
//! ```text
//! KALAM_TIMING=1 cargo run --release
//! ```
//!
//! Then read the `[timing]` lines. They mark the boundaries A0 cares about:
//!   app           → cold-start countdown reference (`timing::start()`)
//!   window_shown  → first window drawn (cold-start END) — a `now` snapshot
//!   book_open     → reader init done, EPUB parsed (a span)
//!   chapter_load  → chapter HTML handed to WebKit (a span)
//!   chapter_done  → WebKit finished rendering (span_end of chapter_load)
//!   dict_lookup   → dictionary search returned (a span)
//!
//! A book open + one chapter turn is enough to answer whether the reader paths
//! need preloaders (A0 step 5).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static ENABLED: OnceLock<bool> = OnceLock::new();
static START: OnceLock<Instant> = OnceLock::new();
static SPANS: OnceLock<Mutex<HashMap<&'static str, Instant>>> = OnceLock::new();

fn enabled() -> bool {
    *ENABLED.get_or_init(|| std::env::var_os("KALAM_TIMING").is_some())
}

fn start_anchor() -> Instant {
    *START.get_or_init(Instant::now)
}

/// Record the process-start anchor. Call once, near the top of `main()` (before
/// the GTK main loop). Safe to call more than once: the first call wins.
pub fn start() {
    let _ = START.get_or_init(Instant::now);
}

/// Print a single snapshot: `label` as elapsed ms since the process began.
/// For cold start, call this at `window_shown` (no-op unless timing is on).
pub fn now(label: &'static str) {
    if !enabled() {
        return;
    }
    let el = start_anchor().elapsed().as_secs_f64() * 1000.0;
    println!("[timing] {label:<18} {el:>8.1} ms");
}

/// Begin a named span (e.g. "book_open"). If a span with the same label is
/// already open it is replaced.
pub fn span(label: &'static str) {
    if !enabled() {
        return;
    }
    SPANS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("timing span lock")
        .insert(label, Instant::now());
}

/// End a named span opened by `span(label)` and print its elapsed ms. Printing
/// the label of the *previous* named point by hand is useful when one span feeds
/// another (chapter_load → chapter_done).
pub fn span_end(label: &'static str) {
    if !enabled() {
        return;
    }
    let Some(start) = SPANS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("timing span lock")
        .remove(label)
    else {
        return;
    };
    let el = start.elapsed().as_secs_f64() * 1000.0;
    println!("[timing] {label:<18} {el:>8.1} ms");
}
