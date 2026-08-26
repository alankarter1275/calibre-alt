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

/// The settings nav rail, grouped exactly like the reference design.
const NAV_GROUPS: &[(&str, &[SettingsTab])] = &[
    ("Appearance", &[SettingsTab::Appearance]),
    (
        "Library",
        &[
            SettingsTab::Storage,
            SettingsTab::Dictionaries,
            SettingsTab::BookFiles,
        ],
    ),
    ("Sources", &[SettingsTab::Metadata]),
    ("App", &[SettingsTab::Notifications]),
];

/// Theme families for the picker: (base id, display name, description,
/// label prefix used to derive each variant's short name).
const THEME_FAMILIES: &[(&str, &str, &str, &str)] = &[
    (
        "onedark",
        "One Dark",
        "Warm-tinted greys, soft blue accent. The default.",
        "One Dark",
    ),
    (
        "tokyonight",
        "Tokyo Night",
        "Deep indigo, high-chroma violet and blue accents.",
        "Tokyo Night",
    ),
    (
        "everforest",
        "Everforest",
        "Low-saturation greens and warm greys. Easy on the eyes.",
        "Everforest",
    ),
    (
        "catppuccin",
        "Catppuccin Mocha",
        "Soft pastels on a near-black lavender base.",
        "Catppuccin",
    ),
    (
        "gruvbox",
        "Gruvbox",
        "Warm retro browns and ochres — a classic.",
        "Gruvbox",
    ),
    (
        "ayu",
        "Ayu",
        "Muted slate with a distinctive amber accent.",
        "Ayu",
    ),
    (
        "nord",
        "Nord",
        "Cool arctic blue-greys. No darker variant — already deep.",
        "Nord",
    ),
];

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

            // Left settings navigation rail, grouped.
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                add_css_class: "kalam-settings-nav",
                set_spacing: 2,
                set_hexpand: false,
                set_vexpand: true,

                #[name = "nav_list"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 2,
                },

                gtk::Box {
                    set_vexpand: true,
                },
            },

            // Right category content area.
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

                    // 1. Appearance
                    #[name = "appearance_box"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 16,
                        #[watch]
                        set_visible: model.active_tab == SettingsTab::Appearance,

                        #[name = "theme_grid"]
                        gtk::Grid {
                            set_column_homogeneous: true,
                            set_row_spacing: 12,
                            set_column_spacing: 12,
                            set_hexpand: true,
                        },
                    },

                    // 2. Storage & Backup
                    #[name = "storage_box"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 16,
                        #[watch]
                        set_visible: model.active_tab == SettingsTab::Storage,

                        #[name = "paths_host"]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 16,
                        },

                        #[name = "backup_row"]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 16,
                        },

                        #[name = "export_row"]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 16,
                        },
                    },

                    // 3. Dictionaries
                    #[name = "dicts_box"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 16,
                        #[watch]
                        set_visible: model.active_tab == SettingsTab::Dictionaries,

                        #[name = "dict_list"]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 16,
                        },
                    },

                    // 4. Book Files
                    #[name = "book_files_box"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 16,
                        #[watch]
                        set_visible: model.active_tab == SettingsTab::BookFiles,

                        #[name = "file_write_row"]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 16,
                        },
                    },

                    // 5. Metadata Sources
                    #[name = "metadata_box"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 16,
                        #[watch]
                        set_visible: model.active_tab == SettingsTab::Metadata,

                        #[name = "source_list"]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 16,
                        },
                    },

                    // 6. Notifications
                    #[name = "notifications_box"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 16,
                        #[watch]
                        set_visible: model.active_tab == SettingsTab::Notifications,

                        #[name = "notify_list"]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 16,
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
        let active_tab = SettingsTab::Appearance;
        let model = SettingsPageModel {
            catalog,
            dicts,
            active_tab,
        };
        let widgets = view_output!();

        widgets.tab_title.set_label(active_tab.label());
        widgets.tab_subtitle.set_label(active_tab.subtitle());

        for (group, tabs) in NAV_GROUPS.iter() {
            let group_label = gtk::Label::new(Some(*group));
            group_label.add_css_class("kalam-settings-group");
            group_label.set_halign(gtk::Align::Start);
            widgets.nav_list.append(&group_label);
            for tab in tabs.iter() {
                let btn = make_tab_button(*tab, *tab == active_tab);
                let tab_copy = *tab;
                let s = sender.clone();
                btn.connect_clicked(move |_| s.input(SettingsMsg::SelectTab(tab_copy)));
                btn.set_widget_name(&format!("settings-tab-{:?}", tab_copy));
                widgets.nav_list.append(&btn);
            }
        }

        rebuild_dicts(&widgets.dict_list, &model.dicts, &sender);
        build_sources(&widgets.source_list, &model.catalog);
        build_file_write(&widgets.file_write_row, &model.catalog);
        build_paths(&widgets.paths_host);
        build_backup(&widgets.backup_row, &model.catalog);
        build_export(&widgets.export_row, &model.catalog);
        build_notifications(&widgets.notify_list, &sender);
        build_theme_picker(&widgets.theme_grid, &model.catalog);
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
                widgets.tab_title.set_label(tab.label());
                widgets.tab_subtitle.set_label(tab.subtitle());
                update_tab_styles(&widgets.nav_list, tab);
                widgets.scroller.vadjustment().set_value(0.0);
            }
            SettingsMsg::ClearNotifications => {
                crate::notify::clear_history();
                build_notifications(&widgets.notify_list, &sender);
            }
            SettingsMsg::Refresh => {
                self.refresh();
                rebuild_dicts(&widgets.dict_list, &self.dicts, &sender);
                build_notifications(&widgets.notify_list, &sender);
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
                rebuild_dicts(&widgets.dict_list, &self.dicts, &sender);
            }
        }
        self.update_view(widgets, sender);
    }
}

