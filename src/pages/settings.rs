use crate::db::Catalog;
use crate::dict;
use crate::paths::{catalog_db, data_dir, dictionaries_dir, library_dir};
use gtk::prelude::*;
use relm4::prelude::*;
use std::sync::Arc;

#[derive(Debug)]
pub enum SettingsMsg {
    ImportDict,
    DeleteDict(i64),
    Refresh,
}

pub struct SettingsPageModel {
    catalog: Arc<Catalog>,
    dicts: Vec<crate::db::Dictionary>,
    status: String,
}

#[relm4::component(pub)]
impl Component for SettingsPageModel {
    type Init = Arc<Catalog>;
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

            gtk::Label {
                set_label: "Library backup",
                add_css_class: "kalam-page-title",
                set_halign: gtk::Align::Start,
                set_margin_top: 24,
            },

            #[name = "backup_row"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 6,
            },

            gtk::Label {
                set_label: "Book files",
                add_css_class: "kalam-page-title",
                set_halign: gtk::Align::Start,
                set_margin_top: 24,
            },

            #[name = "file_write_row"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 6,
            },

            gtk::Label {
                set_label: "Metadata sources",
                add_css_class: "kalam-page-title",
                set_halign: gtk::Align::Start,
                set_margin_top: 24,
            },
            gtk::Label {
                set_label: concat!(
                    "Used by Edit metadata. Results from every enabled source are ",
                    "merged and badged with their origin."
                ),
                add_css_class: "kalam-page-sub",
                set_halign: gtk::Align::Start,
                set_wrap: true,
            },

            #[name = "source_list"]
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
            format!(
                "{} dictionary pack{} imported.",
                dicts.len(),
                if dicts.len() == 1 { "" } else { "s" }
            )
        };
        let model = SettingsPageModel {
            catalog,
            dicts,
            status,
        };
        let widgets = view_output!();
        widgets.dict_status.set_label(&model.status);
        rebuild_dicts(&widgets.dict_list, &model.dicts, &sender);
        build_sources(&widgets.source_list, &model.catalog);
        build_file_write(&widgets.file_write_row, &model.catalog);
        build_backup(&widgets.backup_row, &model.catalog);
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
                                        eprintln!(
                                            "kalam: dict import failed {}: {e:#}",
                                            path.display()
                                        );
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
            self.status = format!(
                "{} dicts, {} total entries.",
                self.dicts.len(),
                total_entries
            );
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

/// Back up the catalog, and clear the reader cache.
fn build_backup(host: &gtk::Box, catalog: &Arc<Catalog>) {
    while let Some(child) = host.first_child() {
        host.remove(&child);
    }

    let row = gtk::Box::new(gtk::Orientation::Vertical, 8);
    row.add_css_class("kalam-list-row");

    let note = gtk::Label::new(Some(
        "Your highlights, quotes, ratings, shelves, reading history and metadata \
         edits all live in catalog.db. Back it up before upgrades, or to move \
         Kalam to another machine.",
    ));
    note.add_css_class("kalam-card-meta");
    note.set_halign(gtk::Align::Start);
    note.set_xalign(0.0);
    note.set_wrap(true);
    row.append(&note);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let backup_btn = gtk::Button::with_label("Back up library…");
    backup_btn.add_css_class("kalam-secondary-btn");
    {
        let catalog = catalog.clone();
        backup_btn.connect_clicked(move |btn| {
            let dialog = gtk::FileDialog::builder()
                .title("Save library backup")
                .modal(true)
                .initial_name(format!("kalam-backup-{}.db", today_stamp()))
                .build();
            let window = btn.root().and_then(|r| r.downcast::<gtk::Window>().ok());
            let catalog = catalog.clone();
            dialog.save(window.as_ref(), gtk::gio::Cancellable::NONE, move |res| {
                let Ok(file) = res else { return };
                let Some(path) = file.path() else { return };
                match catalog.backup_to(&path) {
                    Ok(size) => crate::notify::success(
                        "Library backed up",
                        &format!(
                            "{} · {}",
                            crate::epub_write::human_size(size),
                            path.display()
                        ),
                    ),
                    Err(err) => {
                        crate::notify::error("Backup failed", &err.to_string());
                    }
                }
            });
        });
    }
    actions.append(&backup_btn);
    row.append(&actions);

    // ── reader cache ────────────────────────────────────────────────────
    let cache_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    cache_row.set_margin_top(6);

    let size = crate::paths::reader_cache_size();
    let cache_label = gtk::Label::new(Some(&format!(
        "Reader cache: {} of extracted books.",
        crate::epub_write::human_size(size)
    )));
    cache_label.add_css_class("kalam-muted");
    cache_label.set_halign(gtk::Align::Start);
    cache_label.set_hexpand(true);
    cache_label.set_xalign(0.0);
    cache_row.append(&cache_label);

    let clear = gtk::Button::with_label("Clear cache");
    clear.add_css_class("kalam-mini-btn");
    clear.set_valign(gtk::Align::Center);
    clear.set_sensitive(size > 0);
    {
        let cache_label = cache_label.clone();
        clear.connect_clicked(move |btn| {
            let (_, freed) = crate::paths::clear_reader_cache();
            cache_label.set_label(&format!(
                "Reader cache cleared, freed {}.",
                crate::epub_write::human_size(freed)
            ));
            btn.set_sensitive(false);
        });
    }
    cache_row.append(&clear);
    row.append(&cache_row);

    let cache_note = gtk::Label::new(Some(
        "Safe to clear: books are re-extracted the next time you open them.",
    ));
    cache_note.add_css_class("kalam-card-meta");
    cache_note.set_halign(gtk::Align::Start);
    cache_note.set_xalign(0.0);
    cache_note.set_wrap(true);
    row.append(&cache_note);

    host.append(&row);
}

