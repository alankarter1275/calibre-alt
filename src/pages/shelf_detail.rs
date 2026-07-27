//! P4 — one shelf: its books, plus membership editing for manual shelves.

use crate::db::{Catalog, Shelf, ShelfKind, SortKey};
use crate::models::Book;
use crate::pages::shelf_editor::{open_shelf_editor, ShelfEditorMode};
use crate::widgets::book_row::build_book_grid;
use gtk::prelude::*;
use relm4::prelude::*;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug)]
pub enum ShelfDetailOut {
    OpenBook { book_id: i64 },
    OpenBookDialog { book_id: i64 },
}

#[derive(Debug)]
pub enum ShelfDetailMsg {
    SearchChanged(String),
    SortChanged(SortKey),
    EditShelf,
    AddBooks,
    Refresh,
}

pub struct ShelfDetailModel {
    catalog: Arc<Catalog>,
    shelf: Option<Shelf>,
    books: Vec<Book>,
    query: String,
    sort: SortKey,
}

#[relm4::component(pub)]
impl Component for ShelfDetailModel {
    type Init = (Arc<Catalog>, i64);
    type Input = ShelfDetailMsg;
    type Output = ShelfDetailOut;
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

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,

                        #[name = "title"]
                        gtk::Label {
                            add_css_class: "kalam-page-title",
                            set_halign: gtk::Align::Start,
                        },
                        #[name = "kind_badge"]
                        gtk::Label {
                            add_css_class: "kalam-card-badge",
                            set_valign: gtk::Align::Center,
                        },
                    },
                    #[name = "subtitle"]
                    gtk::Label {
                        add_css_class: "kalam-page-sub",
                        set_halign: gtk::Align::Start,
                        set_xalign: 0.0,
                        set_wrap: true,
                    },
                },

                #[name = "add_btn"]
                gtk::Button {
                    set_label: "+ Add books",
                    add_css_class: "kalam-secondary-btn",
                    set_valign: gtk::Align::Center,
                    connect_clicked => ShelfDetailMsg::AddBooks,
                },
                gtk::Button {
                    set_label: "Edit",
                    add_css_class: "kalam-secondary-btn",
                    set_valign: gtk::Align::Center,
                    connect_clicked => ShelfDetailMsg::EditShelf,
                },
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 8,

                gtk::SearchEntry {
                    set_hexpand: true,
                    set_placeholder_text: Some("Search this shelf…"),
                    connect_search_changed[sender] => move |e| {
                        sender.input(ShelfDetailMsg::SearchChanged(e.text().to_string()));
                    },
                },

                #[name = "sort_box"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 4,
                },
            },

            #[name = "status"]
            gtk::Label {
                add_css_class: "kalam-muted",
                set_halign: gtk::Align::Start,
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
        (catalog, shelf_id): Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let shelf = catalog.get_shelf(shelf_id).ok().flatten();
        let sort = SortKey::Added;
        let books = shelf
            .as_ref()
            .and_then(|s| catalog.shelf_books(s, sort, "").ok())
            .unwrap_or_default();

        let model = ShelfDetailModel {
            catalog,
            shelf,
            books,
            query: String::new(),
            sort,
        };
        let widgets = view_output!();

        for key in SortKey::ALL {
            let label = if *key == SortKey::Added && model.is_manual() {
                // Manual shelves keep their hand-sorted order under "Added".
                "Shelf order"
            } else {
                key.label()
            };
            let btn = gtk::ToggleButton::with_label(label);
            btn.add_css_class("kalam-secondary-btn");
            if *key == SortKey::Added {
                btn.set_active(true);
            }
            let k = *key;
            let s = sender.clone();
            btn.connect_toggled(move |b| {
                if b.is_active() {
                    s.input(ShelfDetailMsg::SortChanged(k));
                }
            });
            widgets.sort_box.append(&btn);
        }
        group_toggles(&widgets.sort_box);

        model.refresh_header(&widgets);
        model.rebuild(&widgets, &sender);
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
            ShelfDetailMsg::SearchChanged(q) => {
                self.query = q;
                self.reload();
            }
            ShelfDetailMsg::SortChanged(sort) => {
                self.sort = sort;
                self.reload();
            }
            ShelfDetailMsg::EditShelf => {
                if let Some(shelf) = &self.shelf {
                    let s = sender.clone();
                    open_shelf_editor(
                        window_of(root).as_ref(),
                        self.catalog.clone(),
                        ShelfEditorMode::Edit { shelf_id: shelf.id },
                        move || s.input(ShelfDetailMsg::Refresh),
                    );
                }
            }
            ShelfDetailMsg::AddBooks => {
                if let Some(shelf) = &self.shelf {
                    if shelf.kind == ShelfKind::Manual {
                        let s = sender.clone();
                        open_book_picker(
                            window_of(root).as_ref(),
                            self.catalog.clone(),
                            shelf.id,
                            move || s.input(ShelfDetailMsg::Refresh),
                        );
                    }
                }
            }
            ShelfDetailMsg::Refresh => {
                if let Some(shelf) = &self.shelf {
                    self.shelf = self.catalog.get_shelf(shelf.id).ok().flatten();
                }
                self.reload();
            }
        }

        self.refresh_header(widgets);
        self.rebuild(widgets, &sender);
        self.update_view(widgets, sender);
    }
}