impl SettingsPageModel {
    fn refresh(&mut self) {
        self.dicts = self.catalog.list_dictionaries().unwrap_or_default();
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

/// One card with an accent glyph, a title, an optional description, a hairline
/// divider, and a body host the caller fills with rows. Appended to `host`.
fn section_card(host: &gtk::Box, glyph: &str, title: &str, desc: Option<&str>) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 0);
    card.add_css_class("kalam-section-card");

    let head = gtk::Box::new(gtk::Orientation::Vertical, 3);
    head.add_css_class("kalam-section-head");
    let title_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let icon = gtk::Label::new(Some(glyph));
    icon.add_css_class("kalam-section-icon");
    title_row.append(&icon);
    let title_label = gtk::Label::new(Some(title));
    title_label.add_css_class("kalam-section-title");
    title_label.set_halign(gtk::Align::Start);
    title_row.append(&title_label);
    head.append(&title_row);
    if let Some(text) = desc {
        let desc_label = gtk::Label::new(Some(text));
        desc_label.add_css_class("kalam-section-desc");
        desc_label.set_wrap(true);
        desc_label.set_xalign(0.0);
        desc_label.set_halign(gtk::Align::Start);
        head.append(&desc_label);
    }
    card.append(&head);

    let divider = gtk::Separator::new(gtk::Orientation::Horizontal);
    divider.add_css_class("kalam-section-divider");
    card.append(&divider);

    let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
    body.add_css_class("kalam-section-body");
    card.append(&body);

    host.append(&card);
    body
}

/// Label + description on the left, one control on the right. Hairlines
/// between rows come from CSS, so rows simply stack.
fn setting_row(body: &gtk::Box, label: &str, desc: &str, right: &impl IsA<gtk::Widget>) {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    row.add_css_class("kalam-setting-row");

    let left = gtk::Box::new(gtk::Orientation::Vertical, 3);
    left.set_hexpand(true);
    let label_widget = gtk::Label::new(Some(label));
    label_widget.add_css_class("kalam-setting-label");
    label_widget.set_halign(gtk::Align::Start);
    label_widget.set_xalign(0.0);
    left.append(&label_widget);
    let desc_widget = gtk::Label::new(Some(desc));
    desc_widget.add_css_class("kalam-setting-desc");
    desc_widget.set_wrap(true);
    desc_widget.set_xalign(0.0);
    desc_widget.set_halign(gtk::Align::Start);
    left.append(&desc_widget);
    row.append(&left);

    let control = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    control.set_valign(gtk::Align::Center);
    control.append(right);
    row.append(&control);

    body.append(&row);
}