/// `2026-07-28`, for backup filenames.
fn today_stamp() -> String {
    gtk::glib::DateTime::now_local()
        .and_then(|d| d.format("%Y-%m-%d"))
        .map(|s| s.to_string())
        .unwrap_or_default()
}

/// Toggle for writing metadata back into the EPUB itself.
fn build_file_write(host: &gtk::Box, catalog: &Arc<Catalog>) {
    use crate::epub_write::{set_write_enabled, write_enabled};

    while let Some(child) = host.first_child() {
        host.remove(&child);
    }

    let row = gtk::Box::new(gtk::Orientation::Vertical, 6);
    row.add_css_class("kalam-list-row");

    let check = gtk::CheckButton::with_label("Also write metadata into the EPUB file");
    check.set_active(write_enabled(catalog));
    {
        let catalog = catalog.clone();
        check.connect_toggled(move |c| set_write_enabled(&catalog, c.is_active()));
    }
    row.append(&check);

    let note = gtk::Label::new(Some(
        "On: saving in Edit metadata also updates the book file, so Calibre and other \
         readers see your changes. The untouched original is kept once as \
         <name>.epub.orig, and the new file is only swapped in after it is verified.\n\
         Off: edits stay inside Kalam and your files are never modified.",
    ));
    note.add_css_class("kalam-card-meta");
    note.set_halign(gtk::Align::Start);
    note.set_xalign(0.0);
    note.set_wrap(true);
    row.append(&note);

    // ── backup cleanup ──────────────────────────────────────────────────
    let backups = crate::epub_write::list_backups();
    let total: u64 = backups.iter().map(|(_, size)| size).sum();

    let cleanup_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    cleanup_row.set_margin_top(6);

    let summary = gtk::Label::new(Some(&if backups.is_empty() {
        "No original backups stored.".to_string()
    } else {
        format!(
            "{} original{} kept, using {}.",
            backups.len(),
            if backups.len() == 1 { "" } else { "s" },
            crate::epub_write::human_size(total)
        )
    }));
    summary.add_css_class("kalam-muted");
    summary.set_halign(gtk::Align::Start);
    summary.set_hexpand(true);
    summary.set_wrap(true);
    summary.set_xalign(0.0);
    cleanup_row.append(&summary);

    let clean = gtk::Button::with_label("Delete backups");
    clean.add_css_class("kalam-mini-btn");
    clean.add_css_class("kalam-mini-btn-danger");
    clean.set_sensitive(!backups.is_empty());
    clean.set_valign(gtk::Align::Center);
    {
        let summary = summary.clone();
        clean.connect_clicked(move |btn| {
            let (count, freed) = crate::epub_write::delete_backups();
            summary.set_label(&format!(
                "Deleted {count} backup{}, freed {}.",
                if count == 1 { "" } else { "s" },
                crate::epub_write::human_size(freed)
            ));
            btn.set_sensitive(false);
        });
    }
    cleanup_row.append(&clean);
    row.append(&cleanup_row);

    let warn = gtk::Label::new(Some(
        "Deleting backups is permanent: you lose the ability to undo metadata \
         written into those files.",
    ));
    warn.add_css_class("kalam-muted");
    warn.set_halign(gtk::Align::Start);
    warn.set_xalign(0.0);
    warn.set_wrap(true);
    row.append(&warn);

    host.append(&row);
}

