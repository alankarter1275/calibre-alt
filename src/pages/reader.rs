//! Immersive EPUB reader (WebKitGTK) — chapter-wise continuous scroll.

use crate::db::Catalog;
use crate::epub_book::{reading_css, OpenBook, ReadingTheme};
use crate::paths::reader_cache_dir;
use gtk::prelude::*;
use relm4::prelude::*;
use std::rc::Rc;
use webkit6::prelude::*;

#[derive(Debug)]
pub enum ReaderOut {
    Close,
}

#[derive(Debug)]
pub enum ReaderMsg {
    Close,
    TocToggle,
    TocSelect(usize),
    PrevChapter,
    NextChapter,
    Theme(ReadingTheme),
    FontDelta(i32),
    ScrollFraction(f64),
    RequestNext,
    FlushProgress,
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
    toc_visible: bool,
    loading: bool,
    dirty: bool,
    webview: webkit6::WebView,
}

#[relm4::component(pub)]
impl Component for ReaderModel {
    type Init = (Rc<Catalog>, i64);
    type Input = ReaderMsg;
    type Output = ReaderOut;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            add_css_class: "kalam-reader",
            set_hexpand: true,
            set_vexpand: true,

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                add_css_class: "kalam-reader-bar",
                set_spacing: 8,

                gtk::Button {
                    set_label: "← Library",
                    add_css_class: "kalam-secondary-btn",
                    connect_clicked => ReaderMsg::Close,
                },

                gtk::Button {
                    set_label: "TOC",
                    add_css_class: "kalam-secondary-btn",
                    connect_clicked => ReaderMsg::TocToggle,
                },

                #[name = "title_label"]
                gtk::Label {
                    add_css_class: "kalam-reader-title",
                    set_hexpand: true,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    set_halign: gtk::Align::Center,
                },

                gtk::Button {
                    set_label: "A−",
                    add_css_class: "kalam-secondary-btn",
                    connect_clicked => ReaderMsg::FontDelta(-1),
                },
                gtk::Button {
                    set_label: "A+",
                    add_css_class: "kalam-secondary-btn",
                    connect_clicked => ReaderMsg::FontDelta(1),
                },

                gtk::Button {
                    set_label: "Light",
                    add_css_class: "kalam-secondary-btn",
                    connect_clicked => ReaderMsg::Theme(ReadingTheme::Light),
                },
                gtk::Button {
                    set_label: "Sepia",
                    add_css_class: "kalam-secondary-btn",
                    connect_clicked => ReaderMsg::Theme(ReadingTheme::Sepia),
                },
                gtk::Button {
                    set_label: "Dark",
                    add_css_class: "kalam-secondary-btn",
                    connect_clicked => ReaderMsg::Theme(ReadingTheme::Dark),
                },

                gtk::Button {
                    set_label: "‹",
                    add_css_class: "kalam-secondary-btn",
                    set_tooltip_text: Some("Previous chapter (P)"),
                    connect_clicked => ReaderMsg::PrevChapter,
                },
                #[name = "chapter_label"]
                gtk::Label {
                    add_css_class: "kalam-muted",
                    set_width_chars: 8,
                },
                gtk::Button {
                    set_label: "›",
                    add_css_class: "kalam-secondary-btn",
                    set_tooltip_text: Some("Next chapter (N)"),
                    connect_clicked => ReaderMsg::NextChapter,
                },
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_hexpand: true,
                set_vexpand: true,

