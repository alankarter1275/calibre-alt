use crate::db::{Catalog, ShelfKind};
use crate::models::Book;
use crate::pages::metadata_editor::open_metadata_editor;
use crate::widgets::book_row::cover_widget;
use crate::widgets::book_row::invalidate_cover_cache;
use crate::widgets::charts::star_picker;
use gtk::prelude::*;
use relm4::prelude::*;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug)]
pub enum BookPageOut {
    #[allow(dead_code)]
    Back,
    OpenReader,
    Deleted {
        #[allow(dead_code)]
        book_id: i64,
    },
}

#[derive(Debug)]
pub enum BookPageMsg {
    Delete,
    ToggleReadingList,
    ToggleFinished,
    SetRating(u8),
    EditMetadata,
    ShowShelfMenu,
    Refresh,
}

pub struct BookPageModel {
    catalog: Arc<Catalog>,
    book: Option<Book>,
    in_reading_list: bool,
    finished: bool,
    shelves: Vec<(i64, String)>,
}

#[relm4::component(pub)]
impl Component for BookPageModel {
    type Init = (Arc<Catalog>, i64);
    type Input = BookPageMsg;
    type Output = BookPageOut;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 16,
            set_hexpand: true,

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 24,
                set_valign: gtk::Align::Start,

                #[name = "cover_host"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 4,
                    set_hexpand: true,

                    #[name = "title"]
                    gtk::Label {
                        add_css_class: "kalam-detail-title",
                        set_halign: gtk::Align::Start,
                        set_wrap: true,
                    },
                    #[name = "author"]
                    gtk::Label {
                        add_css_class: "kalam-detail-author",
                        set_halign: gtk::Align::Start,
                    },
                    #[name = "series"]
                    gtk::Label {
                        add_css_class: "kalam-muted",
                        set_halign: gtk::Align::Start,
                    },

                    gtk::Label {
                        set_label: "FORMAT",
                        add_css_class: "kalam-detail-section-title",
                        set_halign: gtk::Align::Start,
                    },
                    #[name = "format"]
                    gtk::Label {
                        add_css_class: "kalam-muted",
                        set_halign: gtk::Align::Start,
                    },

                    gtk::Label {
                        set_label: "PROGRESS",
                        add_css_class: "kalam-detail-section-title",
                        set_halign: gtk::Align::Start,
                    },
                    #[name = "progress"]
                    gtk::Label {
                        add_css_class: "kalam-progress",
                        set_halign: gtk::Align::Start,
                    },

