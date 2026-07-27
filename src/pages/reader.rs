//! Immersive EPUB reader — P3 with highlights, quotes, offline dictionary.

use crate::db::{Annotation, Catalog, HighlightColor};
use crate::epub_book::{reading_css, OpenBook, ReadingTheme};
use crate::paths::reader_cache_dir;
use gtk::prelude::*;
use relm4::prelude::*;
use serde::Deserialize;
use std::rc::Rc;
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

#[derive(Debug)]
#[allow(dead_code)]
pub enum ReaderMsg {
    Close,
    TocSelect(usize),
    PrevChapter,
    NextChapter,
    Theme(ReadingTheme),
    FontDelta(i32),
    JsRaw(String),
    Progress(f64),
    AnnotationsReload,
    DeleteAnnotation(i64),
    JumpToChapter(usize),
    DictSearch(String),
    DictSearchSelect(String),
    SaveCurrentWord,
    ClearDict,
    ToggleAnnoPopover,
    ToggleDictPopover,
}

pub struct ReaderModel {
    catalog: Rc<Catalog>,
    book_id: i64,
    book_title: String,
    open: OpenBook,
    chapter: usize,
    fraction: f64,
    theme: ReadingTheme,
    font_px: u32,
    line_height: f32,
    margin_em: f32,
    loading: bool,
    webview: webkit6::WebView,
    chapter_annotations: Vec<Annotation>,
    all_book_annotations: Vec<Annotation>,
    dict_query: String,
    dict_results: Vec<crate::db::DictEntry>,
    dict_lookup_word: Option<String>,
    dict_lookup_def: Option<String>,
    dict_lookup_rect_json: Option<String>,
    dict_context: Option<String>,
    last_selection: Option<String>,
    /// P4: open reading session row + when it started, for time tracking.
    session_id: Option<i64>,
    session_start: std::time::Instant,
    session_start_pct: i64,
}

