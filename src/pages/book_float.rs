//! Compact floating book detail — Suwayomi-style panel adapted for ebooks.
//!
//! Layout: cover left · title/badges/actions/description right · ✕ close.
//! Not a full-height tall window.

use crate::db::Catalog;
use crate::models::Book;
use crate::widgets::book_row::cover_widget;
use gtk::prelude::*;
use relm4::prelude::*;
use std::rc::Rc;

#[derive(Debug)]
pub enum BookFloatOut {
    Close,
    OpenFullPage {
        book_id: i64,
    },
    OpenReader {
        book_id: i64,
    },
    Deleted {
        book_id: i64,
    },
}

#[derive(Debug)]
pub enum BookFloatMsg {
    Close,
    OpenFull,
    Read,
    Remove,
}

pub struct BookFloatModel {
    catalog: Rc<Catalog>,
    book: Option<Book>,
}

#[relm4::component(pub)]
impl Component for BookFloatModel {
    type Init = (Rc<Catalog>, i64);
    type Input = BookFloatMsg;
    type Output = BookFloatOut;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            add_css_class: "kalam-float",
            set_hexpand: true,
            set_vexpand: true,

            // ── Title bar: title + ✕ ──────────────────────────────
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                add_css_class: "kalam-float-header",
                set_spacing: 12,

                #[name = "header_title"]
                gtk::Label {
                    add_css_class: "kalam-float-title",
                    set_halign: gtk::Align::Start,
                    set_hexpand: true,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                },

                gtk::Button {
                    set_label: "✕",
                    add_css_class: "kalam-float-close",
                    set_tooltip_text: Some("Close (Q)"),
                    connect_clicked => BookFloatMsg::Close,
                },
            },

            // ── Body: cover | details ─────────────────────────────
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                add_css_class: "kalam-float-body",
                set_spacing: 20,
                set_hexpand: true,
                set_vexpand: true,

                // Left column: cover + secondary actions
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 10,
                    set_valign: gtk::Align::Start,

                    #[name = "cover_host"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        add_css_class: "kalam-float-cover-host",
                    },

                    gtk::Button {
                        set_label: "Open full page",
                        add_css_class: "kalam-float-side-btn",
                        set_halign: gtk::Align::Fill,
                        connect_clicked => BookFloatMsg::OpenFull,
                    },
                    gtk::Button {
                        set_label: "Remove",
                        add_css_class: "kalam-float-side-btn",
                        set_halign: gtk::Align::Fill,
                        connect_clicked => BookFloatMsg::Remove,
                    },
                },

                // Right column: badges, read, description, meta
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 10,
                    set_hexpand: true,
                    set_valign: gtk::Align::Start,

                    #[name = "badges"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 6,
                        set_halign: gtk::Align::Start,
                    },

                    gtk::Button {
                        set_label: "▶  Read",
                        add_css_class: "kalam-primary-btn",
                        add_css_class: "kalam-float-read",
                        set_halign: gtk::Align::Start,
                        connect_clicked => BookFloatMsg::Read,
                    },

                    #[name = "description"]
                    gtk::Label {
                        add_css_class: "kalam-float-desc",
                        set_halign: gtk::Align::Start,
                        set_wrap: true,
                        set_xalign: 0.0,
                        set_max_width_chars: 52,
                        set_lines: 5,
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                    },

                    #[name = "tags"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 6,
                        set_halign: gtk::Align::Start,
                    },

                    // Meta grid
                    gtk::Grid {
                        set_column_spacing: 16,
                        set_row_spacing: 6,
                        set_margin_top: 6,

                        attach[0, 0, 1, 1] = &gtk::Label {
                            set_label: "STATUS",
                            add_css_class: "kalam-float-meta-key",
                            set_halign: gtk::Align::Start,
                        },
                        #[name = "status_val"]
                        attach[1, 0, 1, 1] = &gtk::Label {
                            add_css_class: "kalam-float-meta-val",
                            set_halign: gtk::Align::Start,
                        },

                        attach[0, 1, 1, 1] = &gtk::Label {
                            set_label: "AUTHOR",
                            add_css_class: "kalam-float-meta-key",
                            set_halign: gtk::Align::Start,
                        },
                        #[name = "author_val"]
                        attach[1, 1, 1, 1] = &gtk::Label {
                            add_css_class: "kalam-float-meta-val",
                            set_halign: gtk::Align::Start,
                            set_ellipsize: gtk::pango::EllipsizeMode::End,
                            set_max_width_chars: 36,
                        },

                        attach[0, 2, 1, 1] = &gtk::Label {
                            set_label: "SERIES",
                            add_css_class: "kalam-float-meta-key",
                            set_halign: gtk::Align::Start,
                        },
                        #[name = "series_val"]
                        attach[1, 2, 1, 1] = &gtk::Label {
                            add_css_class: "kalam-float-meta-val",
                            set_halign: gtk::Align::Start,
                            set_ellipsize: gtk::pango::EllipsizeMode::End,
                            set_max_width_chars: 36,
                        },

                        attach[0, 3, 1, 1] = &gtk::Label {
                            set_label: "FORMAT",
                            add_css_class: "kalam-float-meta-key",
                            set_halign: gtk::Align::Start,
                        },
                        #[name = "format_val"]
                        attach[1, 3, 1, 1] = &gtk::Label {
                            add_css_class: "kalam-float-meta-val",
                            set_halign: gtk::Align::Start,
                        },

                        attach[0, 4, 1, 1] = &gtk::Label {
                            set_label: "ADDED",
                            add_css_class: "kalam-float-meta-key",
                            set_halign: gtk::Align::Start,
                        },
                        #[name = "added_val"]
                        attach[1, 4, 1, 1] = &gtk::Label {
                            add_css_class: "kalam-float-meta-val",
                            set_halign: gtk::Align::Start,
                        },
                    },
                },
            },
        }
    }

    fn init(
        (catalog, book_id): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let book = catalog.get_book(book_id).ok().flatten();
        let model = BookFloatModel { catalog, book };
        let widgets = view_output!();
        fill(&widgets, model.book.as_ref());

        // q / Escape close when the panel has focus
        let key = gtk::EventControllerKey::new();
        let s = sender.clone();
        key.connect_key_pressed(move |_, keyval, _keycode, _state| {
            use gtk::gdk::Key;
            if keyval == Key::q || keyval == Key::Q || keyval == Key::Escape {
                s.input(BookFloatMsg::Close);
                return gtk::glib::Propagation::Stop;
            }
            gtk::glib::Propagation::Proceed
        });
        root.add_controller(key);
        root.set_can_focus(true);
        root.grab_focus();

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
            BookFloatMsg::Close => {
                sender.output(BookFloatOut::Close).ok();
            }
            BookFloatMsg::OpenFull => {
                if let Some(b) = &self.book {
                    let id = b.id;
                    sender
                        .output(BookFloatOut::OpenFullPage { book_id: id })
                        .ok();
                }
            }
            BookFloatMsg::Read => {
                if let Some(b) = &self.book {
                    let id = b.id;
                    sender
                        .output(BookFloatOut::OpenReader { book_id: id })
                        .ok();
                }
            }
            BookFloatMsg::Remove => {
                if let Some(b) = &self.book {
                    let id = b.id;
                    if self.catalog.delete_book(id).is_ok() {
                        self.book = None;
                        sender
                            .output(BookFloatOut::Deleted { book_id: id })
                            .ok();
                    }
                }
            }
        }
        fill(widgets, self.book.as_ref());
        self.update_view(widgets, sender);
    }
}

