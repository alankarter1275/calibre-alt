//! Kalam — personal ebook manager & reader (Linux).
//!
//! Phase 0: application shell with slim sidebar navigation and
//! demo flows for Home, Library, Shelves → Shelf → Book.

mod app;
mod models;
mod pages;
mod style;
mod widgets;

use app::AppModel;
use relm4::{gtk, RelmApp};

fn main() {
    // RelmApp::new initializes GTK; only touch Settings after that.
    let app = RelmApp::new("app.kalam.Kalam");

    // Prefer a dark baseline until the custom design system lands.
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(true);
    }

    relm4::set_global_css(style::APP_CSS);
    app.run::<AppModel>(());
}
