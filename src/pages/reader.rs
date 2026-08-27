//! Immersive EPUB reader.

use crate::db::{Annotation, Catalog, DictEntry, HighlightColor, ReadingBookmark, SavedWord};
use crate::epub_book::{reading_css, OpenBook, ReadingTheme};
use crate::models::Book;
use crate::paths::reader_cache_dir;
use crate::widgets::book_row::cover_widget;
use gtk::glib;
use gtk::prelude::*;
use relm4::prelude::*;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use webkit6::prelude::*;

#[derive(Debug)]
pub enum ReaderOut {
    Close,
}

#[derive(Debug, Clone, Deserialize)]
struct JsPayload {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    fraction: Option<f64>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    color: Option<String>,
    #[serde(rename = "startPath", default)]
    start_path: Option<String>,
    #[serde(rename = "startOffset", default)]
    start_offset: Option<i64>,
    #[serde(rename = "endPath", default)]
    end_path: Option<String>,
    #[serde(rename = "endOffset", default)]
    end_offset: Option<i64>,
    #[serde(rename = "tmpId", default)]
    tmp_id: Option<String>,
    #[serde(default)]
    word: Option<String>,
    #[serde(default)]
    context: Option<String>,
    #[serde(default)]
    rect: Option<serde_json::Value>,
    #[serde(default)]
    definition: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeftSidebarTab {
    Toc,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RightSidebarTab {
    Highlights,
    Bookmarks,
    Words,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HighlightFilter {
    All,
    Yellow,
    Green,
    Blue,
    Pink,
    Orange,
    Quotes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WordScope {
    Chapter,
    Book,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReaderSettingsPane {
    Reading,
    Ui,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReaderUiSetting {
    SidebarGap,
    LeftSidebarWidth,
    RightSidebarWidth,
    SidebarRadius,
    BottomPillSize,
    BackChipSize,
    DimStrength,
}

#[derive(Debug, Clone, Copy)]
struct ReaderUiPrefs {
    sidebar_gap: i32,
    left_sidebar_width: i32,
    right_sidebar_width: i32,
    sidebar_radius: i32,
    bottom_pill_size: i32,
    back_chip_size: i32,
    dim_strength: i32,
}

struct ReaderUiSettingControls {
    value_label: gtk::Label,
    preset_buttons: Vec<(i32, gtk::Button)>,
    custom_button: gtk::Button,
}

struct ReaderSettingsControls {
    root: gtk::Box,
    settings_stack: gtk::Stack,
    pane_buttons: Vec<(ReaderSettingsPane, gtk::Button)>,
    font_size_label: gtk::Label,
    line_height_label: gtk::Label,
    column_width_label: gtk::Label,
    theme_dots: Vec<(ReadingTheme, gtk::Button)>,
    ui_controls: Vec<(ReaderUiSetting, ReaderUiSettingControls)>,
}

const UI_PRESETS_SIDEBAR_GAP: [(&str, i32); 3] = [("Tight", 8), ("Normal", 14), ("Airy", 20)];
const UI_PRESETS_LEFT_WIDTH: [(&str, i32); 3] = [("Narrow", 220), ("Normal", 248), ("Wide", 280)];
const UI_PRESETS_RIGHT_WIDTH: [(&str, i32); 3] = [("Narrow", 196), ("Normal", 212), ("Wide", 236)];
const UI_PRESETS_RADIUS: [(&str, i32); 3] = [("Soft", 14), ("Round", 20), ("Full", 26)];
const UI_PRESETS_PILL_SIZE: [(&str, i32); 3] = [("Compact", 30), ("Normal", 34), ("Large", 40)];
const UI_PRESETS_BACK_SIZE: [(&str, i32); 3] = [("Compact", 28), ("Normal", 32), ("Large", 38)];
const UI_PRESETS_DIM: [(&str, i32); 3] = [("Light", 12), ("Medium", 22), ("Strong", 32)];

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ReaderMsg {
    Close,
    TocSelect(usize),
    PrevChapter,
    NextChapter,
    Theme(ReadingTheme),
    FontDelta(i32),
    LineHeightDelta(i32),
    ColumnWidthDelta(i32),
    SwitchSettingsPane(ReaderSettingsPane),
    SetUiSetting(ReaderUiSetting, i32),
    AdjustUiSetting(ReaderUiSetting, i32),
    JsRaw(String),
    Progress(f64),
    AnnotationsReload,
    DeleteAnnotation(i64),
    DeleteBookmark(i64),
    JumpToChapter(usize),
    JumpToLocation(usize, f64),
    DictSearch(String),
    DictSearchSelect(String),
    SaveCurrentWord,
    ClearDict,
    AddBookmark,
    OpenLeftSidebar,
    OpenRightSidebar,
    SwitchLeftTab(LeftSidebarTab),
    SwitchRightTab(RightSidebarTab),
    SetHighlightFilter(HighlightFilter),
    SetWordScope(WordScope),
    ScheduleCloseLeft,
    ScheduleCloseRight,
    ForceCloseLeft,
    ForceCloseRight,
    CloseSidebars,
    HideChrome,
    ShowBackChrome,
    ShowBottomChrome,
    ShowAllChrome,
}

pub struct ReaderModel {
    catalog: Arc<Catalog>,
    book_id: i64,
    book_title: String,
    book_authors: String,
    book_cover_path: Option<PathBuf>,
    open: OpenBook,
    chapter: usize,
    fraction: f64,
    theme: ReadingTheme,
    font_px: u32,
    line_height: f32,
    column_px: u32,
    loading: bool,
    webview: webkit6::WebView,
    chapter_annotations: Vec<Annotation>,
    all_book_annotations: Vec<Annotation>,
    bookmarks: Vec<ReadingBookmark>,
    saved_words: Vec<SavedWord>,
    dict_query: String,
    dict_results: Vec<DictEntry>,
    dict_lookup_word: Option<String>,
    dict_lookup_def: Option<String>,
    dict_lookup_rect_json: Option<String>,
    dict_context: Option<String>,
    last_selection: Option<String>,
    session_id: Option<i64>,
    session_start: std::time::Instant,
    session_start_pct: i64,
    left_tab: LeftSidebarTab,
    right_tab: RightSidebarTab,
    left_sidebar_open: bool,
    right_sidebar_open: bool,
    highlight_filter: HighlightFilter,
    word_scope: WordScope,
    settings_pane: ReaderSettingsPane,
    ui_prefs: ReaderUiPrefs,
    show_back_button: bool,
    show_bottom_pill: bool,
    left_close_timer: Option<glib::SourceId>,
    right_close_timer: Option<glib::SourceId>,
    left_sidebar_shell: Option<gtk::Revealer>,
    right_sidebar_shell: Option<gtk::Revealer>,
    left_sidebar_box: Option<gtk::Box>,
    right_sidebar_box: Option<gtk::Box>,
    left_stack: gtk::Stack,
    right_stack: gtk::Stack,
    toc_scroll: gtk::ScrolledWindow,
    toc_list: gtk::Box,
    highlights_list: gtk::Box,
    bookmarks_list: gtk::Box,
    words_list: gtk::Box,
    settings_stack: gtk::Stack,
    settings_pane_buttons: Vec<(ReaderSettingsPane, gtk::Button)>,
    font_size_label: gtk::Label,
    line_height_label: gtk::Label,
    column_width_label: gtk::Label,
    theme_dots: Vec<(ReadingTheme, gtk::Button)>,
    ui_controls: Vec<(ReaderUiSetting, ReaderUiSettingControls)>,
    highlight_filter_buttons: Vec<(HighlightFilter, gtk::Button)>,
    word_scope_buttons: Vec<(WordScope, gtk::Button)>,
    ui_css_provider: gtk::CssProvider,
}

#[relm4::component(pub)]
impl Component for ReaderModel {
    type Init = (Arc<Catalog>, i64);
    type Input = ReaderMsg;
    type Output = ReaderOut;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Overlay {
            add_css_class: "kalam-reader",
            add_css_class: "kalam-reader-ui-live",
            set_hexpand: true,
            set_vexpand: true,

            #[wrap(Some)]
            set_child = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_hexpand: true,
                set_vexpand: true,

                #[name = "reader_stage"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,
                    set_vexpand: true,
                    add_css_class: "kalam-reader-stage",

                    #[name = "web_host"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true,
                        set_vexpand: true,
                    },
                },
            },

            add_overlay = &gtk::Button {
                add_css_class: "kalam-reader-dim",
                #[watch]
                set_visible: model.left_sidebar_open || model.right_sidebar_open,
                set_hexpand: true,
                set_vexpand: true,
                set_halign: gtk::Align::Fill,
                set_valign: gtk::Align::Fill,
                connect_clicked => ReaderMsg::CloseSidebars,
            },

            add_overlay = &gtk::Box {
                add_css_class: "kalam-reader-hover-edge",
                set_width_request: 28,
                set_hexpand: false,
                set_vexpand: true,
                set_halign: gtk::Align::Start,
                set_valign: gtk::Align::Fill,
            },

            add_overlay = &gtk::Box {
                add_css_class: "kalam-reader-hover-edge",
                set_width_request: 28,
                set_hexpand: false,
                set_vexpand: true,
                set_halign: gtk::Align::End,
                set_valign: gtk::Align::Fill,
            },

            add_overlay = &gtk::Box {
                #[watch]
                set_visible: model.show_back_button,
                set_halign: gtk::Align::Start,
                set_valign: gtk::Align::Start,
                set_margin_top: 14,
                set_margin_start: 16,

                gtk::Button {
                    set_child: Some(&crate::icons::labelled("go-previous-symbolic", 16, "Library", 6)),
                    add_css_class: "kalam-reader-back",
                    connect_clicked => ReaderMsg::Close,
                },
            },

            add_overlay = &gtk::Box {
                #[watch]
                set_visible: model.show_bottom_pill,
                add_css_class: "kalam-reader-bottom-dock",
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::End,
                set_margin_bottom: 18,

                gtk::Box {
                    add_css_class: "kalam-reader-bottom-pill",
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 4,
                    set_valign: gtk::Align::Center,

                    gtk::Button {
                        set_child: Some(&crate::icons::symbolic_with_classes("go-previous-symbolic", 15, &["kalam-inline-icon"])),
                        add_css_class: "kalam-reader-pill-nav",
                        set_tooltip_text: Some("Previous chapter (P)"),
                        connect_clicked => ReaderMsg::PrevChapter,
                    },

                    gtk::Box {
                        add_css_class: "kalam-reader-pill-info",
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 10,
                        set_valign: gtk::Align::Center,

                        #[name = "progress_label"]
                        gtk::Label {
                            add_css_class: "kalam-reader-pill-pages",
                            set_valign: gtk::Align::Center,
                        },

                        #[name = "pill_chapter_label"]
                        gtk::Label {
                            add_css_class: "kalam-reader-pill-chapter",
                            set_max_width_chars: 36,
                            set_ellipsize: gtk::pango::EllipsizeMode::End,
                            set_valign: gtk::Align::Center,
                        },
                    },

                    gtk::Button {
                        set_child: Some(&crate::icons::symbolic_with_classes("go-next-symbolic", 15, &["kalam-inline-icon"])),
                        add_css_class: "kalam-reader-pill-nav",
                        set_tooltip_text: Some("Next chapter (N)"),
                        connect_clicked => ReaderMsg::NextChapter,
                    },
                },
            },

            add_overlay = &gtk::Revealer {
                add_css_class: "kalam-reader-sidebar-shell",
                add_css_class: "kalam-reader-sidebar-shell-left",
                #[watch]
                set_reveal_child: model.left_sidebar_open,
                set_transition_type: gtk::RevealerTransitionType::SlideRight,
                set_halign: gtk::Align::Start,
                set_valign: gtk::Align::Fill,

                #[wrap(Some)]
                set_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    add_css_class: "kalam-reader-sidebar",
                    add_css_class: "kalam-reader-sidebar-left",

                    gtk::Box {
                        add_css_class: "kalam-reader-book-head",
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 10,

                        #[name = "left_cover_host"]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            add_css_class: "kalam-reader-cover-slot",
                            set_width_request: 48,
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 4,
                            set_hexpand: true,

                            #[name = "sidebar_book_title"]
                            gtk::Label {
                                add_css_class: "kalam-reader-book-title",
                                add_css_class: "kalam-title-serif",
                                set_halign: gtk::Align::Start,
                                set_wrap: true,
                                set_xalign: 0.0,
                            },

                            #[name = "sidebar_book_author"]
                            gtk::Label {
                                add_css_class: "kalam-reader-book-author",
                                set_halign: gtk::Align::Start,
                                set_xalign: 0.0,
                            },

                            #[name = "sidebar_progress"]
                            gtk::ProgressBar {
                                add_css_class: "kalam-reader-progress",
                                set_show_text: false,
                                set_fraction: 0.0,
                            },
                        },
                    },

                    #[name = "left_panel_host"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true,
                        set_vexpand: true,
                    },

                    gtk::Box {
                        add_css_class: "kalam-reader-tabbar",
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 2,
                        set_homogeneous: true,

                        #[name = "left_toc_tab"]
                        gtk::Button {
                            set_child: Some(&reader_sidebar_tab_content("view-list-bullet-symbolic", "TOC")),
                            add_css_class: "kalam-reader-tab",
                            set_hexpand: true,
                            connect_clicked => ReaderMsg::SwitchLeftTab(LeftSidebarTab::Toc),
                        },

                        #[name = "left_settings_tab"]
                        gtk::Button {
                            set_child: Some(&reader_sidebar_tab_content("preferences-system-symbolic", "Settings")),
                            add_css_class: "kalam-reader-tab",
                            set_hexpand: true,
                            connect_clicked => ReaderMsg::SwitchLeftTab(LeftSidebarTab::Settings),
                        },
                    },
                },
            },

            add_overlay = &gtk::Revealer {
                add_css_class: "kalam-reader-sidebar-shell",
                add_css_class: "kalam-reader-sidebar-shell-right",
                #[watch]
                set_reveal_child: model.right_sidebar_open,
                set_transition_type: gtk::RevealerTransitionType::SlideLeft,
                set_halign: gtk::Align::End,
                set_valign: gtk::Align::Fill,

                #[wrap(Some)]
                set_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    add_css_class: "kalam-reader-sidebar",
                    add_css_class: "kalam-reader-sidebar-right",

                    #[name = "right_panel_host"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true,
                        set_vexpand: true,
                    },

                    gtk::Box {
                        add_css_class: "kalam-reader-tabbar",
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 2,
                        set_homogeneous: true,

                        #[name = "right_highlights_tab"]
                        gtk::Button {
                            set_child: Some(&reader_sidebar_tab_content("highlight-symbolic", "Highlights")),
                            add_css_class: "kalam-reader-tab",
                            set_hexpand: true,
                            connect_clicked => ReaderMsg::SwitchRightTab(RightSidebarTab::Highlights),
                        },

                        #[name = "right_bookmarks_tab"]
                        gtk::Button {
                            set_child: Some(&reader_sidebar_tab_content("bookmark-new-symbolic", "Marks")),
                            add_css_class: "kalam-reader-tab",
                            set_hexpand: true,
                            connect_clicked => ReaderMsg::SwitchRightTab(RightSidebarTab::Bookmarks),
                        },

                        #[name = "right_words_tab"]
                        gtk::Button {
                            set_child: Some(&reader_sidebar_tab_content("accessories-dictionary-symbolic", "Words")),
                            add_css_class: "kalam-reader-tab",
                            set_hexpand: true,
                            connect_clicked => ReaderMsg::SwitchRightTab(RightSidebarTab::Words),
                        },
                    },
                },
            },
        }
    }

    fn init(
        (catalog, book_id): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let book = catalog.get_book(book_id).ok().flatten();
        let webview = webkit6::WebView::new();
        webview.set_hexpand(true);
        webview.set_vexpand(true);

        let (book_meta, open, chapter, fraction) = if let Some(book) = book.clone() {
            let cache = reader_cache_dir(&book.uuid);
            match OpenBook::open(&book.file_path, &cache) {
                Ok(open) => {
                    let (ch, frac) = catalog
                        .get_reading_progress(book_id)
                        .ok()
                        .flatten()
                        .unwrap_or((0, 0.0));
                    let ch = ch.min(open.chapter_count().saturating_sub(1));
                    (book, open, ch, frac)
                }
                Err(err) => {
                    eprintln!("kalam: open epub failed: {err:#}");
                    let msg = format!(
                        "<html><body style='padding:2rem;background:#f5f0e8;color:#2c2820;font-family:Georgia,serif'><h1>Could not open book</h1><pre>{err:#}</pre><p>Press Esc to go back.</p></body></html>"
                    );
                    webview.load_html(&msg, None);
                    (book, OpenBook::empty_placeholder(), 0, 0.0)
                }
            }
        } else {
            (
                Book {
                    id: book_id,
                    uuid: String::new(),
                    title: "Missing book".into(),
                    authors: String::new(),
                    series: None,
                    description: String::new(),
                    format: crate::models::BookFormat::Epub,
                    file_name: String::new(),
                    file_hash: String::new(),
                    cover_name: None,
                    added_at: String::new(),
                    progress: 0,
                    rating: 0,
                    publisher: String::new(),
                    published: String::new(),
                    series_index: 0.0,
                    tags: Vec::new(),
                    cover_path: None,
                    file_path: PathBuf::new(),
                },
                OpenBook::empty_placeholder(),
                0,
                0.0,
            )
        };

        let chapter_annotations = if open.chapter_count() > 0 {
            catalog
                .get_annotations_for_chapter(book_id, chapter as i64)
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let all_book_annotations = catalog
            .get_annotations_for_book(book_id)
            .unwrap_or_default();
        let bookmarks = catalog.list_reading_bookmarks(book_id).unwrap_or_default();
        let saved_words = catalog.list_saved_words("").unwrap_or_default();

        let catalog_theme = catalog
            .get_pref("reader.theme")
            .map(|v| ReadingTheme::from_str_lossy(&v))
            .unwrap_or(ReadingTheme::Sepia);
        let catalog_font = catalog.get_pref_i64("reader.font_px", 17).clamp(13, 24) as u32;
        let catalog_line_height = catalog
            .get_pref("reader.line_height")
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(1.8)
            .clamp(1.3, 2.5);
        let catalog_column = catalog
            .get_pref_i64("reader.column_px", 620)
            .clamp(400, 860) as u32;
        let ui_prefs = ReaderUiPrefs::load(&catalog);
        let ui_css_provider = gtk::CssProvider::new();
        register_reader_ui_provider(&ui_css_provider);

        let toc_list = gtk::Box::new(gtk::Orientation::Vertical, 0);
        toc_list.set_hexpand(true);
        toc_list.set_vexpand(true);
        let left_stack = gtk::Stack::new();
        left_stack.set_hexpand(true);
        left_stack.set_vexpand(true);
        let toc_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .hexpand(true)
            .vexpand(true)
            .child(&toc_list)
            .build();
        toc_scroll.add_css_class("kalam-reader-panel-scroll");
        left_stack.add_named(&toc_scroll, Some("toc"));

        let ReaderSettingsControls {
            root: settings_panel,
            settings_stack,
            pane_buttons: settings_pane_buttons,
            font_size_label,
            line_height_label,
            column_width_label,
            theme_dots,
            ui_controls,
        } = build_reader_settings_panel(
            &sender,
            catalog_theme,
            catalog_font,
            catalog_line_height,
            catalog_column,
            ui_prefs,
        );
        let settings_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .hexpand(true)
            .vexpand(true)
            .child(&settings_panel)
            .build();
        settings_scroll.add_css_class("kalam-reader-panel-scroll");
        left_stack.add_named(&settings_scroll, Some("settings"));

        let highlights_list = gtk::Box::new(gtk::Orientation::Vertical, 0);
        highlights_list.set_hexpand(true);
        highlights_list.set_vexpand(true);
        let (highlights_panel, highlight_filter_buttons) =
            build_highlights_panel(&sender, HighlightFilter::All, &highlights_list);
        let bookmarks_list = gtk::Box::new(gtk::Orientation::Vertical, 0);
        bookmarks_list.set_hexpand(true);
        bookmarks_list.set_vexpand(true);
        let bookmarks_panel = build_bookmarks_panel(&sender, &bookmarks_list);
        let words_list = gtk::Box::new(gtk::Orientation::Vertical, 0);
        words_list.set_hexpand(true);
        words_list.set_vexpand(true);
        let (words_panel, word_scope_buttons, _words_search_entry) =
            build_words_panel(&sender, WordScope::Chapter, &words_list);

        let right_stack = gtk::Stack::new();
        right_stack.set_hexpand(true);
        right_stack.set_vexpand(true);
        right_stack.add_named(&highlights_panel, Some("highlights"));
        right_stack.add_named(&bookmarks_panel, Some("bookmarks"));
        right_stack.add_named(&words_panel, Some("words"));

        let model = ReaderModel {
            catalog,
            book_id,
            book_title: book_meta.title.clone(),
            book_authors: book_meta.authors_display().to_string(),
            book_cover_path: book_meta.cover_path.clone(),
            open,
            chapter,
            fraction,
            theme: catalog_theme,
            font_px: catalog_font,
            line_height: catalog_line_height,
            column_px: catalog_column,
            loading: false,
            webview: webview.clone(),
            chapter_annotations,
            all_book_annotations,
            bookmarks,
            saved_words,
            dict_query: String::new(),
            dict_results: Vec::new(),
            dict_lookup_word: None,
            dict_lookup_def: None,
            dict_lookup_rect_json: None,
            dict_context: None,
            last_selection: None,
            session_id: None,
            session_start: std::time::Instant::now(),
            session_start_pct: 0,
            left_tab: LeftSidebarTab::Toc,
            right_tab: RightSidebarTab::Highlights,
            left_sidebar_open: false,
            right_sidebar_open: false,
            highlight_filter: HighlightFilter::All,
            word_scope: WordScope::Chapter,
            settings_pane: ReaderSettingsPane::Reading,
            ui_prefs,
            show_back_button: true,
            show_bottom_pill: true,
            left_close_timer: None,
            right_close_timer: None,
            left_sidebar_shell: None,
            right_sidebar_shell: None,
            left_sidebar_box: None,
            right_sidebar_box: None,
            left_stack,
            right_stack,
            toc_scroll,
            toc_list,
            highlights_list,
            bookmarks_list,
            words_list,
            settings_stack,
            settings_pane_buttons,
            font_size_label,
            line_height_label,
            column_width_label,
            theme_dots,
            ui_controls,
            highlight_filter_buttons,
            word_scope_buttons,
            ui_css_provider,
        };

        let mut model = model;
        if model.open.chapter_count() > 0 {
            let _ = model.catalog.mark_book_opened(book_id);
            let start_pct = model
                .catalog
                .get_book(book_id)
                .ok()
                .flatten()
                .map(|b| b.progress as i64)
                .unwrap_or(0);
            model.session_start_pct = start_pct;
            model.session_start = std::time::Instant::now();
            model.session_id = model.catalog.start_reading_session(book_id, start_pct).ok();
        }

        let widgets = view_output!();
        widgets.web_host.append(&webview);
        widgets.left_panel_host.append(&model.left_stack);
        widgets.right_panel_host.append(&model.right_stack);
        let left_sidebar_box = widgets
            .left_panel_host
            .parent()
            .and_then(|w| w.downcast::<gtk::Box>().ok());
        let right_sidebar_box = widgets
            .right_panel_host
            .parent()
            .and_then(|w| w.downcast::<gtk::Box>().ok());
        model.left_sidebar_shell = left_sidebar_box
            .as_ref()
            .and_then(|sidebar| sidebar.parent())
            .and_then(|w| w.downcast::<gtk::Revealer>().ok());
        model.right_sidebar_shell = right_sidebar_box
            .as_ref()
            .and_then(|sidebar| sidebar.parent())
            .and_then(|w| w.downcast::<gtk::Revealer>().ok());
        model.left_sidebar_box = left_sidebar_box.clone();
        model.right_sidebar_box = right_sidebar_box.clone();
        rebuild_cover_host(&widgets.left_cover_host, model.book_cover_path.as_deref());
        sync_reader_stage_theme(&widgets.reader_stage, model.theme);
        update_chrome_labels(&widgets, &model);
        update_sidebar_header(&widgets, &model);
        sync_sidebar_tabs(&widgets, &model);
        apply_reader_ui_prefs(&model);
        sync_reader_controls(&model);
        rebuild_toc(&model.toc_list, &model.open, model.chapter, &sender);
        rebuild_highlights_list(&model, &sender);
        rebuild_bookmarks_list(&model, &sender);
        rebuild_words_list(&model, &sender);

        if let Some(left_hover) = overlay_child_box(&root, 2) {
            connect_hover_zone(
                &left_hover,
                &sender,
                ReaderMsg::OpenLeftSidebar,
                ReaderMsg::ScheduleCloseLeft,
            );
        }
        if let Some(right_hover) = overlay_child_box(&root, 3) {
            connect_hover_zone(
                &right_hover,
                &sender,
                ReaderMsg::OpenRightSidebar,
                ReaderMsg::ScheduleCloseRight,
            );
        }
        if let Some(left_sidebar) = left_sidebar_box {
            connect_hover_zone(
                &left_sidebar,
                &sender,
                ReaderMsg::OpenLeftSidebar,
                ReaderMsg::ScheduleCloseLeft,
            );
        }
        if let Some(right_sidebar) = right_sidebar_box {
            connect_hover_zone(
                &right_sidebar,
                &sender,
                ReaderMsg::OpenRightSidebar,
                ReaderMsg::ScheduleCloseRight,
            );
        }

        let s = sender.clone();
        webview.connect_title_notify(move |wv| {
            if let Some(title) = wv.title() {
                let t = title.to_string();
                if t.contains("kalam://")
                    || (t.starts_with('{') && t.contains("\"type\""))
                    || t.starts_with("kalam-selection::")
                    || t.starts_with("kalam-progress::")
                {
                    s.input(ReaderMsg::JsRaw(t));
                }
            }
        });

        let s = sender.clone();
        webview.connect_load_changed(move |_wv, event| {
            if event == webkit6::LoadEvent::Finished {
                s.input(ReaderMsg::AnnotationsReload);
            }
        });

        if let Some(ucm) = webview.user_content_manager() {
            let _ = ucm.register_script_message_handler("kalam", None);
            let s = sender.clone();
            ucm.connect_script_message_received(Some("kalam"), move |_mgr, msg| {
                s.input(ReaderMsg::JsRaw(msg.to_string()));
            });
        }

        let s = sender.clone();
        webview.connect_decide_policy(move |_wv, decision, decision_type| {
            if decision_type == webkit6::PolicyDecisionType::NavigationAction {
                if let Some(nav_decision) =
                    decision.downcast_ref::<webkit6::NavigationPolicyDecision>()
                {
                    if let Some(mut nav_action) = nav_decision.navigation_action() {
                        if let Some(request) = nav_action.request() {
                            if let Some(uri) = request.uri() {
                                let uri_str = uri.to_string();
                                if uri_str.starts_with("kalam://") {
                                    let payload = uri_str.trim_start_matches("kalam://");
                                    let decoded = url_decode(payload);
                                    s.input(ReaderMsg::JsRaw(decoded));
                                    decision.ignore();
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
            false
        });

        if model.open.chapter_count() > 0 {
            load_chapter(&model);
        }

        let key = gtk::EventControllerKey::new();
        let s = sender.clone();
        key.connect_key_pressed(move |_, keyval, _, _| {
            use gtk::gdk::Key;
            match keyval {
                Key::Escape => {
                    s.input(ReaderMsg::Close);
                    gtk::glib::Propagation::Stop
                }
                Key::n | Key::N | Key::Right => {
                    s.input(ReaderMsg::NextChapter);
                    gtk::glib::Propagation::Stop
                }
                Key::p | Key::P | Key::Left => {
                    s.input(ReaderMsg::PrevChapter);
                    gtk::glib::Propagation::Stop
                }
                Key::plus | Key::equal => {
                    s.input(ReaderMsg::FontDelta(1));
                    gtk::glib::Propagation::Stop
                }
                Key::minus => {
                    s.input(ReaderMsg::FontDelta(-1));
                    gtk::glib::Propagation::Stop
                }
                Key::t | Key::T => {
                    s.input(ReaderMsg::SwitchLeftTab(LeftSidebarTab::Toc));
                    gtk::glib::Propagation::Stop
                }
                Key::s | Key::S => {
                    s.input(ReaderMsg::SwitchLeftTab(LeftSidebarTab::Settings));
                    gtk::glib::Propagation::Stop
                }
                Key::h | Key::H => {
                    s.input(ReaderMsg::SwitchRightTab(RightSidebarTab::Highlights));
                    gtk::glib::Propagation::Stop
                }
                Key::b | Key::B => {
                    s.input(ReaderMsg::SwitchRightTab(RightSidebarTab::Bookmarks));
                    gtk::glib::Propagation::Stop
                }
                Key::m | Key::M => {
                    s.input(ReaderMsg::AddBookmark);
                    gtk::glib::Propagation::Stop
                }
                Key::w | Key::W => {
                    s.input(ReaderMsg::SwitchRightTab(RightSidebarTab::Words));
                    gtk::glib::Propagation::Stop
                }
                _ => gtk::glib::Propagation::Proceed,
            }
        });
        root.add_controller(key);
        root.set_can_focus(true);

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        let mut refresh_sidebar_header = false;
        let mut refresh_toc = false;
        let mut refresh_highlights = false;
        let mut refresh_bookmarks = false;
        let mut refresh_words = false;
        let mut refresh_controls = false;
        let mut refresh_chrome = false;
        let mut refresh_stage = false;
        let mut refresh_tabs = false;

        match msg {
            ReaderMsg::Close => {
                if self.left_sidebar_open || self.right_sidebar_open {
                    self.close_sidebars();
                    refresh_tabs = true;
                } else {
                    if self.fraction < 0.05 {
                        self.fraction = 0.15;
                    }
                    self.save_progress();
                    sender.output(ReaderOut::Close).ok();
                }
            }
            ReaderMsg::TocSelect(idx) | ReaderMsg::JumpToChapter(idx) => {
                if idx < self.open.chapter_count() && idx != self.chapter && !self.loading {
                    self.go_chapter(idx, 0.0);
                    refresh_sidebar_header = true;
                    refresh_toc = true;
                    refresh_highlights = true;
                    refresh_bookmarks = true;
                    refresh_words = true;
                    refresh_chrome = true;
                }
            }
            ReaderMsg::JumpToLocation(idx, frac) => {
                if idx < self.open.chapter_count() && !self.loading {
                    self.go_chapter(idx, frac);
                    refresh_sidebar_header = true;
                    refresh_toc = true;
                    refresh_highlights = true;
                    refresh_bookmarks = true;
                    refresh_words = true;
                    refresh_chrome = true;
                }
            }
            ReaderMsg::PrevChapter => {
                if self.chapter > 0 && !self.loading {
                    self.go_chapter(self.chapter - 1, 0.0);
                    refresh_sidebar_header = true;
                    refresh_toc = true;
                    refresh_highlights = true;
                    refresh_bookmarks = true;
                    refresh_words = true;
                    refresh_chrome = true;
                }
            }
            ReaderMsg::NextChapter => {
                if self.chapter + 1 < self.open.chapter_count() && !self.loading {
                    self.fraction = 1.0;
                    self.go_chapter(self.chapter + 1, 0.0);
                    refresh_sidebar_header = true;
                    refresh_toc = true;
                    refresh_highlights = true;
                    refresh_bookmarks = true;
                    refresh_words = true;
                    refresh_chrome = true;
                }
            }
            ReaderMsg::Theme(theme) => {
                self.theme = theme;
                self.catalog.set_pref("reader.theme", theme.as_str());
                self.loading = true;
                load_chapter(self);
                self.loading = false;
                refresh_controls = true;
                refresh_stage = true;
            }
            ReaderMsg::FontDelta(delta) => {
                let next = (self.font_px as i32 + delta).clamp(13, 24) as u32;
                if next != self.font_px {
                    self.font_px = next;
                    self.catalog.set_pref("reader.font_px", &next.to_string());
                    self.loading = true;
                    load_chapter(self);
                    self.loading = false;
                    refresh_controls = true;
                }
            }
            ReaderMsg::LineHeightDelta(delta) => {
                let next =
                    ((self.line_height * 10.0).round() as i32 + delta).clamp(13, 25) as f32 / 10.0;
                if (next - self.line_height).abs() > f32::EPSILON {
                    self.line_height = next;
                    self.catalog
                        .set_pref("reader.line_height", &format!("{next:.1}"));
                    self.loading = true;
                    load_chapter(self);
                    self.loading = false;
                    refresh_controls = true;
                }
            }
            ReaderMsg::ColumnWidthDelta(delta) => {
                let next = (self.column_px as i32 + delta).clamp(400, 860) as u32;
                if next != self.column_px {
                    self.column_px = next;
                    self.catalog.set_pref("reader.column_px", &next.to_string());
                    self.loading = true;
                    load_chapter(self);
                    self.loading = false;
                    refresh_controls = true;
                }
            }
            ReaderMsg::SwitchSettingsPane(pane) => {
                if pane != self.settings_pane {
                    self.settings_pane = pane;
                    refresh_controls = true;
                }
            }
            ReaderMsg::SetUiSetting(setting, value) => {
                if update_reader_ui_setting(self, setting, value) {
                    apply_reader_ui_prefs(self);
                    refresh_controls = true;
                }
            }
            ReaderMsg::AdjustUiSetting(setting, delta) => {
                let next = self.ui_prefs.get(setting) + delta;
                if update_reader_ui_setting(self, setting, next) {
                    apply_reader_ui_prefs(self);
                    refresh_controls = true;
                }
            }
            ReaderMsg::JsRaw(raw) => {
                let cleaned = raw.trim();
                let json_part = if cleaned.starts_with("kalam://") {
                    url_decode(cleaned.trim_start_matches("kalam://"))
                } else if cleaned.contains("kalam://") {
                    if let Some(idx) = cleaned.find("kalam://") {
                        url_decode(&cleaned[idx + 8..])
                    } else {
                        cleaned.to_string()
                    }
                } else {
                    cleaned.to_string()
                };
                let decoded = url_decode(&json_part);
                if let Ok(payload) = serde_json::from_str::<JsPayload>(&decoded) {
                    let kind = payload.kind.clone();
                    self.handle_js_payload(payload, sender.clone());
                    refresh_sidebar_header = true;
                    refresh_chrome = true;
                    match kind.as_str() {
                        "highlight" | "quote" => refresh_highlights = true,
                        "save-word" => {
                            refresh_words = true;
                            refresh_tabs = true;
                        }
                        "dict-shortcut" => {
                            refresh_words = true;
                            refresh_tabs = true;
                        }
                        _ => {}
                    }
                } else if let Ok(payload) = serde_json::from_str::<JsPayload>(&json_part) {
                    let kind = payload.kind.clone();
                    self.handle_js_payload(payload, sender.clone());
                    refresh_sidebar_header = true;
                    refresh_chrome = true;
                    match kind.as_str() {
                        "highlight" | "quote" => refresh_highlights = true,
                        "save-word" => {
                            refresh_words = true;
                            refresh_tabs = true;
                        }
                        "dict-shortcut" => {
                            refresh_words = true;
                            refresh_tabs = true;
                        }
                        _ => {}
                    }
                } else if let Ok(f) = decoded.parse::<f64>() {
                    if (0.0..=1.0).contains(&f) {
                        self.fraction = f;
                        refresh_sidebar_header = true;
                        refresh_chrome = true;
                    }
                }
            }
            ReaderMsg::Progress(frac) => {
                self.fraction = frac.clamp(0.0, 1.0);
                refresh_sidebar_header = true;
                refresh_chrome = true;
            }
            ReaderMsg::AnnotationsReload => {
                self.reload_annotations();
                self.reload_bookmarks();
                self.reload_saved_words();
                self.inject_highlights();
                refresh_highlights = true;
                refresh_bookmarks = true;
                refresh_words = true;
            }
            ReaderMsg::DeleteAnnotation(id) => {
                crate::notify::report(
                    self.catalog.delete_annotation(id),
                    "Could not delete the highlight",
                );
                self.reload_annotations();
                let script = format!(
                    "if (window.kalamRemoveHighlight) window.kalamRemoveHighlight('{}');",
                    id
                );
                eval_js(&self.webview, &script);
                refresh_highlights = true;
            }
            ReaderMsg::DeleteBookmark(id) => {
                crate::notify::report(
                    self.catalog.delete_reading_bookmark(id),
                    "Could not delete the mark",
                );
                self.reload_bookmarks();
                refresh_bookmarks = true;
            }
            ReaderMsg::DictSearch(q) => {
                self.dict_query = q.clone();
                if q.trim().is_empty() {
                    self.dict_results.clear();
                } else {
                    self.dict_results = self.catalog.search_dict(&q, 30).unwrap_or_default();
                }
                refresh_words = true;
            }
            ReaderMsg::DictSearchSelect(word) => {
                let results = self.catalog.search_dict(&word, 5).unwrap_or_default();
                if let Some(entry) = results.first() {
                    self.dict_lookup_word = Some(entry.word.clone());
                    self.dict_lookup_def = Some(entry.definition.clone());
                    self.show_dict_in_webview(entry.word.clone(), entry.definition.clone(), None);
                    self.right_tab = RightSidebarTab::Words;
                    self.right_sidebar_open = true;
                    refresh_tabs = true;
                }
            }
            ReaderMsg::SaveCurrentWord => {
                if let (Some(word), Some(def)) = (&self.dict_lookup_word, &self.dict_lookup_def) {
                    let saved = self.catalog.insert_saved_word(
                        word,
                        def,
                        None,
                        Some(self.book_id),
                        Some(self.chapter as i64),
                        self.dict_context.as_deref(),
                    );
                    match saved {
                        Ok(_) => {
                            crate::notify::compact("Word saved", word);
                            self.reload_saved_words();
                            refresh_words = true;
                        }
                        Err(e) => crate::notify::error("Could not save the word", &e.to_string()),
                    }
                }
            }
            ReaderMsg::ClearDict => {
                self.dict_lookup_word = None;
                self.dict_lookup_def = None;
                self.dict_lookup_rect_json = None;
                self.dict_context = None;
                eval_js(
                    &self.webview,
                    "if (window.kalamHideDict) window.kalamHideDict();",
                );
            }
            ReaderMsg::AddBookmark => {
                self.right_tab = RightSidebarTab::Bookmarks;
                self.right_sidebar_open = true;
                self.cancel_right_close();
                if self.open.chapter_count() > 0 {
                    let label = self.current_chapter_title().to_string();
                    match self.catalog.insert_reading_bookmark(
                        self.book_id,
                        self.chapter as i64,
                        self.fraction,
                        &label,
                    ) {
                        Ok(_) => crate::notify::compact("Mark saved", &label),
                        Err(e) => crate::notify::error("Could not save the mark", &e.to_string()),
                    }
                    self.reload_bookmarks();
                    refresh_bookmarks = true;
                }
                refresh_tabs = true;
            }
            ReaderMsg::OpenLeftSidebar => {
                self.cancel_left_close();
                if self.left_tab == LeftSidebarTab::Toc {
                    self.position_toc_scroll();
                }
                self.left_sidebar_open = true;
                refresh_tabs = true;
            }
            ReaderMsg::OpenRightSidebar => {
                self.cancel_right_close();
                self.right_sidebar_open = true;
                refresh_tabs = true;
            }
            ReaderMsg::SwitchLeftTab(tab) => {
                self.left_tab = tab;
                self.cancel_left_close();
                if matches!(tab, LeftSidebarTab::Toc) {
                    self.position_toc_scroll();
                }
                self.left_sidebar_open = true;
                refresh_tabs = true;
                if matches!(tab, LeftSidebarTab::Settings) {
                    refresh_controls = true;
                }
            }
            ReaderMsg::SwitchRightTab(tab) => {
                self.right_tab = tab;
                self.right_sidebar_open = true;
                self.cancel_right_close();
                refresh_tabs = true;
                match tab {
                    RightSidebarTab::Highlights => refresh_highlights = true,
                    RightSidebarTab::Bookmarks => refresh_bookmarks = true,
                    RightSidebarTab::Words => refresh_words = true,
                }
            }
            ReaderMsg::SetHighlightFilter(filter) => {
                self.highlight_filter = filter;
                refresh_controls = true;
                refresh_highlights = true;
            }
            ReaderMsg::SetWordScope(scope) => {
                self.word_scope = scope;
                refresh_controls = true;
                refresh_words = true;
            }
            ReaderMsg::ScheduleCloseLeft => {
                self.schedule_left_close(sender.clone());
            }
            ReaderMsg::ScheduleCloseRight => {
                self.schedule_right_close(sender.clone());
            }
            ReaderMsg::ForceCloseLeft => {
                self.left_sidebar_open = false;
                self.left_close_timer = None;
                refresh_tabs = true;
            }
            ReaderMsg::ForceCloseRight => {
                self.right_sidebar_open = false;
                self.right_close_timer = None;
                refresh_tabs = true;
            }
            ReaderMsg::CloseSidebars => {
                self.close_sidebars();
                refresh_tabs = true;
            }
            ReaderMsg::HideChrome => {
                self.show_back_button = false;
                self.show_bottom_pill = false;
                refresh_chrome = true;
            }
            ReaderMsg::ShowBackChrome => {
                self.show_back_button = true;
                refresh_chrome = true;
            }
            ReaderMsg::ShowBottomChrome => {
                self.show_bottom_pill = true;
                refresh_chrome = true;
            }
            ReaderMsg::ShowAllChrome => {
                self.show_back_button = true;
                self.show_bottom_pill = true;
                refresh_chrome = true;
            }
        }

        if refresh_stage {
            sync_reader_stage_theme(&widgets.reader_stage, self.theme);
        }
        if refresh_sidebar_header {
            update_sidebar_header(widgets, self);
        }
        if refresh_chrome {
            update_chrome_labels(widgets, self);
        }
        if refresh_controls {
            sync_reader_controls(self);
        }
        if refresh_tabs {
            sync_sidebar_tabs(widgets, self);
        }
        if refresh_toc {
            rebuild_toc(&self.toc_list, &self.open, self.chapter, &sender);
            if self.left_sidebar_open && self.left_tab == LeftSidebarTab::Toc {
                self.position_toc_scroll();
            }
        }
        if refresh_highlights {
            rebuild_highlights_list(self, &sender);
        }
        if refresh_bookmarks {
            rebuild_bookmarks_list(self, &sender);
        }
        if refresh_words {
            rebuild_words_list(self, &sender);
        }

        self.update_view(widgets, sender);
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: relm4::Sender<Self::Output>) {
        self.save_progress();
        self.close_session();
        self.cancel_left_close();
        self.cancel_right_close();
        self.webview.stop_loading();
    }
}

impl ReaderModel {
    fn css(&self) -> String {
        reading_css(self.theme, self.font_px, self.line_height, self.column_px)
    }

    fn progress_pct(&self) -> i64 {
        let count = self.open.chapter_count();
        if count == 0 {
            return 0;
        }
        ((((self.chapter as f64) + self.fraction) / count as f64) * 100.0)
            .round()
            .clamp(0.0, 100.0) as i64
    }

    fn current_chapter_title(&self) -> &str {
        self.open
            .spine
            .get(self.chapter)
            .map(|item| item.title.as_str())
            .unwrap_or("Reading")
    }

    fn save_progress(&mut self) {
        if self.open.chapter_count() == 0 {
            return;
        }
        let _ = self.catalog.set_reading_progress(
            self.book_id,
            self.chapter,
            self.fraction,
            self.open.chapter_count(),
        );
        let pct = self.progress_pct();
        let _ = self.catalog.auto_finish_if_complete(self.book_id, pct);
    }

    fn close_session(&mut self) {
        let Some(session_id) = self.session_id.take() else {
            return;
        };
        let seconds = self.session_start.elapsed().as_secs() as i64;
        let _ = self
            .catalog
            .end_reading_session(session_id, seconds, self.progress_pct());
    }

    fn go_chapter(&mut self, idx: usize, frac: f64) {
        self.save_progress();
        self.chapter = idx;
        self.fraction = frac.clamp(0.0, 1.0);
        self.loading = true;
        self.reload_annotations();
        self.reload_bookmarks();
        self.reload_saved_words();
        load_chapter(self);
        self.loading = false;
    }

    fn reload_annotations(&mut self) {
        self.chapter_annotations = self
            .catalog
            .get_annotations_for_chapter(self.book_id, self.chapter as i64)
            .unwrap_or_default();
        self.all_book_annotations = self
            .catalog
            .get_annotations_for_book(self.book_id)
            .unwrap_or_default();
    }

    fn reload_bookmarks(&mut self) {
        self.bookmarks = self
            .catalog
            .list_reading_bookmarks(self.book_id)
            .unwrap_or_default();
    }

    fn reload_saved_words(&mut self) {
        self.saved_words = self.catalog.list_saved_words("").unwrap_or_default();
    }

    fn toc_display_position(&self) -> Option<(usize, usize)> {
        let active_spine = toc_active_spine_index(&self.open, self.chapter)?;
        if self.open.toc.is_empty() {
            let total = self.open.spine.len();
            if total == 0 {
                None
            } else {
                Some((active_spine.min(total.saturating_sub(1)), total))
            }
        } else {
            let visible: Vec<usize> = self
                .open
                .toc
                .iter()
                .filter_map(|entry| entry.spine_index)
                .collect();
            let total = visible.len();
            if total == 0 {
                None
            } else {
                visible
                    .iter()
                    .position(|idx| *idx == active_spine)
                    .map(|current| (current, total))
            }
        }
    }

    fn position_toc_scroll(&self) {
        let Some((current, total)) = self.toc_display_position() else {
            return;
        };
        let adj = self.toc_scroll.vadjustment();
        let max = (adj.upper() - adj.page_size()).max(0.0);
        if max <= 0.0 || total <= 1 {
            adj.set_value(0.0);
            return;
        }
        let content_span = adj.upper().max(adj.page_size());
        let target_center = content_span * ((current as f64 + 0.5) / total as f64);
        let target = (target_center - (adj.page_size() / 2.0)).clamp(0.0, max);
        adj.set_value(target);
    }

    fn close_sidebars(&mut self) {
        self.left_sidebar_open = false;
        self.right_sidebar_open = false;
        self.cancel_left_close();
        self.cancel_right_close();
    }

    fn cancel_left_close(&mut self) {
        if let Some(id) = self.left_close_timer.take() {
            id.remove();
        }
    }

    fn cancel_right_close(&mut self) {
        if let Some(id) = self.right_close_timer.take() {
            id.remove();
        }
    }

    fn schedule_left_close(&mut self, sender: ComponentSender<Self>) {
        self.cancel_left_close();
        self.left_close_timer = Some(glib::timeout_add_local(
            Duration::from_millis(320),
            move || {
                sender.input(ReaderMsg::ForceCloseLeft);
                glib::ControlFlow::Break
            },
        ));
    }

    fn schedule_right_close(&mut self, sender: ComponentSender<Self>) {
        self.cancel_right_close();
        self.right_close_timer = Some(glib::timeout_add_local(
            Duration::from_millis(320),
            move || {
                sender.input(ReaderMsg::ForceCloseRight);
                glib::ControlFlow::Break
            },
        ));
    }

    fn inject_highlights(&self) {
        if self.chapter_annotations.is_empty() {
            return;
        }
        let simple: Vec<serde_json::Value> = self
            .chapter_annotations
            .iter()
            .map(|a| {
                serde_json::json!({
                    "id": a.id,
                    "start_path": a.start_path,
                    "start_offset": a.start_offset,
                    "end_path": a.end_path,
                    "end_offset": a.end_offset,
                    "color": a.color,
                })
            })
            .collect();
        if let Ok(json) = serde_json::to_string(&simple) {
            let escaped = json
                .replace('\\', "\\\\")
                .replace('\'', "\\'")
                .replace('\n', "\\n");
            let script = format!(
                "if (window.kalamInjectHighlights) window.kalamInjectHighlights('{}');",
                escaped
            );
            eval_js(&self.webview, &script);
        }
    }

    fn show_dict_in_webview(&self, word: String, definition: String, rect_json: Option<String>) {
        let word_esc = word
            .replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('\n', "\\n");
        let def_esc = definition
            .replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('\n', "\\n")
            .chars()
            .take(2000)
            .collect::<String>();
        let rect_part = if let Some(rect) = rect_json {
            let rect_esc = rect.replace('\\', "\\\\").replace('\'', "\\'");
            format!("'{}'", rect_esc)
        } else {
            "null".to_string()
        };
        let script = format!(
            "if (window.kalamShowDict) window.kalamShowDict('{}', '{}', {});",
            word_esc, def_esc, rect_part
        );
        eval_js(&self.webview, &script);
    }

    fn handle_js_payload(&mut self, payload: JsPayload, sender: ComponentSender<Self>) {
        match payload.kind.as_str() {
            "progress" => {
                if let Some(f) = payload.fraction {
                    self.fraction = f.clamp(0.0, 1.0);
                }
            }
            "next" => {
                sender.input(ReaderMsg::NextChapter);
            }
            "selection" => {
                self.last_selection = payload.text;
            }
            "highlight" => {
                let color = payload.color.unwrap_or_else(|| "yellow".into());
                let text = payload.text.unwrap_or_default();
                let sp = payload.start_path.unwrap_or_default();
                let so = payload.start_offset.unwrap_or(0);
                let ep = payload.end_path.unwrap_or_default();
                let eo = payload.end_offset.unwrap_or(0);
                let tmp_id = payload
                    .tmp_id
                    .unwrap_or_else(|| format!("tmp_{}", chrono_now()));
                if sp.is_empty() || ep.is_empty() || text.trim().is_empty() {
                    return;
                }
                let col = HighlightColor::from_str_lossy(&color).as_str().to_string();
                match self.catalog.insert_annotation(
                    self.book_id,
                    "highlight",
                    self.chapter as i64,
                    &sp,
                    so,
                    &ep,
                    eo,
                    &col,
                    &text,
                    "",
                ) {
                    Ok(real_id) => {
                        let script = format!(
                            "try {{ var nodes = document.querySelectorAll('span[data-annotation-id=\\\"{tmp}\\\"]'); nodes.forEach(function(n){{ n.dataset.annotationId='{real}'; }}); }} catch(e) {{}}",
                            tmp = tmp_id.replace('\'', "\\'"),
                            real = real_id,
                        );
                        eval_js(&self.webview, &script);
                        self.reload_annotations();
                        sender.input(ReaderMsg::AnnotationsReload);
                    }
                    Err(e) => crate::notify::error("Could not save the highlight", &e.to_string()),
                }
            }
            "quote" => {
                let text = payload.text.unwrap_or_default();
                let sp = payload.start_path.unwrap_or_default();
                let so = payload.start_offset.unwrap_or(0);
                let ep = payload.end_path.unwrap_or_default();
                let eo = payload.end_offset.unwrap_or(0);
                if sp.is_empty() || ep.is_empty() || text.trim().is_empty() {
                    return;
                }
                match self.catalog.insert_annotation(
                    self.book_id,
                    "quote",
                    self.chapter as i64,
                    &sp,
                    so,
                    &ep,
                    eo,
                    "yellow",
                    &text,
                    "",
                ) {
                    Ok(_) => {
                        crate::notify::compact("Quote saved", "");
                        self.reload_annotations();
                        sender.input(ReaderMsg::AnnotationsReload);
                    }
                    Err(e) => crate::notify::error("Could not save the quote", &e.to_string()),
                }
            }
            "dict-lookup" => {
                let word = payload.word.unwrap_or_default();
                let context = payload.context;
                let rect_json = payload.rect.map(|v| v.to_string());
                if word.trim().is_empty() {
                    return;
                }
                self.dict_context = context.clone();
                self.dict_lookup_rect_json = rect_json.clone();
                let results = self.catalog.search_dict(&word, 5).unwrap_or_default();
                if let Some(entry) = results.first() {
                    self.dict_lookup_word = Some(entry.word.clone());
                    self.dict_lookup_def = Some(entry.definition.clone());
                    self.show_dict_in_webview(
                        entry.word.clone(),
                        entry.definition.clone(),
                        rect_json,
                    );
                } else {
                    let def = format!(
                        "No definition found for '{}'. Total dict entries: {}",
                        word,
                        self.catalog.dict_entry_count().unwrap_or(0)
                    );
                    self.dict_lookup_word = Some(word.clone());
                    self.dict_lookup_def = Some(def.clone());
                    self.show_dict_in_webview(word, def, rect_json);
                }
            }
            "save-word" => {
                let word = payload.word.unwrap_or_default();
                let def = payload.definition.unwrap_or_default();
                if !word.trim().is_empty() && !def.trim().is_empty() {
                    match self.catalog.insert_saved_word(
                        &word,
                        &def,
                        None,
                        Some(self.book_id),
                        Some(self.chapter as i64),
                        payload.context.as_deref().or(self.dict_context.as_deref()),
                    ) {
                        Ok(_) => {
                            crate::notify::compact("Word saved", &word);
                            self.reload_saved_words();
                            self.right_tab = RightSidebarTab::Words;
                            self.right_sidebar_open = true;
                        }
                        Err(e) => crate::notify::error("Could not save the word", &e.to_string()),
                    }
                }
            }
            "dict-shortcut" => {
                self.right_tab = RightSidebarTab::Words;
                self.right_sidebar_open = true;
            }
            "reader-ui-hide" => {
                self.show_back_button = false;
                self.show_bottom_pill = false;
            }
            "reader-ui-show-back" => {
                self.show_back_button = true;
                self.show_bottom_pill = false;
            }
            "reader-ui-show-pill" => {
                self.show_bottom_pill = true;
                self.show_back_button = false;
            }
            "reader-ui-show-all" => {
                self.show_back_button = true;
                self.show_bottom_pill = true;
            }
            _ => {}
        }
    }

    fn filtered_annotations(&self) -> Vec<&Annotation> {
        self.all_book_annotations
            .iter()
            .filter(|anno| match self.highlight_filter {
                HighlightFilter::All => anno.kind == "highlight" || anno.kind == "quote",
                HighlightFilter::Yellow => {
                    anno.kind == "highlight" && anno.color.eq_ignore_ascii_case("yellow")
                }
                HighlightFilter::Green => {
                    anno.kind == "highlight" && anno.color.eq_ignore_ascii_case("green")
                }
                HighlightFilter::Blue => {
                    anno.kind == "highlight" && anno.color.eq_ignore_ascii_case("blue")
                }
                HighlightFilter::Pink => {
                    anno.kind == "highlight" && anno.color.eq_ignore_ascii_case("pink")
                }
                HighlightFilter::Orange => {
                    anno.kind == "highlight" && anno.color.eq_ignore_ascii_case("orange")
                }
                HighlightFilter::Quotes => anno.kind == "quote",
            })
            .collect()
    }

    fn filtered_saved_words(&self) -> Vec<&SavedWord> {
        self.saved_words
            .iter()
            .filter(|word| match self.word_scope {
                WordScope::Chapter => {
                    word.book_id == Some(self.book_id)
                        && word.chapter_index == Some(self.chapter as i64)
                }
                WordScope::Book => word.book_id == Some(self.book_id),
                WordScope::All => true,
            })
            .collect()
    }
}

fn reader_sidebar_tab_content(icon: &str, label: &str) -> gtk::Box {
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 3);
    box_.set_halign(gtk::Align::Center);
    box_.append(&crate::icons::symbolic_with_classes(
        icon,
        17,
        &["kalam-inline-icon"],
    ));
    let label_widget = gtk::Label::new(Some(label));
    label_widget.set_halign(gtk::Align::Center);
    box_.append(&label_widget);
    box_
}

fn overlay_child_box(overlay: &gtk::Overlay, index: usize) -> Option<gtk::Box> {
    let mut child = overlay.first_child()?;
    for _ in 0..index {
        child = child.next_sibling()?;
    }
    child.downcast::<gtk::Box>().ok()
}

fn connect_hover_zone(
    widget: &impl IsA<gtk::Widget>,
    sender: &ComponentSender<ReaderModel>,
    open_msg: ReaderMsg,
    close_msg: ReaderMsg,
) {
    let motion = gtk::EventControllerMotion::new();
    let s = sender.clone();
    motion.connect_enter(move |_, _, _| s.input(open_msg.clone()));
    let s = sender.clone();
    motion.connect_leave(move |_| s.input(close_msg.clone()));
    widget.add_controller(motion);
}

impl ReaderUiPrefs {
    fn load(catalog: &Catalog) -> Self {
        let mut prefs = Self {
            sidebar_gap: reader_ui_default(ReaderUiSetting::SidebarGap),
            left_sidebar_width: reader_ui_default(ReaderUiSetting::LeftSidebarWidth),
            right_sidebar_width: reader_ui_default(ReaderUiSetting::RightSidebarWidth),
            sidebar_radius: reader_ui_default(ReaderUiSetting::SidebarRadius),
            bottom_pill_size: reader_ui_default(ReaderUiSetting::BottomPillSize),
            back_chip_size: reader_ui_default(ReaderUiSetting::BackChipSize),
            dim_strength: reader_ui_default(ReaderUiSetting::DimStrength),
        };
        for setting in [
            ReaderUiSetting::SidebarGap,
            ReaderUiSetting::LeftSidebarWidth,
            ReaderUiSetting::RightSidebarWidth,
            ReaderUiSetting::SidebarRadius,
            ReaderUiSetting::BottomPillSize,
            ReaderUiSetting::BackChipSize,
            ReaderUiSetting::DimStrength,
        ] {
            let saved =
                catalog.get_pref_i64(reader_ui_pref_key(setting), prefs.get(setting) as i64) as i32;
            let _ = prefs.set(setting, saved);
        }
        prefs
    }

    fn get(&self, setting: ReaderUiSetting) -> i32 {
        match setting {
            ReaderUiSetting::SidebarGap => self.sidebar_gap,
            ReaderUiSetting::LeftSidebarWidth => self.left_sidebar_width,
            ReaderUiSetting::RightSidebarWidth => self.right_sidebar_width,
            ReaderUiSetting::SidebarRadius => self.sidebar_radius,
            ReaderUiSetting::BottomPillSize => self.bottom_pill_size,
            ReaderUiSetting::BackChipSize => self.back_chip_size,
            ReaderUiSetting::DimStrength => self.dim_strength,
        }
    }

    fn set(&mut self, setting: ReaderUiSetting, value: i32) -> bool {
        let next = clamp_reader_ui_value(setting, value);
        let slot = match setting {
            ReaderUiSetting::SidebarGap => &mut self.sidebar_gap,
            ReaderUiSetting::LeftSidebarWidth => &mut self.left_sidebar_width,
            ReaderUiSetting::RightSidebarWidth => &mut self.right_sidebar_width,
            ReaderUiSetting::SidebarRadius => &mut self.sidebar_radius,
            ReaderUiSetting::BottomPillSize => &mut self.bottom_pill_size,
            ReaderUiSetting::BackChipSize => &mut self.back_chip_size,
            ReaderUiSetting::DimStrength => &mut self.dim_strength,
        };
        if *slot == next {
            return false;
        }
        *slot = next;
        true
    }
}

fn reader_settings_pane_name(pane: ReaderSettingsPane) -> &'static str {
    match pane {
        ReaderSettingsPane::Reading => "reading",
        ReaderSettingsPane::Ui => "ui",
    }
}

fn reader_ui_pref_key(setting: ReaderUiSetting) -> &'static str {
    match setting {
        ReaderUiSetting::SidebarGap => "reader.ui.sidebar_gap_px",
        ReaderUiSetting::LeftSidebarWidth => "reader.ui.left_sidebar_width_px",
        ReaderUiSetting::RightSidebarWidth => "reader.ui.right_sidebar_width_px",
        ReaderUiSetting::SidebarRadius => "reader.ui.sidebar_radius_px",
        ReaderUiSetting::BottomPillSize => "reader.ui.bottom_pill_size_px",
        ReaderUiSetting::BackChipSize => "reader.ui.back_chip_size_px",
        ReaderUiSetting::DimStrength => "reader.ui.dim_strength_pct",
    }
}

fn reader_ui_default(setting: ReaderUiSetting) -> i32 {
    match setting {
        ReaderUiSetting::SidebarGap => 8,
        ReaderUiSetting::LeftSidebarWidth => 248,
        ReaderUiSetting::RightSidebarWidth => 212,
        ReaderUiSetting::SidebarRadius => 20,
        ReaderUiSetting::BottomPillSize => 34,
        ReaderUiSetting::BackChipSize => 32,
        ReaderUiSetting::DimStrength => 22,
    }
}

fn reader_ui_presets(setting: ReaderUiSetting) -> &'static [(&'static str, i32)] {
    match setting {
        ReaderUiSetting::SidebarGap => &UI_PRESETS_SIDEBAR_GAP,
        ReaderUiSetting::LeftSidebarWidth => &UI_PRESETS_LEFT_WIDTH,
        ReaderUiSetting::RightSidebarWidth => &UI_PRESETS_RIGHT_WIDTH,
        ReaderUiSetting::SidebarRadius => &UI_PRESETS_RADIUS,
        ReaderUiSetting::BottomPillSize => &UI_PRESETS_PILL_SIZE,
        ReaderUiSetting::BackChipSize => &UI_PRESETS_BACK_SIZE,
        ReaderUiSetting::DimStrength => &UI_PRESETS_DIM,
    }
}

fn clamp_reader_ui_value(setting: ReaderUiSetting, value: i32) -> i32 {
    match setting {
        ReaderUiSetting::SidebarGap => value.clamp(0, 36),
        ReaderUiSetting::LeftSidebarWidth => value.clamp(180, 340),
        ReaderUiSetting::RightSidebarWidth => value.clamp(160, 320),
        ReaderUiSetting::SidebarRadius => value.clamp(0, 36),
        ReaderUiSetting::BottomPillSize => value.clamp(26, 52),
        ReaderUiSetting::BackChipSize => value.clamp(24, 48),
        ReaderUiSetting::DimStrength => value.clamp(0, 50),
    }
}

fn reader_ui_step(setting: ReaderUiSetting) -> i32 {
    match setting {
        ReaderUiSetting::SidebarGap => 2,
        ReaderUiSetting::LeftSidebarWidth => 4,
        ReaderUiSetting::RightSidebarWidth => 4,
        ReaderUiSetting::SidebarRadius => 2,
        ReaderUiSetting::BottomPillSize => 2,
        ReaderUiSetting::BackChipSize => 2,
        ReaderUiSetting::DimStrength => 2,
    }
}

fn reader_ui_value_text(setting: ReaderUiSetting, value: i32) -> String {
    match setting {
        ReaderUiSetting::DimStrength => format!("{value}%"),
        _ => format!("{value}px"),
    }
}

fn register_reader_ui_provider(provider: &gtk::CssProvider) {
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn reader_ui_css(prefs: ReaderUiPrefs) -> String {
    let pill = prefs.bottom_pill_size;
    let pill_pad_y = ((pill - 22) / 4).clamp(4, 8);
    let pill_pad_x = ((pill - 14) / 3).clamp(6, 12);
    let pill_info_pad = ((pill as f32 * 0.34).round() as i32).clamp(10, 18);
    let pill_icon = (pill / 2).clamp(14, 20);
    let pill_pages_font = ((pill as f32 * 0.44).round() as i32).clamp(12, 16);
    let pill_chapter_font = ((pill as f32 * 0.48).round() as i32).clamp(14, 18);
    let back = prefs.back_chip_size;
    let back_pad_y = ((back - 18) / 2).clamp(5, 10);
    let back_pad_left = (back_pad_y + 3).clamp(8, 14);
    let back_pad_right = (back_pad_y + 6).clamp(11, 17);
    let back_icon = (back / 2).clamp(14, 18);
    let dim_alpha = prefs.dim_strength as f32 / 100.0;
    format!(
        r#"
.kalam-reader-ui-live .kalam-reader-sidebar {{
    border-radius: {radius}px;
}}

.kalam-reader-ui-live .kalam-reader-sidebar-left,
.kalam-reader-ui-live .kalam-reader-sidebar-right {{
    border-radius: {radius}px;
}}

.kalam-reader-ui-live button.kalam-reader-dim {{
    background: alpha(@kalam_bg, {dim_alpha:.2});
}}

.kalam-reader-ui-live .kalam-reader-bottom-pill {{
    padding: {pill_pad_y}px {pill_pad_x}px;
}}

.kalam-reader-ui-live button.kalam-reader-pill-nav {{
    min-width: {pill}px;
    min-height: {pill}px;
    padding: 0 {pill_pad_x}px;
}}

.kalam-reader-ui-live .kalam-reader-pill-nav image {{
    -gtk-icon-size: {pill_icon}px;
}}

.kalam-reader-ui-live .kalam-reader-pill-info {{
    padding: 0 {pill_info_pad}px;
}}

.kalam-reader-ui-live .kalam-reader-pill-pages {{
    font-size: {pill_pages_font}px;
}}

.kalam-reader-ui-live .kalam-reader-pill-chapter {{
    font-size: {pill_chapter_font}px;
}}

.kalam-reader-ui-live button.kalam-reader-back {{
    min-height: {back}px;
    padding: {back_pad_y}px {back_pad_right}px {back_pad_y}px {back_pad_left}px;
}}

.kalam-reader-ui-live .kalam-reader-back image {{
    -gtk-icon-size: {back_icon}px;
}}
"#,
        radius = prefs.sidebar_radius,
        dim_alpha = dim_alpha,
        pill_pad_y = pill_pad_y,
        pill_pad_x = pill_pad_x,
        pill = pill,
        pill_icon = pill_icon,
        pill_info_pad = pill_info_pad,
        pill_pages_font = pill_pages_font,
        pill_chapter_font = pill_chapter_font,
        back = back,
        back_pad_y = back_pad_y,
        back_pad_right = back_pad_right,
        back_pad_left = back_pad_left,
        back_icon = back_icon,
    )
}

fn apply_reader_ui_prefs(model: &ReaderModel) {
    if let Some(shell) = &model.left_sidebar_shell {
        let gap = model.ui_prefs.sidebar_gap;
        shell.set_margin_start(gap);
        shell.set_margin_top(gap);
        shell.set_margin_bottom(gap);
    }
    if let Some(shell) = &model.right_sidebar_shell {
        let gap = model.ui_prefs.sidebar_gap;
        shell.set_margin_end(gap);
        shell.set_margin_top(gap);
        shell.set_margin_bottom(gap);
    }
    if let Some(sidebar) = &model.left_sidebar_box {
        sidebar.set_size_request(model.ui_prefs.left_sidebar_width, -1);
    }
    if let Some(sidebar) = &model.right_sidebar_box {
        sidebar.set_size_request(model.ui_prefs.right_sidebar_width, -1);
    }
    model
        .ui_css_provider
        .load_from_string(&reader_ui_css(model.ui_prefs));
}

fn update_reader_ui_setting(model: &mut ReaderModel, setting: ReaderUiSetting, value: i32) -> bool {
    if !model.ui_prefs.set(setting, value) {
        return false;
    }
    model.catalog.set_pref(
        reader_ui_pref_key(setting),
        &model.ui_prefs.get(setting).to_string(),
    );
    true
}

fn build_reader_settings_panel(
    sender: &ComponentSender<ReaderModel>,
    theme: ReadingTheme,
    font_px: u32,
    line_height: f32,
    column_px: u32,
    ui_prefs: ReaderUiPrefs,
) -> ReaderSettingsControls {
    let wrap = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let switcher = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    switcher.add_css_class("kalam-reader-settings-switcher");
    switcher.set_margin_all(12);
    switcher.set_homogeneous(true);
    let mut pane_buttons = Vec::new();
    for (label, pane) in [
        ("Reading", ReaderSettingsPane::Reading),
        ("UI", ReaderSettingsPane::Ui),
    ] {
        let btn = gtk::Button::with_label(label);
        btn.add_css_class("kalam-reader-settings-switch");
        let s = sender.clone();
        btn.connect_clicked(move |_| s.input(ReaderMsg::SwitchSettingsPane(pane)));
        switcher.append(&btn);
        pane_buttons.push((pane, btn));
    }
    wrap.append(&switcher);

    let stack = gtk::Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);

    let reading_page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let theme_section = reader_settings_section("Reading theme");
    let dots_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let mut dots = Vec::new();
    for (label, value, class_name) in [
        ("Sepia", ReadingTheme::Sepia, "kalam-reader-theme-dot-sepia"),
        ("Light", ReadingTheme::Light, "kalam-reader-theme-dot-light"),
        ("Dark", ReadingTheme::Dark, "kalam-reader-theme-dot-dark"),
        ("Ink", ReadingTheme::Ink, "kalam-reader-theme-dot-ink"),
    ] {
        let btn = gtk::Button::new();
        btn.add_css_class("kalam-reader-theme-dot");
        btn.add_css_class(class_name);
        btn.set_tooltip_text(Some(label));
        let s = sender.clone();
        btn.connect_clicked(move |_| s.input(ReaderMsg::Theme(value)));
        dots_row.append(&btn);
        dots.push((value, btn));
    }
    theme_section.append(&dots_row);
    reading_page.append(&theme_section);
    reading_page.append(&reader_panel_divider());

    let type_section = reader_settings_section("Type");
    let font_size_label = gtk::Label::new(Some(&font_px.to_string()));
    font_size_label.add_css_class("kalam-reader-stepper-value");
    type_section.append(&reader_stepper_row(
        "Font size",
        &font_size_label,
        sender,
        ReaderMsg::FontDelta(-1),
        ReaderMsg::FontDelta(1),
    ));
    let line_height_label = gtk::Label::new(Some(&format!("{line_height:.1}")));
    line_height_label.add_css_class("kalam-reader-stepper-value");
    type_section.append(&reader_stepper_row(
        "Line height",
        &line_height_label,
        sender,
        ReaderMsg::LineHeightDelta(-1),
        ReaderMsg::LineHeightDelta(1),
    ));
    reading_page.append(&type_section);
    reading_page.append(&reader_panel_divider());

    let width_section = reader_settings_section("Column width");
    let column_width_label = gtk::Label::new(Some(&column_px.to_string()));
    column_width_label.add_css_class("kalam-reader-stepper-value");
    width_section.append(&reader_stepper_row(
        "Width",
        &column_width_label,
        sender,
        ReaderMsg::ColumnWidthDelta(-20),
        ReaderMsg::ColumnWidthDelta(20),
    ));
    reading_page.append(&width_section);
    stack.add_named(
        &reading_page,
        Some(reader_settings_pane_name(ReaderSettingsPane::Reading)),
    );

    let ui_page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let mut ui_controls = Vec::new();

    let sidebar_section = reader_settings_section("Sidebars");
    for (title, setting) in [
        ("Edge gap", ReaderUiSetting::SidebarGap),
        ("Left width", ReaderUiSetting::LeftSidebarWidth),
        ("Right width", ReaderUiSetting::RightSidebarWidth),
        ("Corner radius", ReaderUiSetting::SidebarRadius),
    ] {
        let (row, controls) =
            reader_ui_setting_block(title, setting, ui_prefs.get(setting), sender);
        sidebar_section.append(&row);
        ui_controls.push((setting, controls));
    }
    ui_page.append(&sidebar_section);
    ui_page.append(&reader_panel_divider());

    let controls_section = reader_settings_section("Floating controls");
    for (title, setting) in [
        ("Bottom pill", ReaderUiSetting::BottomPillSize),
        ("Back chip", ReaderUiSetting::BackChipSize),
    ] {
        let (row, controls) =
            reader_ui_setting_block(title, setting, ui_prefs.get(setting), sender);
        controls_section.append(&row);
        ui_controls.push((setting, controls));
    }
    ui_page.append(&controls_section);
    ui_page.append(&reader_panel_divider());

    let overlay_section = reader_settings_section("Overlay");
    let (dim_row, dim_controls) = reader_ui_setting_block(
        "Background dim",
        ReaderUiSetting::DimStrength,
        ui_prefs.get(ReaderUiSetting::DimStrength),
        sender,
    );
    overlay_section.append(&dim_row);
    ui_controls.push((ReaderUiSetting::DimStrength, dim_controls));
    ui_page.append(&overlay_section);
    stack.add_named(
        &ui_page,
        Some(reader_settings_pane_name(ReaderSettingsPane::Ui)),
    );

    if theme == ReadingTheme::Sepia {
        for (_, dot) in &dots {
            dot.remove_css_class("active");
        }
    }

    wrap.append(&stack);

    ReaderSettingsControls {
        root: wrap,
        settings_stack: stack,
        pane_buttons,
        font_size_label,
        line_height_label,
        column_width_label,
        theme_dots: dots,
        ui_controls,
    }
}

fn reader_ui_setting_block(
    label: &str,
    setting: ReaderUiSetting,
    value: i32,
    sender: &ComponentSender<ReaderModel>,
) -> (gtk::Box, ReaderUiSettingControls) {
    let wrap = gtk::Box::new(gtk::Orientation::Vertical, 8);
    wrap.add_css_class("kalam-reader-ui-setting");

    let head = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    head.add_css_class("kalam-reader-ui-setting-head");
    let label_widget = gtk::Label::new(Some(label));
    label_widget.add_css_class("kalam-reader-ui-setting-label");
    label_widget.set_hexpand(true);
    label_widget.set_halign(gtk::Align::Start);
    head.append(&label_widget);
    let value_label = gtk::Label::new(Some(&reader_ui_value_text(setting, value)));
    value_label.add_css_class("kalam-reader-ui-setting-value");
    value_label.add_css_class("kalam-reader-stepper-value");
    value_label.set_halign(gtk::Align::End);
    head.append(&value_label);
    wrap.append(&head);

    let presets = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    presets.add_css_class("kalam-reader-ui-preset-row");
    presets.set_homogeneous(true);
    let mut preset_buttons = Vec::new();
    for &(preset_label, preset_value) in reader_ui_presets(setting) {
        let btn = gtk::Button::with_label(preset_label);
        btn.add_css_class("kalam-reader-filter-chip");
        btn.add_css_class("kalam-reader-ui-preset");
        let s = sender.clone();
        btn.connect_clicked(move |_| s.input(ReaderMsg::SetUiSetting(setting, preset_value)));
        presets.append(&btn);
        preset_buttons.push((preset_value, btn));
    }
    let custom_button = gtk::Button::with_label("Custom");
    custom_button.add_css_class("kalam-reader-filter-chip");
    custom_button.add_css_class("kalam-reader-ui-preset");
    custom_button.add_css_class("kalam-reader-ui-preset-custom");
    custom_button.set_focus_on_click(false);
    presets.append(&custom_button);
    wrap.append(&presets);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.add_css_class("kalam-reader-ui-stepper-row");
    actions.set_halign(gtk::Align::End);
    let minus = gtk::Button::new();
    minus.add_css_class("kalam-reader-stepper-btn");
    minus.set_child(Some(&crate::icons::symbolic_with_classes(
        "list-remove-symbolic",
        14,
        &["kalam-inline-icon"],
    )));
    let s = sender.clone();
    minus.connect_clicked(move |_| {
        s.input(ReaderMsg::AdjustUiSetting(
            setting,
            -reader_ui_step(setting),
        ))
    });
    actions.append(&minus);
    let plus = gtk::Button::new();
    plus.add_css_class("kalam-reader-stepper-btn");
    plus.set_child(Some(&crate::icons::symbolic_with_classes(
        "list-add-symbolic",
        14,
        &["kalam-inline-icon"],
    )));
    let s = sender.clone();
    plus.connect_clicked(move |_| {
        s.input(ReaderMsg::AdjustUiSetting(setting, reader_ui_step(setting)))
    });
    actions.append(&plus);
    wrap.append(&actions);

    (
        wrap,
        ReaderUiSettingControls {
            value_label,
            preset_buttons,
            custom_button,
        },
    )
}

fn reader_settings_section(label: &str) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 10);
    section.add_css_class("kalam-reader-section");
    let heading = gtk::Label::new(Some(label));
    heading.add_css_class("kalam-reader-section-label");
    heading.set_halign(gtk::Align::Start);
    section.append(&heading);
    section
}

