//! Kalam — personal ebook manager & reader (Linux).
//!
//! Phase 1: persistent library + EPUB import on top of the P0 shell.

mod app;
mod db;
mod epub;
mod epub_book;
mod models;
mod pages;
mod paths;
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
