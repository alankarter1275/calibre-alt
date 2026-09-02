//! A0 — one long-lived WebKit `WebView`, reused across book opens.
//!
//! # Why
//!
//! The reader used to call `webkit6::WebView::new()` in `init()`, so every book
//! open spawned a fresh WebKit web process. The A0 step-1 measurements put that
//! at **~400 ms** (versus ~3.5 ms to revisit a book whose process was still
//! warm) — the single largest avoidable delay the timing harness found.
//!
//! This module keeps exactly **one** parked `WebView` alive between readers.
//! The reader borrows it in `init()` and hands it back in `shutdown()`, so the
//! WebKit process stays warm and the second book open skips the spawn.
//!
//! # Why pool the WebView instead of caching the reader page
//!
//! Keeping the whole reader component parked (via the app's page cache) would
//! also keep the WebView alive, but it would keep everything *else* alive too:
//! the reading session would go on counting minutes while the user browsed the
//! library, and progress would not be written until the page was finally
//! dropped. Pooling only the widget keeps the reader's lifecycle exactly as it
//! is — session start/end and progress save still happen on every entry and
//! exit — and moves nothing but the expensive object.
//!
//! # Handler discipline (the part that bites)
//!
//! A recycled `WebView` still carries the signal handlers the *previous* reader
//! connected, each holding that component's `Sender`. Left alone they pile up
//! one set per book open and deliver events to dead components. So the split is:
//!
//! - **Permanent, set up here once** — sizing, the context-menu suppression
//!   (captures nothing), and `register_script_message_handler("kalam")`, which
//!   WebKit refuses to accept twice for the same name on one manager.
//! - **Per reader, connected there and disconnected in `shutdown()`** —
//!   everything that captures a `ComponentSender`. The reader records those
//!   `SignalHandlerId`s and drops them before calling [`release`].
//!
//! # Memory
//!
//! Parking a `WebView` holds its WebKit process (~100–200 MB) after the first
//! book is opened, instead of releasing it on leave. That is the deliberate
//! trade for the 400 ms. It is bounded — one view, never a per-book pool — and
//! the page is blanked on release so the book's DOM is freed even though the
//! process is not. `KALAM_NO_WEBVIEW_POOL=1` turns pooling off (the view is
//! dropped on leave and rebuilt on entry, i.e. the old behaviour) for
//! A/B measurement against `KALAM_TIMING=1`, and as an escape hatch on a
//! memory-tight machine.

use gtk::prelude::*;
use std::cell::RefCell;
use webkit6::prelude::*;

thread_local! {
    /// The parked view, if any. Thread-local because GTK objects belong to the
    /// main thread — which also makes it impossible to touch this from a worker.
    static PARKED: RefCell<Option<webkit6::WebView>> = const { RefCell::new(None) };
}

/// `KALAM_NO_WEBVIEW_POOL=1` disables reuse. Same spirit as `KALAM_NO_CSS=1`:
/// a one-variable way to prove whether this module is responsible for a
/// behaviour before theorising about it.
fn pooling_enabled() -> bool {
    pooling_enabled_for(std::env::var_os("KALAM_NO_WEBVIEW_POOL").as_deref())
}

/// The env-var rule as a pure function, so it is unit-testable without a
/// display (constructing a real `WebView` needs GTK and a screen; CI has
/// neither) and without mutating process-wide state mid-test-run.
///
/// Presence disables pooling, whatever the value — matching `KALAM_NO_CSS`,
/// which also only checks that the variable is set.
fn pooling_enabled_for(flag: Option<&std::ffi::OsStr>) -> bool {
    flag.is_none()
}

/// Take the parked `WebView`, or build one if the pool is empty.
///
/// The returned view is parentless, blank, and carries only the permanent
/// setup described in the module docs — the caller connects its own handlers
/// and must disconnect them before calling [`release`].
///
/// Nothing is created until the first call, so a session that never opens a
/// book never starts WebKit at all.
pub fn acquire() -> webkit6::WebView {
    if pooling_enabled() {
        if let Some(view) = PARKED.with(|parked| parked.borrow_mut().take()) {
            return view;
        }
    }

    let view = webkit6::WebView::new();
    view.set_hexpand(true);
    view.set_vexpand(true);
    // The reader is not a browser: suppress WebKit's Back/Forward/Stop/Reload
    // context menu so a right-click cannot navigate the EPUB view. Captures
    // nothing, so it is safe to leave connected for the view's whole life.
    view.connect_context_menu(|_, _, _| true);
    // Registered once per view: WebKit rejects a second registration of the
    // same handler name on one content manager. The per-reader part is the
    // `script-message-received::kalam` *signal*, connected by the reader.
    if let Some(ucm) = view.user_content_manager() {
        let _ = ucm.register_script_message_handler("kalam", None);
    }
    view
}

/// Hand a `WebView` back after the reader is done with it.
///
/// The caller must already have disconnected every handler it connected.
/// Detaches the view from its container, stops and blanks the page, and parks
/// it for the next reader (or drops it when pooling is disabled).
pub fn release(view: webkit6::WebView) {
    // A parked view must be parentless: the next reader appends it to its own
    // container, and GTK aborts if a widget already has a parent.
    if let Some(parent) = view.parent() {
        match parent.downcast_ref::<gtk::Box>() {
            Some(container) => container.remove(&view),
            None => view.unparent(),
        }
    }

    // Blank the page so the finished book's DOM, images and any running timers
    // go away. The process stays warm; only the document is discarded. Any
    // Finished event this triggers lands with no handlers connected, and the
    // next reader's chapter load supersedes it.
    view.stop_loading();
    view.load_html("<!doctype html><html><head></head><body></body></html>", None);

    if !pooling_enabled() {
        return;
    }
    PARKED.with(|parked| {
        *parked.borrow_mut() = Some(view);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn pooling_is_on_by_default() {
        assert!(
            pooling_enabled_for(None),
            "with the escape-hatch variable unset the WebView must be reused"
        );
    }

    #[test]
    fn the_escape_hatch_disables_pooling_whatever_its_value() {
        // Presence is what counts, exactly like KALAM_NO_CSS. Someone setting
        // it to "0" wants it off, and silently pooling anyway would be a trap.
        for value in ["1", "0", "", "yes"] {
            assert!(
                !pooling_enabled_for(Some(OsStr::new(value))),
                "KALAM_NO_WEBVIEW_POOL={value:?} must disable pooling"
            );
        }
    }
}
