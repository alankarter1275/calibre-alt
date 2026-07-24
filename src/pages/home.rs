use crate::models::{book_by_id, sample_books};
use crate::widgets::book_row::build_book_row;
use gtk::prelude::*;
use relm4::prelude::*;

#[derive(Debug)]
pub enum HomeOut {
    OpenBook { book_id: u64 },
    OpenBookDialog { book_id: u64 },
}

pub struct HomePageModel;

#[relm4::component(pub)]
impl SimpleComponent for HomePageModel {
    type Init = ();
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
                set_label: "Continue reading, recent activity, and your queue.",
                add_css_class: "kalam-page-sub",
                set_halign: gtk::Align::Start,
            },

            // Continue card
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                add_css_class: "kalam-home-continue",
                set_spacing: 10,

                gtk::Label {
                    set_label: "CONTINUE",
                    add_css_class: "kalam-section-label",
                    set_halign: gtk::Align::Start,
                },

                #[name = "continue_host"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                },
            },

            gtk::Label {
                set_label: "RECENT",
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
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = HomePageModel;
        let widgets = view_output!();

        // Continue = highest in-progress sample book
        if let Some(book) = sample_books()
            .iter()
            .filter(|b| b.progress > 0 && b.progress < 100)
            .max_by_key(|b| b.progress)
        {
            let id = book.id;
            let s1 = sender.clone();
            let s2 = sender.clone();
            let row = build_book_row(
                book,
                move || {
                    s1.output(HomeOut::OpenBook { book_id: id }).ok();
                },
                move || {
                    s2.output(HomeOut::OpenBookDialog { book_id: id }).ok();
                },
            );
            widgets.continue_host.append(&row);
        } else if let Some(book) = book_by_id(1) {
            let id = book.id;
            let s1 = sender.clone();
            let s2 = sender.clone();
            let row = build_book_row(
                book,
                move || {
                    s1.output(HomeOut::OpenBook { book_id: id }).ok();
                },
                move || {
                    s2.output(HomeOut::OpenBookDialog { book_id: id }).ok();
                },
            );
            widgets.continue_host.append(&row);
        }

        for book in sample_books().iter().take(4) {
            let id = book.id;
            let s1 = sender.clone();
            let s2 = sender.clone();
            let row = build_book_row(
                book,
                move || {
                    s1.output(HomeOut::OpenBook { book_id: id }).ok();
                },
                move || {
                    s2.output(HomeOut::OpenBookDialog { book_id: id }).ok();
                },
            );
            widgets.recent_host.append(&row);
        }

        ComponentParts { model, widgets }
    }
}