fn reader_stepper_row(
    label: &str,
    value_label: &gtk::Label,
    sender: &ComponentSender<ReaderModel>,
    minus_msg: ReaderMsg,
    plus_msg: ReaderMsg,
) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row.add_css_class("kalam-reader-setting-row");

    let label_widget = gtk::Label::new(Some(label));
    label_widget.add_css_class("kalam-reader-setting-name");
    label_widget.set_hexpand(true);
    label_widget.set_halign(gtk::Align::Start);
    row.append(&label_widget);

    let stepper = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    stepper.add_css_class("kalam-reader-stepper");

    let minus = gtk::Button::new();
    minus.add_css_class("kalam-reader-stepper-btn");
    minus.set_child(Some(&crate::icons::symbolic_with_classes(
        "list-remove-symbolic",
        14,
        &["kalam-inline-icon"],
    )));
    let s = sender.clone();
    minus.connect_clicked(move |_| s.input(minus_msg.clone()));
    stepper.append(&minus);

    stepper.append(value_label);

    let plus = gtk::Button::new();
    plus.add_css_class("kalam-reader-stepper-btn");
    plus.set_child(Some(&crate::icons::symbolic_with_classes(
        "list-add-symbolic",
        14,
        &["kalam-inline-icon"],
    )));
    let s = sender.clone();
    plus.connect_clicked(move |_| s.input(plus_msg.clone()));
    stepper.append(&plus);

    row.append(&stepper);
    row
}