/// The app's pill switch. `on_toggle` fires only for user changes: the
/// initial state is set before the handler is attached.
fn toggle_switch(initial: bool, on_toggle: impl Fn(bool) + 'static) -> gtk::Switch {
    let sw = gtk::Switch::new();
    sw.set_active(initial);
    sw.set_valign(gtk::Align::Center);
    sw.add_css_class("kalam-switch");
    sw.connect_active_notify(move |sw| on_toggle(sw.is_active()));
    sw
}

/// A pill chip label: `kalam-chip` plus one colour class.
fn chip_label(text: &str, class: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("kalam-chip");
    label.add_css_class(class);
    label.set_valign(gtk::Align::Center);
    label
}

/// Mix `fg` over `bg` at fraction `t`; both are #rrggbb.
fn blend_hex(fg: &str, bg: &str, t: f32) -> String {
    let parse = |s: &str| u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0);
    let (f, b) = (parse(fg), parse(bg));
    let mix = |shift: u32| {
        let fc = ((f >> shift) & 0xff) as f32;
        let bc = ((b >> shift) & 0xff) as f32;
        (fc * t + bc * (1.0 - t)).round() as u32
    };
    format!("#{:02x}{:02x}{:02x}", mix(16), mix(8), mix(0))
}

/// Attach a widget-local CSS class carrying literal per-theme colours.
/// Providers load at APPLICATION priority, like the global sheet, and later
/// providers win at equal priority — the same trick as the old swatch strip.
fn add_styled_class(widget: &impl IsA<gtk::Widget>, class: &str, decls: &str) {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(&format!(".{class} {{ {decls} }}"));
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    widget.add_css_class(class);
}

/// Theme families in a 2-column grid; each family is a raised block holding
/// one card per variant.
fn build_theme_picker(host: &gtk::Grid, catalog: &Arc<Catalog>) {
    while let Some(child) = host.first_child() {
        host.remove(&child);
    }
    let active = crate::theme::current(catalog).id;
    for (index, (base, name, desc, prefix)) in THEME_FAMILIES.iter().enumerate() {
        let family: Vec<&crate::theme::Theme> = crate::theme::ALL
            .iter()
            .filter(|t| t.id == *base || t.id.starts_with(&format!("{base}-")))
            .collect();
        if family.is_empty() {
            continue;
        }
        let block = theme_family_block(&family, name, desc, prefix, active, host, catalog);
        host.attach(&block, (index % 2) as i32, (index / 2) as i32, 1, 1);
    }
}

fn theme_family_block(
    family: &[&crate::theme::Theme],
    name: &str,
    desc: &str,
    prefix: &str,
    active: &str,
    host: &gtk::Grid,
    catalog: &Arc<Catalog>,
) -> gtk::Box {
    let block = gtk::Box::new(gtk::Orientation::Vertical, 6);
    block.add_css_class("kalam-theme-family-card");

    let name_label = gtk::Label::new(Some(name));
    name_label.add_css_class("kalam-theme-family-name");
    name_label.set_halign(gtk::Align::Start);
    block.append(&name_label);

    let desc_label = gtk::Label::new(Some(desc));
    desc_label.add_css_class("kalam-theme-family-desc");
    desc_label.set_wrap(true);
    desc_label.set_xalign(0.0);
    desc_label.set_halign(gtk::Align::Start);
    block.append(&desc_label);

    let variants = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    variants.set_homogeneous(true);
    for theme in family {
        variants.append(&theme_variant_button(theme, prefix, active, host, catalog));
    }
    block.append(&variants);
    block
}