#[relm4::component(pub)]
impl Component for ReaderModel {
    type Init = (Rc<Catalog>, i64);
    type Input = ReaderMsg;
    type Output = ReaderOut;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Overlay {
            add_css_class: "kalam-reader",
            set_hexpand: true,
            set_vexpand: true,

            #[wrap(Some)]
            set_child = &gtk::Box {
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

            add_overlay = &gtk::Box {
                set_halign: gtk::Align::Start,
                set_valign: gtk::Align::Start,
                set_margin_top: 14,
                set_margin_start: 14,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    add_css_class: "kalam-reader-top-float",
                    set_spacing: 6,

                    gtk::Button {
                        set_label: "✕",
                        add_css_class: "kalam-reader-pill-btn",
                        set_tooltip_text: Some("Back (Esc)"),
                        connect_clicked => ReaderMsg::Close,
                    },

                    #[name = "crumb_label"]
                    gtk::Label {
                        add_css_class: "kalam-reader-crumb",
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                        set_max_width_chars: 36,
                    },
                },
            },

            add_overlay = &gtk::Box {
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::End,
                set_margin_bottom: 18,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    add_css_class: "kalam-reader-pill",
                    set_spacing: 2,

                    gtk::Button {
                        set_label: "‹",
                        add_css_class: "kalam-reader-pill-btn",
                        set_tooltip_text: Some("Previous chapter (P)"),
                        connect_clicked => ReaderMsg::PrevChapter,
                    },

                    #[name = "toc_btn"]
                    gtk::MenuButton {
                        set_label: "☰",
                        add_css_class: "kalam-reader-pill-btn",
                        set_tooltip_text: Some("Contents (T)"),
                        set_direction: gtk::ArrowType::Up,
                    },

                    #[name = "anno_btn"]
                    gtk::MenuButton {
                        set_label: "✎",
                        add_css_class: "kalam-reader-pill-btn",
                        set_tooltip_text: Some("Highlights & quotes"),
                        set_direction: gtk::ArrowType::Up,
                    },

                    #[name = "chapter_label"]
                    gtk::Label {
                        add_css_class: "kalam-reader-pill-meta",
                        set_width_chars: 7,
                    },

                    #[name = "dict_btn"]
                    gtk::MenuButton {
                        set_label: "Aa",
                        add_css_class: "kalam-reader-pill-btn",
                        set_tooltip_text: Some("Dictionary & typography (D)"),
                        set_direction: gtk::ArrowType::Up,
                    },

                    gtk::Button {
                        set_label: "›",
                        add_css_class: "kalam-reader-pill-btn",
                        set_tooltip_text: Some("Next chapter (N)"),
                        connect_clicked => ReaderMsg::NextChapter,
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

        let (book_title, open, chapter, fraction) = if let Some(book) = book {
            let cache = reader_cache_dir(&book.uuid);
            match OpenBook::open(&book.file_path, &cache) {
                Ok(open) => {
                    let (ch, frac) = catalog
                        .get_reading_progress(book_id)
                        .ok()
                        .flatten()
                        .unwrap_or((0, 0.0));
                    let ch = ch.min(open.chapter_count().saturating_sub(1));
                    (book.title, open, ch, frac)
                }
                Err(err) => {
                    eprintln!("kalam: open epub failed: {err:#}");
                    let msg = format!(
                        "<html><body style='padding:2rem;background:#f4ecd8;\
color:#3e3226;font-family:Georgia,serif'>\
<h1>Could not open book</h1><pre>{err:#}</pre>\
<p>Press Esc to go back.</p></body></html>"
                    );
                    webview.load_html(&msg, None);
                    (book.title, OpenBook::empty_placeholder(), 0, 0.0)
                }
            }
        } else {
            ("Missing book".into(), OpenBook::empty_placeholder(), 0, 0.0)
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

        let catalog_theme = catalog
            .get_pref("reader.theme")
            .map(|v| ReadingTheme::from_str_lossy(&v))
            .unwrap_or(ReadingTheme::Sepia);
        let catalog_font = catalog.get_pref_i64("reader.font_px", 19).clamp(14, 36) as u32;

        let model = ReaderModel {
            catalog,
            book_id,
            book_title,
            open,
            chapter,
            fraction,
            // Restored from app_prefs so the reader reopens the way it was left.
            theme: catalog_theme,
            font_px: catalog_font,
            line_height: 1.65,
            margin_em: 1.4,
            loading: false,
            webview: webview.clone(),
            chapter_annotations,
            all_book_annotations,
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
        };

        let mut model = model;
        // P4: history + time tracking. Only for books that actually opened —
        // a failed EPUB shouldn't pollute History or the reading stats.
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
        let model = model;

        let widgets = view_output!();
        widgets.web_host.append(&webview);
        update_chrome_labels(&widgets, &model);

        // TOC popover
        let toc_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
        build_toc(&toc_box, &model.open, &sender);
        let toc_scroll = gtk::ScrolledWindow::builder()
            .min_content_height(300)
            .min_content_width(280)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&toc_box)
            .build();
        let toc_wrap = gtk::Box::new(gtk::Orientation::Vertical, 6);
        toc_wrap.set_margin_all(10);
        let toc_title = gtk::Label::new(Some("Contents"));
        toc_title.add_css_class("kalam-reader-popover-title");
        toc_title.set_halign(gtk::Align::Start);
        toc_wrap.append(&toc_title);
        toc_wrap.append(&toc_scroll);
        let toc_pop = gtk::Popover::new();
        toc_pop.add_css_class("kalam-reader-popover");
        toc_pop.set_child(Some(&toc_wrap));
        toc_pop.set_position(gtk::PositionType::Top);
        widgets.toc_btn.set_popover(Some(&toc_pop));

        // Annotations popover
        let anno_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
        anno_box.set_margin_all(12);
        anno_box.set_hexpand(true);
        anno_box.set_size_request(460, -1);
        let anno_title = gtk::Label::new(Some("Highlights & quotes"));
        anno_title.add_css_class("kalam-reader-popover-title");
        anno_title.set_halign(gtk::Align::Start);
        anno_box.append(&anno_title);

        let anno_scroll = gtk::ScrolledWindow::builder()
            .min_content_height(360)
            .min_content_width(425)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .hexpand(true)
            .vexpand(false)
            .build();
        let anno_list = gtk::Box::new(gtk::Orientation::Vertical, 8);
        anno_list.set_margin_all(4);
        anno_list.set_hexpand(true);
        anno_list.set_size_request(410, -1);
        anno_scroll.set_child(Some(&anno_list));
        anno_box.append(&anno_scroll);

        let anno_pop = gtk::Popover::new();
        anno_pop.add_css_class("kalam-reader-popover");
        anno_pop.set_child(Some(&anno_box));
        anno_pop.set_position(gtk::PositionType::Top);
        anno_pop.set_size_request(460, -1);
        unsafe {
            anno_pop.set_data("kalam-anno-list", anno_list.clone());
        }
        widgets.anno_btn.set_popover(Some(&anno_pop));

        let _anno_list_clone = anno_list.clone();
        gtk::glib::idle_add_local_once(move || {
            let _ = _anno_list_clone;
        });

        // Aa / Dictionary popover
        let aa_wrap = gtk::Box::new(gtk::Orientation::Vertical, 10);
        aa_wrap.set_margin_all(12);

        let size_l = gtk::Label::new(Some("TEXT SIZE"));
        size_l.add_css_class("kalam-reader-popover-title");
        size_l.set_halign(gtk::Align::Start);
        aa_wrap.append(&size_l);
        let size_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        size_row.set_halign(gtk::Align::Fill);
        let a_minus = gtk::Button::with_label("A−");
        a_minus.add_css_class("kalam-reader-pill-btn");
        let s1 = sender.clone();
        a_minus.connect_clicked(move |_| s1.input(ReaderMsg::FontDelta(-1)));

        // Current size, so the control reports state instead of just changing it.
        let font_size_label = gtk::Label::new(Some(&format!("{}px", model.font_px)));
        font_size_label.add_css_class("kalam-reader-size-value");
        font_size_label.set_hexpand(true);

        let a_plus = gtk::Button::with_label("A+");
        a_plus.add_css_class("kalam-reader-pill-btn");
        let s2 = sender.clone();
        a_plus.connect_clicked(move |_| s2.input(ReaderMsg::FontDelta(1)));
        size_row.append(&a_minus);
        size_row.append(&font_size_label);
        size_row.append(&a_plus);
        aa_wrap.append(&size_row);

        let theme_l = gtk::Label::new(Some("THEME"));
        theme_l.add_css_class("kalam-reader-popover-title");
        theme_l.set_halign(gtk::Align::Start);
        aa_wrap.append(&theme_l);
        // Each button is painted in the colours it applies, so the choice is
        // visible at a glance; the active one carries a tick.
        let theme_row = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let mut theme_buttons = Vec::new();
        for (label, theme) in [
            ("Light", ReadingTheme::Light),
            ("Sepia", ReadingTheme::Sepia),
            ("Dark", ReadingTheme::Dark),
        ] {
            let b = gtk::Button::new();

            let inner = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let tick = gtk::Label::new(Some("✓"));
            tick.add_css_class("kalam-theme-tick");
            tick.set_width_chars(1);
            let name = gtk::Label::new(Some(label));
            name.set_halign(gtk::Align::Start);
            name.set_hexpand(true);
            inner.append(&tick);
            inner.append(&name);
            b.set_child(Some(&inner));

            b.add_css_class("kalam-reader-theme-btn");
            // Swatch colours live in the global stylesheet (style.rs) keyed by
            // theme name, so no per-open CssProvider is registered.
            b.add_css_class(&format!("kalam-theme-{}", theme.as_str()));

            let s = sender.clone();
            b.connect_clicked(move |_| s.input(ReaderMsg::Theme(theme)));
            theme_row.append(&b);
            theme_buttons.push((theme, tick));
        }
        aa_wrap.append(&theme_row);

        let dict_l = gtk::Label::new(Some("DICTIONARY"));
        dict_l.add_css_class("kalam-reader-popover-title");
        dict_l.set_halign(gtk::Align::Start);
        dict_l.set_margin_top(8);
        aa_wrap.append(&dict_l);

        let dict_search_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let dict_entry = gtk::SearchEntry::new();
        dict_entry.set_placeholder_text(Some("Search word…"));
        dict_entry.set_hexpand(true);
        let s = sender.clone();
        dict_entry.connect_search_changed(move |e| {
            s.input(ReaderMsg::DictSearch(e.text().to_string()));
        });
        dict_search_box.append(&dict_entry);
        aa_wrap.append(&dict_search_box);

        let dict_scroll = gtk::ScrolledWindow::builder()
            .min_content_height(160)
            .min_content_width(300)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        let dict_list = gtk::Box::new(gtk::Orientation::Vertical, 6);
        dict_scroll.set_child(Some(&dict_list));
        aa_wrap.append(&dict_scroll);

        let dict_res_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        dict_res_box.set_margin_top(6);
        let dict_res_label = gtk::Label::new(None);
        dict_res_label.set_wrap(true);
        dict_res_label.set_xalign(0.0);
        dict_res_label.add_css_class("kalam-muted");
        dict_res_box.append(&dict_res_label);

        let dict_actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let save_word_btn = gtk::Button::with_label("Save word");
        save_word_btn.add_css_class("kalam-secondary-btn");
        let s = sender.clone();
        save_word_btn.connect_clicked(move |_| s.input(ReaderMsg::SaveCurrentWord));
        dict_actions.append(&save_word_btn);
        let clear_btn = gtk::Button::with_label("Clear");
        clear_btn.add_css_class("kalam-secondary-btn");
        let s = sender.clone();
        clear_btn.connect_clicked(move |_| s.input(ReaderMsg::ClearDict));
        dict_actions.append(&clear_btn);
        dict_res_box.append(&dict_actions);
        aa_wrap.append(&dict_res_box);

        let aa_pop = gtk::Popover::new();
        aa_pop.add_css_class("kalam-reader-popover");
        aa_pop.set_child(Some(&aa_wrap));
        aa_pop.set_position(gtk::PositionType::Top);
        unsafe {
            aa_pop.set_data("kalam-dict-list", dict_list.clone());
            aa_pop.set_data("kalam-dict-res", dict_res_label.clone());
            aa_pop.set_data("kalam-font-size", font_size_label.clone());
            for (theme, tick) in &theme_buttons {
                aa_pop.set_data(theme_tick_key(*theme), tick.clone());
            }
        }
        widgets.dict_btn.set_popover(Some(&aa_pop));
        // Reflect the restored theme on first open.
        refresh_theme_buttons(&widgets, model.theme);

        // Title notify fallback
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

        // Load changed
        let s = sender.clone();
        webview.connect_load_changed(move |_wv, event| {
            if event == webkit6::LoadEvent::Finished {
                s.input(ReaderMsg::AnnotationsReload);
            }
        });

        // Script message handler
        if let Some(ucm) = webview.user_content_manager() {
            let _ = ucm.register_script_message_handler("kalam", None);
            let s = sender.clone();
            ucm.connect_script_message_received(Some("kalam"), move |_mgr, msg| {
                let js = msg.to_string();
                s.input(ReaderMsg::JsRaw(js));
            });
        }

        // Decide policy fallback for kalam://
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
                Key::d | Key::D => {
                    s.input(ReaderMsg::DictSearch(String::new()));
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
        match msg {
            ReaderMsg::Close => {
                if self.fraction < 0.05 {
                    self.fraction = 0.15;
                }
                self.save_progress();
                sender.output(ReaderOut::Close).ok();
            }
            ReaderMsg::TocSelect(idx) => {
                if idx < self.open.chapter_count() && idx != self.chapter && !self.loading {
                    widgets.toc_btn.popdown();
                    self.go_chapter(idx, 0.0);
                }
            }
            ReaderMsg::PrevChapter => {
                if self.chapter > 0 && !self.loading {
                    self.go_chapter(self.chapter - 1, 0.0);
                }
            }
            ReaderMsg::NextChapter => {
                if self.chapter + 1 < self.open.chapter_count() && !self.loading {
                    self.fraction = 1.0;
                    self.go_chapter(self.chapter + 1, 0.0);
                }
            }
            ReaderMsg::Theme(t) => {
                self.theme = t;
                self.catalog.set_pref("reader.theme", t.as_str());
                refresh_theme_buttons(widgets, t);
                widgets.dict_btn.popdown();
                self.loading = true;
                load_chapter(self);
                self.loading = false;
            }
            ReaderMsg::FontDelta(d) => {
                let next = (self.font_px as i32 + d).clamp(14, 36) as u32;
                if next != self.font_px {
                    self.font_px = next;
                    self.catalog.set_pref("reader.font_px", &next.to_string());
                    set_font_size_label(widgets, next);
                    self.loading = true;
                    load_chapter(self);
                    self.loading = false;
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
                    self.handle_js_payload(payload, sender.clone());
                } else if let Ok(payload) = serde_json::from_str::<JsPayload>(&json_part) {
                    self.handle_js_payload(payload, sender.clone());
                } else if let Ok(f) = decoded.parse::<f64>() {
                    if (0.0..=1.0).contains(&f) {
                        self.fraction = f;
                    }
                } else if decoded.starts_with("progress/") {
                    if let Some(num) = decoded.strip_prefix("progress/") {
                        if let Ok(f) = num.parse::<f64>() {
                            self.fraction = f;
                        }
                    }
                }
            }
            ReaderMsg::Progress(frac) => {
                self.fraction = frac.clamp(0.0, 1.0);
            }
            ReaderMsg::AnnotationsReload => {
                self.chapter_annotations = self
                    .catalog
                    .get_annotations_for_chapter(self.book_id, self.chapter as i64)
                    .unwrap_or_default();
                self.all_book_annotations = self
                    .catalog
                    .get_annotations_for_book(self.book_id)
                    .unwrap_or_default();
                self.inject_highlights();
                if let Some(pop) = widgets.anno_btn.popover() {
                    if let Some(pop) = pop.downcast_ref::<gtk::Popover>() {
                        unsafe {
                            if let Some(list) = pop.data::<gtk::Box>("kalam-anno-list") {
                                let list = list.as_ref().clone();
                                rebuild_anno_list(&list, &self.all_book_annotations, &sender);
                            }
                        }
                    }
                }
            }
            ReaderMsg::DeleteAnnotation(id) => {
                let _ = self.catalog.delete_annotation(id);
                self.chapter_annotations = self
                    .catalog
                    .get_annotations_for_chapter(self.book_id, self.chapter as i64)
                    .unwrap_or_default();
                self.all_book_annotations = self
                    .catalog
                    .get_annotations_for_book(self.book_id)
                    .unwrap_or_default();
                let script = format!(
                    "if (window.kalamRemoveHighlight) window.kalamRemoveHighlight('{}');",
                    id
                );
                eval_js(&self.webview, &script);
                if let Some(pop) = widgets.anno_btn.popover() {
                    if let Some(pop) = pop.downcast_ref::<gtk::Popover>() {
                        unsafe {
                            if let Some(list) = pop.data::<gtk::Box>("kalam-anno-list") {
                                let list = list.as_ref().clone();
                                rebuild_anno_list(&list, &self.all_book_annotations, &sender);
                            }
                        }
                    }
                }
            }
            ReaderMsg::JumpToChapter(idx) => {
                if idx < self.open.chapter_count() {
                    self.go_chapter(idx, 0.0);
                    widgets.anno_btn.popdown();
                }
            }
            ReaderMsg::DictSearch(q) => {
                self.dict_query = q.clone();
                if q.trim().is_empty() {
                    self.dict_results.clear();
                } else {
                    self.dict_results = self.catalog.search_dict(&q, 30).unwrap_or_default();
                }
                if let Some(pop) = widgets.dict_btn.popover() {
                    if let Some(pop) = pop.downcast_ref::<gtk::Popover>() {
                        unsafe {
                            if let Some(list) = pop.data::<gtk::Box>("kalam-dict-list") {
                                let list = list.as_ref().clone();
                                rebuild_dict_list(&list, &self.dict_results, &sender);
                            }
                        }
                    }
                }
            }
            ReaderMsg::DictSearchSelect(word) => {
                let results = self.catalog.search_dict(&word, 5).unwrap_or_default();
                if let Some(entry) = results.first() {
                    self.dict_lookup_word = Some(entry.word.clone());
                    self.dict_lookup_def = Some(entry.definition.clone());
                    let rect_json = self.dict_lookup_rect_json.clone();
                    self.show_dict_in_webview(
                        entry.word.clone(),
                        entry.definition.clone(),
                        rect_json.clone(),
                    );
                    if let Some(pop) = widgets.dict_btn.popover() {
                        if let Some(pop) = pop.downcast_ref::<gtk::Popover>() {
                            unsafe {
                                if let Some(label) = pop.data::<gtk::Label>("kalam-dict-res") {
                                    label.as_ref().set_label(&format!(
                                        "{}: {}",
                                        entry.word,
                                        truncate_def(&entry.definition, 400)
                                    ));
                                }
                            }
                        }
                    }
                }
                widgets.dict_btn.popdown();
            }
            ReaderMsg::SaveCurrentWord => {
                if let (Some(w), Some(def)) = (&self.dict_lookup_word, &self.dict_lookup_def) {
                    let _ = self.catalog.insert_saved_word(
                        w,
                        def,
                        None,
                        Some(self.book_id),
                        Some(self.chapter as i64),
                        self.dict_context.as_deref(),
                    );
                    self.dict_lookup_word = None;
                    self.dict_lookup_def = None;
                    if let Some(pop) = widgets.dict_btn.popover() {
                        if let Some(pop) = pop.downcast_ref::<gtk::Popover>() {
                            unsafe {
                                if let Some(label) = pop.data::<gtk::Label>("kalam-dict-res") {
                                    label.as_ref().set_label("Word saved to Saved words.");
                                }
                            }
                        }
                    }
                }
            }
            ReaderMsg::ClearDict => {
                self.dict_lookup_word = None;
                self.dict_lookup_def = None;
                self.dict_lookup_rect_json = None;
                self.dict_context = None;
                if let Some(pop) = widgets.dict_btn.popover() {
                    if let Some(pop) = pop.downcast_ref::<gtk::Popover>() {
                        unsafe {
                            if let Some(label) = pop.data::<gtk::Label>("kalam-dict-res") {
                                label.as_ref().set_label("");
                            }
                        }
                    }
                }
                eval_js(
                    &self.webview,
                    "if (window.kalamHideDict) window.kalamHideDict();",
                );
            }
            ReaderMsg::ToggleAnnoPopover => {
                widgets.anno_btn.popup();
            }
            ReaderMsg::ToggleDictPopover => {
                widgets.dict_btn.popup();
            }
        }

        update_chrome_labels(widgets, self);
        self.update_view(widgets, sender);
    }

    fn shutdown(&mut self, widgets: &mut Self::Widgets, _output: relm4::Sender<Self::Output>) {
        self.save_progress();
        self.close_session();

        // Popovers are their own toplevel surfaces, so they are *not* disposed
        // along with the MenuButton that owns them. Leaving them attached while
        // the reader is torn down mid-navigation makes GTK probe a half-disposed
        // widget later — the `gtk_widget_is_ancestor: assertion 'GTK_IS_WIDGET
        // (widget)' failed` criticals on the console. Pop them down and detach.
        for btn in [&widgets.toc_btn, &widgets.anno_btn, &widgets.dict_btn] {
            btn.popdown();
            btn.set_popover(None::<&gtk::Popover>);
        }

        // Stop the WebView before its widget goes away: an in-flight load that
        // completes after disposal fires callbacks against dead widgets.
        self.webview.stop_loading();
    }
}

impl ReaderModel {
    fn css(&self) -> String {
        reading_css(self.theme, self.font_px, self.line_height, self.margin_em)
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
        // P4: crossing the end auto-marks the book finished (once).
        let pct = self.progress_pct();
        let _ = self.catalog.auto_finish_if_complete(self.book_id, pct);
    }

    fn progress_pct(&self) -> i64 {
        let count = self.open.chapter_count();
        if count == 0 {
            return 0;
        }
        let overall = ((self.chapter as f64) + self.fraction) / (count as f64) * 100.0;
        overall.round().clamp(0.0, 100.0) as i64
    }

    /// Close the open reading-session row. Idempotent: called from shutdown,
    /// and the id is cleared so a second call is a no-op.
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
        self.fraction = frac;
        self.loading = true;
        self.chapter_annotations = self
            .catalog
            .get_annotations_for_chapter(self.book_id, idx as i64)
            .unwrap_or_default();
        load_chapter(self);
        self.loading = false;
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
        let rect_part = if let Some(rj) = rect_json {
            let rj_esc = rj.replace('\\', "\\\\").replace('\'', "\\'");
            format!("'{}'", rj_esc)
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
                            "try {{ var nodes = document.querySelectorAll('span[data-annotation-id=\"{tmp}\"]'); nodes.forEach(function(n){{ n.dataset.annotationId='{real}'; }}); }}catch(e){{}}",
                            tmp = tmp_id.replace('\'', "\\'"),
                            real = real_id
                        );
                        eval_js(&self.webview, &script);
                        self.chapter_annotations = self
                            .catalog
                            .get_annotations_for_chapter(self.book_id, self.chapter as i64)
                            .unwrap_or_default();
                        self.all_book_annotations = self
                            .catalog
                            .get_annotations_for_book(self.book_id)
                            .unwrap_or_default();
                        sender.input(ReaderMsg::AnnotationsReload);
                    }
                    Err(e) => eprintln!("kalam: insert highlight failed: {e}"),
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
                if self
                    .catalog
                    .insert_annotation(
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
                    )
                    .is_ok()
                {
                    self.all_book_annotations = self
                        .catalog
                        .get_annotations_for_book(self.book_id)
                        .unwrap_or_default();
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
                    let _ = self.catalog.insert_saved_word(
                        &word,
                        &def,
                        None,
                        Some(self.book_id),
                        Some(self.chapter as i64),
                        payload.context.as_deref().or(self.dict_context.as_deref()),
                    );
                }
            }
            "dict-shortcut" => {
                sender.input(ReaderMsg::ToggleDictPopover);
            }
            _ => {}
        }
    }
}

