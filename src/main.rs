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
mod shelf_rules;
mod style;
mod theme;
mod widgets;

use app::AppModel;
use relm4::RelmApp;

fn main() {
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