fn reader_panel_divider() -> gtk::Separator {
    let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
    sep.add_css_class("kalam-reader-panel-divider");
    sep
}

fn build_highlights_panel(
    sender: &ComponentSender<ReaderModel>,
    _active: HighlightFilter,
    list: &gtk::Box,
) -> (gtk::Box, Vec<(HighlightFilter, gtk::Button)>) {
    let wrap = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let chips = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    chips.add_css_class("kalam-reader-filter-row");
    chips.set_margin_top(10);
    chips.set_margin_start(12);
    chips.set_margin_end(12);
    chips.set_margin_bottom(10);

    let mut buttons = Vec::new();
    for (label, filter, class_name) in [
        ("All", HighlightFilter::All, None),
        (
            "Yellow",
            HighlightFilter::Yellow,
            Some("kalam-reader-filter-yellow"),
        ),
        (
            "Green",
            HighlightFilter::Green,
            Some("kalam-reader-filter-green"),
        ),
        (
            "Blue",
            HighlightFilter::Blue,
            Some("kalam-reader-filter-blue"),
        ),
        (
            "Pink",
            HighlightFilter::Pink,
            Some("kalam-reader-filter-pink"),
        ),
        (
            "Orange",
            HighlightFilter::Orange,
            Some("kalam-reader-filter-orange"),
        ),
        ("Quotes", HighlightFilter::Quotes, None),
    ] {
        let btn = gtk::Button::with_label(label);
        btn.add_css_class("kalam-reader-filter-chip");
        if let Some(class_name) = class_name {
            btn.add_css_class(class_name);
        }
        let s = sender.clone();
        btn.connect_clicked(move |_| s.input(ReaderMsg::SetHighlightFilter(filter)));
        chips.append(&btn);
        buttons.push((filter, btn));
    }
    wrap.append(&chips);

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .child(list)
        .build();
    scroll.add_css_class("kalam-reader-panel-scroll");
    wrap.append(&scroll);

    (wrap, buttons)
}

