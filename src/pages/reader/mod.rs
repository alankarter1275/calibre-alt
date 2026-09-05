//! Immersive EPUB reader module.

mod chapter;
mod chrome;
mod js_bridge;
mod lists;
mod mod_model;
mod panels;
mod session;
mod settings_panel;
mod types;
mod ui_prefs;

pub use mod_model::ReaderModel;
pub use types::ReaderOut;

use chapter::load_chapter;
use chrome::{
    connect_hover_zone, overlay_child_box, rebuild_cover_host, reader_sidebar_tab_content,
    sync_reader_controls, sync_reader_stage_theme, sync_reader_stacks, sync_sidebar_tabs,
    update_chrome_labels, update_sidebar_header,
};
use js_bridge::url_decode;
use lists::{rebuild_bookmarks_list, rebuild_highlights_list, rebuild_toc, rebuild_words_list};
use panels::{build_bookmarks_panel, build_highlights_panel, build_words_panel};
use settings_panel::build_reader_settings_panel;
use types::*;
use ui_prefs::{apply_reader_ui_prefs, register_reader_ui_provider, update_reader_ui_setting};

use crate::db::Catalog;
use crate::epub_book::{OpenBook, ReadingTheme};
use crate::models::Book;
use crate::paths::reader_cache_dir;
use crate::service::LibraryService;
use gtk::glib;
use gtk::prelude::*;
use relm4::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use webkit6::prelude::*;