                    gtk::Label {
                        set_label: "YOUR RATING",
                        add_css_class: "kalam-detail-section-title",
                        set_halign: gtk::Align::Start,
                    },
                    #[name = "rating_host"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,
                    },

                    gtk::Label {
                        set_label: "TAGS",
                        add_css_class: "kalam-detail-section-title",
                        set_halign: gtk::Align::Start,
                    },
                    #[name = "tags"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 6,
                    },

                    gtk::Label {
                        set_label: "PATH",
                        add_css_class: "kalam-detail-section-title",
                        set_halign: gtk::Align::Start,
                    },
                    #[name = "path"]
                    gtk::Label {
                        add_css_class: "kalam-muted",
                        set_halign: gtk::Align::Start,
                        set_selectable: true,
                        set_wrap: true,
                        set_xalign: 0.0,
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 10,
                        set_margin_top: 16,

                        gtk::Button {
                            set_label: "Read",
                            add_css_class: "kalam-primary-btn",
                            connect_clicked[sender] => move |_| {
                                sender.output(BookPageOut::OpenReader).ok();
                            },
                        },
                        #[name = "tbr_btn"]
                        gtk::Button {
                            add_css_class: "kalam-secondary-btn",
                            connect_clicked => BookPageMsg::ToggleReadingList,
                        },
                        #[name = "finish_btn"]
                        gtk::Button {
                            add_css_class: "kalam-secondary-btn",
                            connect_clicked => BookPageMsg::ToggleFinished,
                        },
                        #[name = "shelf_btn"]
                        gtk::Button {
                            set_label: "Shelves…",
                            add_css_class: "kalam-secondary-btn",
                            connect_clicked => BookPageMsg::ShowShelfMenu,
                        },
                        gtk::Button {
                            set_label: "Edit metadata",
                            add_css_class: "kalam-secondary-btn",
                            connect_clicked => BookPageMsg::EditMetadata,
                        },
                        gtk::Button {
                            set_label: "Remove",
                            add_css_class: "kalam-secondary-btn",
                            connect_clicked => BookPageMsg::Delete,
                        },
                    },

                    gtk::Label {
                        set_label: "SHELVES",
                        add_css_class: "kalam-detail-section-title",
                        set_halign: gtk::Align::Start,
                    },
                    #[name = "shelf_chips"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 6,
                    },
                },
            },

            gtk::Label {
                set_label: "DESCRIPTION",
                add_css_class: "kalam-detail-section-title",
                set_halign: gtk::Align::Start,
            },
            #[name = "description"]
            gtk::Label {
                add_css_class: "kalam-muted",
                set_halign: gtk::Align::Start,
                set_wrap: true,
                set_xalign: 0.0,
            },

            gtk::Label {
                set_label: "Open Read for the immersive EPUB viewer.",
                add_css_class: "kalam-placeholder",
                set_wrap: true,
                set_margin_top: 8,
            },
        }
    }

    fn init(
        (catalog, book_id): Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let book = catalog.get_book(book_id).ok().flatten();
        let in_reading_list = catalog.is_in_reading_list(book_id).unwrap_or(false);
        let finished = catalog.book_finished_at(book_id).ok().flatten().is_some();
        let shelves = catalog.shelves_for_book(book_id).unwrap_or_default();
        let model = BookPageModel {
            catalog,
            book,
            in_reading_list,
            finished,
            shelves,
        };
        let widgets = view_output!();
        fill(
            &widgets.cover_host,
            &widgets.tags,
            &widgets.title,
            &widgets.author,
            &widgets.series,
            &widgets.format,
            &widgets.progress,
            &widgets.path,
            &widgets.description,
            model.book.as_ref(),
        );
        model.refresh_p4(&widgets, &sender);
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
            BookPageMsg::Delete => {
                if let Some(book) = &self.book {
                    let id = book.id;
                    // Drop the cached texture so a re-import of the same path
                    // cannot show the old cover.
                    if let Some(path) = &book.cover_path {
                        invalidate_cover_cache(path);
                    }
                    let title = book.title.clone();
                    match self.catalog.delete_book(id) {
                        Ok(()) => {
                            crate::notify::success("Book removed", &title);
                            self.book = None;
                            sender.output(BookPageOut::Deleted { book_id: id }).ok();
                        }
                        // Silently doing nothing was the worst outcome here:
                        // the book stayed and no reason was given.
                        Err(err) => {
                            crate::notify::error("Could not remove the book", &err.to_string())
                        }
                    }
                }
            }
            BookPageMsg::ToggleReadingList => {
                if let Some(book) = &self.book {
                    let id = book.id;
                    let title = book.title.clone();
                    if self.in_reading_list {
                        if crate::notify::report(
                            self.catalog.remove_from_reading_list(id),
                            "Could not update the reading list",
                        ) {
                            crate::notify::info("Removed from reading list", &title);
                        }
                    } else if crate::notify::report(
                        self.catalog.add_to_reading_list(id),
                        "Could not update the reading list",
                    ) {
                        crate::notify::success("Added to reading list", &title);
                    }
                    self.reload_p4(id);
                }
            }
            BookPageMsg::SetRating(half_stars) => {
                if let Some(book) = &self.book {
                    let id = book.id;
                    crate::notify::report(
                        self.catalog.set_book_rating(id, half_stars),
                        "Could not save the rating",
                    );
                    self.book = self.catalog.get_book(id).ok().flatten();
                }
            }
            BookPageMsg::ToggleFinished => {
                if let Some(book) = &self.book {
                    let id = book.id;
                    let becoming = !self.finished;
                    let title = book.title.clone();
                    if crate::notify::report(
                        self.catalog.set_book_finished(id, becoming),
                        "Could not update the book",
                    ) {
                        if becoming {
                            crate::notify::success("Marked as finished", &title);
                        } else {
                            crate::notify::info("Marked as unread", &title);
                        }
                    }
                    self.book = self.catalog.get_book(id).ok().flatten();
                    self.reload_p4(id);
                }
            }
            BookPageMsg::EditMetadata => {
                if let Some(book) = &self.book {
                    let id = book.id;
                    let s = sender.clone();
                    open_metadata_editor(
                        root.root()
                            .and_then(|r| r.downcast::<gtk::Window>().ok())
                            .as_ref(),
                        self.catalog.clone(),
                        id,
                        move || s.input(BookPageMsg::Refresh),
                    );
                }
            }
            BookPageMsg::ShowShelfMenu => {
                if let Some(book) = &self.book {
                    let id = book.id;
                    let s = sender.clone();
                    open_shelf_menu(
                        root.root()
                            .and_then(|r| r.downcast::<gtk::Window>().ok())
                            .as_ref(),
                        self.catalog.clone(),
                        id,
                        move || s.input(BookPageMsg::Refresh),
                    );
                }
            }
            BookPageMsg::Refresh => {
                if let Some(book) = &self.book {
                    let id = book.id;
                    self.book = self.catalog.get_book(id).ok().flatten();
                    self.reload_p4(id);
                }
            }
        }
        fill(
            &widgets.cover_host,
            &widgets.tags,
            &widgets.title,
            &widgets.author,
            &widgets.series,
            &widgets.format,
            &widgets.progress,
            &widgets.path,
            &widgets.description,
            self.book.as_ref(),
        );
        self.refresh_p4(widgets, &sender);
        self.update_view(widgets, sender);
    }
}

