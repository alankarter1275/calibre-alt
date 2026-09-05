use crate::sources::{RemoteBookDetails, RemoteChapter, SourceManager};
use gtk::prelude::*;
use relm4::prelude::*;
use std::sync::Arc;

#[derive(Debug)]
pub enum RemoteDetailOut {
    Back,
    OpenReader {
        source_id: String,
        chapter_id: String,
        title: String,
    },
}

#[derive(Debug)]
pub enum RemoteDetailMsg {
    LoadInfo,
    InfoSuccess(RemoteBookDetails, Vec<RemoteChapter>),
    InfoFailed(String),
    AddToLibrary,
    ReadChapter { chapter_id: String, title: String },
    OpenExternalUrl(String),
}

pub struct RemoteDetailInit {
    pub manager: Arc<SourceManager>,
    pub catalog: Arc<crate::db::Catalog>,
    pub source_id: String,
    pub remote_id: String,
}

pub struct RemoteDetailModel {
    manager: Arc<SourceManager>,
    catalog: Arc<crate::db::Catalog>,
    source_id: String,
    remote_id: String,
    details: Option<RemoteBookDetails>,
    chapters: Vec<RemoteChapter>,
    status: String,
    is_loading: bool,
    is_in_library: bool,
}

impl RemoteDetailModel {
    pub fn new(init: RemoteDetailInit) -> Self {
        Self {
            manager: init.manager,
            catalog: init.catalog,
            source_id: init.source_id,
            remote_id: init.remote_id,
            details: None,
            chapters: Vec::new(),
            status: "Loading manga details…".to_string(),
            is_loading: true,
            is_in_library: false,
        }
    }
}

#[relm4::component(pub)]
impl Component for RemoteDetailModel {
    type Init = RemoteDetailInit;
    type Input = RemoteDetailMsg;
    type Output = RemoteDetailOut;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 16,
            add_css_class: "kalam-page-container",

            // ── Top Navigation Bar ──────────────────────────────────────────
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 12,
                add_css_class: "kalam-page-header",

                gtk::Button {
                    set_icon_name: "go-previous-symbolic",
                    add_css_class: "flat",
                    set_tooltip_text: Some("Back to Browse"),
                    connect_clicked[sender] => move |_| {
                        let _ = sender.output(RemoteDetailOut::Back);
                    },
                },

                gtk::Label {
                    #[watch]
                    set_label: if let Some(d) = &model.details { &d.title } else { "Manga Details" },
                    add_css_class: "kalam-title-large",
                    set_halign: gtk::Align::Start,
                    set_hexpand: true,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                },

