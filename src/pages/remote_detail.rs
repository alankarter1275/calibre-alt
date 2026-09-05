use crate::sources::{RemoteBookDetails, RemoteChapter, SourceManager, Source};
use gtk::prelude::*;
use relm4::prelude::*;
use std::sync::Arc;

#[derive(Debug)]
pub enum RemoteDetailOut {
    Back,
    OpenReader { source_id: String, chapter_id: String },
}

#[derive(Debug)]
pub enum RemoteDetailMsg {
    LoadInfo,
    InfoSuccess(RemoteBookDetails, Vec<RemoteChapter>),
    InfoFailed(String),
    AddToLibrary,
    ReadChapter(String),
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
            status: "Loading...".to_string(),
            is_loading: true,
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

            // Header Section
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 12,
                add_css_class: "kalam-page-header",
                
                gtk::Button {
                    set_icon_name: "go-previous-symbolic",
                    add_css_class: "flat",
                    connect_clicked[sender] => move |_| {
                        let _ = sender.output(RemoteDetailOut::Back);
                    },
                },

                gtk::Label {
                    #[watch]
                    set_label: if let Some(d) = &model.details { &d.title } else { "Loading..." },
                    add_css_class: "kalam-title-large",
                    set_halign: gtk::Align::Start,
                    set_hexpand: true,
                },

                gtk::Button {
                    set_label: "Add to Library",
                    add_css_class: "kalam-btn-filled",
                    #[watch]
                    set_sensitive: !model.is_loading && model.details.is_some(),
                    connect_clicked => RemoteDetailMsg::AddToLibrary,
                }
            },

            gtk::Label {
                #[watch]
                set_label: &model.status,
                #[watch]
                set_visible: model.is_loading,
                add_css_class: "kalam-subtitle-muted",
            },

            // Metadata Box
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 16,
                #[watch]
                set_visible: model.details.is_some(),
                
                #[name = "cover_pic"]
                gtk::Picture {
                    set_content_fit: gtk::ContentFit::Cover,
                    set_size_request: (160, 240),
                    add_css_class: "kalam-book-card",
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                    
                    gtk::Label {
                        #[watch]
                        set_label: if let Some(d) = &model.details { &d.author } else { "" },
                        add_css_class: "kalam-subtitle-muted",
                        set_halign: gtk::Align::Start,
                    },
                    gtk::Label {
                        #[watch]
                        set_label: if let Some(d) = &model.details { &d.description } else { "" },
                        set_wrap: true,
                        set_halign: gtk::Align::Start,
                    },
                }
            },

            // Chapters Grid
            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,
                #[watch]
                set_visible: !model.is_loading && !model.chapters.is_empty(),
                
                #[name = "chapters_box"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 4,
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
        
        // Trigger initial load
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
                self.status = "Loading book info...".to_string();
                
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
                        move |res| {
                            match res {
                                Ok((d, c)) => s.input(RemoteDetailMsg::InfoSuccess(d, c)),
                                Err(e) => s.input(RemoteDetailMsg::InfoFailed(e.to_string())),
                            }
                        }
                    );
                } else {
                    self.is_loading = false;
                    self.status = "Source not found".to_string();
                }
            }
            RemoteDetailMsg::InfoSuccess(details, chapters) => {
                self.is_loading = false;
                
                if let (Some(url), Some(source)) = (&details.cover_url, self.manager.get(&self.source_id)) {
                    let u = url.clone();
                    let pic = widgets.cover_pic.clone();
                    crate::tasks::spawn(
                        move |_| source.fetch_image(&u).ok(),
                        |_| {},
                        move |bytes| {
                            if let Some(b) = bytes {
                                if let Ok(tex) = gtk::gdk::Texture::from_bytes(&gtk::glib::Bytes::from(&b)) {
                                    pic.set_paintable(Some(&tex));
                                }
                            }
                        }
                    );
                }
                
                self.details = Some(details);
                self.chapters = chapters;
                
                // Clear existing
                while let Some(child) = widgets.chapters_box.first_child() {
                    widgets.chapters_box.remove(&child);
                }

                // Render chapters
                for ch in &self.chapters {
                    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                    
                    let title = if ch.title.is_empty() {
                        format!("Chapter {}", ch.number)
                    } else {
                        format!("Chapter {} - {}", ch.number, ch.title)
                    };
                    
                    let label = gtk::Label::new(Some(&title));
                    label.set_hexpand(true);
                    label.set_halign(gtk::Align::Start);
                    row.append(&label);
                    
                    let btn = gtk::Button::with_label("Read");
                    let ch_id = ch.chapter_id.clone();
                    let s = sender.clone();
                    btn.connect_clicked(move |_| s.input(RemoteDetailMsg::ReadChapter(ch_id.clone())));
                    row.append(&btn);
                    
                    widgets.chapters_box.append(&row);
                }
            }
            RemoteDetailMsg::InfoFailed(err) => {
                self.is_loading = false;
                self.status = format!("Failed to load: {}", err);
            }
            RemoteDetailMsg::AddToLibrary => {
                if let Some(details) = &self.details {
                    // Assuming AppModel handles catalog, wait RemoteDetailModel needs Catalog
                    // Let's print a warning if we don't have catalog.
                    // Wait! Let's pass the catalog in RemoteDetailInit so we can save it.
                    if let Err(e) = self.catalog.add_remote_book(details, &self.source_id) {
                        self.status = format!("Failed to save: {}", e);
                    } else if let Err(e) = self.catalog.add_remote_chapters(&self.remote_id, &self.source_id, &self.chapters) {
                        self.status = format!("Failed to save chapters: {}", e);
                    } else {
                        self.status = "Added to library successfully!".to_string();
                    }
                }
            }
            RemoteDetailMsg::ReadChapter(chapter_id) => {
                let _ = sender.output(RemoteDetailOut::OpenReader {
                    source_id: self.source_id.clone(),
                    chapter_id,
                });
            }
        }
        self.update_view(widgets, sender);
    }
}
