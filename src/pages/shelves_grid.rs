use crate::models::sample_shelves;
use crate::widgets::shelf_card::build_shelf_card;
use gtk::prelude::*;
use relm4::prelude::*;

#[derive(Debug)]
pub enum ShelvesOut {
    OpenShelf { shelf_id: u64 },
}

pub struct ShelvesGridModel;

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

            #[name = "grid"]
            gtk::FlowBox {
                set_valign: gtk::Align::Start,
                set_max_children_per_line: 2,
                set_min_children_per_line: 2,
                set_selection_mode: gtk::SelectionMode::None,
                set_column_spacing: 12,
                set_row_spacing: 12,
                set_homogeneous: true,
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ShelvesGridModel;
        let widgets = view_output!();

        for shelf in sample_shelves() {
            let id = shelf.id;
            let s = sender.clone();
            let card = build_shelf_card(shelf, move || {
                s.output(ShelvesOut::OpenShelf { shelf_id: id }).ok();
            });
            widgets.grid.insert(&card, -1);
        }

        ComponentParts { model, widgets }
    }
}