impl ShelfDetailModel {
    fn is_manual(&self) -> bool {
        self.shelf
            .as_ref()
            .map(|s| s.kind == ShelfKind::Manual)
            .unwrap_or(false)
    }

    fn reload(&mut self) {
        self.books = self
            .shelf
            .as_ref()
            .and_then(|s| self.catalog.shelf_books(s, self.sort, &self.query).ok())
            .unwrap_or_default();
    }

    fn refresh_header(&self, widgets: &ShelfDetailModelWidgets) {
        match &self.shelf {
            Some(shelf) => {
                widgets.title.set_label(&shelf.name);
                widgets.kind_badge.set_label(shelf.kind.label());
                widgets.kind_badge.set_visible(true);
                widgets.subtitle.set_label(&shelf.summary());
                // Only manual shelves have membership to edit.
                widgets.add_btn.set_visible(shelf.kind == ShelfKind::Manual);

                let n = self.books.len();
                widgets.status.set_label(&format!(
                    "{n} book{} · click cover for float · Ctrl+click for full page",
                    if n == 1 { "" } else { "s" }
                ));
            }
            None => {
                widgets.title.set_label("Shelf not found");
                widgets.kind_badge.set_visible(false);
                widgets.subtitle.set_label("This shelf was deleted.");
                widgets.add_btn.set_visible(false);
                widgets.status.set_label("");
            }
        }
    }

    fn rebuild(&self, widgets: &ShelfDetailModelWidgets, sender: &ComponentSender<Self>) {
        let list = &widgets.list;
        while let Some(child) = list.first_child() {
            list.remove(&child);
        }

        if self.shelf.is_none() {
            return;
        }

        if self.books.is_empty() {
            let empty = gtk::Label::new(Some(if !self.query.trim().is_empty() {
                "Nothing on this shelf matches your search."
            } else if self.is_manual() {
                "This shelf is empty — use “+ Add books” to put something on it."
            } else {
                "No books match these rules yet. Use “Edit” to adjust them."
            }));
            empty.add_css_class("kalam-placeholder");
            empty.set_wrap(true);
            empty.set_halign(gtk::Align::Start);
            list.append(&empty);
            return;
        }

        let s1 = sender.clone();
        let s2 = sender.clone();
        let grid = build_book_grid(
            &self.books,
            move |id| {
                s1.output(ShelfDetailOut::OpenBook { book_id: id }).ok();
            },
            move |id| {
                s2.output(ShelfDetailOut::OpenBookDialog { book_id: id })
                    .ok();
            },
        );
        list.append(&grid);

        // Manual shelves get a per-book remove/reorder strip under the grid.
        if self.is_manual() && self.query.trim().is_empty() {
            let manage = gtk::Expander::new(Some("Manage shelf order"));
            manage.add_css_class("kalam-manage-expander");
            let rows = gtk::Box::new(gtk::Orientation::Vertical, 4);
            rows.set_margin_top(8);

            let shelf_id = self.shelf.as_ref().map(|s| s.id).unwrap_or(0);
            for book in &self.books {
                rows.append(&self.manage_row(shelf_id, book, sender));
            }
            manage.set_child(Some(&rows));
            list.append(&manage);
        }
    }

