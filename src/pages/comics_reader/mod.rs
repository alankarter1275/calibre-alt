//! Moku-style comics reader page component.
//!
//! Provides an immersive image-pager reader for CBZ/CBR comic archives with
//! LTR/RTL/Webtoon reading modes, page scrubbing, and viewport memory caching.

pub mod types;

pub use types::*;

use crate::comics::{extract_comic_page, list_comic_pages};
use crate::db::Catalog;
use crate::models::Book;
use gtk::gdk;
use gtk::prelude::*;
use relm4::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ComicsReaderModel {
    catalog: Arc<Catalog>,
    #[allow(dead_code)]
    pub book_id: i64,
    pub book: Option<Book>,
    pub pages: Vec<String>,
    pub current_page: usize,
    pub direction: ReadingDirection,
    pub fit_mode: FitMode,
    pub show_chrome: bool,
    pub textures: HashMap<usize, gdk::Texture>,
}

impl ComicsReaderModel {
    pub fn new(catalog: Arc<Catalog>, book_id: i64) -> Self {
        let book = catalog.get_book(book_id).ok().flatten();
        let pages = if let Some(ref b) = book {
            list_comic_pages(&b.file_path).unwrap_or_default()
        } else {
            Vec::new()
        };

        let mut model = Self {
            catalog,
            book_id,
            book,
            pages,
            current_page: 0,
            direction: ReadingDirection::Ltr,
            fit_mode: FitMode::Width,
            show_chrome: true,
            textures: HashMap::new(),
        };

        model.preload_nearby_pages();
        model
    }

