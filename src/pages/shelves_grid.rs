use gtk::prelude::*;
use relm4::prelude::*;

/// P1: shelves engine is P4 — show honest empty state.
pub struct ShelvesGridModel;

#[derive(Debug)]
pub enum ShelvesOut {
    #[allow(dead_code)]
    OpenShelf { shelf_id: u64 },
}

#[relm4::component(pub)]
impl SimpleComponent for ShelvesGridModel {
    type Init = ();
    type Input = ();
    type Output = ShelvesOut;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 12,
            set_hexpand: true,

            gtk::Label {
                set_label: "Shelves",
                add_css_class: "kalam-page-title",
                set_halign: gtk::Align::Start,
            },
            gtk::Label {
                set_label: concat!(
                    "Smart filters and manual collections — ",
                    "like Calibre virtual libraries.",
                ),
                add_css_class: "kalam-page-sub",
                set_halign: gtk::Align::Start,
            },

            gtk::Label {
                set_label: concat!(
                    "Shelves are planned for P4, once the real library is solid.\n",
                    "Import books under My Library → All books for now.",
                ),
                add_css_class: "kalam-placeholder",
                set_wrap: true,
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ShelvesGridModel;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }
}