fn update_chrome_labels(widgets: &ReaderModelWidgets, model: &ReaderModel) {
    if model.open.chapter_count() == 0 {
        widgets.crumb_label.set_label(&model.book_title);
        widgets.chapter_label.set_label("—");
        return;
    }
    widgets.crumb_label.set_label(&format!(
        "{} · {}",
        model.book_title, model.open.spine[model.chapter].title
    ));
    widgets.chapter_label.set_label(&format!(
        "{}/{}",
        model.chapter + 1,
        model.open.chapter_count()
    ));
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
                "<html><body style='padding:2rem;background:#f4ecd8;\
color:#3e3226;font-family:Georgia,serif'>\
<h1>Could not load chapter</h1><pre>{err:#}</pre></body></html>"
            );
            model.webview.load_html(&err_html, None);
        }
    }
}

fn build_toc(list: &gtk::Box, open: &OpenBook, sender: &ComponentSender<ReaderModel>) {
    while let Some(c) = list.first_child() {
        list.remove(&c);
    }
    if open.toc.is_empty() {
        for (i, item) in open.spine.iter().enumerate() {
            append_toc_btn(list, &item.title, i, sender);
        }
    } else {
        for entry in &open.toc {
            if let Some(idx) = entry.spine_index {
                append_toc_btn(list, &entry.label, idx, sender);
            }
        }
    }
}

