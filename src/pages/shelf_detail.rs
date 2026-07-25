//! Kept as a stub for route compatibility; shelves return in P4.

use gtk::prelude::*;
use relm4::prelude::*;

#[derive(Debug)]
pub enum ShelfDetailOut {
    #[allow(dead_code)]
    OpenBook { book_id: i64 },
    #[allow(dead_code)]
    OpenBookDialog { book_id: i64 },
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

            gtk::Label {
                set_label: "Shelf",
                add_css_class: "kalam-page-title",
                set_halign: gtk::Align::Start,
            },
            gtk::Label {
                set_label: "Shelves are not available until P4.",
                add_css_class: "kalam-placeholder",
                set_wrap: true,
            },
        }
    }

    fn init(
        shelf_id: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ShelfDetailModel { shelf_id };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }
}