    fn manage_row(&self, shelf_id: i64, book: &Book, sender: &ComponentSender<Self>) -> gtk::Box {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.add_css_class("kalam-manage-row");

        let title = gtk::Label::new(Some(&book.title));
        title.set_halign(gtk::Align::Start);
        title.set_hexpand(true);
        title.set_ellipsize(gtk::pango::EllipsizeMode::End);
        row.append(&title);

        let book_id = book.id;
        let sortable = self.sort == SortKey::Added;

        let up = gtk::Button::with_label("↑");
        up.add_css_class("kalam-mini-btn");
        up.set_sensitive(sortable);
        up.set_tooltip_text(Some(if sortable {
            "Move up"
        } else {
            "Switch to “Shelf order” to reorder"
        }));
        {
            let catalog = self.catalog.clone();
            let s = sender.clone();
            up.connect_clicked(move |_| {
                let _ = catalog.move_shelf_book(shelf_id, book_id, -1);
                s.input(ShelfDetailMsg::Refresh);
            });
        }
        row.append(&up);

        let down = gtk::Button::with_label("↓");
        down.add_css_class("kalam-mini-btn");
        down.set_sensitive(sortable);
        down.set_tooltip_text(Some("Move down"));
        {
            let catalog = self.catalog.clone();
            let s = sender.clone();
            down.connect_clicked(move |_| {
                let _ = catalog.move_shelf_book(shelf_id, book_id, 1);
                s.input(ShelfDetailMsg::Refresh);
            });
        }
        row.append(&down);

        let remove = gtk::Button::with_label("Remove");
        remove.add_css_class("kalam-mini-btn");
        remove.add_css_class("kalam-mini-btn-danger");
        {
            let catalog = self.catalog.clone();
            let s = sender.clone();
            remove.connect_clicked(move |_| {
                crate::notify::report(
                    catalog.remove_book_from_shelf(shelf_id, book_id),
                    "Could not remove from the shelf",
                );
                s.input(ShelfDetailMsg::Refresh);
            });
        }
        row.append(&remove);

        row
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

fn group_toggles(box_: &gtk::Box) {
    let mut leader: Option<gtk::ToggleButton> = None;
    let mut child = box_.first_child();
    while let Some(w) = child {
        let next = w.next_sibling();
        if let Ok(btn) = w.downcast::<gtk::ToggleButton>() {
            match &leader {
                Some(l) => btn.set_group(Some(l)),
                None => leader = Some(btn),
            }
        }
        child = next;
    }
}

/// Checklist of every book in the library, ticked for the ones already on this
/// manual shelf. Toggling writes straight through to the DB.
fn open_book_picker(
    parent: Option<&gtk::Window>,
    catalog: Arc<Catalog>,
    shelf_id: i64,
    on_changed: impl Fn() + 'static,
) {
    let window = gtk::Window::builder()
        .title("Add books to shelf")
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

    let hint = gtk::Label::new(Some("Tick the books that belong on this shelf."));
    hint.add_css_class("kalam-muted");
    hint.set_halign(gtk::Align::Start);
    root.append(&hint);

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
            let on_shelf: Vec<i64> = catalog
                .get_shelf(shelf_id)
                .ok()
                .flatten()
                .and_then(|s| catalog.shelf_books(&s, SortKey::Title, "").ok())
                .unwrap_or_default()
                .iter()
                .map(|b| b.id)
                .collect();

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
                check.set_active(on_shelf.contains(&book.id));

                let catalog = catalog.clone();
                let on_changed = on_changed.clone();
                let book_id = book.id;
                check.connect_toggled(move |c| {
                    if c.is_active() {
                        let _ = catalog.add_book_to_shelf(shelf_id, book_id);
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