                gtk::Button {
                    #[watch]
                    set_label: if model.is_in_library { "✓ In Library" } else { "+ Add to Library" },
                    add_css_class: "kalam-btn-filled",
                    #[watch]
                    set_sensitive: !model.is_loading && model.details.is_some() && !model.is_in_library,
                    connect_clicked => RemoteDetailMsg::AddToLibrary,
                }
            },

            // ── Status Banner (while loading or on error) ───────────────────
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 8,
                #[watch]
                set_visible: model.is_loading,

                gtk::Spinner {
                    #[watch]
                    set_spinning: model.is_loading,
                },

                gtk::Label {
                    #[watch]
                    set_label: &model.status,
                    add_css_class: "kalam-subtitle-muted",
                },
            },

            // ── Manga Info Card ─────────────────────────────────────────────
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 20,
                add_css_class: "kalam-card",
                #[watch]
                set_visible: model.details.is_some(),

                // Cover Image
                #[name = "cover_pic"]
                gtk::Picture {
                    set_content_fit: gtk::ContentFit::Cover,
                    set_size_request: (160, 230),
                    add_css_class: "kalam-book-card",
                },

                // Metadata Details
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                    set_hexpand: true,

                    gtk::Label {
                        #[watch]
                        set_label: if let Some(d) = &model.details { &d.title } else { "" },
                        add_css_class: "kalam-title-medium",
                        set_halign: gtk::Align::Start,
                        set_wrap: true,
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 12,

                        gtk::Label {
                            #[watch]
                            set_label: if let Some(d) = &model.details {
                                if d.author.is_empty() { "Unknown Author" } else { &d.author }
                            } else {
                                ""
                            },
                            add_css_class: "kalam-subtitle-muted",
                            set_halign: gtk::Align::Start,
                        },

                        gtk::Label {
                            #[watch]
                            set_label: if let Some(d) = &model.details {
                                &d.status
                            } else {
                                ""
                            },
                            add_css_class: "kalam-badge",
                            set_halign: gtk::Align::Start,
                        },
                    },

                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        set_max_content_height: 120,
                        set_propagate_natural_height: true,

                        gtk::Label {
                            #[watch]
                            set_label: if let Some(d) = &model.details { &d.description } else { "" },
                            set_wrap: true,
                            set_halign: gtk::Align::Start,
                            set_valign: gtk::Align::Start,
                            add_css_class: "kalam-body",
                        },
                    },
                }
            },

            // ── Chapters Section ────────────────────────────────────────────
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 8,
                #[watch]
                set_visible: !model.chapters.is_empty(),

                gtk::Label {
                    #[watch]
                    set_label: &format!("Chapters ({})", model.chapters.len()),
                    add_css_class: "kalam-title-medium",
                    set_halign: gtk::Align::Start,
                },
            },

            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,
                #[watch]
                set_visible: !model.chapters.is_empty(),

                #[name = "chapters_box"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 6,
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = RemoteDetailModel::new(init);
        let widgets = view_output!();

        sender.input(RemoteDetailMsg::LoadInfo);

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
            RemoteDetailMsg::LoadInfo => {
                self.is_loading = true;
                self.status = "Loading manga details and chapters…".to_string();

                if let Some(source) = self.manager.get(&self.source_id) {
                    let s = sender.clone();
                    let r_id = self.remote_id.clone();

                    crate::tasks::spawn(
                        move |_reporter| {
                            let details = source.get_details(&r_id)?;
                            let chapters = source.get_chapters(&r_id)?;
                            Ok::<_, anyhow::Error>((details, chapters))
                        },
                        |_| {},
                        move |res| match res {
                            Ok((d, c)) => s.input(RemoteDetailMsg::InfoSuccess(d, c)),
                            Err(e)     => s.input(RemoteDetailMsg::InfoFailed(e.to_string())),
                        },
                    );
                } else {
                    self.is_loading = false;
                    self.status = "Source not found.".to_string();
                }
            }

            RemoteDetailMsg::InfoSuccess(details, chapters) => {
                self.is_loading = false;

                // Load cover image in background
                if let (Some(url), Some(source)) = (&details.cover_url, self.manager.get(&self.source_id)) {
                    let u = url.clone();
                    let pic = widgets.cover_pic.clone();
                    crate::tasks::spawn(
                        move |_| source.fetch_image(&u).ok(),
                        |_| {},
                        move |bytes| {
                            if let Some(b) = bytes {
                                let stream = gtk::gio::MemoryInputStream::from_bytes(&gtk::glib::Bytes::from(&b));
                                if let Ok(pixbuf) = gtk::gdk_pixbuf::Pixbuf::from_stream_at_scale(&stream, 160, 230, true, None::<&gtk::gio::Cancellable>) {
                                    pic.set_paintable(Some(&gtk::gdk::Texture::for_pixbuf(&pixbuf)));
                                }
                            }
                        },
                    );
                }

                self.details = Some(details);
                self.chapters = chapters;

                // Clear previous chapters list
                while let Some(child) = widgets.chapters_box.first_child() {
                    widgets.chapters_box.remove(&child);
                }

                let manga_title = self.details.as_ref().map(|d| d.title.clone()).unwrap_or_default();

                // Render styled chapter rows
                for ch in &self.chapters {
                    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                    row.add_css_class("kalam-card");
                    row.set_margin_start(4);
                    row.set_margin_end(4);

                    // Chapter number badge
                    let num_lbl = gtk::Label::new(Some(&format!("Ch. {}", ch.number)));
                    num_lbl.add_css_class("kalam-body-emphasis");
                    num_lbl.set_size_request(80, -1);
                    num_lbl.set_halign(gtk::Align::Start);
                    row.append(&num_lbl);

                    // Chapter Title (or dash if untitled)
                    let title_text = if ch.title.is_empty() {
                        "Untitled".to_string()
                    } else {
                        ch.title.clone()
                    };
                    let title_lbl = gtk::Label::new(Some(&title_text));
                    title_lbl.set_hexpand(true);
                    title_lbl.set_halign(gtk::Align::Start);
                    title_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
                    title_lbl.add_css_class("kalam-body");
                    row.append(&title_lbl);

                    // Action button: Internal Reader vs External Web Link
                    if let Some(ref ext_url) = ch.url {
                        let badge = gtk::Label::new(Some("MangaPlus"));
                        badge.add_css_class("kalam-badge");
                        row.append(&badge);

                        let btn = gtk::Button::with_label("Open Web ↗");
                        btn.add_css_class("kalam-btn-tonal");
                        let url_clone = ext_url.clone();
                        let s = sender.clone();
                        btn.connect_clicked(move |_| {
                            s.input(RemoteDetailMsg::OpenExternalUrl(url_clone.clone()));
                        });
                        row.append(&btn);
                    } else {
                        let btn = gtk::Button::with_label("Read");
                        btn.add_css_class("kalam-btn-filled");
                        let ch_id = ch.chapter_id.clone();
                        let reader_title = format!("{} - Ch. {}", manga_title, ch.number);
                        let s = sender.clone();
                        btn.connect_clicked(move |_| {
                            s.input(RemoteDetailMsg::ReadChapter {
                                chapter_id: ch_id.clone(),
                                title: reader_title.clone(),
                            });
                        });
                        row.append(&btn);
                    }

                    widgets.chapters_box.append(&row);
                }
            }

            RemoteDetailMsg::InfoFailed(err) => {
                self.is_loading = false;
                self.status = format!("Failed to load: {}", err);
            }

            RemoteDetailMsg::AddToLibrary => {
                if let Some(details) = &self.details {
                    let res1 = self.catalog.add_remote_book(details, &self.source_id);
                    let res2 = self.catalog.add_remote_chapters(&self.remote_id, &self.source_id, &self.chapters);

                    if let Err(e) = res1 {
                        self.status = format!("Error adding to library: {}", e);
                    } else if let Err(e) = res2 {
                        self.status = format!("Error saving chapters: {}", e);
                    } else {
                        self.is_in_library = true;
                        self.status = "Added to library!".to_string();
                    }
                }
            }

            RemoteDetailMsg::ReadChapter { chapter_id, title } => {
                let _ = sender.output(RemoteDetailOut::OpenReader {
                    source_id: self.source_id.clone(),
                    chapter_id,
                    title,
                });
            }

            RemoteDetailMsg::OpenExternalUrl(url) => {
                // Launch external chapter in system default web browser
                let _ = gtk::glib::spawn_command_line_async(&format!("xdg-open '{}'", url));
            }
        }

        self.update_view(widgets, sender);
    }
}