fn fill(widgets: &BookFloatModelWidgets, book: Option<&Book>) {
    while let Some(c) = widgets.cover_host.first_child() {
        widgets.cover_host.remove(&c);
    }
    while let Some(c) = widgets.badges.first_child() {
        widgets.badges.remove(&c);
    }
    while let Some(c) = widgets.tags.first_child() {
        widgets.tags.remove(&c);
    }

    let Some(book) = book else {
        widgets.header_title.set_label("Book not found");
        widgets.description.set_label("This book was removed.");
        widgets.cover_host.append(&cover_widget(None, 140, 210));
        return;
    };

    widgets.header_title.set_label(&book.title);

    let cover = cover_widget(book.cover_path.as_deref(), 140, 210);
    cover.add_css_class("kalam-float-cover");
    widgets.cover_host.append(&cover);

    // Badges: format + progress
    let fmt = chip(book.format.as_str(), "kalam-badge-format");
    widgets.badges.append(&fmt);

    let prog = if book.progress == 0 {
        chip("UNREAD", "kalam-badge-unread")
    } else if book.progress >= 100 {
        chip("FINISHED", "kalam-badge-done")
    } else {
        chip(
            &format!("{}% READ", book.progress),
            "kalam-badge-progress",
        )
    };
    widgets.badges.append(&prog);

    if book.description.trim().is_empty() {
        widgets.description.set_label("No description.");
    } else {
        widgets.description.set_label(&book.description);
    }

    for tag in book.tags.iter().take(8) {
        let t = chip(tag, "kalam-chip");
        widgets.tags.append(&t);
    }

    let status = if book.progress >= 100 {
        "Finished"
    } else if book.progress > 0 {
        "Reading"
    } else {
        "Unread"
    };
    widgets.status_val.set_label(status);
    widgets.author_val.set_label(book.authors_display());
    widgets
        .series_val
        .set_label(book.series.as_deref().unwrap_or("—"));
    widgets.format_val.set_label(book.format.as_str());
    // Show date part of ISO timestamp if present
    let added = book.added_at.split('T').next().unwrap_or(&book.added_at);
    widgets.added_val.set_label(added);
}

fn chip(text: &str, class: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.add_css_class(class);
    l
}