/// One clickable theme card: variant name, mini UI preview, swatch strip,
/// and a ✓ seal on the active theme.
fn theme_variant_button(
    theme: &crate::theme::Theme,
    prefix: &str,
    active: &str,
    host: &gtk::Grid,
    catalog: &Arc<Catalog>,
) -> gtk::Button {
    let is_active = theme.id == active;
    let raw = theme
        .label
        .strip_prefix(prefix)
        .unwrap_or(theme.label)
        .trim()
        .trim_matches(|c| c == '(' || c == ')' || c == ' ');
    let variant: &str = if raw.is_empty() { "Normal" } else { raw };

    let card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    card.add_css_class("kalam-theme-card");
    if is_active {
        card.add_css_class("active");
    }
    // Card background: the theme's own bg, tinted with the accent when active.
    let card_bg = if is_active {
        blend_hex(theme.accent, theme.bg, 0.06)
    } else {
        theme.bg.to_string()
    };
    add_styled_class(
        &card,
        &format!("kalam-tc-bg-{}", theme.id),
        &format!("background-color: {card_bg};"),
    );

    // Name row: variant name, "default" tag, spacer, ✓ seal.
    let name_row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    let name_label = gtk::Label::new(Some(variant));
    name_label.add_css_class("kalam-theme-name");
    name_label.set_halign(gtk::Align::Start);
    name_row.append(&name_label);
    if is_active {
        let tag = gtk::Label::new(Some("default"));
        tag.add_css_class("kalam-theme-variant");
        tag.set_valign(gtk::Align::Center);
        name_row.append(&tag);
    }
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    name_row.append(&spacer);
    if is_active {
        let check = gtk::Label::new(Some("✓"));
        check.add_css_class("kalam-theme-check");
        check.set_valign(gtk::Align::Center);
        add_styled_class(
            &check,
            &format!("kalam-tc-chk-{}", theme.id),
            &format!("color: {};", theme.bg),
        );
        name_row.append(&check);
    }
    card.append(&name_row);

    // Mini preview: sidebar rail with two dots + main bars and a card.
    let preview = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    preview.add_css_class("kalam-theme-preview");
    preview.set_overflow(gtk::Overflow::Hidden);
    preview.set_height_request(52);

    let rail = gtk::Box::new(gtk::Orientation::Vertical, 4);
    rail.set_size_request(18, -1);
    add_styled_class(
        &rail,
        &format!("kalam-tc-rail-{}", theme.id),
        &format!("background-color: {};", theme.sidebar),
    );
    let dot_accent = gtk::Box::new(gtk::Orientation::Vertical, 0);
    dot_accent.set_size_request(6, 6);
    dot_accent.set_margin_top(6);
    dot_accent.set_halign(gtk::Align::Center);
    add_styled_class(
        &dot_accent,
        &format!("kalam-tc-dot-a-{}", theme.id),
        &format!("background-color: {}; border-radius: 999px;", theme.accent),
    );
    rail.append(&dot_accent);
    let dot_border = gtk::Box::new(gtk::Orientation::Vertical, 0);
    dot_border.set_size_request(6, 6);
    dot_border.set_halign(gtk::Align::Center);
    add_styled_class(
        &dot_border,
        &format!("kalam-tc-dot-b-{}", theme.id),
        &format!("background-color: {}; border-radius: 999px;", theme.border),
    );
    rail.append(&dot_border);
    preview.append(&rail);

    let main = gtk::Box::new(gtk::Orientation::Vertical, 4);
    main.add_css_class("kalam-theme-preview-main");
    main.set_hexpand(true);
    let bar1 = gtk::Box::new(gtk::Orientation::Vertical, 0);
    bar1.set_size_request(55, 4);
    bar1.set_halign(gtk::Align::Start);
    add_styled_class(
        &bar1,
        &format!("kalam-tc-bar1-{}", theme.id),
        &format!("background-color: {}; border-radius: 999px;", theme.border),
    );
    main.append(&bar1);
    let mini = gtk::Box::new(gtk::Orientation::Vertical, 0);
    mini.set_hexpand(true);
    mini.set_size_request(-1, 18);
    add_styled_class(
        &mini,
        &format!("kalam-tc-card-{}", theme.id),
        &format!(
            "background-color: {}; border: 1px solid {}; border-radius: 4px;",
            theme.surface, theme.border
        ),
    );
    main.append(&mini);
    let bar2 = gtk::Box::new(gtk::Orientation::Vertical, 0);
    bar2.set_size_request(35, 4);
    bar2.set_halign(gtk::Align::Start);
    add_styled_class(
        &bar2,
        &format!("kalam-tc-bar2-{}", theme.id),
        &format!("background-color: {}; border-radius: 999px;", theme.border),
    );
    main.append(&bar2);
    preview.append(&main);
    card.append(&preview);

    // Swatch strip: surface2 / surface / border / accent / text.
    let swatches = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    for (slot, colour) in [
        theme.surface_2,
        theme.surface,
        theme.border,
        theme.accent,
        theme.text,
    ]
    .iter()
    .enumerate()
    {
        let cell = gtk::Box::new(gtk::Orientation::Vertical, 0);
        cell.set_hexpand(true);
        cell.set_size_request(-1, 5);
        add_styled_class(
            &cell,
            &format!("kalam-tc-s{slot}-{}", theme.id),
            &format!("background-color: {colour}; border-radius: 999px;"),
        );
        swatches.append(&cell);
    }
    card.append(&swatches);

    let btn = gtk::Button::new();
    btn.set_child(Some(&card));
    btn.add_css_class("kalam-theme-btn");
    btn.set_tooltip_text(Some(theme.label));

    let host = host.clone();
    let catalog = catalog.clone();
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

/// Data locations: four label/desc rows with mono path boxes.
fn build_paths(host: &gtk::Box) {
    while let Some(child) = host.first_child() {
        host.remove(&child);
    }
    let body = section_card(
        host,
        "☷",
        "Data locations",
        Some("These paths are set at first run. Moving data requires copying the files manually."),
    );
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
    for (title, sub, path) in items {
        setting_row(&body, title, sub, &path_box(&path));
    }
}

/// A mono path box; long paths ellipsize but stay selectable and have the
/// full path as a tooltip.
fn path_box(path: &str) -> gtk::Box {
    let wrap = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    wrap.add_css_class("kalam-settings-path-box");
    let label = gtk::Label::new(Some(path));
    label.set_selectable(true);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    label.set_max_width_chars(34);
    label.set_tooltip_text(Some(path));
    wrap.append(&label);
    wrap
}

/// Back up the catalog, and clear the reader cache.
fn build_backup(host: &gtk::Box, catalog: &Arc<Catalog>) {
    while let Some(child) = host.first_child() {
        host.remove(&child);
    }
    let body = section_card(host, "↓", "Backup & cache", None);

    let backup_btn = gtk::Button::with_label("Back up library…");
    backup_btn.add_css_class("kalam-btn-outlined");
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
    setting_row(
        &body,
        "Back up library database",
        "Creates a snapshot of catalog.db containing all metadata, annotations, shelves, ratings, and reading history.",
        &backup_btn,
    );

    let right = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    right.set_valign(gtk::Align::Center);
    let size = crate::paths::reader_cache_size();
    let cache_badge = chip_label(&crate::epub_write::human_size(size), "kalam-chip-neutral");
    right.append(&cache_badge);
    let clear = gtk::Button::with_label("Clear cache");
    clear.add_css_class("kalam-btn-danger");
    clear.set_sensitive(size > 0);
    {
        let cache_badge = cache_badge.clone();
        clear.connect_clicked(move |btn| {
            let (_, freed) = crate::paths::clear_reader_cache();
            cache_badge.set_label(&format!("Freed {}", crate::epub_write::human_size(freed)));
            btn.set_sensitive(false);
        });
    }
    right.append(&clear);
    setting_row(
        &body,
        "Extracted EPUB cache",
        "Temporary files extracted for the WebKitGTK reader. Safe to clear; books re-extract on open.",
        &right,
    );
}

/// Export saved quotes to Markdown, from Settings.
fn build_export(host: &gtk::Box, catalog: &Arc<Catalog>) {
    while let Some(child) = host.first_child() {
        host.remove(&child);
    }
    let body = section_card(host, "⧉", "Export", None);

    let export_btn = gtk::Button::with_label("Export quotes");
    export_btn.add_css_class("kalam-btn-outlined");
    export_btn.set_valign(gtk::Align::Center);
    {
        let catalog = catalog.clone();
        export_btn.connect_clicked(move |_| {
            match crate::pages::saved_quotes::export_all_quotes_markdown(&catalog) {
                Ok((count, path)) => crate::notify::success(
                    &format!(
                        "{count} quote{} exported",
                        if count == 1 { "" } else { "s" }
                    ),
                    &path.display().to_string(),
                ),
                Err(err) => crate::notify::error("Could not export quotes", &err),
            }
        });
    }
    setting_row(
        &body,
        "Export quotes to Markdown",
        "Saves all saved quotes to ~/Quotes.md",
        &export_btn,
    );
}

/// Installed dictionary packs: icon, name, mono meta, Remove — plus the
/// filled Import button in the card footer.
fn rebuild_dicts(
    host: &gtk::Box,
    dicts: &[crate::db::Dictionary],
    sender: &ComponentSender<SettingsPageModel>,
) {
    while let Some(child) = host.first_child() {
        host.remove(&child);
    }
    let body = section_card(
        host,
        "✎",
        "Installed packs",
        Some("StarDict (.ifo/.idx/.dict), SQLite (.db), and TSV formats are supported."),
    );

    if dicts.is_empty() {
        let empty = gtk::Label::new(Some(
            "No dictionaries yet. Import StarDict, SQLite or TSV packs for offline lookup (D key in the reader).",
        ));
        empty.add_css_class("kalam-placeholder");
        empty.set_wrap(true);
        empty.set_xalign(0.0);
        body.append(&empty);
    } else {
        for d in dicts {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            row.add_css_class("kalam-setting-row");

            let icon = gtk::Label::new(Some("✎"));
            icon.add_css_class("kalam-dict-icon");
            icon.set_size_request(32, 32);
            icon.set_halign(gtk::Align::Center);
            icon.set_valign(gtk::Align::Center);
            row.append(&icon);

            let info = gtk::Box::new(gtk::Orientation::Vertical, 2);
            info.set_hexpand(true);
            let name_label = gtk::Label::new(Some(&d.name));
            name_label.add_css_class("kalam-setting-label");
            name_label.set_halign(gtk::Align::Start);
            name_label.set_xalign(0.0);
            name_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            info.append(&name_label);
            let meta = match &d.lang {
                Some(lang) => format!("{lang} · {} entries", d.entry_count),
                None => format!("{} entries", d.entry_count),
            };
            let meta_label = gtk::Label::new(Some(&meta));
            meta_label.add_css_class("kalam-dict-meta");
            meta_label.set_halign(gtk::Align::Start);
            meta_label.set_xalign(0.0);
            info.append(&meta_label);
            row.append(&info);

            let remove = gtk::Button::with_label("Remove");
            remove.add_css_class("kalam-btn-danger");
            remove.add_css_class("kalam-btn-sm");
            remove.set_valign(gtk::Align::Center);
            let id = d.id;
            let s = sender.clone();
            remove.connect_clicked(move |_| s.input(SettingsMsg::DeleteDict(id)));
            row.append(&remove);

            body.append(&row);
        }
    }

    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    footer.add_css_class("kalam-card-footer");
    let import_btn = gtk::Button::with_label("+ Import dictionary");
    import_btn.add_css_class("kalam-btn-filled");
    {
        let s = sender.clone();
        import_btn.connect_clicked(move |_| s.input(SettingsMsg::ImportDict));
    }
    footer.append(&import_btn);
    let refresh_btn = gtk::Button::with_label("↻");
    refresh_btn.add_css_class("kalam-btn-ghost");
    refresh_btn.set_tooltip_text(Some("Rescan dictionary packs"));
    {
        let s = sender.clone();
        refresh_btn.connect_clicked(move |_| s.input(SettingsMsg::Refresh));
    }
    footer.append(&refresh_btn);
    body.append(&footer);
}

/// Toggle for writing metadata back into the EPUB itself, and deleting backups.
fn build_file_write(host: &gtk::Box, catalog: &Arc<Catalog>) {
    use crate::epub_write::{set_write_enabled, write_enabled};

    while let Some(child) = host.first_child() {
        host.remove(&child);
    }
    let body = section_card(host, "☰", "EPUB writeback", None);

    {
        let catalog = catalog.clone();
        let sw = toggle_switch(write_enabled(&catalog), move |on| {
            set_write_enabled(&catalog, on);
        });
        setting_row(
            &body,
            "Also write metadata into the EPUB file",
            "On: saving in Edit metadata also updates the book file, so Calibre and other readers see your changes. The untouched original is kept once as <name>.epub.orig, and the new file is only swapped in after it is verified. Off: edits stay inside Kalam and your files are never modified.",
            &sw,
        );
    }

    let right = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    right.set_valign(gtk::Align::Center);
    let backups = crate::epub_write::list_backups();
    let total: u64 = backups.iter().map(|(_, size)| size).sum();
    let summary_text = if backups.is_empty() {
        "No originals kept".to_string()
    } else {
        format!(
            "{} original{} · {}",
            backups.len(),
            if backups.len() == 1 { "" } else { "s" },
            crate::epub_write::human_size(total)
        )
    };
    let summary = chip_label(&summary_text, "kalam-chip-neutral");
    right.append(&summary);
    let clean = gtk::Button::with_label("Delete backups");
    clean.add_css_class("kalam-btn-danger");
    clean.set_sensitive(!backups.is_empty());
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
    right.append(&clean);
    setting_row(
        &body,
        "Original backups",
        "Kept once per book when writeback is on. Deleting is permanent: you lose the ability to undo metadata written into those files.",
        &right,
    );
}

/// `2026-07-28`, for backup filenames.
fn today_stamp() -> String {
    gtk::glib::DateTime::now_local()
        .and_then(|d| d.format("%Y-%m-%d"))
        .map(|s| s.to_string())
        .unwrap_or_default()
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

/// One section card per metadata provider, each with a real switch and its
/// own extra rows.
fn build_sources(host: &gtk::Box, catalog: &Arc<Catalog>) {
    use crate::metadata::{set_source_enabled, source_enabled, SourceId};

    while let Some(child) = host.first_child() {
        host.remove(&child);
    }

    let ol_body = section_card(
        host,
        "★",
        "Open Library",
        Some("Internet Archive. No key needed. Strong on older and public-domain titles."),
    );
    {
        let catalog = catalog.clone();
        let sw = toggle_switch(source_enabled(&catalog, SourceId::OpenLibrary), move |on| {
            set_source_enabled(&catalog, SourceId::OpenLibrary, on);
        });
        setting_row(
            &ol_body,
            "Enable Open Library lookup",
            "Adds a search panel inside the metadata editor.",
            &sw,
        );
    }

    let gb_body = section_card(
        host,
        "★",
        "Google Books",
        Some(
            "Broad coverage, good for recent and non-English books. Works without a key, but anonymous requests share a global quota and can be rate limited.",
        ),
    );
    {
        let catalog = catalog.clone();
        let sw = toggle_switch(source_enabled(&catalog, SourceId::GoogleBooks), move |on| {
            set_source_enabled(&catalog, SourceId::GoogleBooks, on);
        });
        setting_row(
            &gb_body,
            "Enable Google Books lookup",
            "Adds results from Google Books to the metadata editor.",
            &sw,
        );
    }

    // API key row.
    {
        let right = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        right.set_valign(gtk::Align::Center);
        let entry = gtk::Entry::new();
        entry.set_placeholder_text(Some("Optional API key — lifts the shared rate limit"));
        entry.set_text(&catalog.get_pref("meta.googlebooks.key").unwrap_or_default());
        entry.set_hexpand(true);
        entry.add_css_class("kalam-setting-entry");
        right.append(&entry);
        let save = gtk::Button::with_label("Save key");
        save.add_css_class("kalam-btn-outlined");
        save.add_css_class("kalam-btn-sm");
        {
            let catalog = catalog.clone();
            let entry = entry.clone();
            save.connect_clicked(move |_| {
                let key = entry.text().trim().to_string();
                catalog.set_pref("meta.googlebooks.key", &key);
                if key.is_empty() {
                    crate::notify::info("Google Books key cleared", "Using the shared quota");
                } else {
                    crate::notify::success("Google Books key saved", "Your own quota is in use");
                }
            });
        }
        right.append(&save);
        setting_row(
            &gb_body,
            "API key",
            "Free from console.cloud.google.com — create a project, enable the Books API, then make an API key. No card required.",
            &right,
        );
    }

    // Country row.
    {
        let right = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        right.set_valign(gtk::Align::Center);
        let country = gtk::Entry::new();
        country.set_max_length(2);
        country.set_width_chars(4);
        country.set_placeholder_text(Some("IN"));
        country.set_text(
            &catalog
                .get_pref("meta.googlebooks.country")
                .unwrap_or_else(crate::metadata::google_books::detect_country),
        );
        country.add_css_class("kalam-setting-entry");
        right.append(&country);
        let save = gtk::Button::with_label("Save");
        save.add_css_class("kalam-btn-outlined");
        save.add_css_class("kalam-btn-sm");
        {
            let catalog = catalog.clone();
            let country = country.clone();
            save.connect_clicked(move |_| {
                let code = country.text().trim().to_uppercase();
                catalog.set_pref("meta.googlebooks.country", &code);
                crate::notify::success("Country saved", &code);
            });
        }
        right.append(&save);
        let hint = country_hint_label();
        right.append(&hint);
        setting_row(
            &gb_body,
            "Country",
            "Two-letter code. Google only serves results for countries it has rights in.",
            &right,
        );
    }
}

/// Recent notifications, so a toast that faded can still be read.
fn build_notifications(host: &gtk::Box, sender: &ComponentSender<SettingsPageModel>) {
    while let Some(child) = host.first_child() {
        host.remove(&child);
    }
    let body = section_card(
        host,
        "●",
        "Activity log",
        Some("The last 25 events this session. Toasts fade; this keeps the record."),
    );

    let entries = crate::notify::history();
    if entries.is_empty() {
        let empty = gtk::Label::new(Some("Nothing yet this session."));
        empty.add_css_class("kalam-muted");
        empty.set_halign(gtk::Align::Start);
        body.append(&empty);
    } else {
        for entry in entries.iter().take(25) {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            row.add_css_class("kalam-setting-row");

            let badge_class = match entry.kind {
                crate::notify::Kind::Success => "kalam-chip-success",
                crate::notify::Kind::Error => "kalam-chip-danger",
                crate::notify::Kind::Info => "kalam-chip-info",
                crate::notify::Kind::Progress => "kalam-chip-neutral",
            };
            row.append(&chip_label(entry.kind.label(), badge_class));

            let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
            text.set_hexpand(true);
            let title = gtk::Label::new(Some(&entry.title));
            title.add_css_class("kalam-setting-label");
            title.set_halign(gtk::Align::Start);
            title.set_xalign(0.0);
            title.set_ellipsize(gtk::pango::EllipsizeMode::End);
            text.append(&title);
            if !entry.detail.trim().is_empty() {
                let detail = gtk::Label::new(Some(entry.detail.trim()));
                detail.add_css_class("kalam-setting-desc");
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

            body.append(&row);
        }
    }

    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    footer.set_halign(gtk::Align::End);
    footer.add_css_class("kalam-card-footer");
    let clear = gtk::Button::with_label("Clear history");
    clear.add_css_class("kalam-btn-ghost");
    clear.add_css_class("kalam-btn-sm");
    {
        let s = sender.clone();
        clear.connect_clicked(move |_| s.input(SettingsMsg::ClearNotifications));
    }
    footer.append(&clear);
    body.append(&footer);
}