fn report_errors(errors: &[String]) {
    for err in errors {
        crate::notify::error("Could not open this book properly", err);
    }
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
                add_css_class: "kalam-reader-back-dock",
                #[watch]
                set_visible: model.show_back_button,
                set_halign: gtk::Align::Start,
                set_valign: gtk::Align::Start,

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
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_halign: gtk::Align::Start,
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
        let service = LibraryService::new(catalog);
        let catalog = service.catalog();
        let snap = service.reader(book_id);
        report_errors(&snap.errors);
        let book = snap.book.clone();
        crate::timing::span("book_open");
        let webview = crate::webview_pool::acquire();

        let (book_meta, open, chapter, fraction) = if let Some(book) = book.clone() {
            let cache = reader_cache_dir(&book.uuid);
            match OpenBook::open(&book.file_path, &cache) {
                Ok(open) => {
                    crate::timing::span_end("book_open");
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
        let all_book_annotations = snap.annotations;
        let bookmarks = snap.bookmarks;
        let saved_words = snap.saved_words;

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
        let ui_prefs = ReaderUiPrefs::load(catalog);
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
            catalog.get_pref_i64("dict_sense_hint", 1) != 0,
            catalog.get_pref_i64("dict_history_enabled", 1) != 0,
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
            service,
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
            webview_handlers: None,
            chapter_annotations,
            all_book_annotations,
            annotation_search_query: String::new(),
            pending_annotation_jump: None,
            editing_annotation: None,
            annotation_note_draft: None,
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
            left_close_token: 0,
            right_close_token: 0,
            left_sidebar_shell: None,
            right_sidebar_shell: None,
            left_sidebar_box: None,
            right_sidebar_box: None,
            back_dock: None,
            bottom_dock: None,
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
            let _ = model.service.catalog().mark_book_opened(book_id);
            let start_pct = book.as_ref().map(|b| b.progress as i64).unwrap_or(0);
            model.session_start_pct = start_pct;
            model.session_start = std::time::Instant::now();
            model.session_id = model
                .service
                .catalog()
                .start_reading_session(book_id, start_pct)
                .ok();
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
        model.back_dock = overlay_child_box(&root, 4);
        model.bottom_dock = overlay_child_box(&root, 5);
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
        update_sidebar_header(&widgets, &model, &sender);
        sync_sidebar_tabs(&widgets, &model);
        apply_reader_ui_prefs(&model);
        sync_reader_stacks(&model);
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
        let title_handler = webview.connect_title_notify(move |wv| {
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
        let load_handler = webview.connect_load_changed(move |_wv, event| {
            if event == webkit6::LoadEvent::Finished {
                crate::timing::span_end("chapter_load");
                s.input(ReaderMsg::AnnotationsReload);
            }
        });

        let script_handler = webview.user_content_manager().map(|ucm| {
            let s = sender.clone();
            let id = ucm.connect_script_message_received(Some("kalam"), move |_mgr, msg| {
                s.input(ReaderMsg::JsRaw(msg.to_string()));
            });
            (ucm, id)
        });

        let s = sender.clone();
        let policy_handler = webview.connect_decide_policy(move |_wv, decision, decision_type| {
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

        model.webview_handlers = Some(WebViewHandlers {
            title: title_handler,
            load_changed: load_handler,
            decide_policy: policy_handler,
            script_message: script_handler,
        });

        if model.open.chapter_count() > 0 {
            load_chapter(&model);
            model.preload_next_chapter();
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
                self.close_annotation_editor();
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
                    self.flush_annotation_note_draft();
                    self.editing_annotation = None;
                    self.pending_annotation_jump = None;
                    self.go_chapter(idx, 0.0);
                    refresh_sidebar_header = true;
                    refresh_toc = true;
                    refresh_highlights = true;
                    refresh_bookmarks = true;
                    refresh_words = true;
                    refresh_chrome = true;
                }
            }
            ReaderMsg::ToggleAnnotation(id) => {
                let Some(annotation) = self
                    .all_book_annotations
                    .iter()
                    .find(|annotation| annotation.id == id)
                    .cloned()
                else {
                    return;
                };
                let idx = annotation.chapter_index as usize;
                if idx >= self.open.chapter_count() || self.loading {
                    return;
                }
                if self.editing_annotation == Some(annotation.id) {
                    self.flush_annotation_note_draft();
                    self.editing_annotation = None;
                    self.pending_annotation_jump = None;
                    refresh_highlights = true;
                    refresh_tabs = true;
                } else {
                    self.flush_annotation_note_draft();
                    self.right_tab = RightSidebarTab::Highlights;
                    self.right_sidebar_open = true;
                    self.cancel_right_close();
                    self.editing_annotation = Some(annotation.id);
                    self.pending_annotation_jump = Some(annotation.id);
                    if idx != self.chapter {
                        self.go_chapter(idx, 0.0);
                        refresh_sidebar_header = true;
                        refresh_toc = true;
                        refresh_bookmarks = true;
                        refresh_words = true;
                        refresh_chrome = true;
                    } else {
                        self.restore_pending_annotation();
                    }
                    refresh_tabs = true;
                    refresh_highlights = true;
                }
            }
            ReaderMsg::JumpToLocation(idx, frac) => {
                if idx < self.open.chapter_count() && !self.loading {
                    self.flush_annotation_note_draft();
                    self.editing_annotation = None;
                    self.pending_annotation_jump = None;
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
                    self.flush_annotation_note_draft();
                    self.editing_annotation = None;
                    self.pending_annotation_jump = None;
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
                    self.flush_annotation_note_draft();
                    self.editing_annotation = None;
                    self.pending_annotation_jump = None;
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
                self.close_annotation_editor();
                self.theme = theme;
                self.service
                    .catalog()
                    .set_pref("reader.theme", theme.as_str());
                self.loading = true;
                load_chapter(self);
                self.loading = false;
                refresh_controls = true;
                refresh_stage = true;
            }
            ReaderMsg::FontDelta(delta) => {
                let next = (self.font_px as i32 + delta).clamp(13, 24) as u32;
                if next != self.font_px {
                    self.close_annotation_editor();
                    self.font_px = next;
                    self.service
                        .catalog()
                        .set_pref("reader.font_px", &next.to_string());
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
                    self.close_annotation_editor();
                    self.line_height = next;
                    self.service
                        .catalog()
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
                    self.close_annotation_editor();
                    self.column_px = next;
                    self.service
                        .catalog()
                        .set_pref("reader.column_px", &next.to_string());
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
            ReaderMsg::SetDictSenseHint(on) => {
                self.service
                    .catalog()
                    .set_pref("dict_sense_hint", if on { "1" } else { "0" });
            }
            ReaderMsg::SetDictHistory(on) => {
                self.service
                    .catalog()
                    .set_pref("dict_history_enabled", if on { "1" } else { "0" });
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
                self.flush_annotation_note_draft();
                self.reload_annotations();
                self.reload_bookmarks();
                self.reload_saved_words();
                self.inject_highlights();
                self.restore_pending_annotation();
                refresh_highlights = true;
                refresh_bookmarks = true;
                refresh_words = true;
            }
            ReaderMsg::RecolorAnnotation(id, color) => {
                self.flush_annotation_note_draft();
                let color_name = color.as_str();
                let is_highlight = self
                    .all_book_annotations
                    .iter()
                    .find(|annotation| annotation.id == id)
                    .is_some_and(|annotation| annotation.kind == "highlight");
                if !is_highlight {
                    return;
                }
                let unchanged = self
                    .all_book_annotations
                    .iter()
                    .find(|annotation| annotation.id == id)
                    .is_some_and(|annotation| annotation.color.eq_ignore_ascii_case(color_name));
                if unchanged {
                    return;
                }
                match self
                    .service
                    .catalog()
                    .update_annotation_color(id, color_name)
                {
                    Ok(()) => {
                        for annotation in &mut self.all_book_annotations {
                            if annotation.id == id {
                                annotation.color = color_name.to_string();
                            }
                        }
                        for annotation in &mut self.chapter_annotations {
                            if annotation.id == id {
                                annotation.color = color_name.to_string();
                            }
                        }
                        let script = format!(
                            "if (window.kalamRecolorHighlight) window.kalamRecolorHighlight({}, '{}');",
                            id, color_name
                        );
                        js_bridge::eval_js(&self.webview, &script);
                        refresh_highlights = true;
                    }
                    Err(err) => {
                        crate::notify::error("Could not recolor the highlight", &err.to_string());
                    }
                }
            }
            ReaderMsg::AnnotationSearchChanged(query) => {
                self.close_annotation_editor();
                let query = query.trim().to_string();
                if query != self.annotation_search_query {
                    self.annotation_search_query = query;
                    refresh_highlights = true;
                }
            }
            ReaderMsg::DeleteAnnotation(id) => {
                self.flush_annotation_note_draft();
                crate::notify::report(
                    self.service.catalog().delete_annotation(id),
                    "Could not delete the highlight",
                );
                if self.editing_annotation == Some(id) {
                    self.editing_annotation = None;
                    self.annotation_note_draft = None;
                }
                self.reload_annotations();
                let script = format!(
                    "if (window.kalamRemoveHighlight) window.kalamRemoveHighlight('{}');",
                    id
                );
                js_bridge::eval_js(&self.webview, &script);
                refresh_highlights = true;
            }
            ReaderMsg::AnnotationNoteChanged(id, note) => {
                if self.editing_annotation == Some(id) {
                    self.annotation_note_draft = Some((id, note));
                }
            }
            ReaderMsg::SaveAnnotationNote(id, note) => {
                if self.persist_annotation_note(id, &note)
                    && matches!(
                        self.annotation_note_draft.as_ref(),
                        Some((draft_id, _)) if *draft_id == id
                    )
                {
                    self.annotation_note_draft = None;
                }
            }
            ReaderMsg::DeleteBookmark(id) => {
                crate::notify::report(
                    self.service.catalog().delete_reading_bookmark(id),
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
                    self.dict_results = self.lookup_dict(&q, 30);
                }
                refresh_words = true;
            }
            ReaderMsg::DictSearchSelect(word) => {
                let data = self
                    .service
                    .catalog()
                    .lookup_entry(&word)
                    .unwrap_or_else(|_| crate::db::EntryData {
                        word: word.clone(),
                        ..Default::default()
                    });
                self.dict_lookup_word = Some(data.word.clone());
                self.dict_lookup_def = data
                    .senses
                    .first()
                    .map(|s| s.def.clone())
                    .or_else(|| (!data.suggestions.is_empty()).then(|| data.word.clone()));
                self.show_dict_in_webview(&word, None, None);
                self.right_tab = RightSidebarTab::Words;
                self.right_sidebar_open = true;
                refresh_tabs = true;
            }
            ReaderMsg::SaveCurrentWord => {
                if let (Some(word), Some(def)) = (&self.dict_lookup_word, &self.dict_lookup_def) {
                    let saved = self.service.catalog().insert_saved_word(
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
                js_bridge::eval_js(
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
                    match self.service.catalog().insert_reading_bookmark(
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
            ReaderMsg::OpenAuthor(name) => {
                sender.output(ReaderOut::OpenAuthor { name }).ok();
            }
            ReaderMsg::OpenLeftSidebar => {
                self.close_annotation_editor();
                self.cancel_left_close();
                self.right_sidebar_open = false;
                self.cancel_right_close();
                if self.left_tab == LeftSidebarTab::Toc {
                    self.position_toc_scroll();
                }
                self.left_sidebar_open = true;
                refresh_tabs = true;
            }
            ReaderMsg::OpenRightSidebar => {
                self.cancel_right_close();
                self.left_sidebar_open = false;
                self.cancel_left_close();
                self.right_sidebar_open = true;
                refresh_tabs = true;
            }
            ReaderMsg::SwitchLeftTab(tab) => {
                self.close_annotation_editor();
                self.left_tab = tab;
                self.cancel_left_close();
                self.right_sidebar_open = false;
                self.cancel_right_close();
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
                if !matches!(tab, RightSidebarTab::Highlights) {
                    self.close_annotation_editor();
                }
                self.right_tab = tab;
                self.right_sidebar_open = true;
                self.cancel_right_close();
                self.left_sidebar_open = false;
                self.cancel_left_close();
                refresh_tabs = true;
                match tab {
                    RightSidebarTab::Highlights => refresh_highlights = true,
                    RightSidebarTab::Bookmarks => refresh_bookmarks = true,
                    RightSidebarTab::Words => refresh_words = true,
                }
            }
            ReaderMsg::SetHighlightFilter(filter) => {
                self.close_annotation_editor();
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
            ReaderMsg::ForceCloseLeft(token) => {
                if token == self.left_close_token {
                    self.left_sidebar_open = false;
                    self.left_close_timer = None;
                    refresh_tabs = true;
                }
            }
            ReaderMsg::ForceCloseRight(token) => {
                if token == self.right_close_token {
                    self.close_annotation_editor();
                    self.right_sidebar_open = false;
                    self.right_close_timer = None;
                    refresh_tabs = true;
                }
            }
            ReaderMsg::CloseSidebars => {
                self.close_annotation_editor();
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
            update_sidebar_header(widgets, self, &sender);
        }
        if refresh_chrome {
            update_chrome_labels(widgets, self);
        }
        if refresh_controls || refresh_tabs {
            sync_reader_stacks(self);
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
        self.close_annotation_editor();
        self.save_progress();
        self.close_session();
        self.cancel_left_close();
        self.cancel_right_close();
        self.webview.stop_loading();

        if let Some(handlers) = self.webview_handlers.take() {
            self.webview.disconnect(handlers.title);
            self.webview.disconnect(handlers.load_changed);
            self.webview.disconnect(handlers.decide_policy);
            if let Some((ucm, id)) = handlers.script_message {
                ucm.disconnect(id);
            }
            crate::webview_pool::release(self.webview.clone());
        }
    }
}

impl ReaderModel {
    pub(crate) fn cancel_left_close(&mut self) {
        self.left_close_token = self.left_close_token.wrapping_add(1);
        self.left_close_timer = None;
    }

    pub(crate) fn cancel_right_close(&mut self) {
        self.right_close_token = self.right_close_token.wrapping_add(1);
        self.right_close_timer = None;
    }

    pub(crate) fn schedule_left_close(&mut self, sender: ComponentSender<Self>) {
        self.cancel_left_close();
        let token = self.left_close_token;
        let tx = sender.input_sender().clone();
        self.left_close_timer = Some(glib::timeout_add_local(
            Duration::from_millis(320),
            move || {
                let _ = tx.send(ReaderMsg::ForceCloseLeft(token));
                glib::ControlFlow::Break
            },
        ));
    }

    pub(crate) fn schedule_right_close(&mut self, sender: ComponentSender<Self>) {
        self.cancel_right_close();
        let token = self.right_close_token;
        let tx = sender.input_sender().clone();
        self.right_close_timer = Some(glib::timeout_add_local(
            Duration::from_millis(320),
            move || {
                let _ = tx.send(ReaderMsg::ForceCloseRight(token));
                glib::ControlFlow::Break
            },
        ));
    }
}