    /// Keep memory bounded: decode only current_page ± 2 adjacent pages.
    fn preload_nearby_pages(&mut self) {
        if self.pages.is_empty() {
            return;
        }
        let total = self.pages.len();
        let curr = self.current_page;

        let min_keep = curr.saturating_sub(2);
        let max_keep = (curr + 2).min(total.saturating_sub(1));

        // Retain only textures inside [min_keep, max_keep]
        self.textures.retain(|&idx, _| idx >= min_keep && idx <= max_keep);

        // Load missing textures in window
        if let Some(ref b) = self.book {
            for idx in min_keep..=max_keep {
                if !self.textures.contains_key(&idx) {
                    if let Some(page_name) = self.pages.get(idx) {
                        if let Ok(bytes) = extract_comic_page(&b.file_path, page_name) {
                            let gbytes = glib::Bytes::from(&bytes);
                            if let Ok(tex) = gdk::Texture::from_bytes(&gbytes) {
                                self.textures.insert(idx, tex);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn set_page(&mut self, idx: usize) {
        if self.pages.is_empty() {
            return;
        }
        let clamped = idx.clamp(0, self.pages.len() - 1);
        self.current_page = clamped;
        self.preload_nearby_pages();

        // Save progress to database
        if let Some(ref b) = self.book {
            let _ = self.catalog.set_reading_progress(b.id, clamped, 0.0, self.pages.len());
        }
    }

    pub fn next_page(&mut self) {
        if self.current_page + 1 < self.pages.len() {
            self.set_page(self.current_page + 1);
        }
    }

    pub fn prev_page(&mut self) {
        if self.current_page > 0 {
            self.set_page(self.current_page - 1);
        }
    }
}

#[relm4::component(pub)]
impl Component for ComicsReaderModel {
    type Init = (Arc<Catalog>, i64);
    type Input = ComicsReaderMsg;
    type Output = ComicsReaderOut;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            add_css_class: "kalam-comics-stage",
            set_hexpand: true,
            set_vexpand: true,

            // Top Bar Chrome
            #[name = "top_bar"]
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                add_css_class: "kalam-comics-topbar",
                set_spacing: 12,

                gtk::Button {
                    set_label: "← Back",
                    add_css_class: "kalam-secondary-btn",
                    connect_clicked => ComicsReaderMsg::Close,
                },

                gtk::Label {
                    #[watch]
                    set_label: &format!(
                        "{} — Page {} of {}",
                        model.book.as_ref().map(|b| b.title.as_str()).unwrap_or("Comic"),
                        model.current_page + 1,
                        model.pages.len().max(1)
                    ),
                    add_css_class: "kalam-comics-title",
                    set_hexpand: true,
                    set_halign: gtk::Align::Start,
                },

                gtk::Button {
                    #[watch]
                    set_label: model.direction.label(),
                    add_css_class: "kalam-secondary-btn",
                    connect_clicked => ComicsReaderMsg::ToggleDirection,
                },

                gtk::Button {
                    #[watch]
                    set_label: model.fit_mode.label(),
                    add_css_class: "kalam-secondary-btn",
                    connect_clicked => ComicsReaderMsg::ToggleFitMode,
                },
            },

            // Central Page Display Area
            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    set_hexpand: true,
                    set_vexpand: true,

                    #[name = "picture"]
                    gtk::Picture {
                        set_can_shrink: true,
                        set_content_fit: gtk::ContentFit::Contain,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                    },
                },
            },

            // Bottom Bar Chrome
            #[name = "bottom_bar"]
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                add_css_class: "kalam-comics-bottombar",
                set_spacing: 12,

                gtk::Button {
                    set_label: "Previous",
                    add_css_class: "kalam-secondary-btn",
                    connect_clicked => ComicsReaderMsg::PrevPage,
                },

                gtk::Scale {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_hexpand: true,
                    set_draw_value: false,
                    #[watch]
                    set_range: (0.0, (model.pages.len().saturating_sub(1)) as f64),
                    #[watch]
                    set_value: model.current_page as f64,
                    connect_value_changed[sender] => move |scale| {
                        let val = scale.value().round() as usize;
                        sender.input(ComicsReaderMsg::SetPage(val));
                    },
                },

                gtk::Button {
                    set_label: "Next",
                    add_css_class: "kalam-secondary-btn",
                    connect_clicked => ComicsReaderMsg::NextPage,
                },
            },
        }
    }

    fn init(
        (catalog, book_id): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ComicsReaderModel::new(catalog, book_id);
        let widgets = view_output!();

        // Paint current page texture if available
        if let Some(texture) = model.textures.get(&model.current_page) {
            widgets.picture.set_paintable(Some(texture));
        }

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
            ComicsReaderMsg::SetPage(idx) => {
                self.set_page(idx);
            }
            ComicsReaderMsg::NextPage => {
                match self.direction {
                    ReadingDirection::Ltr | ReadingDirection::Webtoon => self.next_page(),
                    ReadingDirection::Rtl => self.prev_page(),
                }
            }
            ComicsReaderMsg::PrevPage => {
                match self.direction {
                    ReadingDirection::Ltr | ReadingDirection::Webtoon => self.prev_page(),
                    ReadingDirection::Rtl => self.next_page(),
                }
            }
            ComicsReaderMsg::ToggleDirection => {
                self.direction = self.direction.next();
            }
            ComicsReaderMsg::ToggleFitMode => {
                self.fit_mode = self.fit_mode.next();
                match self.fit_mode {
                    FitMode::Width => widgets.picture.set_content_fit(gtk::ContentFit::Fill),
                    FitMode::Height => widgets.picture.set_content_fit(gtk::ContentFit::Contain),
                    FitMode::Original => widgets.picture.set_content_fit(gtk::ContentFit::ScaleDown),
                }
            }
            ComicsReaderMsg::ToggleChrome => {
                self.show_chrome = !self.show_chrome;
                widgets.top_bar.set_visible(self.show_chrome);
                widgets.bottom_bar.set_visible(self.show_chrome);
            }
            ComicsReaderMsg::Close => {
                let _ = sender.output(ComicsReaderOut::Close);
            }
            ComicsReaderMsg::PageLoaded { index, data } => {
                let gbytes = glib::Bytes::from(&data);
                if let Ok(tex) = gdk::Texture::from_bytes(&gbytes) {
                    self.textures.insert(index, tex);
                }
            }
        }

        // Update active texture on picture widget
        if let Some(texture) = self.textures.get(&self.current_page) {
            widgets.picture.set_paintable(Some(texture));
        } else {
            widgets.picture.set_paintable(None::<&gdk::Texture>);
        }

        self.update_view(widgets, sender);
    }
}
