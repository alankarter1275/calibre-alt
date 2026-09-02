//! My Library → All books: search, sort, import, cover grid.

use crate::db::{Catalog, SortKey};
use crate::epub;
use crate::models::Book;
use crate::service::LibraryService;
use crate::widgets::book_row::build_book_grid;
use gtk::prelude::*;
use relm4::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug)]
pub enum AllBooksOut {
    OpenBook { book_id: i64 },
    OpenBookDialog { book_id: i64 },
}

#[derive(Debug)]
pub enum AllBooksMsg {
    SearchChanged(String),
    SortChanged(SortKey),
    PickFiles,
    FilesChosen(Vec<PathBuf>),
}

/// Progress from the import worker.
#[derive(Debug)]
pub enum ImportProgress {
    /// One file finished: 1-based index, total, and its title.
    Step {
        done: usize,
        total: usize,
        title: String,
    },
    Finished(ImportTally),
}

#[derive(Debug, Default)]
pub struct ImportTally {
    pub imported: usize,
    pub dupes: usize,
    pub errors: usize,
    pub restored: usize,
    pub last_title: String,
}

pub struct AllBooksModel {
    service: LibraryService,
    books: Vec<Book>,
    query: String,
    sort: SortKey,
    status: String,
    /// True while a background import is running; disables the button.
    importing: bool,
}

#[relm4::component(pub)]
impl Component for AllBooksModel {
    type Init = Arc<Catalog>;
    type Input = AllBooksMsg;
    type Output = AllBooksOut;
    type CommandOutput = ImportProgress;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 12,
            set_hexpand: true,

            gtk::Label {
                set_label: "All books",
                add_css_class: "kalam-page-title",
                set_halign: gtk::Align::Start,
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 8,

                #[name = "search"]
                gtk::SearchEntry {
                    set_hexpand: true,
                    set_placeholder_text: Some("Search title, author, series…"),
                    connect_search_changed[sender] => move |entry| {
                        sender.input(AllBooksMsg::SearchChanged(entry.text().to_string()));
                    },
                },

                #[name = "sort_box"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 4,
                },

                gtk::Button {
                    #[watch]
                    set_label: if model.importing {
                        "Importing…"
                    } else {
                        "+ Import EPUB"
                    },
                    add_css_class: "kalam-primary-btn",
                    #[watch]
                    set_sensitive: !model.importing,
                    connect_clicked => AllBooksMsg::PickFiles,
                },
            },

            #[name = "status_label"]
            gtk::Label {
                #[watch]
                set_label: &model.status,
                add_css_class: "kalam-muted",
                set_halign: gtk::Align::Start,
            },

            #[name = "list"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 0,
                set_halign: gtk::Align::Start,
                set_hexpand: false,
                set_vexpand: false,
            },
        }
    }

    fn init(
        catalog: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let sort = SortKey::Added;
        let service = LibraryService::new(catalog);
        let snap = service.all_books(sort, "");
        // `reload()` has always shown "Database error: .." in the status line;
        // `init()` used `unwrap_or_default()`, so the very first paint claimed
        // an empty library where a refresh would have told the truth.
        let status = match snap.errors.first() {
            Some(err) => format!("Database error: {err}"),
            None => status_line(snap.books.len(), ""),
        };

        let model = AllBooksModel {
            service,
            books: snap.books,
            query: String::new(),
            sort,
            status,
            importing: false,
        };
        let widgets = view_output!();

        for key in SortKey::ALL {
            let btn = gtk::ToggleButton::with_label(key.label());
            btn.add_css_class("kalam-secondary-btn");
            if *key == SortKey::Added {
                btn.set_active(true);
            }
            let k = *key;
            let s = sender.clone();
            btn.connect_toggled(move |b| {
                if b.is_active() {
                    s.input(AllBooksMsg::SortChanged(k));
                }
            });
            widgets.sort_box.append(&btn);
        }
        group_toggles(&widgets.sort_box);

        rebuild_list(&widgets.list, &model.books, &model.query, &sender);

        ComponentParts { model, widgets }
    }

    fn update_cmd_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            ImportProgress::Step { done, total, title } => {
                // Only the counter moves per file; the list is rebuilt once at
                // the end so a large import does not thrash the grid.
                self.status = if title.is_empty() {
                    format!("Importing {done} of {total}…")
                } else {
                    format!("Importing {done} of {total} — {title}")
                };
                widgets.status_label.set_label(&self.status);
                return;
            }
            ImportProgress::Finished(tally) => {
                self.importing = false;
                self.reload();

                if tally.imported > 0 {
                    crate::notify::success(
                        &format!(
                            "{} book{} imported",
                            tally.imported,
                            if tally.imported == 1 { "" } else { "s" }
                        ),
                        &tally.last_title,
                    );
                }

                // Say when edits came back, so a restored title does not look
                // like the import ignored the file.
                let restored_note = if tally.restored > 0 {
                    format!(" {} kept your earlier metadata edits.", tally.restored)
                } else {
                    String::new()
                };
                self.status = format!(
                    "Import done — {} added, {} already in library, {} failed.{restored_note}{}",
                    tally.imported,
                    tally.dupes,
                    tally.errors,
                    if tally.last_title.is_empty() {
                        String::new()
                    } else {
                        format!(" Last: {}", tally.last_title)
                    }
                );
            }
        }

        rebuild_list(&widgets.list, &self.books, &self.query, &sender);
        self.update_view(widgets, sender);
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            AllBooksMsg::SearchChanged(q) => {
                self.query = q;
                self.status.clear();
                self.reload();
            }
            AllBooksMsg::SortChanged(sort) => {
                self.sort = sort;
                self.status.clear();
                self.reload();
            }
            AllBooksMsg::PickFiles => {
                let dialog = gtk::FileDialog::builder()
                    .title("Import EPUB books")
                    .modal(true)
                    .build();

                let filter = gtk::FileFilter::new();
                filter.set_name(Some("EPUB books"));
                filter.add_suffix("epub");
                filter.add_mime_type("application/epub+zip");
                let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
                filters.append(&filter);
                dialog.set_filters(Some(&filters));

                let s = sender.clone();
                let window = root.root().and_then(|r| r.downcast::<gtk::Window>().ok());
                let window = window.or_else(|| {
                    relm4::main_application()
                        .active_window()
                        .and_then(|w| w.downcast::<gtk::Window>().ok())
                });

                dialog.open_multiple(
                    window.as_ref(),
                    gtk::gio::Cancellable::NONE,
                    move |result| {
                        if let Ok(files) = result {
                            let mut paths = Vec::new();
                            let n = files.n_items();
                            for i in 0..n {
                                if let Some(obj) = files.item(i) {
                                    if let Ok(file) = obj.downcast::<gtk::gio::File>() {
                                        if let Some(p) = file.path() {
                                            paths.push(p);
                                        }
                                    }
                                }
                            }
                            if !paths.is_empty() {
                                s.input(AllBooksMsg::FilesChosen(paths));
                            }
                        }
                    },
                );
            }
            AllBooksMsg::FilesChosen(paths) => {
                // Importing parses, hashes and copies each file. Doing that
                // inline froze the window with no sign of progress, so it runs
                // on a worker thread and reports back per file.
                let total = paths.len();
                self.importing = true;
                self.status = format!("Importing 1 of {total}…");

                let catalog = self.service.catalog().clone();
                sender.spawn_command(move |out| {
                    let mut tally = ImportTally::default();
                    for (i, path) in paths.iter().enumerate() {
                        match epub::import_epub(&catalog, path) {
                            Ok(r) if r.duplicate => {
                                tally.dupes += 1;
                                tally.last_title = r.title.clone();
                            }
                            Ok(r) => {
                                tally.imported += 1;
                                if r.restored {
                                    tally.restored += 1;
                                }
                                tally.last_title = r.title.clone();
                            }
                            Err(err) => {
                                tally.errors += 1;
                                let name = path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_default();
                                crate::notify::error(
                                    &format!("Could not import {name}"),
                                    &format!("{err:#}"),
                                );
                            }
                        }
                        out.send(ImportProgress::Step {
                            done: i + 1,
                            total,
                            title: tally.last_title.clone(),
                        })
                        .ok();
                    }
                    out.send(ImportProgress::Finished(tally)).ok();
                });
            }
        }

        rebuild_list(&widgets.list, &self.books, &self.query, &sender);
        self.update_view(widgets, sender);
    }
}

