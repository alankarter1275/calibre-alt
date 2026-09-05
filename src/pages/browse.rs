use crate::sources::{RemoteBookCard, SearchPage, SourceManager, Source};
use gtk::prelude::*;
use relm4::prelude::*;
use std::sync::Arc;

#[derive(Debug)]
pub enum BrowseOut {
    OpenRemoteBook { source_id: String, remote_id: String },
}

#[derive(Debug)]
pub enum BrowseMsg {
    Search(String),
    SearchSuccess(SearchPage),
    SearchFailed(String),
    OpenBook(String), // remote_id
    CoverLoaded { remote_id: String, bytes: Vec<u8> },
}

pub struct BrowseModel {
    manager: Arc<SourceManager>,
    active_source: Option<Arc<dyn Source>>,
    query: String,
    results: Vec<RemoteBookCard>,
    status: String,
    is_loading: bool,
}

impl BrowseModel {
    pub fn new(manager: Arc<SourceManager>) -> Self {
        let active = manager.all().into_iter().next();
        Self {
            manager,
            active_source: active,
            query: String::new(),
            results: Vec::new(),
            status: "Search to find manga.".to_string(),
            is_loading: false,
        }
    }
}

#[relm4::component(pub)]
impl Component for BrowseModel {
    type Init = Arc<SourceManager>;
    type Input = BrowseMsg;
    type Output = BrowseOut;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 16,
            add_css_class: "kalam-page-container",

            // ── Header ───────────────────────────────────────────────────────────
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 12,
                add_css_class: "kalam-page-header",

