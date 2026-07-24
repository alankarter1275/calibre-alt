use crate::models::LibrarySection;
use gtk::prelude::*;
use relm4::prelude::*;

#[derive(Debug)]
pub enum LibraryOut {
    OpenSection(LibrarySection),
}

pub struct LibraryPageModel;

#[relm4::component(pub)]
impl SimpleComponent for LibraryPageModel {
    type Init = ();
    type Input = ();
    type Output = LibraryOut;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 12,
            set_hexpand: true,

            gtk::Label {
                set_label: "My Library",
                add_css_class: "kalam-page-title",
                set_halign: gtk::Align::Start,
            },
            gtk::Label {
                set_label: "Catalog, lists, quotes, words, tags, and light analytics.",
                add_css_class: "kalam-page-sub",
                set_halign: gtk::Align::Start,
            },

            #[name = "grid_host"]
            gtk::FlowBox {
                set_valign: gtk::Align::Start,
                set_max_children_per_line: 4,
                set_min_children_per_line: 2,
                set_selection_mode: gtk::SelectionMode::None,
                set_column_spacing: 10,
                set_row_spacing: 10,
                add_css_class: "kalam-hub-grid",
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = LibraryPageModel;
        let widgets = view_output!();

        for section in LibrarySection::ALL {
            let tile = make_hub_tile(*section);
            let sec = *section;
            let s = sender.clone();
            // FlowBox children need to be FlowBoxChild or Widget
            let btn = gtk::Button::new();
            btn.set_child(Some(&tile));
            btn.add_css_class("kalam-hub-tile");
            btn.connect_clicked(move |_| {
                s.output(LibraryOut::OpenSection(sec)).ok();
            });
            widgets.grid_host.insert(&btn, -1);
        }

        ComponentParts { model, widgets }
    }
}

fn make_hub_tile(section: LibrarySection) -> gtk::Box {
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 4);
    box_.set_halign(gtk::Align::Center);

    let icon = gtk::Label::new(Some(section.icon()));
    icon.add_css_class("kalam-hub-tile-icon");
    icon.set_halign(gtk::Align::Center);

    let label = gtk::Label::new(Some(section.label()));
    label.add_css_class("kalam-hub-tile-label");
    label.set_halign(gtk::Align::Center);

    let meta = gtk::Label::new(Some(section.blurb()));
    meta.add_css_class("kalam-hub-tile-meta");
    meta.set_halign(gtk::Align::Center);
    meta.set_wrap(true);
    meta.set_max_width_chars(18);
    meta.set_justify(gtk::Justification::Center);

    box_.append(&icon);
    box_.append(&label);
    box_.append(&meta);
    box_
}
