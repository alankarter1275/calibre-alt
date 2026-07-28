use crate::db::Catalog;
use crate::dict;
use crate::paths::{catalog_db, data_dir, dictionaries_dir, library_dir};
use gtk::prelude::*;
use relm4::prelude::*;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Appearance,
    Storage,
    Dictionaries,
    BookFiles,
    Metadata,
    Notifications,
}

impl SettingsTab {
    pub const ALL: &'static [Self] = &[
        Self::Appearance,
        Self::Storage,
        Self::Dictionaries,
        Self::BookFiles,
        Self::Metadata,
        Self::Notifications,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Appearance => "Appearance",
            Self::Storage => "Storage & Backup",
            Self::Dictionaries => "Dictionaries",
            Self::BookFiles => "Book Files",
            Self::Metadata => "Metadata Sources",
            Self::Notifications => "Notifications",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Appearance => "◎",
            Self::Storage => "☷",
            Self::Dictionaries => "✎",
            Self::BookFiles => "☰",
            Self::Metadata => "★",
            Self::Notifications => "●",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Appearance => "◎  Appearance",
            Self::Storage => "☷  Storage & Backup",
            Self::Dictionaries => "✎  Dictionaries",
            Self::BookFiles => "☰  Book Files",
            Self::Metadata => "★  Metadata Sources",
            Self::Notifications => "●  Notifications",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            Self::Appearance => "Every theme is dark. Changes apply immediately.",
            Self::Storage => {
                "Manage where Kalam stores your catalog database, library EPUB files, backups, and reader cache."
            }
            Self::Dictionaries => {
                "Data locations and offline dictionary packs for lookup in the reader."
            }
            Self::BookFiles => {
                "Manage EPUB file metadata writeback and original backup files."
            }
            Self::Metadata => {
                "Used by Edit metadata. Results from every enabled source are merged and badged with their origin."
            }
            Self::Notifications => {
                "History log of recent activity, alerts, and toasts."
            }
        }
    }
}

#[derive(Debug)]
pub enum SettingsMsg {
    SelectTab(SettingsTab),
    ClearNotifications,
    ImportDict,
    DeleteDict(i64),
    Refresh,
}

pub struct SettingsPageModel {
    catalog: Arc<Catalog>,
    dicts: Vec<crate::db::Dictionary>,
    status: String,
    active_tab: SettingsTab,
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
            set_orientation: gtk::Orientation::Horizontal,
            set_hexpand: true,
            set_vexpand: true,

            // Left Settings Navigation Rail (220px)
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                add_css_class: "kalam-settings-nav",
                set_spacing: 6,
                set_hexpand: false,
                set_vexpand: true,

                gtk::Label {
                    set_label: "SETTINGS",
                    add_css_class: "kalam-section-label",
                    set_halign: gtk::Align::Start,
                    set_margin_bottom: 8,
                },

