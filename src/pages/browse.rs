use crate::models::NavItem;
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
            status: "Search to find manga or fiction.".to_string(),
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

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 12,

                gtk::SearchEntry {
                    set_placeholder_text: Some("Search Title or Author..."),
                    set_hexpand: true,
                    connect_search_changed[sender] => move |entry| {
                        sender.input(BrowseMsg::Search(entry.text().to_string()));
                    },
                },
            },

            gtk::Label {
                #[watch]
                set_label: &model.status,
                #[watch]
                set_visible: model.results.is_empty(),
                add_css_class: "kalam-subtitle-muted",
            },

            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,
                #[watch]
                set_visible: !model.results.is_empty(),
                
                #[name = "grid_box"]
                gtk::FlowBox {
                    set_selection_mode: gtk::SelectionMode::None,
                    set_valign: gtk::Align::Start,
                    set_max_children_per_line: 10,
                    set_min_children_per_line: 2,
                    set_row_spacing: 16,
                    set_column_spacing: 16,
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
            BrowseMsg::Search(q) => {
                self.query = q.clone();
                if q.is_empty() {
                    self.results.clear();
                    self.status = "Search to find manga or fiction.".to_string();
                    self.update_view(widgets, sender);
                    return;
                }
                
                if let Some(source) = self.active_source.clone() {
                    self.is_loading = true;
                    self.status = "Searching...".to_string();
                    self.results.clear();
                    
                    let s = sender.clone();
                    crate::tasks::spawn(
                        move |_reporter| {
                            source.search(&q, 1, &[])
                        },
                        |_| {},
                        move |res| {
                            match res {
                                Ok(page) => s.input(BrowseMsg::SearchSuccess(page)),
                                Err(e) => s.input(BrowseMsg::SearchFailed(e.to_string())),
                            }
                        }
                    );
                }
            }
            BrowseMsg::SearchSuccess(page) => {
                self.is_loading = false;
                self.results = page.results;
                if self.results.is_empty() {
                    self.status = "No results found.".to_string();
                } else {
                    self.status = format!("Found {} results", self.results.len());
                }

                // Render grid
                while let Some(child) = widgets.grid_box.first_child() {
                    widgets.grid_box.remove(&child);
                }
                
                for res in &self.results {
                    let card = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                    let title = gtk::Label::new(Some(&res.title));
                    card.append(&title);
                    
                    let btn = gtk::Button::with_label("View");
                    let remote_id = res.remote_id.clone();
                    let s = sender.clone();
                    btn.connect_clicked(move |_| s.input(BrowseMsg::OpenBook(remote_id.clone())));
                    card.append(&btn);
                    
                    widgets.grid_box.append(&card);
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