impl AllBooksModel {
    fn reload(&mut self) {
        let snap = self.service.all_books(self.sort, &self.query);
        if let Some(err) = snap.errors.first() {
            self.books.clear();
            self.status = format!("Database error: {err}");
            return;
        }
        let n = snap.books.len();
        self.books = snap.books;
        // An import summary outranks the generic count: it is the result of
        // something the user just did.
        if !self.status.starts_with("Import done") {
            self.status = status_line(n, &self.query);
        }
    }
}

fn status_line(n: usize, query: &str) -> String {
    if query.trim().is_empty() {
        if n == 0 {
            "No books yet — import an EPUB to get started.".into()
        } else {
            format!(
                "{n} book{} · click cover for float · Ctrl+click for full page",
                if n == 1 { "" } else { "s" }
            )
        }
    } else {
        format!("{n} result{}", if n == 1 { "" } else { "s" })
    }
}

fn group_toggles(box_: &gtk::Box) {
    let mut group_leader: Option<gtk::ToggleButton> = None;
    let mut child = box_.first_child();
    while let Some(w) = child {
        let next = w.next_sibling();
        if let Ok(btn) = w.downcast::<gtk::ToggleButton>() {
            if let Some(ref leader) = group_leader {
                btn.set_group(Some(leader));
            } else {
                group_leader = Some(btn);
            }
        }
        child = next;
    }
}

/// `query` distinguishes the two very different reasons the grid can be empty.
/// Telling a user with 300 books that their "library is empty" because a
/// search matched nothing is simply wrong, and it hides the fix: clear it.
fn rebuild_list(
    list: &gtk::Box,
    books: &[Book],
    query: &str,
    sender: &ComponentSender<AllBooksModel>,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    if books.is_empty() {
        let query = query.trim();
        let message = if query.is_empty() {
            "Your library is empty.\nClick “+ Import EPUB” to add books.".to_string()
        } else {
            format!("No books match “{query}”.\nTry another search, or clear it to see everything.")
        };
        let empty = gtk::Label::new(Some(&message));
        empty.add_css_class("kalam-placeholder");
        empty.set_wrap(true);
        list.append(&empty);
        return;
    }

    let s = sender.clone();
    let s2 = sender.clone();
    let grid = build_book_grid(
        books,
        move |id| {
            s.output(AllBooksOut::OpenBook { book_id: id }).ok();
        },
        move |id| {
            s2.output(AllBooksOut::OpenBookDialog { book_id: id }).ok();
        },
    );
    list.append(&grid);
}