fn append_toc_btn(list: &gtk::Box, label: &str, idx: usize, sender: &ComponentSender<ReaderModel>) {
    let btn = gtk::Button::with_label(label);
    btn.add_css_class("kalam-toc-item");
    btn.set_halign(gtk::Align::Fill);
    let s = sender.clone();
    btn.connect_clicked(move |_| s.input(ReaderMsg::TocSelect(idx)));
    list.append(&btn);
}

/// Stable `set_data` key per theme — `set_data` needs a `&'static str`.
fn theme_tick_key(theme: ReadingTheme) -> &'static str {
    match theme {
        ReadingTheme::Light => "kalam-tick-light",
        ReadingTheme::Sepia => "kalam-tick-sepia",
        ReadingTheme::Dark => "kalam-tick-dark",
    }
}

/// Show the tick only on the active theme's button.
fn refresh_theme_buttons(widgets: &ReaderModelWidgets, active: ReadingTheme) {
    let Some(pop) = widgets.dict_btn.popover() else {
        return;
    };
    let Some(pop) = pop.downcast_ref::<gtk::Popover>() else {
        return;
    };
    for theme in [ReadingTheme::Light, ReadingTheme::Sepia, ReadingTheme::Dark] {
        unsafe {
            if let Some(tick) = pop.data::<gtk::Label>(theme_tick_key(theme)) {
                tick.as_ref()
                    .set_opacity(if theme == active { 1.0 } else { 0.0 });
            }
        }
    }
}