fn build_bookmarks_panel(sender: &ComponentSender<ReaderModel>, list: &gtk::Box) -> gtk::Box {
    let wrap = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_margin_all(12);
    let add_btn = gtk::Button::new();
    add_btn.set_child(Some(&crate::icons::labelled(
        "bookmark-new-symbolic",
        16,
        "Add current place",
        6,
    )));
    add_btn.add_css_class("kalam-btn-tonal");
    let s = sender.clone();
    add_btn.connect_clicked(move |_| s.input(ReaderMsg::AddBookmark));
    actions.append(&add_btn);
    wrap.append(&actions);

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .child(list)
        .build();
    scroll.add_css_class("kalam-reader-panel-scroll");
    wrap.append(&scroll);
    wrap
}

fn build_words_panel(
    sender: &ComponentSender<ReaderModel>,
    _scope: WordScope,
    list: &gtk::Box,
) -> (gtk::Box, Vec<(WordScope, gtk::Button)>, gtk::SearchEntry) {
    let wrap = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let search_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    search_row.add_css_class("kalam-reader-search-row");
    search_row.set_margin_all(12);
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Look up a word…"));
    search.set_hexpand(true);
    search.add_css_class("kalam-reader-search");
    let s = sender.clone();
    search.connect_search_changed(move |entry| {
        s.input(ReaderMsg::DictSearch(entry.text().to_string()));
    });
    search_row.append(&search);
    wrap.append(&search_row);

    let chips = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    chips.add_css_class("kalam-reader-filter-row");
    chips.set_margin_start(12);
    chips.set_margin_end(12);
    chips.set_margin_bottom(10);
    let mut buttons = Vec::new();
    for (label, scope) in [
        ("Chapter", WordScope::Chapter),
        ("This book", WordScope::Book),
        ("All", WordScope::All),
    ] {
        let btn = gtk::Button::with_label(label);
        btn.add_css_class("kalam-reader-filter-chip");
        let s = sender.clone();
        btn.connect_clicked(move |_| s.input(ReaderMsg::SetWordScope(scope)));
        chips.append(&btn);
        buttons.push((scope, btn));
    }
    wrap.append(&chips);

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .child(list)
        .build();
    scroll.add_css_class("kalam-reader-panel-scroll");
    wrap.append(&scroll);

    (wrap, buttons, search)
}

