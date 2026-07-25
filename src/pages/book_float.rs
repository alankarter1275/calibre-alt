//! Floating book detail — Suwayomi-style panel adapted for ebooks.
//!
//! Tall cover column on the left (full panel height), details on the right,
//! Read button bottom-right. ✕ / Q / Esc to close.

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
        #[allow(dead_code)]
        book_id: i64,
    },
    Deleted {
        #[allow(dead_code)]
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
            set_orientation: gtk::Orientation::Horizontal,
            add_css_class: "kalam-float",
            set_hexpand: true,
            set_vexpand: true,

            // ── LEFT: fixed-width cover column ───────────────────
            #[name = "cover_col"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                add_css_class: "kalam-float-cover-col",
                set_vexpand: true,
                set_hexpand: false,
                set_valign: gtk::Align::Fill,

                #[name = "cover_host"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    add_css_class: "kalam-float-cover-host",
                    set_vexpand: true,
                    set_hexpand: true,
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Fill,
                },

                // Side actions under cover (like Suwayomi left rail)
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    add_css_class: "kalam-float-side-actions",
                    set_spacing: 4,

                    gtk::Button {
                        set_label: "  Open full page",
                        add_css_class: "kalam-float-side-btn",
                        set_halign: gtk::Align::Fill,
                        connect_clicked => BookFloatMsg::OpenFull,
                    },
                    gtk::Button {
                        set_label: "  Remove",
                        add_css_class: "kalam-float-side-btn",
                        set_halign: gtk::Align::Fill,
                        connect_clicked => BookFloatMsg::Remove,
                    },
                },
            },

            // ── RIGHT: details ───────────────────────────────────
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                add_css_class: "kalam-float-right",
                set_hexpand: true,
                set_vexpand: true,
                set_spacing: 0,

                // Header: title + ✕
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

                gtk::ScrolledWindow {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_propagate_natural_height: true,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        add_css_class: "kalam-float-body",
                        set_spacing: 12,
                        set_hexpand: true,

                        #[name = "badges"]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 6,
                            set_halign: gtk::Align::Start,
                        },

                        #[name = "description"]
                        gtk::Label {
                            add_css_class: "kalam-float-desc",
                            set_halign: gtk::Align::Start,
                            set_wrap: true,
                            set_xalign: 0.0,
                            set_max_width_chars: 56,
                        },

                        #[name = "tags"]
                        gtk::FlowBox {
                            set_selection_mode: gtk::SelectionMode::None,
                            set_max_children_per_line: 8,
                            set_min_children_per_line: 2,
                            set_column_spacing: 6,
                            set_row_spacing: 6,
                            set_halign: gtk::Align::Start,
                        },

                        gtk::Grid {
                            set_column_spacing: 28,
                            set_row_spacing: 8,
                            set_margin_top: 8,

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
                            attach[2, 0, 1, 1] = &gtk::Label {
                                set_label: "AUTHOR",
                                add_css_class: "kalam-float-meta-key",
                                set_halign: gtk::Align::Start,
                            },
                            #[name = "author_val"]
                            attach[3, 0, 1, 1] = &gtk::Label {
                                add_css_class: "kalam-float-meta-val",
                                set_halign: gtk::Align::Start,
                                set_ellipsize: gtk::pango::EllipsizeMode::End,
                                set_max_width_chars: 28,
                            },

                            attach[0, 1, 1, 1] = &gtk::Label {
                                set_label: "SERIES",
                                add_css_class: "kalam-float-meta-key",
                                set_halign: gtk::Align::Start,
                            },
                            #[name = "series_val"]
                            attach[1, 1, 1, 1] = &gtk::Label {
                                add_css_class: "kalam-float-meta-val",
                                set_halign: gtk::Align::Start,
                                set_ellipsize: gtk::pango::EllipsizeMode::End,
                                set_max_width_chars: 28,
                            },
                            attach[2, 1, 1, 1] = &gtk::Label {
                                set_label: "FORMAT",
                                add_css_class: "kalam-float-meta-key",
                                set_halign: gtk::Align::Start,
                            },
                            #[name = "format_val"]
                            attach[3, 1, 1, 1] = &gtk::Label {
                                add_css_class: "kalam-float-meta-val",
                                set_halign: gtk::Align::Start,
                            },

                            attach[0, 2, 1, 1] = &gtk::Label {
                                set_label: "ADDED",
                                add_css_class: "kalam-float-meta-key",
                                set_halign: gtk::Align::Start,
                            },
                            #[name = "added_val"]
                            attach[1, 2, 1, 1] = &gtk::Label {
                                add_css_class: "kalam-float-meta-val",
                                set_halign: gtk::Align::Start,
                            },
                        },
                    },
                },

                // Footer: Read bottom-right
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    add_css_class: "kalam-float-footer",

                    gtk::Box {
                        set_hexpand: true,
                    },

                    gtk::Button {
                        set_label: "▶  Read",
                        add_css_class: "kalam-primary-btn",
                        add_css_class: "kalam-float-read",
                        set_halign: gtk::Align::End,
                        connect_clicked => BookFloatMsg::Read,
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
        // Pin left rail width so the cover never stretches with the window.
        widgets.cover_col.set_size_request(228, -1);
        widgets.cover_col.set_hexpand(false);
        fill(&widgets, model.book.as_ref());

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
        let ph = cover_widget(None, 200, 300);
        ph.add_css_class("kalam-float-cover");
        widgets.cover_host.append(&ph);
        return;
    };

    widgets.header_title.set_label(&book.title);

    // Fixed portrait cover in the left column (does not blow up the panel).
    let cover = cover_widget(book.cover_path.as_deref(), 200, 300);
    cover.add_css_class("kalam-float-cover");
    cover.set_hexpand(false);
    cover.set_vexpand(false);
    cover.set_halign(gtk::Align::Center);
    widgets.cover_host.append(&cover);

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

    let desc = crate::epub::strip_html(&book.description);
    if desc.trim().is_empty() {
        widgets.description.set_label("No description.");
    } else {
        widgets.description.set_label(&desc);
    }

    for tag in book.tags.iter().take(12) {
        let t = chip(tag, "kalam-chip");
        widgets.tags.insert(&t, -1);
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
    let added = book.added_at.split('T').next().unwrap_or(&book.added_at);
    widgets.added_val.set_label(added);
}

fn chip(text: &str, class: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.add_css_class(class);
    l
}
