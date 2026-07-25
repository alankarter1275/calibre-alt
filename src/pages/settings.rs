use crate::paths::{catalog_db, data_dir, library_dir};
use gtk::prelude::*;
use relm4::prelude::*;

pub struct SettingsPageModel;

#[relm4::component(pub)]
impl SimpleComponent for SettingsPageModel {
    type Init = ();
    type Input = ();
    type Output = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 12,
            set_hexpand: true,

            gtk::Label {
                set_label: "Settings",
                add_css_class: "kalam-page-title",
                set_halign: gtk::Align::Start,
            },
            gtk::Label {
                set_label: "Paths used by this installation (read-only for P1).",
                add_css_class: "kalam-page-sub",
                set_halign: gtk::Align::Start,
            },

            gtk::Label {
                set_label: "DATA DIRECTORY",
                add_css_class: "kalam-detail-section-title",
                set_halign: gtk::Align::Start,
            },
            gtk::Label {
                set_label: data_dir().to_string_lossy().as_ref(),
                add_css_class: "kalam-muted",
                set_halign: gtk::Align::Start,
                set_selectable: true,
            },

            gtk::Label {
                set_label: "CATALOG DATABASE",
                add_css_class: "kalam-detail-section-title",
                set_halign: gtk::Align::Start,
            },
            gtk::Label {
                set_label: catalog_db().to_string_lossy().as_ref(),
                add_css_class: "kalam-muted",
                set_halign: gtk::Align::Start,
                set_selectable: true,
            },

            gtk::Label {
                set_label: "LIBRARY FILES",
                add_css_class: "kalam-detail-section-title",
                set_halign: gtk::Align::Start,
            },
            gtk::Label {
                set_label: library_dir().to_string_lossy().as_ref(),
                add_css_class: "kalam-muted",
                set_halign: gtk::Align::Start,
                set_selectable: true,
            },

            gtk::Label {
                set_label: "Appearance, dictionary packs, and shortcuts come in later phases.",
                add_css_class: "kalam-placeholder",
                set_wrap: true,
                set_margin_top: 12,
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = SettingsPageModel;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }
}
