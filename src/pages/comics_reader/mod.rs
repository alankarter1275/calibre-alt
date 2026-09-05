//! Moku-style comics reader page component.
//!
//! Provides an immersive image-pager reader for CBZ/CBR comic archives with
//! LTR/RTL/Webtoon reading modes, page scrubbing, slide-out settings panel,
//! floating minimal chrome, and bounded memory caching.

pub mod types;
pub mod provider;
pub mod providers;

pub use types::*;

use gtk::gdk;
use gtk::prelude::*;
use relm4::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ComicsReaderModel {
    pub title: String,
    pub provider: Arc<dyn provider::ImageProvider>,
    pub current_page: usize,
    pub direction: ReadingDirection,
    pub fit_mode: FitMode,
    pub show_chrome: bool,
    pub show_settings: bool,
    pub textures: HashMap<usize, gdk::Texture>,
}

impl ComicsReaderModel {
    pub fn new(init: types::ComicsReaderInit) -> Self {
        let model = Self {
            title: init.title,
            provider: init.provider,
            current_page: 0,
            direction: ReadingDirection::Ltr,
            fit_mode: FitMode::Width,
            show_chrome: true,
            show_settings: false,
            textures: HashMap::new(),
        };

        // Note: we can't spawn tasks directly from new without the sender,
        // so preload_nearby_pages must be called from `init` trait method,
        // or we handle loading synchronously if it's local.
        // For now we just return the model and let `init` trigger the fetch.
        model
    }

    /// Ask the provider for images in the window `curr ± 2`.
    /// Note: this now returns a list of indices we need to fetch via a task.
    fn get_missing_pages(&mut self) -> Vec<usize> {
        let total = self.provider.page_count();
        if total == 0 {
            return Vec::new();
        }
        let curr = self.current_page;
        let min_keep = curr.saturating_sub(2);
        let max_keep = (curr + 2).min(total.saturating_sub(1));

        // Retain only textures inside [min_keep, max_keep]
        self.textures.retain(|&idx, _| idx >= min_keep && idx <= max_keep);

        let mut missing = Vec::new();
        for idx in min_keep..=max_keep {
            if !self.textures.contains_key(&idx) {
                missing.push(idx);
            }
        }
        missing
    }

    pub fn set_page(&mut self, idx: usize) {
        let total = self.provider.page_count();
        if total == 0 {
            return;
        }
        let clamped = idx.clamp(0, total - 1);
        self.current_page = clamped;
        
        // Note: DB progress saving will need to be re-added via the new abstraction later
    }

    pub fn trigger_loads(&mut self, sender: &relm4::ComponentSender<Self>) {
        let missing = self.get_missing_pages();
        let provider = self.provider.clone();

        for idx in missing {
            // Insert a dummy texture or mark as loading if we wanted to be strict.
            // For now, just spawn the task if it's not already in textures.
            // If it's already fetching, it might fetch twice if we change pages fast.
            let s = sender.clone();
            let prov = provider.clone();
            crate::tasks::spawn(
                move |_| {
                    prov.fetch_page(idx).ok()
                },
                |_| {},
                move |bytes| {
                    if let Some(b) = bytes {
                        let gbytes = glib::Bytes::from(&b);
                        if let Ok(tex) = gdk::Texture::from_bytes(&gbytes) {
                            s.input(types::ComicsReaderMsg::PageLoaded(idx, Some(tex)));
                        } else {
                            s.input(types::ComicsReaderMsg::PageLoaded(idx, None));
                        }
                    } else {
                        s.input(types::ComicsReaderMsg::PageLoaded(idx, None));
                    }
                }
            );
        }
    }