fn set_font_size_label(widgets: &ReaderModelWidgets, px: u32) {
    let Some(pop) = widgets.dict_btn.popover() else {
        return;
    };
    let Some(pop) = pop.downcast_ref::<gtk::Popover>() else {
        return;
    };
    unsafe {
        if let Some(label) = pop.data::<gtk::Label>("kalam-font-size") {
            label.as_ref().set_label(&format!("{px}px"));
        }
    }
}

fn rebuild_anno_list(list: &gtk::Box, annos: &[Annotation], sender: &ComponentSender<ReaderModel>) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    if annos.is_empty() {
        let l = gtk::Label::new(Some(
            "No highlights or quotes in this book yet. Select text → highlight (color) or save quote (❝).",
        ));
        l.add_css_class("kalam-placeholder");
        l.set_wrap(true);
        l.set_xalign(0.0);
        list.append(&l);
        return;
    }
    for a in annos.iter().take(100) {
        let row = gtk::Box::new(gtk::Orientation::Vertical, 4);
        row.set_margin_bottom(8);
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let chap_l = gtk::Label::new(Some(&format!("Ch {} · {}", a.chapter_index + 1, a.color)));
        chap_l.add_css_class("kalam-muted");
        chap_l.set_halign(gtk::Align::Start);
        chap_l.set_hexpand(true);
        header.append(&chap_l);
        let del = gtk::Button::with_label("✕");
        del.add_css_class("kalam-secondary-btn");
        let id = a.id;
        let s = sender.clone();
        del.connect_clicked(move |_| s.input(ReaderMsg::DeleteAnnotation(id)));
        header.append(&del);
        row.append(&header);

        let txt = gtk::Label::new(Some(&a.text_excerpt));
        txt.set_wrap(true);
        txt.set_xalign(0.0);
        txt.set_max_width_chars(80);
        txt.add_css_class("kalam-quote-text");
        row.append(&txt);

        let jump = gtk::Button::with_label("Jump");
        jump.add_css_class("kalam-secondary-btn");
        let ch = a.chapter_index as usize;
        let s = sender.clone();
        jump.connect_clicked(move |_| s.input(ReaderMsg::JumpToChapter(ch)));
        row.append(&jump);

        let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
        sep.set_margin_top(6);
        row.append(&sep);
        list.append(&row);
    }
}