/// Explains why a country code is needed at all.
fn country_hint_label() -> gtk::Label {
    let label = gtk::Label::new(Some(
        "Two-letter code. Google only serves results for countries it has rights in.",
    ));
    label.add_css_class("kalam-muted");
    label.set_wrap(true);
    label.set_xalign(0.0);
    label
}

/// Toggle each metadata provider, plus the optional Google Books key.
fn build_sources(host: &gtk::Box, catalog: &Arc<Catalog>) {
    use crate::metadata::{set_source_enabled, source_enabled, SourceId};

    while let Some(child) = host.first_child() {
        host.remove(&child);
    }

    for id in SourceId::ALL {
        let row = gtk::Box::new(gtk::Orientation::Vertical, 6);
        row.add_css_class("kalam-list-row");

        let head = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let check = gtk::CheckButton::with_label(id.label());
        check.set_active(source_enabled(catalog, *id));
        check.set_hexpand(true);
        {
            let catalog = catalog.clone();
            let id = *id;
            check.connect_toggled(move |c| set_source_enabled(&catalog, id, c.is_active()));
        }
        head.append(&check);

        let badge = gtk::Label::new(Some(id.badge()));
        badge.add_css_class("kalam-card-badge");
        badge.add_css_class(id.css_class());
        badge.set_valign(gtk::Align::Center);
        head.append(&badge);
        row.append(&head);

        let note = gtk::Label::new(Some(match id {
            SourceId::OpenLibrary => {
                "Internet Archive. No key needed. Strong on older and public-domain titles."
            }
            SourceId::GoogleBooks => {
                "Broad coverage, good for recent and non-English books. Works without a key, \
                 but anonymous requests share a global quota and can be rate limited."
            }
        }));
        note.add_css_class("kalam-card-meta");
        note.set_halign(gtk::Align::Start);
        note.set_xalign(0.0);
        note.set_wrap(true);
        row.append(&note);

        // Google Books is the only source with a key, so the field is local
        // to it rather than a generic per-source setting.
        if *id == SourceId::GoogleBooks {
            let key_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let entry = gtk::Entry::new();
            entry.set_placeholder_text(Some("Optional API key — lifts the shared rate limit"));
            entry.set_text(&catalog.get_pref("meta.googlebooks.key").unwrap_or_default());
            entry.set_hexpand(true);
            key_row.append(&entry);

            let save = gtk::Button::with_label("Save key");
            save.add_css_class("kalam-mini-btn");
            {
                let catalog = catalog.clone();
                let entry = entry.clone();
                save.connect_clicked(move |_| {
                    catalog.set_pref("meta.googlebooks.key", entry.text().trim());
                });
            }
            key_row.append(&save);
            row.append(&key_row);

            let hint = gtk::Label::new(Some(
                "Free from console.cloud.google.com — create a project, enable the Books API, \
                 then make an API key. No card required.",
            ));
            hint.add_css_class("kalam-muted");
            hint.set_halign(gtk::Align::Start);
            hint.set_xalign(0.0);
            hint.set_wrap(true);
            row.append(&hint);

            // Google refuses requests whose IP it cannot geolocate — common on
            // VPNs and some ISPs — so the country is sent explicitly.
            let country_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let country_label = gtk::Label::new(Some("Country"));
            country_label.add_css_class("kalam-muted");
            country_row.append(&country_label);

            let country = gtk::Entry::new();
            country.set_max_length(2);
            country.set_width_chars(4);
            country.set_placeholder_text(Some("IN"));
            country.set_text(
                &catalog
                    .get_pref("meta.googlebooks.country")
                    .unwrap_or_else(crate::metadata::google_books::detect_country),
            );
            country_row.append(&country);

            let save_country = gtk::Button::with_label("Save");
            save_country.add_css_class("kalam-mini-btn");
            {
                let catalog = catalog.clone();
                let country = country.clone();
                save_country.connect_clicked(move |_| {
                    catalog.set_pref(
                        "meta.googlebooks.country",
                        &country.text().trim().to_uppercase(),
                    );
                });
            }
            country_row.append(&save_country);

            let country_hint = country_hint_label();
            country_row.append(&country_hint);
            row.append(&country_row);
        }

        host.append(&row);
    }
}