    pub fn next_page(&mut self) {
        if self.current_page + 1 < self.provider.page_count() {
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
    type Init = types::ComicsReaderInit;
    type Input = ComicsReaderMsg;
    type Output = ComicsReaderOut;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Overlay {
            add_css_class: "kalam-comics-overlay",
            set_hexpand: true,
            set_vexpand: true,

            // ── 1. Base Layer: Comic Image Viewport ─────────────────────
            #[name = "viewport_box"]
            gtk::ScrolledWindow {
                add_css_class: "kalam-comics-viewport",
                set_hexpand: true,
                set_vexpand: true,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    set_hexpand: true,
                    set_vexpand: true,

                    gtk::Spinner {
                        #[watch]
                        set_spinning: !model.textures.contains_key(&model.current_page),
                        #[watch]
                        set_visible: !model.textures.contains_key(&model.current_page),
                        set_size_request: (48, 48),
                    },

                    #[name = "picture"]
                    gtk::Picture {
                        set_can_shrink: true,
                        set_content_fit: gtk::ContentFit::Contain,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        #[watch]
                        set_visible: model.textures.contains_key(&model.current_page),
                    },
                },
            },

            // ── 2. Overlay: Dim Backdrop for Settings Drawer ───────────
            add_overlay = &gtk::Button {
                add_css_class: "kalam-reader-dim",
                #[watch]
                set_visible: model.show_settings,
                set_hexpand: true,
                set_vexpand: true,
                set_halign: gtk::Align::Fill,
                set_valign: gtk::Align::Fill,
                connect_clicked => ComicsReaderMsg::CloseSettings,
            },

            // ── 3. Overlay: Floating Minimal Top Bar (Moku-style) ───────
            add_overlay = &gtk::Revealer {
                #[watch]
                set_reveal_child: model.show_chrome,
                set_transition_type: gtk::RevealerTransitionType::SlideDown,
                set_halign: gtk::Align::Fill,
                set_valign: gtk::Align::Start,

                #[wrap(Some)]
                set_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    add_css_class: "kalam-comics-topbar",
                    set_spacing: 12,

                    // Clean close button
                    gtk::Button {
                        set_child: Some(&crate::icons::symbolic_with_classes("window-close-symbolic", 16, &["kalam-inline-icon"])),
                        add_css_class: "kalam-comics-icon-btn",
                        set_tooltip_text: Some("Close reader (Esc)"),
                        connect_clicked => ComicsReaderMsg::Close,
                    },

                    // Breadcrumb title: Title / Page X of Y
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,
                        set_hexpand: true,
                        set_valign: gtk::Align::Center,

                        gtk::Label {
                            #[watch]
                            set_label: &model.title,
                            add_css_class: "kalam-comics-title",
                            set_ellipsize: gtk::pango::EllipsizeMode::End,
                            set_halign: gtk::Align::Start,
                        },

                        gtk::Label {
                            set_label: "/",
                            add_css_class: "kalam-comics-separator",
                        },

                        gtk::Label {
                            #[watch]
                            set_label: &format!(
                                "Page {} of {}",
                                model.current_page + 1,
                                model.provider.page_count().max(1)
                            ),
                            add_css_class: "kalam-comics-page-count",
                        },
                    },

                    // Right controls: Fit Mode, Direction Badge, Settings Gear
                    gtk::Button {
                        #[watch]
                        set_child: Some(&crate::icons::symbolic_with_classes(model.fit_mode.icon(), 16, &["kalam-inline-icon"])),
                        add_css_class: "kalam-comics-icon-btn",
                        #[watch]
                        set_tooltip_text: Some(&format!("Fit Mode: {} (F)", model.fit_mode.label())),
                        connect_clicked => ComicsReaderMsg::ToggleFitMode,
                    },

                    gtk::Button {
                        #[watch]
                        set_label: model.direction.short_label(),
                        add_css_class: "kalam-comics-badge-btn",
                        #[watch]
                        set_tooltip_text: Some(&format!("Direction: {}", model.direction.label())),
                        connect_clicked => ComicsReaderMsg::ToggleDirection,
                    },

                    gtk::Button {
                        set_child: Some(&crate::icons::symbolic_with_classes("emblem-system-symbolic", 16, &["kalam-inline-icon"])),
                        add_css_class: "kalam-comics-icon-btn",
                        set_tooltip_text: Some("Reader Settings"),
                        connect_clicked => ComicsReaderMsg::ToggleSettings,
                    },
                },
            },