fn sync_reader_controls(model: &ReaderModel) {
    model
        .left_stack
        .set_visible_child_name(match model.left_tab {
            LeftSidebarTab::Toc => "toc",
            LeftSidebarTab::Settings => "settings",
        });
    model
        .right_stack
        .set_visible_child_name(match model.right_tab {
            RightSidebarTab::Highlights => "highlights",
            RightSidebarTab::Bookmarks => "bookmarks",
            RightSidebarTab::Words => "words",
        });
    model
        .settings_stack
        .set_visible_child_name(reader_settings_pane_name(model.settings_pane));

    model.font_size_label.set_label(&model.font_px.to_string());
    model
        .line_height_label
        .set_label(&format!("{:.1}", model.line_height));
    model
        .column_width_label
        .set_label(&model.column_px.to_string());

    for (pane, btn) in &model.settings_pane_buttons {
        toggle_active(btn, *pane == model.settings_pane);
    }
    for (theme, btn) in &model.theme_dots {
        toggle_active(btn, *theme == model.theme);
    }
    for (setting, controls) in &model.ui_controls {
        let value = model.ui_prefs.get(*setting);
        controls
            .value_label
            .set_label(&reader_ui_value_text(*setting, value));
        let mut matched_preset = false;
        for (preset_value, btn) in &controls.preset_buttons {
            let active = *preset_value == value;
            toggle_active(btn, active);
            matched_preset |= active;
        }
        toggle_active(&controls.custom_button, !matched_preset);
    }
    for (filter, btn) in &model.highlight_filter_buttons {
        toggle_active(btn, *filter == model.highlight_filter);
    }
    for (scope, btn) in &model.word_scope_buttons {
        toggle_active(btn, *scope == model.word_scope);
    }
}

