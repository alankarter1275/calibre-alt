//! Kalam — personal ebook manager & reader (Linux).
//!
//! Phase 5: metadata editing, cover replacement and Open Library lookup, on
//! top of the P1 catalog, P2 reader, P3 annotations and P4 library depth.

mod app;
mod db;
mod dict;
mod epub;
mod epub_book;
mod models;
mod openlibrary;
mod pages;
mod paths;
mod shelf_rules;
mod style;
mod widgets;

use app::AppModel;
use relm4::RelmApp;

fn main() {
    // RelmApp::new initializes GTK; only touch Adwaita/GTK after that.
    let app = RelmApp::new("app.kalam.Kalam");

    // Dark baseline via Adwaita (GtkSettings prefer-dark is unsupported with libadwaita).
    let style = adw::StyleManager::default();
    style.set_color_scheme(adw::ColorScheme::ForceDark);

    relm4::set_global_css(style::APP_CSS);

    // Ensure data dirs exist early so import never races mkdir.
    if let Err(err) = paths::ensure_data_dirs() {
        eprintln!("kalam: failed to create data directories: {err}");
    }

    app.run::<AppModel>(());
}

// Re-export adw for StyleManager (relm4 enables libadwaita).
use relm4::adw;