            // ── 4. Overlay: Floating Scrubber Dock at Bottom ────────────
            add_overlay = &gtk::Revealer {
                #[watch]
                set_reveal_child: model.show_chrome,
                set_transition_type: gtk::RevealerTransitionType::SlideUp,
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::End,

                #[wrap(Some)]
                set_child = &gtk::Box {
                    add_css_class: "kalam-comics-bottom-dock",
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 12,
                    set_valign: gtk::Align::Center,

                    // Prev page button
                    gtk::Button {
                        set_child: Some(&crate::icons::symbolic_with_classes("go-previous-symbolic", 15, &["kalam-inline-icon"])),
                        add_css_class: "kalam-comics-pill-nav",
                        set_tooltip_text: Some("Previous page (Left arrow)"),
                        connect_clicked => ComicsReaderMsg::PrevPage,
                    },

                    // Thin Scrubber Slider
                    gtk::Scale {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_width_request: 320,
                        set_hexpand: false,
                        set_draw_value: false,
                        add_css_class: "kalam-comics-slider",
                        #[watch]
                        set_range: (0.0, (model.provider.page_count().saturating_sub(1)) as f64),
                        #[watch]
                        set_value: model.current_page as f64,
                        connect_value_changed[sender] => move |scale| {
                            let val = scale.value().round() as usize;
                            sender.input(ComicsReaderMsg::SetPage(val));
                        },
                    },

                    // Page indicator pill
                    gtk::Label {
                        #[watch]
                        set_label: &format!("{}/{}", model.current_page + 1, model.provider.page_count().max(1)),
                        add_css_class: "kalam-comics-pill-label",
                        set_valign: gtk::Align::Center,
                    },

                    // Next page button
                    gtk::Button {
                        set_child: Some(&crate::icons::symbolic_with_classes("go-next-symbolic", 15, &["kalam-inline-icon"])),
                        add_css_class: "kalam-comics-pill-nav",
                        set_tooltip_text: Some("Next page (Right arrow / Space)"),
                        connect_clicked => ComicsReaderMsg::NextPage,
                    },
                },
            },

            // ── 5. Overlay: Slide-out Reader Settings Drawer ───────────
            add_overlay = &gtk::Revealer {
                #[watch]
                set_reveal_child: model.show_settings,
                set_transition_type: gtk::RevealerTransitionType::SlideLeft,
                set_halign: gtk::Align::End,
                set_valign: gtk::Align::Fill,

                #[wrap(Some)]
                set_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    add_css_class: "kalam-comics-settings-drawer",
                    set_width_request: 320,

                    // Header
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        add_css_class: "kalam-comics-drawer-header",
                        set_spacing: 8,

                        gtk::Label {
                            set_label: "Reader Settings",
                            add_css_class: "kalam-comics-drawer-title",
                            set_hexpand: true,
                            set_halign: gtk::Align::Start,
                        },

                        gtk::Button {
                            set_child: Some(&crate::icons::symbolic_with_classes("window-close-symbolic", 15, &["kalam-inline-icon"])),
                            add_css_class: "kalam-comics-icon-btn",
                            set_tooltip_text: Some("Close Settings"),
                            connect_clicked => ComicsReaderMsg::CloseSettings,
                        },
                    },

                    // Scrollable Drawer Body
                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 20,
                            add_css_class: "kalam-comics-drawer-body",

                            // Section: READING DIRECTION
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 8,

                                gtk::Label {
                                    set_label: "READING DIRECTION",
                                    add_css_class: "kalam-comics-section-label",
                                    set_halign: gtk::Align::Start,
                                },

                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 6,
                                    set_homogeneous: true,