                #[name = "toc_scroll"]
                gtk::ScrolledWindow {
                    set_width_request: 240,
                    set_vexpand: true,
                    #[watch]
                    set_visible: model.toc_visible,
                    set_hscrollbar_policy: gtk::PolicyType::Never,

                    #[name = "toc_list"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        add_css_class: "kalam-reader-toc",
                        set_spacing: 2,
                    },
                },

                #[name = "web_host"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,
                    set_vexpand: true,
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
                        "<html><body style='padding:2rem;background:#12141a;color:#e8eaf0;font-family:sans-serif'><h1>Could not open book</h1><pre>{err:#}</pre><p>Press Esc or ← Library to go back.</p></body></html>"
                    );
                    webview.load_html(&msg, None);
                    (book.title, OpenBook::empty_placeholder(), 0, 0.0)
                }
            }
        } else {
            ("Missing book".into(), OpenBook::empty_placeholder(), 0, 0.0)
        };

        let model = ReaderModel {
            catalog,
            book_id,
            book_title,
            open,
            chapter,
            fraction,
            theme: ReadingTheme::Dark,
            font_px: 20,
            line_height: 1.55,
            margin_em: 1.25,
            toc_visible: false,
            loading: false,
            dirty: false,
            webview: webview.clone(),
        };

        let widgets = view_output!();
        widgets.web_host.append(&webview);
        update_chrome_labels(&widgets, &model);
        build_toc(&widgets.toc_list, &model.open, &sender);

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
                Key::t | Key::T => {
                    s.input(ReaderMsg::TocToggle);
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
                _ => gtk::glib::Propagation::Proceed,
            }
        });
        root.add_controller(key);
        root.set_can_focus(true);

        // Intercept kalam:// bridge navigations from injected JS.
        let s_nav = sender.clone();
        webview.connect_decide_policy(move |_wv, decision, decision_type| {
            use webkit6::prelude::*;
            use webkit6::{NavigationPolicyDecision, PolicyDecisionType};
            if decision_type != PolicyDecisionType::NavigationAction {
                return false;
            }
            let Some(nav) = decision.downcast_ref::<NavigationPolicyDecision>() else {
                return false;
            };
            let Some(mut action) = nav.navigation_action() else {
                return false;
            };
            let Some(req) = action.request() else {
                return false;
            };
            let Some(uri) = req.uri() else {
                return false;
            };
            let uri = uri.as_str();
            if let Some(rest) = uri.strip_prefix("kalam://") {
                if rest == "next" {
                    s_nav.input(ReaderMsg::RequestNext);
                } else if let Some(frac_s) = rest.strip_prefix("progress/") {
                    if let Ok(frac) = frac_s.parse::<f64>() {
                        s_nav.input(ReaderMsg::ScrollFraction(frac));
                    }
                }
                decision.ignore();
                return true;
            }
            false
        });

        let s_flush = sender.clone();
        gtk::glib::timeout_add_local(std::time::Duration::from_secs(2), move || {
            s_flush.input(ReaderMsg::FlushProgress);
            gtk::glib::ControlFlow::Continue
        });

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
                self.save_progress();
                sender.output(ReaderOut::Close).ok();
            }
            ReaderMsg::TocToggle => {
                self.toc_visible = !self.toc_visible;
            }
            ReaderMsg::TocSelect(idx) => {
                if idx < self.open.chapter_count() && idx != self.chapter && !self.loading {
                    self.go_chapter(idx, 0.0);
                }
            }
            ReaderMsg::PrevChapter => {
                if self.chapter > 0 && !self.loading {
                    self.go_chapter(self.chapter - 1, 0.0);
                }
            }
            ReaderMsg::NextChapter | ReaderMsg::RequestNext => {
                if self.chapter + 1 < self.open.chapter_count() && !self.loading {
                    self.go_chapter(self.chapter + 1, 0.0);
                }
            }
            ReaderMsg::Theme(t) => {
                self.theme = t;
                self.loading = true;
                load_chapter(self);
            }
            ReaderMsg::FontDelta(d) => {
                let next = (self.font_px as i32 + d).clamp(14, 36) as u32;
                if next != self.font_px {
                    self.font_px = next;
                    self.loading = true;
                    load_chapter(self);
                }
            }
            ReaderMsg::ScrollFraction(fraction) => {
                if self.open.chapter_count() == 0 {
                    return;
                }
                self.fraction = fraction.clamp(0.0, 1.0);
                self.dirty = true;
            }
            ReaderMsg::FlushProgress => {
                if self.dirty {
                    self.save_progress();
                }
            }
        }

        update_chrome_labels(widgets, self);
        self.update_view(widgets, sender);
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: relm4::Sender<Self::Output>) {
        self.save_progress();
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
        self.dirty = false;
    }

    fn go_chapter(&mut self, idx: usize, frac: f64) {
        self.save_progress();
        self.chapter = idx;
        self.fraction = frac;
        self.loading = true;
        load_chapter(self);
    }
}

fn update_chrome_labels(widgets: &ReaderModelWidgets, model: &ReaderModel) {
    if model.open.chapter_count() == 0 {
        widgets.title_label.set_label(&model.book_title);
        widgets.chapter_label.set_label("—");
        return;
    }
    widgets.title_label.set_label(&format!(
        "{} — {}",
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
                "<html><body style='padding:2rem;background:#12141a;color:#e8eaf0;font-family:sans-serif'><h1>Could not load chapter</h1><pre>{err:#}</pre></body></html>"
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