fn sync_sidebar_tabs(widgets: &ReaderModelWidgets, model: &ReaderModel) {
    toggle_active(&widgets.left_toc_tab, model.left_tab == LeftSidebarTab::Toc);
    toggle_active(
        &widgets.left_settings_tab,
        model.left_tab == LeftSidebarTab::Settings,
    );
    toggle_active(
        &widgets.right_highlights_tab,
        model.right_tab == RightSidebarTab::Highlights,
    );
    toggle_active(
        &widgets.right_bookmarks_tab,
        model.right_tab == RightSidebarTab::Bookmarks,
    );
    toggle_active(
        &widgets.right_words_tab,
        model.right_tab == RightSidebarTab::Words,
    );
    sync_reader_controls(model);
}

fn toggle_active(widget: &impl IsA<gtk::Widget>, active: bool) {
    if active {
        widget.add_css_class("active");
    } else {
        widget.remove_css_class("active");
    }
}

fn rebuild_cover_host(host: &gtk::Box, cover_path: Option<&std::path::Path>) {
    while let Some(child) = host.first_child() {
        host.remove(&child);
    }
    host.append(&cover_widget(cover_path, 48, 70));
}

fn update_sidebar_header(widgets: &ReaderModelWidgets, model: &ReaderModel) {
    widgets.sidebar_book_title.set_label(&model.book_title);
    widgets.sidebar_book_author.set_label(&model.book_authors);
    widgets
        .sidebar_progress
        .set_fraction(model.progress_pct() as f64 / 100.0);
}