                #[name = "nav_list"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 4,
                },

                gtk::Box {
                    set_vexpand: true,
                },

                gtk::Label {
                    set_label: "Kalam v0.1.0 · Linux",
                    add_css_class: "kalam-muted",
                    set_halign: gtk::Align::Start,
                    set_margin_top: 12,
                },
            },

            // Right Category Content Area
            #[name = "scroller"]
            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,
                set_hscrollbar_policy: gtk::PolicyType::Never,
                set_vscrollbar_policy: gtk::PolicyType::Automatic,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    add_css_class: "kalam-settings-content",
                    set_spacing: 16,
                    set_hexpand: true,

                    #[name = "tab_title"]
                    gtk::Label {
                        add_css_class: "kalam-page-title",
                        set_halign: gtk::Align::Start,
                    },

                    #[name = "tab_subtitle"]
                    gtk::Label {
                        add_css_class: "kalam-page-sub",
                        set_halign: gtk::Align::Start,
                        set_wrap: true,
                    },

                    // 1. Appearance Tab
                    #[name = "appearance_box"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 14,
                        #[watch]
                        set_visible: model.active_tab == SettingsTab::Appearance,

                        #[name = "theme_row"]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 8,
                        },
                    },

                    // 2. Storage & Backup Tab
                    #[name = "storage_box"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 16,
                        #[watch]
                        set_visible: model.active_tab == SettingsTab::Storage,

                        // Card 1: DATA LOCATIONS
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            add_css_class: "kalam-card",
                            set_spacing: 12,

                            gtk::Label {
                                set_label: "DATA LOCATIONS",
                                add_css_class: "kalam-detail-section-title",
                                set_halign: gtk::Align::Start,
                            },

                            #[name = "paths_host"]
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 8,
                            },
                        },

                        // Card 2: LIBRARY BACKUP & CACHE
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            add_css_class: "kalam-card",
                            set_spacing: 12,

                            gtk::Label {
                                set_label: "LIBRARY BACKUP & CACHE",
                                add_css_class: "kalam-detail-section-title",
                                set_halign: gtk::Align::Start,
                            },

                            #[name = "backup_row"]
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 10,
                            },
                        },
                    },

                    // 3. Dictionaries Tab
                    #[name = "dicts_box"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 16,
                        #[watch]
                        set_visible: model.active_tab == SettingsTab::Dictionaries,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            add_css_class: "kalam-card",
                            set_spacing: 12,

                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 10,

                                gtk::Label {
                                    set_label: "OFFLINE DICTIONARIES",
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
                        },
                    },

                    // 4. Book Files Tab
                    #[name = "book_files_box"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 16,
                        #[watch]
                        set_visible: model.active_tab == SettingsTab::BookFiles,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            add_css_class: "kalam-card",
                            set_spacing: 12,

                            gtk::Label {
                                set_label: "EPUB METADATA & BACKUPS",
                                add_css_class: "kalam-detail-section-title",
                                set_halign: gtk::Align::Start,
                            },

                            #[name = "file_write_row"]
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 10,
                            },
                        },
                    },

                    // 5. Metadata Sources Tab
                    #[name = "metadata_box"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 16,
                        #[watch]
                        set_visible: model.active_tab == SettingsTab::Metadata,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            add_css_class: "kalam-card",
                            set_spacing: 12,

                            gtk::Label {
                                set_label: "METADATA PROVIDERS",
                                add_css_class: "kalam-detail-section-title",
                                set_halign: gtk::Align::Start,
                            },

                            #[name = "source_list"]
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 12,
                            },
                        },
                    },

                    // 6. Notifications Tab
                    #[name = "notifications_box"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 16,
                        #[watch]
                        set_visible: model.active_tab == SettingsTab::Notifications,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            add_css_class: "kalam-card",
                            set_spacing: 12,

                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 10,

                                gtk::Label {
                                    set_label: "ACTIVITY & NOTIFICATIONS",
                                    add_css_class: "kalam-detail-section-title",
                                    set_halign: gtk::Align::Start,
                                    set_hexpand: true,
                                },
                                gtk::Button {
                                    set_label: "Clear history",
                                    add_css_class: "kalam-mini-btn",
                                    set_valign: gtk::Align::Center,
                                    connect_clicked => SettingsMsg::ClearNotifications,
                                },
                            },

                            #[name = "notify_list"]
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 6,
                            },
                        },
                    },
                },
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
        let active_tab = SettingsTab::Appearance;
        let model = SettingsPageModel {
            catalog,
            dicts,
            status,
            active_tab,
        };
        let widgets = view_output!();

        widgets.tab_title.set_label(active_tab.title());
        widgets.tab_subtitle.set_label(active_tab.subtitle());

        for tab in SettingsTab::ALL {
            let btn = make_tab_button(*tab, *tab == active_tab);
            let tab_copy = *tab;
            let s = sender.clone();
            btn.connect_clicked(move |_| s.input(SettingsMsg::SelectTab(tab_copy)));
            btn.set_widget_name(&format!("settings-tab-{:?}", tab));
            widgets.nav_list.append(&btn);
        }

        widgets.dict_status.set_label(&model.status);
        rebuild_dicts(&widgets.dict_list, &model.dicts, &sender);
        build_sources(&widgets.source_list, &model.catalog);
        build_file_write(&widgets.file_write_row, &model.catalog);
        build_paths_list(&widgets.paths_host);
        build_backup(&widgets.backup_row, &model.catalog);
        build_notifications(&widgets.notify_list);
        build_theme_picker(&widgets.theme_row, &model.catalog);
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
            SettingsMsg::SelectTab(tab) => {
                self.active_tab = tab;
                widgets.tab_title.set_label(tab.title());
                widgets.tab_subtitle.set_label(tab.subtitle());
                update_tab_styles(&widgets.nav_list, tab);
                widgets.scroller.vadjustment().set_value(0.0);
            }
            SettingsMsg::ClearNotifications => {
                crate::notify::clear_history();
                build_notifications(&widgets.notify_list);
            }
            SettingsMsg::Refresh => {
                self.refresh();
                widgets.dict_status.set_label(&self.status);
                rebuild_dicts(&widgets.dict_list, &self.dicts, &sender);
                build_notifications(&widgets.notify_list);
            }
            SettingsMsg::ImportDict => {
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
                                crate::notify::activity(
                                    "Importing dictionary…",
                                    &path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().into_owned())
                                        .unwrap_or_default(),
                                );
                                match dict::import_dictionary(&catalog_clone, &path) {
                                    Ok((name, count)) => {
                                        crate::notify::success(
                                            "Dictionary imported",
                                            &format!("{name} · {count} entries"),
                                        );
                                    }
                                    Err(e) => {
                                        crate::notify::error(
                                            "Dictionary import failed",
                                            &format!("{e:#}"),
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
                let name = self
                    .dicts
                    .iter()
                    .find(|d| d.id == id)
                    .map(|d| d.name.clone())
                    .unwrap_or_default();
                crate::notify::outcome_info(
                    self.catalog.delete_dictionary(id),
                    "Dictionary removed",
                    &name,
                    "Could not remove the dictionary",
                );
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

fn make_tab_button(tab: SettingsTab, active: bool) -> gtk::Button {
    let box_content = gtk::Box::new(gtk::Orientation::Horizontal, 10);

    let icon = gtk::Label::new(Some(tab.icon()));
    icon.set_width_chars(2);
    icon.set_halign(gtk::Align::Center);
    box_content.append(&icon);

    let label = gtk::Label::new(Some(tab.label()));
    label.set_halign(gtk::Align::Start);
    box_content.append(&label);

    let btn = gtk::Button::new();
    btn.set_child(Some(&box_content));
    btn.add_css_class("kalam-settings-tab");
    if active {
        btn.add_css_class("active");
    }
    btn.set_focus_on_click(false);
    btn
}

fn update_tab_styles(container: &gtk::Box, active: SettingsTab) {
    let mut child = container.first_child();
    while let Some(widget) = child {
        if let Ok(btn) = widget.clone().downcast::<gtk::Button>() {
            let name = btn.widget_name();
            if name == format!("settings-tab-{:?}", active) {
                btn.add_css_class("active");
            } else {
                btn.remove_css_class("active");
            }
        }
        child = widget.next_sibling();
    }
}

fn build_paths_list(host: &gtk::Box) {
    while let Some(child) = host.first_child() {
        host.remove(&child);
    }
    let items = [
        (
            "Data Directory",
            "Base folder for Kalam application data",
            data_dir().to_string_lossy().into_owned(),
        ),
        (
            "Catalog Database",
            "SQLite database storing library books, shelves, tags, and reading progress",
            catalog_db().to_string_lossy().into_owned(),
        ),
        (
            "Library Files",
            "Directory where EPUB books and extracted covers are stored",
            library_dir().to_string_lossy().into_owned(),
        ),
        (
            "Dictionaries Directory",
            "Location for offline StarDict, SQLite, and TSV dictionary packs",
            dictionaries_dir().to_string_lossy().into_owned(),
        ),
    ];
    for (i, (title, sub, path)) in items.iter().enumerate() {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.add_css_class("kalam-settings-row");
        if i > 0 {
            let div = gtk::Separator::new(gtk::Orientation::Horizontal);
            div.set_margin_top(4);
            div.set_margin_bottom(4);
            host.append(&div);
        }

        let left = gtk::Box::new(gtk::Orientation::Vertical, 4);
        left.set_hexpand(true);
        let title_l = gtk::Label::new(Some(title));
        title_l.add_css_class("kalam-card-title");
        title_l.set_halign(gtk::Align::Start);
        left.append(&title_l);

        let sub_l = gtk::Label::new(Some(sub));
        sub_l.add_css_class("kalam-card-meta");
        sub_l.set_halign(gtk::Align::Start);
        sub_l.set_wrap(true);
        left.append(&sub_l);
        row.append(&left);

        let right_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        right_box.add_css_class("kalam-settings-path-box");
        right_box.set_valign(gtk::Align::Center);
        let path_l = gtk::Label::new(Some(path));
        path_l.set_selectable(true);
        path_l.add_css_class("kalam-muted");
        right_box.append(&path_l);
        row.append(&right_box);

        host.append(&row);
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

/// Theme swatches, grouped by family.
fn build_theme_picker(host: &gtk::Box, catalog: &Arc<Catalog>) {
    while let Some(child) = host.first_child() {
        host.remove(&child);
    }

    let active = crate::theme::current(catalog).id;

    let mut families: Vec<Vec<&crate::theme::Theme>> = Vec::new();
    for theme in crate::theme::ALL {
        if theme.id.ends_with("-darker") {
            if let Some(last) = families.last_mut() {
                last.push(theme);
                continue;
            }
        }
        families.push(vec![theme]);
    }

    for family in &families {
        host.append(&theme_family_row(family, &active, host, catalog));
    }
}

/// One family: a caption plus its variants.
fn theme_family_row(
    family: &[&crate::theme::Theme],
    active: &str,
    host: &gtk::Box,
    catalog: &Arc<Catalog>,
) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Vertical, 5);
    row.add_css_class("kalam-theme-family");

    let title = family[0]
        .label
        .split(" (")
        .next()
        .unwrap_or(family[0].label);
    let caption = gtk::Label::new(Some(title));
    caption.add_css_class("kalam-theme-family-name");
    caption.set_halign(gtk::Align::Start);
    row.append(&caption);

    let variants = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    variants.set_homogeneous(true);
    for theme in family {
        variants.append(&theme_swatch_button(theme, active, host, catalog));
    }
    row.append(&variants);
    row
}

/// A single clickable swatch.
fn theme_swatch_button(
    theme: &crate::theme::Theme,
    active: &str,
    host: &gtk::Box,
    catalog: &Arc<Catalog>,
) -> gtk::Button {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 5);
    card.add_css_class("kalam-theme-card");
    if theme.id == active {
        card.add_css_class("active");
    }

    let strip = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    strip.add_css_class("kalam-theme-strip");
    strip.set_height_request(26);
    strip.set_overflow(gtk::Overflow::Hidden);
    for (slot, (colour, weight)) in [
        (theme.sidebar, 1),
        (theme.surface, 2),
        (theme.surface_2, 1),
        (theme.accent, 1),
    ]
    .into_iter()
    .enumerate()
    {
        let cell = gtk::Box::new(gtk::Orientation::Vertical, 0);
        cell.set_hexpand(true);
        cell.set_size_request(weight * 12, -1);
        let class = format!("kalam-swatch-{}-{slot}", theme.id);
        let provider = gtk::CssProvider::new();
        provider.load_from_string(&format!(".{class} {{ background: {colour}; }}"));
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
        cell.add_css_class(&class);
        strip.append(&cell);
    }
    card.append(&strip);

    let variant = if theme.id.ends_with("-darker") {
        "Darker"
    } else {
        "Normal"
    };
    let name = gtk::Label::new(Some(variant));
    name.add_css_class("kalam-theme-name");
    name.set_halign(gtk::Align::Start);
    name.set_ellipsize(gtk::pango::EllipsizeMode::End);
    card.append(&name);

    let btn = gtk::Button::new();
    btn.set_child(Some(&card));
    btn.add_css_class("kalam-theme-btn");
    btn.set_tooltip_text(Some(theme.label));

    let catalog = catalog.clone();
    let host = host.clone();
    let chosen = *theme;
    btn.connect_clicked(move |_| {
        crate::theme::save_and_apply(&catalog, &chosen);
        crate::notify::success("Theme changed", chosen.label);
        let host = host.clone();
        let catalog = catalog.clone();
        gtk::glib::idle_add_local_once(move || {
            build_theme_picker(&host, &catalog);
        });
    });

    btn
}

/// Recent notifications, so a toast that faded can still be read.
fn build_notifications(host: &gtk::Box) {
    while let Some(child) = host.first_child() {
        host.remove(&child);
    }

    let entries = crate::notify::history();
    if entries.is_empty() {
        let empty = gtk::Label::new(Some("Nothing yet this session."));
        empty.add_css_class("kalam-muted");
        empty.set_halign(gtk::Align::Start);
        host.append(&empty);
        return;
    }

    for entry in entries.iter().take(25) {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        row.add_css_class("kalam-list-row");

        let badge = gtk::Label::new(Some(entry.kind.label()));
        badge.add_css_class("kalam-card-badge");
        badge.add_css_class(match entry.kind {
            crate::notify::Kind::Error => "kalam-badge-ol",
            _ => "kalam-badge-manual",
        });
        badge.set_valign(gtk::Align::Center);
        row.append(&badge);

        let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
        text.set_hexpand(true);

        let title = gtk::Label::new(Some(&entry.title));
        title.add_css_class("kalam-card-title");
        title.set_halign(gtk::Align::Start);
        title.set_xalign(0.0);
        title.set_ellipsize(gtk::pango::EllipsizeMode::End);
        text.append(&title);

        if !entry.detail.trim().is_empty() {
            let detail = gtk::Label::new(Some(entry.detail.trim()));
            detail.add_css_class("kalam-card-meta");
            detail.set_halign(gtk::Align::Start);
            detail.set_xalign(0.0);
            detail.set_ellipsize(gtk::pango::EllipsizeMode::End);
            detail.set_tooltip_text(Some(&entry.detail));
            text.append(&detail);
        }
        row.append(&text);

        let at = gtk::Label::new(Some(&entry.at));
        at.add_css_class("kalam-muted");
        at.set_valign(gtk::Align::Center);
        row.append(&at);

        host.append(&row);
    }
}

/// Back up the catalog, and clear the reader cache.
fn build_backup(host: &gtk::Box, catalog: &Arc<Catalog>) {
    while let Some(child) = host.first_child() {
        host.remove(&child);
    }

    // Row 1: Library Backup
    let row1 = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row1.add_css_class("kalam-settings-row");

    let left1 = gtk::Box::new(gtk::Orientation::Vertical, 4);
    left1.set_hexpand(true);
    let title1 = gtk::Label::new(Some("Back up library database"));
    title1.add_css_class("kalam-card-title");
    title1.set_halign(gtk::Align::Start);
    left1.append(&title1);

    let sub1 = gtk::Label::new(Some(
        "Creates a snapshot of catalog.db containing all metadata, annotations, shelves, ratings, and reading history.",
    ));
    sub1.add_css_class("kalam-card-meta");
    sub1.set_halign(gtk::Align::Start);
    sub1.set_wrap(true);
    left1.append(&sub1);
    row1.append(&left1);

    let backup_btn = gtk::Button::with_label("Back up library…");
    backup_btn.add_css_class("kalam-secondary-btn");
    backup_btn.set_valign(gtk::Align::Center);
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
    row1.append(&backup_btn);
    host.append(&row1);

    // Divider
    let div = gtk::Separator::new(gtk::Orientation::Horizontal);
    div.set_margin_top(4);
    div.set_margin_bottom(4);
    host.append(&div);

    // Row 2: Reader Cache
    let row2 = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row2.add_css_class("kalam-settings-row");

    let left2 = gtk::Box::new(gtk::Orientation::Vertical, 4);
    left2.set_hexpand(true);
    let title2 = gtk::Label::new(Some("Extracted EPUB cache"));
    title2.add_css_class("kalam-card-title");
    title2.set_halign(gtk::Align::Start);
    left2.append(&title2);

    let sub2 = gtk::Label::new(Some(
        "Temporary files extracted for the WebKitGTK reader. Safe to clear; books re-extract on open.",
    ));
    sub2.add_css_class("kalam-card-meta");
    sub2.set_halign(gtk::Align::Start);
    sub2.set_wrap(true);
    left2.append(&sub2);
    row2.append(&left2);

    let right2 = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    right2.set_valign(gtk::Align::Center);

    let size = crate::paths::reader_cache_size();
    let cache_badge = gtk::Label::new(Some(&crate::epub_write::human_size(size)));
    cache_badge.add_css_class("kalam-card-badge");
    cache_badge.add_css_class("kalam-badge-manual");
    right2.append(&cache_badge);

    let clear = gtk::Button::with_label("Clear cache");
    clear.add_css_class("kalam-mini-btn");
    clear.set_sensitive(size > 0);
    {
        let cache_badge = cache_badge.clone();
        clear.connect_clicked(move |btn| {
            let (_, freed) = crate::paths::clear_reader_cache();
            cache_badge.set_label(&format!("Freed {}", crate::epub_write::human_size(freed)));
            btn.set_sensitive(false);
        });
    }
    right2.append(&clear);
    row2.append(&right2);

    host.append(&row2);
}

/// `2026-07-28`, for backup filenames.
fn today_stamp() -> String {
    gtk::glib::DateTime::now_local()
        .and_then(|d| d.format("%Y-%m-%d"))
        .map(|s| s.to_string())
        .unwrap_or_default()
}

/// Toggle for writing metadata back into the EPUB itself, and deleting backups.
fn build_file_write(host: &gtk::Box, catalog: &Arc<Catalog>) {
    use crate::epub_write::{set_write_enabled, write_enabled};

    while let Some(child) = host.first_child() {
        host.remove(&child);
    }

    let row1 = gtk::Box::new(gtk::Orientation::Vertical, 6);
    row1.add_css_class("kalam-settings-row");

    let check = gtk::CheckButton::with_label("Also write metadata into the EPUB file");
    check.set_active(write_enabled(catalog));
    {
        let catalog = catalog.clone();
        check.connect_toggled(move |c| set_write_enabled(&catalog, c.is_active()));
    }
    row1.append(&check);

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
    row1.append(&note);
    host.append(&row1);

    // Divider
    let div = gtk::Separator::new(gtk::Orientation::Horizontal);
    div.set_margin_top(4);
    div.set_margin_bottom(4);
    host.append(&div);

    // Row 2: backup cleanup
    let backups = crate::epub_write::list_backups();
    let total: u64 = backups.iter().map(|(_, size)| size).sum();

    let row2 = gtk::Box::new(gtk::Orientation::Vertical, 6);
    row2.add_css_class("kalam-settings-row");

    let cleanup_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
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
    summary.add_css_class("kalam-card-title");
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
    row2.append(&cleanup_row);

    let warn = gtk::Label::new(Some(
        "Deleting backups is permanent: you lose the ability to undo metadata \
         written into those files.",
    ));
    warn.add_css_class("kalam-card-meta");
    warn.set_halign(gtk::Align::Start);
    warn.set_xalign(0.0);
    warn.set_wrap(true);
    row2.append(&warn);

    host.append(&row2);
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

    for (i, id) in SourceId::ALL.iter().enumerate() {
        if i > 0 {
            let div = gtk::Separator::new(gtk::Orientation::Horizontal);
            div.set_margin_top(6);
            div.set_margin_bottom(6);
            host.append(&div);
        }

        let row = gtk::Box::new(gtk::Orientation::Vertical, 6);
        row.add_css_class("kalam-settings-row");

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
                    let key = entry.text().trim().to_string();
                    catalog.set_pref("meta.googlebooks.key", &key);
                    if key.is_empty() {
                        crate::notify::info("Google Books key cleared", "Using the shared quota");
                    } else {
                        crate::notify::success(
                            "Google Books key saved",
                            "Your own quota is in use",
                        );
                    }
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
                    let code = country.text().trim().to_uppercase();
                    catalog.set_pref("meta.googlebooks.country", &code);
                    crate::notify::success("Country saved", &code);
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
