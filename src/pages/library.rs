use crate::db::Catalog;
use crate::models::LibrarySection;
use gtk::prelude::*;
use relm4::prelude::*;
use std::rc::Rc;

#[derive(Debug)]
pub enum LibraryOut {
    OpenSection(LibrarySection),
}

pub struct LibraryPageModel {
    catalog: Rc<Catalog>,
    count: usize,
}

#[relm4::component(pub)]
impl SimpleComponent for LibraryPageModel {
    type Init = Rc<Catalog>;
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
                #[watch]
                set_label: &format!(
                    "{} book{} · catalog, lists, quotes, words, tags, analytics.",
                    model.count,
                    if model.count == 1 { "" } else { "s" }
                ),
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
        catalog: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let count = catalog.count_books().unwrap_or(0);
        let model = LibraryPageModel { catalog, count };
        let widgets = view_output!();

        // One stats query for all tiles rather than one per tile.
        let stats = model.catalog.library_stats().unwrap_or_default();
        let tag_count = model
            .catalog
            .list_tags_with_counts()
            .map(|t| t.len() as i64)
            .unwrap_or(0);

        for section in LibrarySection::ALL {
            let tile = make_hub_tile(*section, section_count(*section, &stats, tag_count));
            let sec = *section;
            let s = sender.clone();
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

/// Live count shown under each hub tile, or None where a count is meaningless.
fn section_count(
    section: LibrarySection,
    stats: &crate::db::LibraryStats,
    tag_count: i64,
) -> Option<i64> {
    match section {
        LibrarySection::AllBooks => Some(stats.total_books),
        LibrarySection::ReadingList => Some(stats.reading_list),
        LibrarySection::SavedQuotes => Some(stats.quotes),
        LibrarySection::SavedWords => Some(stats.saved_words),
        LibrarySection::Tags => Some(tag_count),
        LibrarySection::History | LibrarySection::Analytics => None,
    }
}

fn make_hub_tile(section: LibrarySection, count: Option<i64>) -> gtk::Box {
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 4);
    box_.set_halign(gtk::Align::Center);

    let icon = gtk::Label::new(Some(section.icon()));
    icon.add_css_class("kalam-hub-tile-icon");
    icon.set_halign(gtk::Align::Center);

    let label = gtk::Label::new(Some(section.label()));
    label.add_css_class("kalam-hub-tile-label");
    label.set_halign(gtk::Align::Center);

    if let Some(n) = count {
        let badge = gtk::Label::new(Some(&n.to_string()));
        badge.add_css_class("kalam-hub-tile-count");
        badge.set_halign(gtk::Align::Center);
        box_.append(&icon);
        box_.append(&badge);
    }

    let meta = gtk::Label::new(Some(section.blurb()));
    meta.add_css_class("kalam-hub-tile-meta");
    meta.set_halign(gtk::Align::Center);
    meta.set_wrap(true);
    meta.set_max_width_chars(18);
    meta.set_justify(gtk::Justification::Center);

    if count.is_none() {
        box_.append(&icon);
    }
    box_.append(&label);
    box_.append(&meta);
    box_
}