impl BookPageModel {
    fn reload_p4(&mut self, book_id: i64) {
        self.in_reading_list = self.catalog.is_in_reading_list(book_id).unwrap_or(false);
        self.finished = self
            .catalog
            .book_finished_at(book_id)
            .ok()
            .flatten()
            .is_some();
        self.shelves = self.catalog.shelves_for_book(book_id).unwrap_or_default();
    }

    /// Sync the P4 buttons and shelf chips with current state.
    fn refresh_p4(&self, widgets: &BookPageModelWidgets, sender: &ComponentSender<Self>) {
        let has_book = self.book.is_some();
        widgets.tbr_btn.set_visible(has_book);
        widgets.finish_btn.set_visible(has_book);
        widgets.shelf_btn.set_visible(has_book);

        widgets.tbr_btn.set_label(if self.in_reading_list {
            "− Reading list"
        } else {
            "+ Reading list"
        });
        widgets.finish_btn.set_label(if self.finished {
            "Mark unread"
        } else {
            "Mark finished"
        });

        // Star picker is rebuilt so the filled state matches the stored value.
        let host = &widgets.rating_host;
        while let Some(child) = host.first_child() {
            host.remove(&child);
        }
        if let Some(book) = &self.book {
            let s = sender.clone();
            host.append(&star_picker(book.rating, move |v| {
                s.input(BookPageMsg::SetRating(v))
            }));
            let hint = gtk::Label::new(Some(
                match book.rating_stars() {
                    Some(v) => return_rating_text(v),
                    None => "Not rated yet".into(),
                }
                .as_str(),
            ));
            hint.add_css_class("kalam-muted");
            hint.set_valign(gtk::Align::Center);
            host.append(&hint);
        }

        let chips = &widgets.shelf_chips;
        while let Some(child) = chips.first_child() {
            chips.remove(&child);
        }
        if self.shelves.is_empty() {
            let none = gtk::Label::new(Some("Not on any shelf"));
            none.add_css_class("kalam-muted");
            chips.append(&none);
        } else {
            for (_, name) in &self.shelves {
                let chip = gtk::Label::new(Some(name));
                chip.add_css_class("kalam-chip");
                chips.append(&chip);
            }
        }
        let _ = sender;
    }
}

fn return_rating_text(v: f32) -> String {
    format!("{v:.1} / 5 — click again to clear")
}

