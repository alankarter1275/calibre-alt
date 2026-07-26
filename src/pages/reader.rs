//! Immersive EPUB reader — full page + floating chrome (tablet-book look).

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
    TocSelect(usize),
    PrevChapter,
    NextChapter,
    Theme(ReadingTheme),
    FontDelta(i32),
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

            // Top-left floating crumb
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
                        set_max_width_chars: 40,
                    },
                },
            },

            // Bottom floating pill
            add_overlay = &gtk::Box {
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::End,
                set_margin_bottom: 22,

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

                    #[name = "chapter_label"]
                    gtk::Label {
                        add_css_class: "kalam-reader-pill-meta",
                        set_width_chars: 7,
                    },

                    #[name = "aa_btn"]
                    gtk::MenuButton {
                        set_label: "Aa",
                        add_css_class: "kalam-reader-pill-btn",
                        set_tooltip_text: Some("Typography & theme"),
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

        let model = ReaderModel {
            catalog,
            book_id,
            book_title,
            open,
            chapter,
            fraction,
            theme: ReadingTheme::Sepia,
            font_px: 19,
            line_height: 1.65,
            margin_em: 1.4,
            loading: false,
            dirty: false,
            webview: webview.clone(),
        };

        let widgets = view_output!();
        widgets.web_host.append(&webview);
        update_chrome_labels(&widgets, &model);

        // TOC popover
        let toc_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
        build_toc(&toc_box, &model.open, &sender);
        let toc_scroll = gtk::ScrolledWindow::builder()
            .min_content_height(280)
            .min_content_width(260)
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

        // Aa popover
        let aa_wrap = gtk::Box::new(gtk::Orientation::Vertical, 10);
        aa_wrap.set_margin_all(12);
        let size_l = gtk::Label::new(Some("TEXT SIZE"));
        size_l.add_css_class("kalam-reader-popover-title");
        size_l.set_halign(gtk::Align::Start);
        aa_wrap.append(&size_l);
        let size_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        size_row.set_halign(gtk::Align::Center);
        let a_minus = gtk::Button::with_label("A−");
        a_minus.add_css_class("kalam-reader-pill-btn");
        let s1 = sender.clone();
        a_minus.connect_clicked(move |_| s1.input(ReaderMsg::FontDelta(-1)));
        let a_plus = gtk::Button::with_label("A+");
        a_plus.add_css_class("kalam-reader-pill-btn");
        let s2 = sender.clone();
        a_plus.connect_clicked(move |_| s2.input(ReaderMsg::FontDelta(1)));
        size_row.append(&a_minus);
        size_row.append(&a_plus);
        aa_wrap.append(&size_row);

        let theme_l = gtk::Label::new(Some("THEME"));
        theme_l.add_css_class("kalam-reader-popover-title");
        theme_l.set_halign(gtk::Align::Start);
        aa_wrap.append(&theme_l);
        let theme_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        for (label, theme) in [
            ("Light", ReadingTheme::Light),
            ("Sepia", ReadingTheme::Sepia),
            ("Dark", ReadingTheme::Dark),
        ] {
            let b = gtk::Button::with_label(label);
            b.add_css_class("kalam-reader-theme-btn");
            let s = sender.clone();
            b.connect_clicked(move |_| s.input(ReaderMsg::Theme(theme)));
            theme_row.append(&b);
        }
        aa_wrap.append(&theme_row);

        let aa_pop = gtk::Popover::new();
        aa_pop.add_css_class("kalam-reader-popover");
        aa_pop.set_child(Some(&aa_wrap));
        aa_pop.set_position(gtk::PositionType::Top);
        widgets.aa_btn.set_popover(Some(&aa_pop));

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
                _ => gtk::glib::Propagation::Proceed,
            }
        });
        root.add_controller(key);
        root.set_can_focus(true);

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
                widgets.aa_btn.popdown();
                self.loading = true;
                load_chapter(self);
                self.loading = false;
            }
            ReaderMsg::FontDelta(d) => {
                let next = (self.font_px as i32 + d).clamp(14, 36) as u32;
                if next != self.font_px {
                    self.font_px = next;
                    self.loading = true;
                    load_chapter(self);
                    self.loading = false;
                }
            }
            ReaderMsg::FlushProgress => {
                if self.open.chapter_count() > 0 {
                    if self.fraction < 0.85 {
                        self.fraction = (self.fraction + 0.02).min(0.85);
                        self.dirty = true;
                    }
                    if self.dirty {
                        self.save_progress();
                    }
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
        self.loading = false;
        self.dirty = true;
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
