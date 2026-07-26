use crate::db::{Catalog, SortKey};
use crate::widgets::book_row::{build_book_card, CARD_H, CARD_W};
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
            set_vexpand: false,
            set_margin_all: 0,

            gtk::Label {
                set_label: "Home",
                add_css_class: "kalam-page-title",
                set_halign: gtk::Align::Start,
            },
            gtk::Label {
                set_label: "Continue reading and recently added · click = float · Ctrl+click = full page",
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
                set_orientation: gtk::Orientation::Horizontal,
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Start,
                set_hexpand: true,
                set_vexpand: false,
                set_margin_bottom: 8,
            },

            gtk::Label {
                set_label: "RECENTLY ADDED",
                add_css_class: "kalam-section-label",
                set_halign: gtk::Align::Start,
            },

            #[name = "recent_host"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_halign: gtk::Align::Fill,
                set_hexpand: true,
                set_vexpand: false,
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

        let cont = books
            .iter()
            .find(|b| b.progress > 0 && b.progress < 100)
            .or_else(|| books.first());

        if let Some(book) = cont {
            let id = book.id;
            let s1 = sender.clone();
            let s2 = sender.clone();
            let card = build_book_card(
                book,
                move || {
                    s1.output(HomeOut::OpenBook { book_id: id }).ok();
                },
                move || {
                    s2.output(HomeOut::OpenBookDialog { book_id: id }).ok();
                },
            );
            widgets.continue_host.append(&card);
        } else {
            let empty = gtk::Label::new(Some(
                "Nothing to continue — import books from My Library → All books.",
            ));
            empty.add_css_class("kalam-placeholder");
            empty.set_wrap(true);
            empty.set_halign(gtk::Align::Start);
            widgets.continue_host.append(&empty);
        }

        let recent: Vec<_> = books.iter().take(12).cloned().collect();
        if recent.is_empty() {
            let empty = gtk::Label::new(Some("Your library is empty."));
            empty.add_css_class("kalam-muted");
            empty.set_halign(gtk::Align::Start);
            widgets.recent_host.append(&empty);
        } else {
            // Use FlowBox for responsive wrapping — avoids crooked overflow after reader
            let flow = gtk::FlowBox::builder()
                .max_children_per_line(6)
                .min_children_per_line(2)
                .selection_mode(gtk::SelectionMode::None)
                .column_spacing(16)
                .row_spacing(20)
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Start)
                .hexpand(true)
                .vexpand(false)
                .build();
            flow.add_css_class("kalam-book-grid");
            flow.add_css_class("kalam-home-flow");

            for book in &recent {
                let id = book.id;
                let s1 = sender.clone();
                let s2 = sender.clone();
                let card = build_book_card(
                    book,
                    {
                        let s = s1.clone();
                        move || {
                            s.output(HomeOut::OpenBook { book_id: id }).ok();
                        }
                    },
                    {
                        let s = s2.clone();
                        move || {
                            s.output(HomeOut::OpenBookDialog { book_id: id }).ok();
                        }
                    },
                );
                // Wrap card in fixed cell to keep uniform size in FlowBox
                let cell = gtk::Box::new(gtk::Orientation::Vertical, 0);
                cell.set_size_request(CARD_W, CARD_H);
                cell.set_halign(gtk::Align::Center);
                cell.set_valign(gtk::Align::Start);
                cell.append(&card);
                flow.insert(&cell, -1);
            }
            widgets.recent_host.append(&flow);
        }

        ComponentParts { model, widgets }
    }
}
