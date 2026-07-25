use crate::db::{Catalog, SortKey};
use crate::widgets::book_row::build_book_grid;
use gtk::prelude::*;
use relm4::prelude::*;
use std::rc::Rc;

#[derive(Debug)]
pub enum HomeOut {
    OpenBook { book_id: i64 },
    OpenBookDialog { book_id: i64 },
}

pub struct HomePageModel;

#[relm4::component(pub)]
impl SimpleComponent for HomePageModel {
    type Init = Rc<Catalog>;
    type Input = ();
    type Output = HomeOut;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 16,
            set_hexpand: true,

            gtk::Label {
                set_label: "Home",
                add_css_class: "kalam-page-title",
                set_halign: gtk::Align::Start,
            },
            gtk::Label {
                set_label: "Continue reading and recently added · click cover = float · Ctrl+click = full page",
                add_css_class: "kalam-page-sub",
                set_halign: gtk::Align::Start,
            },

            gtk::Label {
                set_label: "CONTINUE",
                add_css_class: "kalam-section-label",
                set_halign: gtk::Align::Start,
            },

            #[name = "continue_host"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
            },

            gtk::Label {
                set_label: "RECENTLY ADDED",
                add_css_class: "kalam-section-label",
                set_halign: gtk::Align::Start,
            },

            #[name = "recent_host"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 0,
            },
        }
    }

    fn init(
        catalog: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = HomePageModel;
        let widgets = view_output!();

        let books = catalog.list_books(SortKey::Added, "").unwrap_or_default();

        let cont: Vec<_> = books
            .iter()
            .filter(|b| b.progress > 0 && b.progress < 100)
            .cloned()
            .take(1)
            .collect();
        let cont = if cont.is_empty() {
            books.iter().take(1).cloned().collect::<Vec<_>>()
        } else {
            cont
        };

        if cont.is_empty() {
            let empty = gtk::Label::new(Some(
                "Nothing to continue — import books from My Library → All books.",
            ));
            empty.add_css_class("kalam-placeholder");
            empty.set_wrap(true);
            widgets.continue_host.append(&empty);
        } else {
            let s = sender.clone();
            let s2 = sender.clone();
            let grid = build_book_grid(
                &cont,
                move |id| {
                    s.output(HomeOut::OpenBook { book_id: id }).ok();
                },
                move |id| {
                    s2.output(HomeOut::OpenBookDialog { book_id: id }).ok();
                },
            );
            widgets.continue_host.append(&grid);
        }

        let recent: Vec<_> = books.iter().take(12).cloned().collect();
        if recent.is_empty() {
            let empty = gtk::Label::new(Some("Your library is empty."));
            empty.add_css_class("kalam-muted");
            widgets.recent_host.append(&empty);
        } else {
            let s = sender.clone();
            let s2 = sender.clone();
            let grid = build_book_grid(
                &recent,
                move |id| {
                    s.output(HomeOut::OpenBook { book_id: id }).ok();
                },
                move |id| {
                    s2.output(HomeOut::OpenBookDialog { book_id: id }).ok();
                },
            );
            widgets.recent_host.append(&grid);
        }

        ComponentParts { model, widgets }
    }
}