                                    gtk::Button {
                                        set_label: "L → R",
                                        add_css_class: "kalam-comics-seg-btn",
                                        #[watch]
                                        set_css_classes: if model.direction == ReadingDirection::Ltr {
                                            &["kalam-comics-seg-btn", "active"]
                                        } else {
                                            &["kalam-comics-seg-btn"]
                                        },
                                        set_tooltip_text: Some("Left to Right"),
                                        connect_clicked => ComicsReaderMsg::SetDirection(ReadingDirection::Ltr),
                                    },

                                    gtk::Button {
                                        set_label: "R → L",
                                        add_css_class: "kalam-comics-seg-btn",
                                        #[watch]
                                        set_css_classes: if model.direction == ReadingDirection::Rtl {
                                            &["kalam-comics-seg-btn", "active"]
                                        } else {
                                            &["kalam-comics-seg-btn"]
                                        },
                                        set_tooltip_text: Some("Right to Left (Manga)"),
                                        connect_clicked => ComicsReaderMsg::SetDirection(ReadingDirection::Rtl),
                                    },

                                    gtk::Button {
                                        set_label: "Webtoon",
                                        add_css_class: "kalam-comics-seg-btn",
                                        #[watch]
                                        set_css_classes: if model.direction == ReadingDirection::Webtoon {
                                            &["kalam-comics-seg-btn", "active"]
                                        } else {
                                            &["kalam-comics-seg-btn"]
                                        },
                                        set_tooltip_text: Some("Webtoon (Vertical strip)"),
                                        connect_clicked => ComicsReaderMsg::SetDirection(ReadingDirection::Webtoon),
                                    },
                                },
                            },

                            // Section: FIT MODE
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 8,

                                gtk::Label {
                                    set_label: "FIT MODE",
                                    add_css_class: "kalam-comics-section-label",
                                    set_halign: gtk::Align::Start,
                                },

                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 6,
                                    set_homogeneous: true,

                                    gtk::Button {
                                        set_label: "Fit Width",
                                        add_css_class: "kalam-comics-seg-btn",
                                        #[watch]
                                        set_css_classes: if model.fit_mode == FitMode::Width {
                                            &["kalam-comics-seg-btn", "active"]
                                        } else {
                                            &["kalam-comics-seg-btn"]
                                        },
                                        connect_clicked => ComicsReaderMsg::SetFitMode(FitMode::Width),
                                    },

                                    gtk::Button {
                                        set_label: "Fit Height",
                                        add_css_class: "kalam-comics-seg-btn",
                                        #[watch]
                                        set_css_classes: if model.fit_mode == FitMode::Height {
                                            &["kalam-comics-seg-btn", "active"]
                                        } else {
                                            &["kalam-comics-seg-btn"]
                                        },
                                        connect_clicked => ComicsReaderMsg::SetFitMode(FitMode::Height),
                                    },

                                    gtk::Button {
                                        set_label: "Original",
                                        add_css_class: "kalam-comics-seg-btn",
                                        #[watch]
                                        set_css_classes: if model.fit_mode == FitMode::Original {
                                            &["kalam-comics-seg-btn", "active"]
                                        } else {
                                            &["kalam-comics-seg-btn"]
                                        },
                                        connect_clicked => ComicsReaderMsg::SetFitMode(FitMode::Original),
                                    },
                                },
                            },

                            // Section: SHORTCUTS
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 8,
                                add_css_class: "kalam-comics-shortcuts-box",

                                gtk::Label {
                                    set_label: "KEYBOARD SHORTCUTS",
                                    add_css_class: "kalam-comics-section-label",
                                    set_halign: gtk::Align::Start,
                                },

                                gtk::Label {
                                    set_label: "← / →       Turn page\nSpace       Next page\nClick image Toggle chrome\nF           Toggle fit mode\nEsc         Close",
                                    add_css_class: "kalam-comics-shortcut-text",
                                    set_halign: gtk::Align::Start,
                                },
                            },
                        },
                    },
                },
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = ComicsReaderModel::new(init);
        model.trigger_loads(&sender);
        let widgets = view_output!();

        // Paint current page texture if available
        if let Some(texture) = model.textures.get(&model.current_page) {
            widgets.picture.set_paintable(Some(texture));
        }

        // Tap/click on the comic viewport toggles the chrome for immersive reading
        let click = gtk::GestureClick::new();
        let s = sender.clone();
        click.connect_released(move |_, _, _, _| {
            s.input(ComicsReaderMsg::ToggleChrome);
        });
        widgets.viewport_box.add_controller(click);

        // Keyboard navigation controller
        let key_controller = gtk::EventControllerKey::new();
        let s = sender.clone();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            match key {
                gdk::Key::Escape => {
                    s.input(ComicsReaderMsg::Close);
                    glib::Propagation::Stop
                }
                gdk::Key::Left | gdk::Key::Page_Up => {
                    s.input(ComicsReaderMsg::PrevPage);
                    glib::Propagation::Stop
                }
                gdk::Key::Right | gdk::Key::Page_Down | gdk::Key::space => {
                    s.input(ComicsReaderMsg::NextPage);
                    glib::Propagation::Stop
                }
                gdk::Key::f | gdk::Key::F => {
                    s.input(ComicsReaderMsg::ToggleFitMode);
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
        root.add_controller(key_controller);

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
                self.trigger_loads(&sender);
            }
            ComicsReaderMsg::NextPage => {
                match self.direction {
                    ReadingDirection::Ltr | ReadingDirection::Webtoon => self.next_page(),
                    ReadingDirection::Rtl => self.prev_page(),
                }
                self.trigger_loads(&sender);
            }
            ComicsReaderMsg::PrevPage => {
                match self.direction {
                    ReadingDirection::Ltr | ReadingDirection::Webtoon => self.prev_page(),
                    ReadingDirection::Rtl => self.next_page(),
                }
                self.trigger_loads(&sender);
            }
            ComicsReaderMsg::PageLoaded(idx, maybe_tex) => {
                if let Some(tex) = maybe_tex {
                    self.textures.insert(idx, tex);
                }
            }
            ComicsReaderMsg::ToggleDirection => {
                self.direction = self.direction.next();
            }
            ComicsReaderMsg::SetDirection(dir) => {
                self.direction = dir;
            }
            ComicsReaderMsg::ToggleFitMode => {
                self.fit_mode = self.fit_mode.next();
                match self.fit_mode {
                    FitMode::Width => widgets.picture.set_content_fit(gtk::ContentFit::Fill),
                    FitMode::Height => widgets.picture.set_content_fit(gtk::ContentFit::Contain),
                    FitMode::Original => widgets.picture.set_content_fit(gtk::ContentFit::ScaleDown),
                }
            }
            ComicsReaderMsg::SetFitMode(fit) => {
                self.fit_mode = fit;
                match self.fit_mode {
                    FitMode::Width => widgets.picture.set_content_fit(gtk::ContentFit::Fill),
                    FitMode::Height => widgets.picture.set_content_fit(gtk::ContentFit::Contain),
                    FitMode::Original => widgets.picture.set_content_fit(gtk::ContentFit::ScaleDown),
                }
            }
            ComicsReaderMsg::ToggleChrome => {
                self.show_chrome = !self.show_chrome;
            }
            ComicsReaderMsg::ToggleSettings => {
                self.show_settings = !self.show_settings;
            }
            ComicsReaderMsg::CloseSettings => {
                self.show_settings = false;
            }
            ComicsReaderMsg::Close => {
                if self.show_settings {
                    self.show_settings = false;
                } else {
                    let _ = sender.output(ComicsReaderOut::Close);
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
