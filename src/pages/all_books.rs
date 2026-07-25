//! My Library → All books: search, sort, import, list.

use crate::db::{Catalog, SortKey};
use crate::epub;
use crate::models::Book;
use crate::widgets::book_row::build_book_row;
use gtk::prelude::*;
use relm4::prelude::*;
use std::path::PathBuf;
use std::rc::Rc;

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
    Delete { book_id: i64 },
}

pub struct AllBooksModel {
    catalog: Rc<Catalog>,
    books: Vec<Book>,
    query: String,
    sort: SortKey,
    status: String,
}

#[relm4::component(pub)]
impl Component for AllBooksModel {
    type Init = Rc<Catalog>;
    type Input = AllBooksMsg;
    type Output = AllBooksOut;
    type CommandOutput = ();

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

            // Toolbar
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
                    set_label: "+ Import EPUB",
                    add_css_class: "kalam-primary-btn",
                    connect_clicked => AllBooksMsg::PickFiles,
                },
            },

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
            },
        }
    }

    fn init(
        catalog: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let sort = SortKey::Added;
        let books = catalog.list_books(sort, "").unwrap_or_default();
        let status = status_line(books.len(), "");

        let model = AllBooksModel {
            catalog,
            books,
            query: String::new(),
            sort,
            status,
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
            // group toggles
            widgets.sort_box.append(&btn);
        }
        // Make sort buttons exclusive
        group_toggles(&widgets.sort_box);

        rebuild_list(&widgets.list, &model.books, &sender);

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
                // Find parent window
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
                let mut imported = 0usize;
                let mut dupes = 0usize;
                let mut errors = 0usize;
                let mut last_title = String::new();
                for path in paths {
                    match epub::import_epub(&self.catalog, &path) {
                        Ok(r) if r.duplicate => {
                            dupes += 1;
                            last_title = r.title;
                        }
                        Ok(r) => {
                            imported += 1;
                            last_title = r.title;
                        }
                        Err(err) => {
                            errors += 1;
                            eprintln!("kalam import error ({}): {err:#}", path.display());
                        }
                    }
                }
                self.reload();
                self.status = format!(
                    "Import done — {imported} added, {dupes} already in library, {errors} failed.{}",
                    if last_title.is_empty() {
                        String::new()
                    } else {
                        format!(" Last: {last_title}")
                    }
                );
            }
            AllBooksMsg::Delete { book_id } => {
                if let Err(err) = self.catalog.delete_book(book_id) {
                    self.status = format!("Delete failed: {err}");
                } else {
                    self.reload();
                    self.status = "Book removed.".into();
                }
            }
        }

        rebuild_list(&widgets.list, &self.books, &sender);
        self.update_view(widgets, sender);
    }
}

impl AllBooksModel {
    fn reload(&mut self) {
        match self.catalog.list_books(self.sort, &self.query) {
            Ok(books) => {
                let n = books.len();
                self.books = books;
                // Keep one-shot action messages; otherwise show count.
                let keep = self.status.starts_with("Import done")
                    || self.status.starts_with("Book removed")
                    || self.status.starts_with("Delete failed");
                if !keep {
                    self.status = status_line(n, &self.query);
                }
            }
            Err(err) => {
                self.books.clear();
                self.status = format!("Database error: {err}");
            }
        }
    }
}

fn status_line(n: usize, query: &str) -> String {
    if query.trim().is_empty() {
        if n == 0 {
            "No books yet — import an EPUB to get started.".into()
        } else {
            format!("{n} book{}", if n == 1 { "" } else { "s" })
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

fn rebuild_list(list: &gtk::Box, books: &[Book], sender: &ComponentSender<AllBooksModel>) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    if books.is_empty() {
        let empty = gtk::Label::new(Some(
            "Your library is empty.\nClick “+ Import EPUB” or drop files here later.",
        ));
        empty.add_css_class("kalam-placeholder");
        empty.set_wrap(true);
        list.append(&empty);
        return;
    }

    for book in books {
        let id = book.id;
        let s1 = sender.clone();
        let s2 = sender.clone();
        let s3 = sender.clone();
        let row = build_book_row(
            book,
            move || {
                s1.output(AllBooksOut::OpenBook { book_id: id }).ok();
            },
            move || {
                s2.output(AllBooksOut::OpenBookDialog { book_id: id }).ok();
            },
        );

        let del = gtk::Button::with_label("Remove");
        del.add_css_class("kalam-secondary-btn");
        del.connect_clicked(move |_| {
            s3.input(AllBooksMsg::Delete { book_id: id });
        });
        row.append(&del);
        list.append(&row);
    }
}