                gtk::Label {
                    set_label: "Browse Sources",
                    add_css_class: "kalam-title-large",
                    set_halign: gtk::Align::Start,
                    set_hexpand: true,
                },
            },

            // ── Search bar ───────────────────────────────────────────────────────
            gtk::SearchEntry {
                set_placeholder_text: Some("Search Title or Author…"),
                set_hexpand: true,
                connect_activate[sender] => move |entry| {
                    // Only fire on Enter — avoids hammering API on every keystroke.
                    sender.input(BrowseMsg::Search(entry.text().to_string()));
                },
            },

            // ── Status line (shows while loading / empty / error) ────────────────
            gtk::Label {
                #[watch]
                set_label: &model.status,
                #[watch]
                set_visible: model.is_loading || model.results.is_empty(),
                add_css_class: "kalam-subtitle-muted",
            },

            // ── Results grid ─────────────────────────────────────────────────────
            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,
                #[watch]
                set_visible: !model.results.is_empty(),

                #[name = "grid_box"]
                gtk::FlowBox {
                    set_selection_mode: gtk::SelectionMode::None,
                    set_valign: gtk::Align::Start,
                    set_homogeneous: true,
                    set_max_children_per_line: 6,
                    set_min_children_per_line: 2,
                    set_row_spacing: 12,
                    set_column_spacing: 12,
                }
            }
        }
    }

    fn init(
        manager: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = BrowseModel::new(manager);
        let widgets = view_output!();
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
            // ── Search triggered ──────────────────────────────────────────────────
            BrowseMsg::Search(q) => {
                self.query = q.clone();
                if q.is_empty() {
                    self.results.clear();
                    self.status = "Search to find manga.".to_string();
                    self.is_loading = false;
                    self.update_view(widgets, sender);
                    return;
                }

                if let Some(source) = self.active_source.clone() {
                    self.is_loading = true;
                    self.status = "Searching…".to_string();
                    self.results.clear();

                    let s = sender.clone();
                    crate::tasks::spawn(
                        move |_| source.search(&q, 1, &[]),
                        |_| {},
                        move |res| match res {
                            Ok(page) => s.input(BrowseMsg::SearchSuccess(page)),
                            Err(e)   => s.input(BrowseMsg::SearchFailed(e.to_string())),
                        },
                    );
                } else {
                    self.status = "No sources available.".to_string();
                }
            }

            // ── Results arrived ───────────────────────────────────────────────────
            BrowseMsg::SearchSuccess(page) => {
                self.is_loading = false;
                self.results = page.results;

                self.status = if self.results.is_empty() {
                    "No results found.".to_string()
                } else {
                    format!("{} results", self.results.len())
                };

                // Clear previous grid children
                while let Some(child) = widgets.grid_box.first_child() {
                    widgets.grid_box.remove(&child);
                }

                for res in &self.results {
                    // ── Card: 160×240 cover + title + author + button ──────────────
                    let card = gtk::Box::new(gtk::Orientation::Vertical, 6);
                    card.set_size_request(160, -1);
                    card.add_css_class("kalam-book-card");

                    // Cover image (placeholder until async fetch completes)
                    let pic = gtk::Picture::new();
                    pic.set_content_fit(gtk::ContentFit::Cover);
                    pic.set_size_request(160, 220);
                    card.append(&pic);

                    // Kick off cover fetch in background
                    if let (Some(url), Some(source)) =
                        (&res.cover_url, self.active_source.clone())
                    {
                        let url = url.clone();
                        let remote_id = res.remote_id.clone();
                        let s = sender.clone();
                        crate::tasks::spawn(
                            move |_| source.fetch_image(&url),
                            |_| {},
                            move |res| {
                                if let Ok(bytes) = res {
                                    s.input(BrowseMsg::CoverLoaded { remote_id, bytes });
                                }
                            },
                        );
                    }

                    // Title
                    let title_lbl = gtk::Label::new(Some(&res.title));
                    title_lbl.set_wrap(true);
                    title_lbl.set_max_width_chars(20);
                    title_lbl.set_lines(2);
                    title_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
                    title_lbl.add_css_class("kalam-body-emphasis");
                    card.append(&title_lbl);

                    // Author
                    if !res.author.is_empty() && res.author != "Unknown" {
                        let author_lbl = gtk::Label::new(Some(&res.author));
                        author_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
                        author_lbl.add_css_class("kalam-subtitle-muted");
                        card.append(&author_lbl);
                    }

                    // "View" button
                    let btn = gtk::Button::with_label("View Details");
                    btn.add_css_class("kalam-btn-tonal");
                    let rid = res.remote_id.clone();
                    let s = sender.clone();
                    btn.connect_clicked(move |_| s.input(BrowseMsg::OpenBook(rid.clone())));
                    card.append(&btn);

                    widgets.grid_box.append(&card);
                }
            }

            // ── Cover image arrived — find the matching Picture and set it ────────
            // NOTE: Because the grid children are FlowBoxChild wrappers, we walk them.
            BrowseMsg::CoverLoaded { remote_id, bytes } => {
                // Match by position: find the result index for this remote_id
                if let Some(idx) = self.results.iter().position(|r| r.remote_id == remote_id) {
                    // nth FlowBoxChild → its child Box → first child is the Picture
                    if let Some(flow_child) = widgets.grid_box.child_at_index(idx as i32) {
                        if let Some(card) = flow_child.child() {
                            let card_box = card.downcast::<gtk::Box>().unwrap();
                            if let Some(pic_widget) = card_box.first_child() {
                                if let Ok(pic) = pic_widget.downcast::<gtk::Picture>() {
                                    if let Ok(tex) = gtk::gdk::Texture::from_bytes(
                                        &gtk::glib::Bytes::from(&bytes),
                                    ) {
                                        pic.set_paintable(Some(&tex));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            BrowseMsg::SearchFailed(err) => {
                self.is_loading = false;
                self.status = format!("Search failed: {}", err);
            }

            BrowseMsg::OpenBook(remote_id) => {
                if let Some(src) = &self.active_source {
                    let _ = sender.output(BrowseOut::OpenRemoteBook {
                        source_id: src.id().to_string(),
                        remote_id,
                    });
                }
            }
        }

        self.update_view(widgets, sender);
    }
}
