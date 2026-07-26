use crate::db::Catalog;
use crate::dict;
use crate::paths::{catalog_db, data_dir, dictionaries_dir, library_dir};
use gtk::prelude::*;
use relm4::prelude::*;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Debug)]
pub enum SettingsMsg {
    ImportDict,
    DeleteDict(i64),
    Refresh,
}

pub struct SettingsPageModel {
    catalog: Rc<Catalog>,
    dicts: Vec<crate::db::Dictionary>,
    status: String,
}

#[relm4::component(pub)]
impl Component for SettingsPageModel {
    type Init = Rc<Catalog>;
    type Input = SettingsMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 14,
            set_hexpand: true,

            gtk::Label {
                set_label: "Settings",
                add_css_class: "kalam-page-title",
                set_halign: gtk::Align::Start,
            },
            gtk::Label {
                set_label: "Data locations and dictionary packs.",
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
                set_label: "DICTIONARIES DIRECTORY",
                add_css_class: "kalam-detail-section-title",
                set_halign: gtk::Align::Start,
            },
            gtk::Label {
                set_label: dictionaries_dir().to_string_lossy().as_ref(),
                add_css_class: "kalam-muted",
                set_halign: gtk::Align::Start,
                set_selectable: true,
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 10,
                gtk::Label {
                    set_label: "Offline dictionaries",
                    add_css_class: "kalam-detail-section-title",
                    set_halign: gtk::Align::Start,
                    set_hexpand: true,
                },
                gtk::Button {
                    set_label: "+ Import dictionary",
                    add_css_class: "kalam-secondary-btn",
                    set_tooltip_text: Some("Import StarDict (.ifo) or SQLite .db or TSV .txt"),
                    connect_clicked => SettingsMsg::ImportDict,
                },
                gtk::Button {
                    set_label: "↻",
                    add_css_class: "kalam-secondary-btn",
                    connect_clicked => SettingsMsg::Refresh,
                },
            },

            #[name = "dict_status"]
            gtk::Label {
                add_css_class: "kalam-muted",
                set_halign: gtk::Align::Start,
                set_wrap: true,
            },

            #[name = "dict_list"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 8,
            },
        }
    }

    fn init(
        catalog: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let dicts = catalog.list_dictionaries().unwrap_or_default();
        let status = if dicts.is_empty() {
            "No dictionaries imported. Import StarDict (.ifo/.idx/.dict) or SQLite pack for offline lookup (D key in reader).".to_string()
        } else {
            format!("{} dictionary pack{} imported.", dicts.len(), if dicts.len()==1{""} else {"s"})
        };
        let model = SettingsPageModel {
            catalog,
            dicts,
            status,
        };
        let widgets = view_output!();
        widgets.dict_status.set_label(&model.status);
        rebuild_dicts(&widgets.dict_list, &model.dicts, &sender);
        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            SettingsMsg::Refresh => {
                self.refresh();
                widgets.dict_status.set_label(&self.status);
                rebuild_dicts(&widgets.dict_list, &self.dicts, &sender);
            }
            SettingsMsg::ImportDict => {
                // GTK file chooser dialog (native)
                let dialog = gtk::FileDialog::builder()
                    .title("Import dictionary")
                    .build();
                let sender_clone = sender.clone();
                let catalog_clone = self.catalog.clone();
                dialog.open(
                    None::<&gtk::Window>,
                    gtk::gio::Cancellable::NONE,
                    move |res| {
                        if let Ok(file) = res {
                            if let Some(path) = file.path() {
                                match dict::import_dictionary(&catalog_clone, &path) {
                                    Ok((name, count)) => {
                                        eprintln!("kalam: dict imported {name} {count} entries");
                                    }
                                    Err(e) => {
                                        eprintln!("kalam: dict import failed {}: {e:#}", path.display());
                                    }
                                }
                                sender_clone.input(SettingsMsg::Refresh);
                            }
                        }
                    },
                );
            }
            SettingsMsg::DeleteDict(id) => {
                let _ = self.catalog.delete_dictionary(id);
                self.refresh();
                widgets.dict_status.set_label(&self.status);
                rebuild_dicts(&widgets.dict_list, &self.dicts, &sender);
            }
        }
        self.update_view(widgets, sender);
    }
}

impl SettingsPageModel {
    fn refresh(&mut self) {
        self.dicts = self.catalog.list_dictionaries().unwrap_or_default();
        let total_entries = self.catalog.dict_entry_count().unwrap_or(0);
        if self.dicts.is_empty() {
            self.status = "No dictionaries.".into();
        } else {
            self.status = format!("{} dicts, {} total entries.", self.dicts.len(), total_entries);
        }
    }
}

fn rebuild_dicts(
    list: &gtk::Box,
    dicts: &[crate::db::Dictionary],
    sender: &ComponentSender<SettingsPageModel>,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    if dicts.is_empty() {
        let l = gtk::Label::new(Some("No dictionaries yet. Supported: StarDict .ifo + .idx + .dict[.dz], SQLite .db with entries(word,definition), or TSV (word<TAB>definition)."));
        l.add_css_class("kalam-placeholder");
        l.set_wrap(true);
        l.set_xalign(0.0);
        list.append(&l);
        return;
    }
    for d in dicts {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        row.add_css_class("kalam-dict-row");
        row.set_margin_bottom(4);

        let icon = gtk::Label::new(Some("📖"));
        row.append(&icon);

        let name_l = gtk::Label::new(Some(&d.name));
        name_l.add_css_class("kalam-muted");
        name_l.set_halign(gtk::Align::Start);
        name_l.set_hexpand(true);
        row.append(&name_l);

        let count_l = gtk::Label::new(Some(&format!("{} entries", d.entry_count)));
        count_l.add_css_class("kalam-chip");
        row.append(&count_l);

        let del = gtk::Button::with_label("Remove");
        del.add_css_class("kalam-secondary-btn");
        let id = d.id;
        let s = sender.clone();
        del.connect_clicked(move |_| s.input(SettingsMsg::DeleteDict(id)));
        row.append(&del);

        list.append(&row);
    }
}
