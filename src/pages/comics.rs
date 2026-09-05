//! Comics & Manga hub page: browse local CBZ/CBR comic archives and launch reader.

use crate::db::{Catalog, SortKey};
use crate::models::{Book, BookFormat};
use crate::pages::all_books::{import_summary, spawn_import, ImportTally};
use crate::service::LibraryService;
use crate::widgets::book_row::build_book_grid;
use gtk::prelude::*;
use relm4::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug)]
pub enum ComicsOut {
    OpenComic { book_id: i64 },
    OpenBookDialog { book_id: i64 },
}

#[derive(Debug)]
pub enum ComicsMsg {
    #[allow(dead_code)]
    Refresh,
    SearchChanged(String),
    PickFiles,
    FilesChosen(Vec<PathBuf>),
    ImportStep {
        done: usize,
        total: usize,
        title: String,
    },
    ImportFinished(ImportTally),
    OpenComic(i64),
    OpenBookDialog(i64),
}

pub struct ComicsModel {
    service: LibraryService,
    comics: Vec<Book>,
    filtered: Vec<Book>,
    search_query: String,
    status: String,
}

impl ComicsModel {
    pub fn new(catalog: Arc<Catalog>) -> Self {
        let service = LibraryService::new(catalog);
        let mut model = Self {
            service,
            comics: Vec::new(),
            filtered: Vec::new(),
            search_query: String::new(),
            status: String::new(),
        };
        model.reload();
        model
    }

    fn reload(&mut self) {
        if let Ok(all_books) = self.service.catalog().list_books(SortKey::Added, "") {
            self.comics = all_books
                .into_iter()
                .filter(|b| matches!(b.format, BookFormat::Cbz | BookFormat::Cbr))
                .collect();
        } else {
            self.comics = Vec::new();
        }
        self.apply_filter();
        self.update_status();
    }

    fn apply_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered = self.comics.clone();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered = self
                .comics
                .iter()
                .filter(|b| {
                    b.title.to_lowercase().contains(&query)
                        || b.authors.to_lowercase().contains(&query)
                })
                .cloned()
                .collect();
        }
    }

    fn update_status(&mut self) {
        let count = self.comics.len();
        self.status = if count == 0 {
            "No comic archives imported yet".to_string()
        } else if count == 1 {
            "1 comic in library".to_string()
        } else {
            format!("{count} comics in library")
        };
    }
}

fn build_empty_state(sender: &ComponentSender<ComicsModel>) -> gtk::Box {
    let empty_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
    empty_box.set_halign(gtk::Align::Center);
    empty_box.set_valign(gtk::Align::Center);
    empty_box.set_margin_top(48);
    empty_box.set_margin_bottom(48);

    let icon = gtk::Image::from_icon_name("image-x-generic-symbolic");
    icon.set_pixel_size(64);
    icon.add_css_class("kalam-subtitle-muted");
    empty_box.append(&icon);

    let title = gtk::Label::new(Some("No comics or manga in your library"));
    title.add_css_class("kalam-title-medium");
    empty_box.append(&title);

    let subtitle = gtk::Label::new(Some(
        "Import local CBZ or CBR archive files to read comics with the Moku viewer.",
    ));
    subtitle.add_css_class("kalam-subtitle-muted");
    empty_box.append(&subtitle);

    let btn = gtk::Button::with_label("Import Comics");
    btn.add_css_class("kalam-btn-filled");
    btn.set_halign(gtk::Align::Center);
    let s = sender.clone();
    btn.connect_clicked(move |_| s.input(ComicsMsg::PickFiles));
    empty_box.append(&btn);

    empty_box
}

#[relm4::component(pub)]
impl Component for ComicsModel {
    type Init = Arc<Catalog>;
    type Input = ComicsMsg;
    type Output = ComicsOut;
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

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 4,
                    set_hexpand: true,