fn update_chrome_labels(widgets: &ReaderModelWidgets, model: &ReaderModel) {
    if model.open.chapter_count() == 0 {
        widgets.progress_label.set_label("—");
        widgets.pill_chapter_label.set_label(&model.book_title);
        return;
    }
    widgets.progress_label.set_label(&format!(
        "{} / {}",
        model.chapter + 1,
        model.open.chapter_count()
    ));
    widgets
        .pill_chapter_label
        .set_label(model.current_chapter_title());
}

fn sync_reader_stage_theme(stage: &gtk::Box, theme: ReadingTheme) {
    for class_name in [
        "kalam-reader-paper-light",
        "kalam-reader-paper-sepia",
        "kalam-reader-paper-dark",
        "kalam-reader-paper-ink",
    ] {
        stage.remove_css_class(class_name);
    }
    stage.add_css_class(match theme {
        ReadingTheme::Light => "kalam-reader-paper-light",
        ReadingTheme::Sepia => "kalam-reader-paper-sepia",
        ReadingTheme::Dark => "kalam-reader-paper-dark",
        ReadingTheme::Ink => "kalam-reader-paper-ink",
    });
}

fn rebuild_toc(
    list: &gtk::Box,
    open: &OpenBook,
    current: usize,
    sender: &ComponentSender<ReaderModel>,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    let active_spine = toc_active_spine_index(open, current);
    if open.toc.is_empty() {
        for (idx, item) in open.spine.iter().enumerate() {
            append_toc_btn(list, &item.title, idx, active_spine, sender);
        }
    } else {
        for entry in &open.toc {
            if let Some(idx) = entry.spine_index {
                append_toc_btn(list, &entry.label, idx, active_spine, sender);
            }
        }
    }
}

