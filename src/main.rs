//! Kalam — personal ebook manager & reader (Linux).
//!
//! Phase 5: metadata editing, cover replacement and Open Library lookup, on
//! top of the P1 catalog, P2 reader, P3 annotations and P4 library depth.

mod app;
mod author;
mod db;
mod dict;
mod epub;
mod epub_book;
mod epub_write;
mod icons;
mod metadata;
mod models;
mod notify;
mod pages;
mod paths;
mod perf;
mod service;
mod shelf_rules;
mod style;
mod tasks;
mod theme;
mod thumbs;
mod timing;
mod webview_pool;
mod widgets;

use app::AppModel;
use relm4::RelmApp;

fn main() {
    // A0 step 1: GUI timing. KALAM_TIMING=1 prints cold-start / book-open /
    // chapter-turn / dict-lookup milliseconds to the terminal (no-op otherwise).
    timing::start();

    // RelmApp::new initializes GTK; only touch Adwaita/GTK after that.
    let app = RelmApp::new("app.kalam.Kalam");

    // Dark baseline via Adwaita (GtkSettings prefer-dark is unsupported with libadwaita).
    let style = adw::StyleManager::default();
    style.set_color_scheme(adw::ColorScheme::ForceDark);

    // Ensure data dirs exist before the catalog is opened to read the theme.
    if let Err(err) = paths::ensure_data_dirs() {
        // Nothing will work if this failed, so say so on screen rather than
        // only on a terminal the user probably did not launch from. The
        // toast queues until the window exists.
        eprintln!("kalam: failed to create data directories: {err}");
        crate::notify::error("Could not create Kalam's data folders", &err.to_string());
    }

    // Install the saved theme before the first window is drawn, so the app
    // never flashes the default palette on the way to the chosen one. Read
    // straight from the prefs table: AppModel opens its own handle a moment
    // later, and threading one through just for this would be worse.
    //
    // KALAM_NO_CSS=1 skips the stylesheet entirely. Kept as a diagnostic: it is
    // how the pixman scrollbar bug was finally pinned on this file rather than
    // on GTK, after several wrong guesses.
    if std::env::var_os("KALAM_NO_CSS").is_none() {
        theme::apply(&startup_theme());
    } else {
        eprintln!("kalam: KALAM_NO_CSS set — running with stock GTK styling");
    }

    // Fail here, not inside AppModel::init. The app cannot do anything without
    // a catalog, and init() runs inside a GTK signal callback where a panic
    // cannot unwind: it aborts the process with a core dump and a backtrace
    // instead of saying what is wrong. Checking first turns "Aborted (core
    // dumped)" into one readable line and exit code 1.
    if let Err(err) = db::Catalog::open() {
        eprintln!("kalam: cannot open the library database.");
        eprintln!("  {err}");
        eprintln!("  file: {}", paths::catalog_db().display());
        eprintln!();
        eprintln!("If that file is corrupt, move it aside and restart:");
        let db = paths::catalog_db();
        eprintln!("  mv {} {}.broken", db.display(), db.display());
        eprintln!("Kalam will build a fresh library. Your book files are kept");
        eprintln!(
            "separately in {} and are not affected.",
            paths::library_dir().display()
        );
        std::process::exit(1);
    }

    app.run::<AppModel>(());
}

/// The saved theme, or the default if the catalog cannot be read yet.
///
/// A failure here is not worth reporting: the catalog is opened again
/// immediately afterwards by `AppModel`, which surfaces the real error.
fn startup_theme() -> theme::Theme {
    match db::Catalog::open() {
        Ok(catalog) => theme::current(&catalog),
        Err(_) => theme::DEFAULT,
    }
}

// Re-export adw for StyleManager (relm4 enables libadwaita).
use relm4::adw;