fn rebuild_dict_list(
    list: &gtk::Box,
    entries: &[crate::db::DictEntry],
    sender: &ComponentSender<ReaderModel>,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    if entries.is_empty() {
        let l = gtk::Label::new(Some("No matches. Import dict packs in Settings."));
        l.add_css_class("kalam-placeholder");
        l.set_wrap(true);
        list.append(&l);
        return;
    }
    for e in entries.iter().take(20) {
        let btn = gtk::Button::new();
        btn.add_css_class("kalam-toc-item");
        let inner = gtk::Box::new(gtk::Orientation::Vertical, 2);
        let w = gtk::Label::new(Some(&e.word));
        w.set_halign(gtk::Align::Start);
        w.add_css_class("kalam-muted");
        inner.append(&w);
        let def = gtk::Label::new(Some(&truncate_def(&e.definition, 120)));
        def.set_wrap(true);
        def.set_xalign(0.0);
        def.set_halign(gtk::Align::Start);
        def.add_css_class("kalam-placeholder");
        inner.append(&def);
        btn.set_child(Some(&inner));
        let word = e.word.clone();
        let s = sender.clone();
        btn.connect_clicked(move |_| s.input(ReaderMsg::DictSearchSelect(word.clone())));
        list.append(&btn);
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
                } else {
                    out.push('%');
                    out.push(a);
                    out.push(b);
                    continue;
                }
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
