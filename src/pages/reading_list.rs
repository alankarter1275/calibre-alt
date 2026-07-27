//! P4 — Reading list: an ordered to-be-read queue.

use crate::db::{Catalog, ReadingListEntry, SortKey};
use crate::widgets::book_row::cover_widget;
use gtk::prelude::*;
use relm4::prelude::*;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug)]
pub enum ReadingListOut {
    OpenBook { book_id: i64 },
    OpenReader { book_id: i64 },
}

#[derive(Debug)]
pub enum ReadingListMsg {
    Move { book_id: i64, delta: i64 },
    Remove(i64),
    AddBooks,
    Refresh,
}

pub struct ReadingListModel {
    catalog: Arc<Catalog>,
    entries: Vec<ReadingListEntry>,
}

#[relm4::component(pub)]
impl Component for ReadingListModel {
    type Init = Arc<Catalog>;
    type Input = ReadingListMsg;
    type Output = ReadingListOut;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 12,
            set_hexpand: true,

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 10,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,

                    gtk::Label {
                        set_label: "Reading list",
                        add_css_class: "kalam-page-title",
                        set_halign: gtk::Align::Start,
                    },
                    gtk::Label {
                        #[watch]
                        set_label: &status_line(model.entries.len()),
                        add_css_class: "kalam-page-sub",
                        set_halign: gtk::Align::Start,
                    },
                },

                gtk::Button {
                    set_label: "+ Add books",
                    add_css_class: "kalam-primary-btn",
                    set_valign: gtk::Align::Center,
                    connect_clicked => ReadingListMsg::AddBooks,
                },
            },

            gtk::ScrolledWindow {
                set_vexpand: true,
                set_hexpand: true,
                set_hscrollbar_policy: gtk::PolicyType::Never,

                #[name = "list"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                },
            },
        }
    }

    fn init(
        catalog: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let entries = catalog.list_reading_list().unwrap_or_default();
        let model = ReadingListModel { catalog, entries };
        let widgets = view_output!();
        rebuild(&widgets.list, &model.entries, &sender);
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
            ReadingListMsg::Move { book_id, delta } => {
                let _ = self.catalog.move_reading_list_entry(book_id, delta);
                self.reload();
            }
            ReadingListMsg::Remove(book_id) => {
                crate::notify::report(
                    self.catalog.remove_from_reading_list(book_id),
                    "Could not update the reading list",
                );
                self.reload();
            }
            ReadingListMsg::AddBooks => {
                let s = sender.clone();
                open_picker(window_of(root).as_ref(), self.catalog.clone(), move || {
                    s.input(ReadingListMsg::Refresh)
                });
            }
            ReadingListMsg::Refresh => self.reload(),
        }
        rebuild(&widgets.list, &self.entries, &sender);
        self.update_view(widgets, sender);
    }
}

impl ReadingListModel {
    fn reload(&mut self) {
        self.entries = self.catalog.list_reading_list().unwrap_or_default();
    }
}

fn status_line(n: usize) -> String {
    if n == 0 {
        "Books you plan to read next, in the order you want them.".into()
    } else {
        format!("{n} book{} queued up", if n == 1 { "" } else { "s" })
    }
}

fn window_of(root: &gtk::Box) -> Option<gtk::Window> {
    root.root()
        .and_then(|r| r.downcast::<gtk::Window>().ok())
        .or_else(|| {
            relm4::main_application()
                .active_window()
                .and_then(|w| w.downcast::<gtk::Window>().ok())
        })
}

fn rebuild(
    list: &gtk::Box,
    entries: &[ReadingListEntry],
    sender: &ComponentSender<ReadingListModel>,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    if entries.is_empty() {
        let empty = gtk::Label::new(Some(concat!(
            "Your reading list is empty.\n\n",
            "Add books here from the book page, the float panel, or “+ Add books” above.",
        )));
        empty.add_css_class("kalam-placeholder");
        empty.set_wrap(true);
        empty.set_halign(gtk::Align::Start);
        list.append(&empty);
        return;
    }

    let last = entries.len().saturating_sub(1);
    for (i, entry) in entries.iter().enumerate() {
        list.append(&build_row(i, last, entry, sender));
    }
}