/// Checklist of manual shelves for one book.
fn open_shelf_menu(
    parent: Option<&gtk::Window>,
    catalog: Arc<Catalog>,
    book_id: i64,
    on_changed: impl Fn() + 'static,
) {
    let window = gtk::Window::builder()
        .title("Shelves")
        .modal(true)
        .default_width(380)
        .default_height(420)
        .build();
    window.add_css_class("kalam-window");
    if let Some(parent) = parent {
        window.set_transient_for(Some(parent));
    }

    let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
    root.set_margin_all(16);

    let all: Vec<_> = catalog
        .list_shelves()
        .unwrap_or_default()
        .into_iter()
        .filter(|s| s.kind == ShelfKind::Manual)
        .collect();

    if all.is_empty() {
        let empty = gtk::Label::new(Some(
            "No manual shelves yet.\n\nCreate one from the Shelves page, then add books to it here.\n\nSmart shelves fill themselves from rules — they can't be edited by hand.",
        ));
        empty.add_css_class("kalam-placeholder");
        empty.set_wrap(true);
        root.append(&empty);
    } else {
        let hint = gtk::Label::new(Some("Tick the shelves this book belongs on."));
        hint.add_css_class("kalam-muted");
        hint.set_halign(gtk::Align::Start);
        root.append(&hint);

        let list = gtk::Box::new(gtk::Orientation::Vertical, 2);
        let current: Vec<i64> = catalog
            .shelves_for_book(book_id)
            .unwrap_or_default()
            .iter()
            .map(|(id, _)| *id)
            .collect();

        let on_changed = Rc::new(on_changed);
        for shelf in all {
            let check = gtk::CheckButton::with_label(&format!(
                "{}  ({} book{})",
                shelf.name,
                shelf.book_count,
                if shelf.book_count == 1 { "" } else { "s" }
            ));
            check.add_css_class("kalam-picker-row");
            check.set_active(current.contains(&shelf.id));

            let catalog = catalog.clone();
            let on_changed = on_changed.clone();
            let shelf_id = shelf.id;
            check.connect_toggled(move |c| {
                if c.is_active() {
                    crate::notify::report(
                        catalog.add_book_to_shelf(shelf_id, book_id),
                        "Could not add to the shelf",
                    );
                } else {
                    crate::notify::report(
                        catalog.remove_book_from_shelf(shelf_id, book_id),
                        "Could not remove from the shelf",
                    );
                }
                on_changed();
            });
            list.append(&check);
        }

        let scroll = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&list)
            .build();
        root.append(&scroll);
    }

    let done = gtk::Button::with_label("Done");
    done.add_css_class("kalam-primary-btn");
    done.set_halign(gtk::Align::End);
    {
        let window = window.clone();
        done.connect_clicked(move |_| window.close());
    }
    root.append(&done);

    window.set_child(Some(&root));
    window.present();
}

#[allow(clippy::too_many_arguments)]
fn fill(
    cover_host: &gtk::Box,
    tags_box: &gtk::Box,
    title: &gtk::Label,
    author: &gtk::Label,
    series: &gtk::Label,
    format: &gtk::Label,
    progress: &gtk::Label,
    path: &gtk::Label,
    description: &gtk::Label,
    book: Option<&Book>,
) {
    while let Some(child) = cover_host.first_child() {
        cover_host.remove(&child);
    }
    while let Some(child) = tags_box.first_child() {
        tags_box.remove(&child);
    }

    if let Some(book) = book {
        let cover_w = 160;
        let cover_h = ((cover_w as f64) * 1.6) as i32;
        let cover = cover_widget(book.cover_path.as_deref(), cover_w, cover_h);
        cover.add_css_class("kalam-detail-cover");
        cover_host.append(&cover);

        title.set_label(&book.title);
        author.set_label(book.authors_display());
        // series_display() folds in the index, e.g. "Lord of the Rings #3".
        if let Some(text) = book.series_display() {
            series.set_label(&text);
            series.set_visible(true);
        } else {
            series.set_visible(false);
        }
        let mut bits = vec![book.format.as_str().to_string()];
        if !book.published.trim().is_empty() {
            bits.push(book.published.clone());
        }
        if !book.publisher.trim().is_empty() {
            bits.push(book.publisher.clone());
        }
        bits.push(format!("added {}", book.added_at));
        format.set_label(&bits.join(" · "));
        progress.set_label(&format!("{}% complete", book.progress));
        path.set_label(&book.file_path.to_string_lossy());
        let desc = crate::epub::strip_html(&book.description);
        if desc.trim().is_empty() {
            description.set_label("No description.");
        } else {
            description.set_label(&desc);
        }
        for tag in &book.tags {
            let chip = gtk::Label::new(Some(tag));
            chip.add_css_class("kalam-chip");
            tags_box.append(&chip);
        }
    } else {
        let cover = cover_widget(None, 160, 256);
        cover_host.append(&cover);
        title.set_label("Book not found");
        author.set_label("");
        series.set_visible(false);
        format.set_label("");
        progress.set_label("");
        path.set_label("");
        description.set_label("This book was removed or does not exist.");
    }
}