                    gtk::Label {
                        set_label: "Comics & Manga",
                        add_css_class: "kalam-title-large",
                        set_halign: gtk::Align::Start,
                    },
                    gtk::Label {
                        #[watch]
                        set_label: &model.status,
                        add_css_class: "kalam-subtitle-muted",
                        set_halign: gtk::Align::Start,
                    },
                },

                gtk::Button {
                    set_label: "+ Import Comics",
                    add_css_class: "kalam-btn-filled",
                    connect_clicked => ComicsMsg::PickFiles,
                },
            },

            // Main Content Area
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 12,
                set_hexpand: true,
                set_vexpand: true,

                // Search Bar (visible if comics exist)
                #[watch]
                set_visible: !model.comics.is_empty(),

                gtk::SearchEntry {
                    set_placeholder_text: Some("Search comics by title or author…"),
                    connect_search_changed[sender] => move |entry| {
                        sender.input(ComicsMsg::SearchChanged(entry.text().to_string()));
                    },
                },

                // Comics Display Container
                #[name = "scrolled_window"]
                gtk::ScrolledWindow {
                    set_hexpand: true,
                    set_vexpand: true,
                },
            },
        }
    }

    fn init(
        catalog: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ComicsModel::new(catalog);
        let widgets = view_output!();

        if model.comics.is_empty() {
            let empty_box = build_empty_state(&sender);
            widgets.scrolled_window.set_child(Some(&empty_box));
        } else {
            let s_click = sender.clone();
            let s_float = sender.clone();
            let grid = build_book_grid(
                &model.filtered,
                move |id| {
                    s_click.input(ComicsMsg::OpenComic(id));
                },
                move |id| {
                    s_float.input(ComicsMsg::OpenBookDialog(id));
                },
            );
            widgets.scrolled_window.set_child(Some(&grid));
        }

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            ComicsMsg::Refresh => {
                self.reload();
            }
            ComicsMsg::SearchChanged(q) => {
                self.search_query = q;
                self.apply_filter();
            }
            ComicsMsg::PickFiles => {
                let dialog = gtk::FileDialog::builder()
                    .title("Import Comic Archives (CBZ/CBR)")
                    .modal(true)
                    .build();

                let filter = gtk::FileFilter::new();
                filter.set_name(Some("Comic Archives (*.cbz, *.cbr)"));
                filter.add_suffix("cbz");
                filter.add_suffix("cbr");
                filter.add_mime_type("application/x-cbz");
                filter.add_mime_type("application/x-cbr");
                let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
                filters.append(&filter);
                dialog.set_filters(Some(&filters));

                let window = root.root().and_then(|r| r.downcast::<gtk::Window>().ok());
                let window = window.or_else(|| {
                    relm4::main_application()
                        .active_window()
                        .and_then(|w| w.downcast::<gtk::Window>().ok())
                });

                let s = sender.clone();
                dialog.open_multiple(
                    window.as_ref(),
                    None::<&gtk::gio::Cancellable>,
                    move |res| {
                        if let Ok(files) = res {
                            let mut paths = Vec::new();
                            for i in 0..files.n_items() {
                                if let Some(file) =
                                    files.item(i).and_then(|obj| obj.downcast::<gtk::gio::File>().ok())
                                {
                                    if let Some(path) = file.path() {
                                        paths.push(path);
                                    }
                                }
                            }
                            if !paths.is_empty() {
                                s.input(ComicsMsg::FilesChosen(paths));
                            }
                        }
                    },
                );
            }
            ComicsMsg::FilesChosen(paths) => {
                let s1 = sender.clone();
                let s2 = sender.clone();
                let catalog = self.service.catalog().clone();
                spawn_import(
                    catalog,
                    paths,
                    move |done, total, title| {
                        s1.input(ComicsMsg::ImportStep { done, total, title });
                    },
                    move |tally| {
                        s2.input(ComicsMsg::ImportFinished(tally));
                    },
                );
            }
            ComicsMsg::ImportStep { done, total, title } => {
                self.status = format!("Importing comic {done}/{total}: {title}…");
            }
            ComicsMsg::ImportFinished(tally) => {
                self.reload();
                self.status = import_summary(&tally);
            }
            ComicsMsg::OpenComic(id) => {
                let _ = sender.output(ComicsOut::OpenComic { book_id: id });
            }
            ComicsMsg::OpenBookDialog(id) => {
                let _ = sender.output(ComicsOut::OpenBookDialog { book_id: id });
            }
        }

        // Rebuild or update scrolled_window child after message
        if self.comics.is_empty() {
            let empty_box = build_empty_state(&sender);
            widgets.scrolled_window.set_child(Some(&empty_box));
        } else {
            let s_click = sender.clone();
            let s_float = sender.clone();
            let grid = build_book_grid(
                &self.filtered,
                move |id| {
                    s_click.input(ComicsMsg::OpenComic(id));
                },
                move |id| {
                    s_float.input(ComicsMsg::OpenBookDialog(id));
                },
            );
            widgets.scrolled_window.set_child(Some(&grid));
        }

        self.update_view(widgets, sender);
    }
}