fn build_row(
    index: usize,
    last: usize,
    entry: &ReadingListEntry,
    sender: &ComponentSender<ReadingListModel>,
) -> gtk::Box {
    let book = &entry.book;
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.add_css_class("kalam-list-row");

    let ordinal = gtk::Label::new(Some(&format!("{}", index + 1)));
    ordinal.add_css_class("kalam-list-ordinal");
    ordinal.set_valign(gtk::Align::Center);
    ordinal.set_width_chars(2);
    row.append(&ordinal);

    let cover = cover_widget(book.cover_path.as_deref(), 44, 70);
    cover.set_valign(gtk::Align::Center);
    row.append(&cover);

    let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text.set_hexpand(true);
    text.set_valign(gtk::Align::Center);

    let title = gtk::Label::new(Some(&book.title));
    title.add_css_class("kalam-card-title");
    title.set_halign(gtk::Align::Start);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    title.set_xalign(0.0);
    text.append(&title);

    let meta = gtk::Label::new(Some(&format!(
        "{} · {}",
        book.authors_display(),
        if book.progress > 0 {
            format!("{}% read", book.progress)
        } else {
            "not started".to_string()
        }
    )));
    meta.add_css_class("kalam-card-meta");
    meta.set_halign(gtk::Align::Start);
    meta.set_ellipsize(gtk::pango::EllipsizeMode::End);
    meta.set_xalign(0.0);
    text.append(&meta);
    row.append(&text);

    let book_id = book.id;

    let up = gtk::Button::with_label("↑");
    up.add_css_class("kalam-mini-btn");
    up.set_valign(gtk::Align::Center);
    up.set_sensitive(index > 0);
    {
        let s = sender.clone();
        up.connect_clicked(move |_| s.input(ReadingListMsg::Move { book_id, delta: -1 }));
    }
    row.append(&up);

    let down = gtk::Button::with_label("↓");
    down.add_css_class("kalam-mini-btn");
    down.set_valign(gtk::Align::Center);
    down.set_sensitive(index < last);
    {
        let s = sender.clone();
        down.connect_clicked(move |_| s.input(ReadingListMsg::Move { book_id, delta: 1 }));
    }
    row.append(&down);

    let read = gtk::Button::with_label("Read");
    read.add_css_class("kalam-mini-btn");
    read.set_valign(gtk::Align::Center);
    {
        let s = sender.clone();
        read.connect_clicked(move |_| {
            s.output(ReadingListOut::OpenReader { book_id }).ok();
        });
    }
    row.append(&read);

    let remove = gtk::Button::with_label("✕");
    remove.add_css_class("kalam-mini-btn");
    remove.add_css_class("kalam-mini-btn-danger");
    remove.set_valign(gtk::Align::Center);
    remove.set_tooltip_text(Some("Remove from reading list"));
    {
        let s = sender.clone();
        remove.connect_clicked(move |_| s.input(ReadingListMsg::Remove(book_id)));
    }
    row.append(&remove);

    // Clicking the row opens the book page.
    let click = gtk::GestureClick::new();
    click.set_button(1);
    {
        let s = sender.clone();
        click.connect_released(move |_, _, _, _| {
            s.output(ReadingListOut::OpenBook { book_id }).ok();
        });
    }
    row.add_controller(click);
    row.set_cursor_from_name(Some("pointer"));

    row
}

/// Library checklist for bulk-queueing books.
fn open_picker(
    parent: Option<&gtk::Window>,
    catalog: Arc<Catalog>,
    on_changed: impl Fn() + 'static,
) {
    let window = gtk::Window::builder()
        .title("Add to reading list")
        .modal(true)
        .default_width(520)
        .default_height(560)
        .build();
    window.add_css_class("kalam-window");
    if let Some(parent) = parent {
        window.set_transient_for(Some(parent));
    }

    let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
    root.set_margin_all(16);

    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Search library…"));
    root.append(&search);

    let list = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&list)
        .build();
    root.append(&scroll);

    let on_changed = Rc::new(on_changed);
    let fill = {
        let catalog = catalog.clone();
        let list = list.clone();
        let on_changed = on_changed.clone();
        Rc::new(move |query: &str| {
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            let books = catalog
                .list_books(SortKey::Title, query)
                .unwrap_or_default();
            if books.is_empty() {
                let empty = gtk::Label::new(Some("No books match."));
                empty.add_css_class("kalam-muted");
                empty.set_halign(gtk::Align::Start);
                list.append(&empty);
                return;
            }
            for book in books {
                let check = gtk::CheckButton::with_label(&format!(
                    "{} — {}",
                    book.title,
                    book.authors_display()
                ));
                check.add_css_class("kalam-picker-row");
                check.set_active(catalog.is_in_reading_list(book.id).unwrap_or(false));

                let catalog = catalog.clone();
                let on_changed = on_changed.clone();
                let book_id = book.id;
                check.connect_toggled(move |c| {
                    if c.is_active() {
                        crate::notify::report(
                            catalog.add_to_reading_list(book_id),
                            "Could not update the reading list",
                        );
                    } else {
                        crate::notify::report(
                            catalog.remove_from_reading_list(book_id),
                            "Could not update the reading list",
                        );
                    }
                    on_changed();
                });
                list.append(&check);
            }
        })
    };

    fill("");
    {
        let fill = fill.clone();
        search.connect_search_changed(move |e| fill(&e.text()));
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