fn append_toc_btn(
    list: &gtk::Box,
    label: &str,
    idx: usize,
    active_spine: Option<usize>,
    sender: &ComponentSender<ReaderModel>,
) {
    let btn = gtk::Button::new();
    btn.add_css_class("kalam-reader-toc-item");
    if Some(idx) == active_spine {
        btn.add_css_class("active");
    }
    let title = gtk::Label::new(Some(label));
    title.set_halign(gtk::Align::Start);
    title.set_hexpand(true);
    title.set_wrap(true);
    title.set_xalign(0.0);
    btn.set_child(Some(&title));
    let s = sender.clone();
    btn.connect_clicked(move |_| s.input(ReaderMsg::TocSelect(idx)));
    list.append(&btn);
}

fn toc_active_spine_index(open: &OpenBook, current: usize) -> Option<usize> {
    if open.toc.is_empty() {
        if open.spine.is_empty() {
            None
        } else {
            Some(current.min(open.spine.len().saturating_sub(1)))
        }
    } else {
        let visible: Vec<usize> = open
            .toc
            .iter()
            .filter_map(|entry| entry.spine_index)
            .collect();
        if visible.is_empty() {
            None
        } else {
            Some(
                visible
                    .iter()
                    .copied()
                    .rfind(|idx| *idx <= current)
                    .unwrap_or(visible[0]),
            )
        }
    }
}

fn append_reader_empty(list: &gtk::Box, text: &str) {
    let wrap = gtk::Box::new(gtk::Orientation::Vertical, 0);
    wrap.set_hexpand(true);
    wrap.set_vexpand(true);

    let top_spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    top_spacer.set_vexpand(true);
    wrap.append(&top_spacer);

    let label = gtk::Label::new(Some(text));
    label.add_css_class("kalam-reader-empty");
    label.set_wrap(true);
    label.set_halign(gtk::Align::Center);
    label.set_justify(gtk::Justification::Center);
    label.set_xalign(0.5);
    wrap.append(&label);

    let bottom_spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    bottom_spacer.set_vexpand(true);
    wrap.append(&bottom_spacer);

    list.append(&wrap);
}

fn rebuild_highlights_list(model: &ReaderModel, sender: &ComponentSender<ReaderModel>) {
    while let Some(child) = model.highlights_list.first_child() {
        model.highlights_list.remove(&child);
    }
    let annos = model.filtered_annotations();
    if annos.is_empty() {
        append_reader_empty(&model.highlights_list, "No highlights yet in this view.");
        return;
    }

    for anno in annos.into_iter().take(150) {
        let outer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        outer.add_css_class("kalam-reader-annotation-row");

        let jump = gtk::Button::new();
        jump.add_css_class("kalam-reader-list-hit");
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let bar = gtk::Box::new(gtk::Orientation::Vertical, 0);
        bar.add_css_class("kalam-reader-annotation-bar");
        bar.add_css_class(color_bar_class(&anno.color));
        row.append(&bar);

        let text_col = gtk::Box::new(gtk::Orientation::Vertical, 4);
        text_col.set_hexpand(true);
        let text = gtk::Label::new(Some(&anno.text_excerpt));
        text.add_css_class("kalam-reader-annotation-text");
        text.set_wrap(true);
        text.set_xalign(0.0);
        text.set_halign(gtk::Align::Start);
        text_col.append(&text);
        let meta_text = if anno.kind == "quote" {
            format!(
                "{} · quote",
                chapter_label(model, anno.chapter_index as usize)
            )
        } else {
            format!(
                "{} · {}",
                chapter_label(model, anno.chapter_index as usize),
                anno.color
            )
        };
        let meta = gtk::Label::new(Some(&meta_text));
        meta.add_css_class("kalam-reader-annotation-meta");
        meta.set_halign(gtk::Align::Start);
        meta.set_xalign(0.0);
        text_col.append(&meta);
        row.append(&text_col);
        jump.set_child(Some(&row));
        let ch = anno.chapter_index as usize;
        let s = sender.clone();
        jump.connect_clicked(move |_| s.input(ReaderMsg::JumpToChapter(ch)));
        outer.append(&jump);

        let delete = gtk::Button::new();
        delete.add_css_class("kalam-btn-icon");
        delete.add_css_class("danger");
        delete.set_child(Some(&crate::icons::symbolic_with_classes(
            "user-trash-symbolic",
            16,
            &["kalam-inline-icon"],
        )));
        let id = anno.id;
        let s = sender.clone();
        delete.connect_clicked(move |_| s.input(ReaderMsg::DeleteAnnotation(id)));
        outer.append(&delete);

        model.highlights_list.append(&outer);
    }
}

fn rebuild_bookmarks_list(model: &ReaderModel, sender: &ComponentSender<ReaderModel>) {
    while let Some(child) = model.bookmarks_list.first_child() {
        model.bookmarks_list.remove(&child);
    }
    if model.bookmarks.is_empty() {
        append_reader_empty(
            &model.bookmarks_list,
            "No marks yet. Use Add current place or press M.",
        );
        return;
    }

    for mark in model.bookmarks.iter().take(150) {
        let outer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        outer.add_css_class("kalam-reader-bookmark-row");

        let jump = gtk::Button::new();
        jump.add_css_class("kalam-reader-list-hit");
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        row.append(&crate::icons::symbolic_with_classes(
            "bookmark-new-symbolic",
            16,
            &["kalam-reader-bookmark-icon"],
        ));
        let text_col = gtk::Box::new(gtk::Orientation::Vertical, 4);
        text_col.set_hexpand(true);
        let title = gtk::Label::new(Some(if mark.label.trim().is_empty() {
            "Reading mark"
        } else {
            &mark.label
        }));
        title.add_css_class("kalam-reader-bookmark-title");
        title.set_halign(gtk::Align::Start);
        title.set_xalign(0.0);
        text_col.append(&title);
        let meta = gtk::Label::new(Some(&format!(
            "{} · {}%",
            chapter_label(model, mark.chapter_index as usize),
            (mark.fraction * 100.0).round() as i64
        )));
        meta.add_css_class("kalam-reader-annotation-meta");
        meta.set_halign(gtk::Align::Start);
        meta.set_xalign(0.0);
        text_col.append(&meta);
        row.append(&text_col);
        jump.set_child(Some(&row));
        let ch = mark.chapter_index as usize;
        let frac = mark.fraction;
        let s = sender.clone();
        jump.connect_clicked(move |_| s.input(ReaderMsg::JumpToLocation(ch, frac)));
        outer.append(&jump);

        let delete = gtk::Button::new();
        delete.add_css_class("kalam-btn-icon");
        delete.add_css_class("danger");
        delete.set_child(Some(&crate::icons::symbolic_with_classes(
            "user-trash-symbolic",
            16,
            &["kalam-inline-icon"],
        )));
        let id = mark.id;
        let s = sender.clone();
        delete.connect_clicked(move |_| s.input(ReaderMsg::DeleteBookmark(id)));
        outer.append(&delete);

        model.bookmarks_list.append(&outer);
    }
}

fn rebuild_words_list(model: &ReaderModel, sender: &ComponentSender<ReaderModel>) {
    while let Some(child) = model.words_list.first_child() {
        model.words_list.remove(&child);
    }

    if !model.dict_query.trim().is_empty() {
        if model.dict_results.is_empty() {
            append_reader_empty(
                &model.words_list,
                "No matches. Import dictionaries in Settings if needed.",
            );
            return;
        }
        for entry in model.dict_results.iter().take(40) {
            let btn = gtk::Button::new();
            btn.add_css_class("kalam-reader-word-row");
            let body = gtk::Box::new(gtk::Orientation::Vertical, 4);
            let title = gtk::Label::new(Some(&entry.word));
            title.add_css_class("kalam-reader-word-name");
            title.set_halign(gtk::Align::Start);
            title.set_xalign(0.0);
            body.append(&title);
            let def = gtk::Label::new(Some(&truncate_def(&entry.definition, 180)));
            def.add_css_class("kalam-reader-word-def");
            def.set_wrap(true);
            def.set_xalign(0.0);
            def.set_halign(gtk::Align::Start);
            body.append(&def);
            let meta = gtk::Label::new(Some("Dictionary match"));
            meta.add_css_class("kalam-reader-word-meta");
            meta.set_halign(gtk::Align::Start);
            meta.set_xalign(0.0);
            body.append(&meta);
            btn.set_child(Some(&body));
            let word = entry.word.clone();
            let s = sender.clone();
            btn.connect_clicked(move |_| s.input(ReaderMsg::DictSearchSelect(word.clone())));
            model.words_list.append(&btn);
        }
        return;
    }

    let words = model.filtered_saved_words();
    if words.is_empty() {
        append_reader_empty(&model.words_list, "No saved words in this view yet.");
        return;
    }

    for word in words.into_iter().take(150) {
        let btn = gtk::Button::new();
        btn.add_css_class("kalam-reader-word-row");
        let body = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let title = gtk::Label::new(Some(&word.word));
        title.add_css_class("kalam-reader-word-name");
        title.set_halign(gtk::Align::Start);
        title.set_xalign(0.0);
        body.append(&title);
        let def = gtk::Label::new(Some(&truncate_def(&word.definition, 180)));
        def.add_css_class("kalam-reader-word-def");
        def.set_wrap(true);
        def.set_xalign(0.0);
        def.set_halign(gtk::Align::Start);
        body.append(&def);
        let meta = gtk::Label::new(Some(&saved_word_meta(model, word)));
        meta.add_css_class("kalam-reader-word-meta");
        meta.set_halign(gtk::Align::Start);
        meta.set_xalign(0.0);
        body.append(&meta);
        btn.set_child(Some(&body));
        let text = word.word.clone();
        let s = sender.clone();
        btn.connect_clicked(move |_| s.input(ReaderMsg::DictSearchSelect(text.clone())));
        model.words_list.append(&btn);
    }
}

fn chapter_label(model: &ReaderModel, chapter_index: usize) -> String {
    if let Some(item) = model.open.spine.get(chapter_index) {
        item.title.clone()
    } else {
        format!("Ch {}", chapter_index + 1)
    }
}

fn color_bar_class(color: &str) -> &'static str {
    match HighlightColor::from_str_lossy(color) {
        HighlightColor::Yellow => "kalam-reader-bar-yellow",
        HighlightColor::Green => "kalam-reader-bar-green",
        HighlightColor::Blue => "kalam-reader-bar-blue",
        HighlightColor::Pink => "kalam-reader-bar-pink",
        HighlightColor::Orange => "kalam-reader-bar-orange",
    }
}

fn saved_word_meta(model: &ReaderModel, word: &SavedWord) -> String {
    match word.chapter_index {
        Some(ch) => format!("{} · saved word", chapter_label(model, ch as usize)),
        None => "Saved word".into(),
    }
}

fn load_chapter(model: &ReaderModel) {
    if model.open.chapter_count() == 0 {
        return;
    }
    match model
        .open
        .chapter_html(model.chapter, &model.css(), model.fraction)
    {
        Ok(html) => {
            let base = model.open.extract_dir.to_string_lossy();
            let base_uri = if base.starts_with('/') {
                format!("file://{base}/")
            } else {
                format!("file:///{}/", base.replace('\\', "/"))
            };
            model.webview.load_html(&html, Some(&base_uri));
        }
        Err(err) => {
            let err_html = format!(
                "<html><body style='padding:2rem;background:#f5f0e8;color:#2c2820;font-family:Georgia,serif'><h1>Could not load chapter</h1><pre>{err:#}</pre></body></html>"
            );
            model.webview.load_html(&err_html, None);
        }
    }
}

fn truncate_def(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n])
    }
}

fn eval_js(webview: &webkit6::WebView, script: &str) {
    webview.evaluate_javascript(script, None, None, None::<&gio::Cancellable>, |res| {
        if let Err(err) = res {
            eprintln!("kalam js eval error: {err}");
        }
    });
}

fn url_decode(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next();
            let h2 = chars.next();
            if let (Some(a), Some(b)) = (h1, h2) {
                let hex = format!("{a}{b}");
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    out.push(byte as char);
                    continue;
                }
                out.push('%');
                out.push(a);
                out.push(b);
            } else {
                out.push('%');
                if let Some(a) = h1 {
                    out.push(a);
                }
                if let Some(b) = h2 {
                    out.push(b);
                }
            }
        } else if c == '+' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

fn chrono_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
