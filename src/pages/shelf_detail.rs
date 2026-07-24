use crate::models::{books_on_shelf, shelf_by_id};
use crate::widgets::book_row::build_book_row;
use gtk::prelude::*;
use relm4::prelude::*;

#[derive(Debug)]
pub enum ShelfDetailOut {
    OpenBook { book_id: u64 },
    OpenBookDialog { book_id: u64 },
}

pub struct ShelfDetailModel {
    #[allow(dead_code)]
    shelf_id: u64,
}

#[relm4::component(pub)]
impl SimpleComponent for ShelfDetailModel {
    type Init = u64;
    type Input = ();
    type Output = ShelfDetailOut;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 12,
            set_hexpand: true,

            #[name = "title"]
            gtk::Label {
                add_css_class: "kalam-page-title",
                set_halign: gtk::Align::Start,
            },
            #[name = "subtitle"]
            gtk::Label {
                add_css_class: "kalam-page-sub",
                set_halign: gtk::Align::Start,
            },

            #[name = "list"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 0,
            },
        }
    }

    fn init(
        shelf_id: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ShelfDetailModel { shelf_id };
        let widgets = view_output!();

        if let Some(shelf) = shelf_by_id(shelf_id) {
            widgets.title.set_label(&shelf.name);
            widgets.subtitle.set_label(&format!(
                "{} · {} books · {}",
                shelf.kind_label(),
                shelf.book_ids.len(),
                shelf.description
            ));
        }

        let books = books_on_shelf(shelf_id);
        if books.is_empty() {
            let empty = gtk::Label::new(Some("This shelf has no books yet."));
            empty.add_css_class("kalam-placeholder");
            widgets.list.append(&empty);
        } else {
            for book in books {
                let id = book.id;
                let s1 = sender.clone();
                let s2 = sender.clone();
                let row = build_book_row(
                    book,
                    move || {
                        s1.output(ShelfDetailOut::OpenBook { book_id: id }).ok();
                    },
                    move || {
                        s2.output(ShelfDetailOut::OpenBookDialog { book_id: id })
                            .ok();
                    },
                );
                widgets.list.append(&row);
            }
        }

        ComponentParts { model, widgets }
    }
}
