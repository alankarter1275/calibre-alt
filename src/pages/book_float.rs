//! Floating book detail — compact book-first panel.
//!
//! Cover and reading progress live on the left; story info and actions live on
//! the right. Clicking the cover opens the full book page. Q / Esc closes.

use crate::db::Catalog;
use crate::models::Book;
use crate::pages::metadata_editor::open_metadata_editor;
use crate::service::LibraryService;
use crate::widgets::book_row::{cover_widget, invalidate_cover_cache};
use crate::widgets::charts::star_picker;
use gtk::prelude::*;
use relm4::prelude::*;
use std::sync::Arc;

const COVER_W: i32 = 120;
const COVER_H: i32 = 176;
const DESC_PREVIEW_CHARS: usize = 240;
const DESC_PREVIEW_HEIGHT: i32 = 106;
const DESC_EXPANDED_HEIGHT: i32 = 154;
const READ_MORE_HEIGHT: i32 = 20;
const DESC_SECTION_HEIGHT: i32 = DESC_EXPANDED_HEIGHT + 10 + READ_MORE_HEIGHT;

#[derive(Debug)]
pub enum BookFloatOut {
    Close,
    OpenFullPage {
        book_id: i64,
    },
    OpenReader {
        book_id: i64,
    },
    OpenAuthor {
        name: String,
    },
    /// Open the shelves checklist panel (in-app float).
    ShowShelves,
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
    OpenAuthor(String),
    Remove,
    ToggleReadingList,
    ToggleFinished,
    SetRating(u8),
    EditMetadata,
    ShowShelfMenu,
    Refresh,
    ToggleDescription,
}

pub struct BookFloatModel {
    service: LibraryService,
    book: Option<Book>,
    in_reading_list: bool,
    finished: bool,
    desc_expanded: bool,
}

#[relm4::component(pub)]
impl Component for BookFloatModel {
    type Init = (Arc<Catalog>, i64);
    type Input = BookFloatMsg;
    type Output = BookFloatOut;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            add_css_class: "kalam-float",
            set_overflow: gtk::Overflow::Hidden,
            set_hexpand: true,
            set_vexpand: true,

            #[name = "cover_col"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                add_css_class: "kalam-float-cover-col",
                set_spacing: 16,
                set_hexpand: false,
                set_vexpand: true,
                set_halign: gtk::Align::Fill,
                set_valign: gtk::Align::Fill,

                #[name = "cover_host"]
                gtk::Box {
                    add_css_class: "kalam-float-cover-host",
                    set_orientation: gtk::Orientation::Vertical,
                    set_halign: gtk::Align::Center,
                },

                #[name = "progress_wrap"]
                gtk::Box {
                    add_css_class: "kalam-float-progress-wrap",
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,

                    gtk::Box {
                        add_css_class: "kalam-float-progress-row",
                        set_orientation: gtk::Orientation::Horizontal,

                        #[name = "progress_pct"]
                        gtk::Label {
                            add_css_class: "kalam-float-progress-pct",
                            set_halign: gtk::Align::Start,
                            set_valign: gtk::Align::Center,
                            set_height_request: 18,
                            set_hexpand: true,
                            set_xalign: 0.0,
                        },

                        #[name = "progress_loc"]
                        gtk::Label {
                            add_css_class: "kalam-float-progress-loc",
                            set_halign: gtk::Align::End,
                            set_valign: gtk::Align::Center,
                            set_height_request: 18,
                            set_xalign: 1.0,
                        },
                    },

                    #[name = "progress_bar"]
                    gtk::ProgressBar {
                        add_css_class: "kalam-float-progress",
                        set_hexpand: true,
                        set_show_text: false,
                    },
                },

                #[name = "left_meta"]
                gtk::Box {
                    add_css_class: "kalam-float-facts",
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,

                    gtk::Box {
                        add_css_class: "kalam-float-fact",
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 0,

                        gtk::Label {
                            set_label: "FORMAT",
                            add_css_class: "kalam-float-fact-label",
                            set_halign: gtk::Align::Start,
                            set_valign: gtk::Align::Center,
                            set_height_request: 16,
                            set_margin_top: 2,
                            set_margin_bottom: 0,
                            set_xalign: 0.0,
                        },

                        #[name = "format_val"]
                        gtk::Label {
                            add_css_class: "kalam-float-fact-val",
                            set_halign: gtk::Align::Start,
                            set_valign: gtk::Align::Center,
                            set_height_request: 18,
                            set_margin_top: 0,
                            set_margin_bottom: 0,
                            set_xalign: 0.0,
                        },
                    },

                    gtk::Box {
                        add_css_class: "kalam-float-fact",
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 0,

                        gtk::Label {
                            set_label: "PUBLISHER",
                            add_css_class: "kalam-float-fact-label",
                            set_halign: gtk::Align::Start,
                            set_valign: gtk::Align::Center,
                            set_height_request: 16,
                            set_margin_top: 2,
                            set_margin_bottom: 0,
                            set_xalign: 0.0,
                        },

                        #[name = "publisher_val"]
                        gtk::Label {
                            add_css_class: "kalam-float-fact-val",
                            set_halign: gtk::Align::Start,
                            set_valign: gtk::Align::Center,
                            set_height_request: 18,
                            set_margin_top: 0,
                            set_margin_bottom: 0,
                            set_wrap: true,
                            set_xalign: 0.0,
                        },
                    },

                    gtk::Box {
                        add_css_class: "kalam-float-fact",
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 0,

                        gtk::Label {
                            set_label: "PUBLISHED",
                            add_css_class: "kalam-float-fact-label",
                            set_halign: gtk::Align::Start,
                            set_valign: gtk::Align::Center,
                            set_height_request: 16,
                            set_margin_top: 2,
                            set_margin_bottom: 0,
                            set_xalign: 0.0,
                        },

                        #[name = "published_val"]
                        gtk::Label {
                            add_css_class: "kalam-float-fact-val",
                            set_halign: gtk::Align::Start,
                            set_valign: gtk::Align::Center,
                            set_height_request: 18,
                            set_margin_top: 0,
                            set_margin_bottom: 0,
                            set_wrap: true,
                            set_xalign: 0.0,
                        },
                    },
                },
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                add_css_class: "kalam-float-right",
                set_hexpand: true,
                set_vexpand: true,
                set_spacing: 0,

                gtk::Box {
                    add_css_class: "kalam-float-topbar",
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 12,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 2,
                        set_hexpand: true,

                        #[name = "header_title"]
                        gtk::Label {
                            add_css_class: "kalam-float-title",
                            add_css_class: "kalam-title-serif",
                            set_halign: gtk::Align::Start,
                            set_wrap: true,
                            set_xalign: 0.0,
                        },

                        #[name = "author_val"]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_halign: gtk::Align::Start,
                            set_spacing: 0,
                            set_margin_top: 1,
                            set_margin_bottom: 1,
                        },

                        #[name = "series_val"]
                        gtk::Label {
                            add_css_class: "kalam-float-series",
                            set_halign: gtk::Align::Start,
                            set_wrap: true,
                            set_xalign: 0.0,
                        },
                    },

                    gtk::Button {
                        set_child: Some(&crate::icons::symbolic_with_classes(
                            "window-close-symbolic",
                            12,
                            &["kalam-inline-icon"],
                        )),
                        set_has_frame: false,
                        add_css_class: "kalam-float-close",
                        set_halign: gtk::Align::End,
                        set_valign: gtk::Align::Start,
                        set_vexpand: false,
                        set_tooltip_text: Some("Close (Q)"),
                        connect_clicked => BookFloatMsg::Close,
                    },
                },

                gtk::Box {
                    add_css_class: "kalam-float-body",
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 10,
                    set_hexpand: true,
                    set_vexpand: true,

                    #[name = "rating_host"]
                    gtk::Box {
                        add_css_class: "kalam-float-rating",
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,
                        set_halign: gtk::Align::Start,
                    },

                    #[name = "desc_section"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 10,
                        set_hexpand: true,
                        set_vexpand: false,

                        #[name = "desc_scroll"]
                        gtk::ScrolledWindow {
                            add_css_class: "kalam-float-desc-scroll",
                            set_hexpand: true,
                            set_vexpand: false,
                            set_hscrollbar_policy: gtk::PolicyType::Never,

                            #[name = "description"]
                            gtk::Label {
                                add_css_class: "kalam-float-desc",
                                add_css_class: "kalam-title-serif-italic",
                                set_halign: gtk::Align::Start,
                                set_valign: gtk::Align::Start,
                                set_wrap: true,
                                set_xalign: 0.0,
                                set_selectable: true,
                            },
                        },

                        #[name = "read_more_btn"]
                        gtk::Button {
                            add_css_class: "kalam-float-read-more",
                            set_halign: gtk::Align::Start,
                            connect_clicked => BookFloatMsg::ToggleDescription,
                        },

                        #[name = "desc_section_spacer"]
                        gtk::Box {
                            set_vexpand: true,
                        },
                    },

                    #[name = "tags"]
                    gtk::FlowBox {
                        add_css_class: "kalam-float-tags",
                        set_selection_mode: gtk::SelectionMode::None,
                        set_column_spacing: 6,
                        set_row_spacing: 6,
                        set_halign: gtk::Align::Start,
                        set_max_children_per_line: 8,
                    },

                    gtk::Box {
                        add_css_class: "kalam-float-actions",
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,
                        set_halign: gtk::Align::Fill,

                        #[name = "read_btn"]
                        gtk::Button {
                            set_child: Some(&crate::icons::labelled(
                                "media-playback-start-symbolic",
                                16,
                                "Read",
                                6,
                            )),
                            add_css_class: "kalam-btn-filled",
                            add_css_class: "kalam-float-read",
                            set_halign: gtk::Align::Start,
                            set_valign: gtk::Align::Center,
                            set_vexpand: false,
                            connect_clicked => BookFloatMsg::Read,
                        },

                        #[name = "tbr_btn"]
                        gtk::Button {
                            set_child: Some(&crate::icons::symbolic_with_classes(
                                "view-list-symbolic",
                                16,
                                &["kalam-inline-icon"],
                            )),
                            set_has_frame: false,
                            add_css_class: "kalam-btn-icon",
                            add_css_class: "kalam-float-icon-btn",
                            set_valign: gtk::Align::Center,
                            set_vexpand: false,
                            connect_clicked => BookFloatMsg::ToggleReadingList,
                        },

                        #[name = "shelf_btn"]
                        gtk::Button {
                            set_child: Some(&crate::icons::symbolic_with_classes(
                                "view-grid-symbolic",
                                16,
                                &["kalam-inline-icon"],
                            )),
                            set_has_frame: false,
                            add_css_class: "kalam-btn-icon",
                            add_css_class: "kalam-float-icon-btn",
                            set_valign: gtk::Align::Center,
                            set_vexpand: false,
                            connect_clicked => BookFloatMsg::ShowShelfMenu,
                        },

                        #[name = "edit_btn"]
                        gtk::Button {
                            set_child: Some(&crate::icons::symbolic_with_classes(
                                "document-edit-symbolic",
                                16,
                                &["kalam-inline-icon"],
                            )),
                            set_has_frame: false,
                            add_css_class: "kalam-btn-icon",
                            add_css_class: "kalam-float-icon-btn",
                            set_valign: gtk::Align::Center,
                            set_vexpand: false,
                            connect_clicked => BookFloatMsg::EditMetadata,
                        },

                        #[name = "finish_btn"]
                        gtk::Button {
                            set_child: Some(&crate::icons::symbolic_with_classes(
                                "object-select-symbolic",
                                16,
                                &["kalam-inline-icon"],
                            )),
                            set_has_frame: false,
                            add_css_class: "kalam-btn-icon",
                            add_css_class: "kalam-float-icon-btn",
                            set_valign: gtk::Align::Center,
                            set_vexpand: false,
                            connect_clicked => BookFloatMsg::ToggleFinished,
                        },

                        gtk::Box {
                            set_hexpand: true,
                        },

                        #[name = "remove_btn"]
                        gtk::Button {
                            set_child: Some(&crate::icons::symbolic_with_classes(
                                "edit-delete-symbolic",
                                16,
                                &["kalam-inline-icon"],
                            )),
                            set_has_frame: false,
                            add_css_class: "kalam-btn-icon",
                            add_css_class: "kalam-float-icon-btn",
                            add_css_class: "danger",
                            add_css_class: "kalam-float-icon-btn-danger",
                            set_valign: gtk::Align::Center,
                            set_vexpand: false,
                            set_tooltip_text: Some("Remove"),
                            connect_clicked => BookFloatMsg::Remove,
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
        let service = LibraryService::new(catalog);
        let snap = service.book_detail(book_id);
        report_errors(&snap.errors);
        let model = BookFloatModel {
            service,
            book: snap.book,
            in_reading_list: snap.in_reading_list,
            finished: snap.finished,
            desc_expanded: false,
        };
        let widgets = view_output!();
        root.set_size_request(720, 420);
        widgets.cover_col.set_size_request(150, -1);
        widgets.progress_wrap.set_size_request(108, -1);
        widgets.progress_wrap.set_halign(gtk::Align::Center);
        widgets.left_meta.set_size_request(120, -1);
        widgets.left_meta.set_halign(gtk::Align::Center);
        widgets.read_btn.set_size_request(-1, 40);
        widgets.tbr_btn.set_size_request(40, 40);
        widgets.shelf_btn.set_size_request(40, 40);
        widgets.edit_btn.set_size_request(40, 40);
        widgets.finish_btn.set_size_request(40, 40);
        widgets.remove_btn.set_size_request(40, 40);
        fill(&widgets, &model, &sender);

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
        root: &Self::Root,
    ) {
        match msg {
            BookFloatMsg::Close => {
                sender.output(BookFloatOut::Close).ok();
            }
            BookFloatMsg::OpenFull => {
                if let Some(book) = &self.book {
                    sender
                        .output(BookFloatOut::OpenFullPage { book_id: book.id })
                        .ok();
                }
            }
            BookFloatMsg::Read => {
                if let Some(book) = &self.book {
                    sender
                        .output(BookFloatOut::OpenReader { book_id: book.id })
                        .ok();
                }
            }
            BookFloatMsg::OpenAuthor(name) => {
                sender.output(BookFloatOut::OpenAuthor { name }).ok();
            }
            BookFloatMsg::Remove => {
                if let Some(book) = &self.book {
                    if let Some(path) = &book.cover_path {
                        invalidate_cover_cache(path);
                    }
                    let id = book.id;
                    let title = book.title.clone();
                    if crate::notify::outcome(
                        self.service.catalog().delete_book(id),
                        "Book removed",
                        &title,
                        "Could not remove the book",
                    ) {
                        self.book = None;
                        sender.output(BookFloatOut::Deleted { book_id: id }).ok();
                    }
                }
            }
            BookFloatMsg::ToggleReadingList => {
                if let Some(book) = &self.book {
                    let id = book.id;
                    let title = book.title.clone();
                    if self.in_reading_list {
                        if crate::notify::report(
                            self.service.catalog().remove_from_reading_list(id),
                            "Could not update the reading list",
                        ) {
                            crate::notify::info("Removed from reading list", &title);
                        }
                    } else if crate::notify::report(
                        self.service.catalog().add_to_reading_list(id),
                        "Could not update the reading list",
                    ) {
                        crate::notify::success("Added to reading list", &title);
                    }
                    self.reload_state(id);
                }
            }
            BookFloatMsg::ToggleFinished => {
                if let Some(book) = &self.book {
                    let id = book.id;
                    let title = book.title.clone();
                    let becoming = !self.finished;
                    if crate::notify::report(
                        self.service.catalog().set_book_finished(id, becoming),
                        "Could not update the book",
                    ) {
                        if becoming {
                            crate::notify::success("Marked as finished", &title);
                        } else {
                            crate::notify::info("Marked as unread", &title);
                        }
                    }
                    self.reload_state(id);
                }
            }
            BookFloatMsg::SetRating(half_stars) => {
                if let Some(book) = &self.book {
                    let id = book.id;
                    let detail = if half_stars == 0 {
                        "Rating cleared".to_string()
                    } else {
                        format!("{:.1} / 5", half_stars as f32 / 2.0)
                    };
                    crate::notify::outcome(
                        self.service.catalog().set_book_rating(id, half_stars),
                        "Rating saved",
                        &detail,
                        "Could not save the rating",
                    );
                    self.reload_state(id);
                }
            }
            BookFloatMsg::EditMetadata => {
                if let Some(book) = &self.book {
                    let id = book.id;
                    let s = sender.clone();
                    open_metadata_editor(root, self.service.catalog().clone(), id, move || {
                        s.input(BookFloatMsg::Refresh)
                    });
                }
            }
            BookFloatMsg::ShowShelfMenu => {
                sender.output(BookFloatOut::ShowShelves).ok();
            }
            BookFloatMsg::Refresh => {
                if let Some(book) = &self.book {
                    self.reload_state(book.id);
                }
            }
            BookFloatMsg::ToggleDescription => {
                self.desc_expanded = !self.desc_expanded;
            }
        }

        fill(widgets, self, &sender);
        self.update_view(widgets, sender);
    }
}

/// Surface read failures. Without this a database problem looked exactly
/// like "this book was deleted".
fn report_errors(errors: &[String]) {
    for err in errors {
        crate::notify::error("Could not read this book", err);
    }
}

impl BookFloatModel {
    fn reload_state(&mut self, book_id: i64) {
        let snap = self.service.book_detail(book_id);
        report_errors(&snap.errors);
        self.book = snap.book;
        self.in_reading_list = snap.in_reading_list;
        self.finished = snap.finished;
    }
}

fn fill(
    widgets: &BookFloatModelWidgets,
    model: &BookFloatModel,
    sender: &ComponentSender<BookFloatModel>,
) {
    clear_box(&widgets.cover_host);
    clear_box(&widgets.author_val);
    clear_flowbox(&widgets.tags);
    clear_box(&widgets.rating_host);

    let has_book = model.book.is_some();
    widgets.read_btn.set_visible(has_book);
    widgets.tbr_btn.set_visible(has_book);
    widgets.shelf_btn.set_visible(has_book);
    widgets.edit_btn.set_visible(has_book);
    widgets.finish_btn.set_visible(has_book);
    widgets.remove_btn.set_visible(has_book);
    widgets.progress_wrap.set_visible(has_book);
    widgets.left_meta.set_visible(has_book);
    widgets.rating_host.set_visible(has_book);
    widgets.desc_section.set_visible(has_book);
    widgets.desc_scroll.set_visible(has_book);
    widgets.tags.set_visible(has_book);
    widgets.desc_section_spacer.set_visible(false);

    let Some(book) = model.book.as_ref() else {
        widgets.header_title.set_label("Book not found");
        widgets.series_val.set_visible(false);
        widgets.description.set_label("This book was removed.");
        widgets.read_more_btn.set_visible(false);
        widgets.progress_pct.set_label("");
        widgets.progress_loc.set_label("");
        widgets.progress_bar.set_fraction(0.0);
        widgets.format_val.set_label("");
        widgets.publisher_val.set_label("");
        widgets.published_val.set_label("");
        widgets
            .cover_host
            .append(&build_cover_display(None, false, sender));
        return;
    };

    widgets.header_title.set_label(&book.title);

    let tx = sender.input_sender().clone();
    crate::widgets::author_links::replace_author_links(
        &widgets.author_val,
        book.authors_display(),
        "kalam-author-link-float",
        std::rc::Rc::new(move |name| {
            let _ = tx.send(BookFloatMsg::OpenAuthor(name));
        }),
    );

    if let Some(series) = book.series_display() {
        widgets.series_val.set_label(&series);
        widgets.series_val.set_visible(true);
    } else {
        widgets.series_val.set_visible(false);
    }

    widgets.cover_host.append(&build_cover_display(
        book.cover_path.as_deref(),
        true,
        sender,
    ));

    let progress_fraction = (book.progress as f64 / 100.0).clamp(0.0, 1.0);
    widgets.progress_bar.set_fraction(progress_fraction);
    widgets
        .progress_pct
        .set_label(&format!("{}%", book.progress.min(100)));
    widgets
        .progress_loc
        .set_label(&progress_location_text(model.service.catalog(), book));

    widgets.format_val.set_label(book.format.as_str());
    widgets.publisher_val.set_label(blank_dash(&book.publisher));
    widgets.published_val.set_label(blank_dash(&book.published));

    let full_desc = clean_description(book);
    let (desc_text, can_expand) = description_preview(&full_desc, model.desc_expanded);
    let expanded = model.desc_expanded && can_expand;
    widgets.description.set_label(&desc_text);
    widgets.desc_scroll.set_vexpand(false);
    widgets.read_more_btn.set_height_request(READ_MORE_HEIGHT);
    if can_expand {
        widgets.desc_section.set_height_request(DESC_SECTION_HEIGHT);
        widgets.read_more_btn.set_visible(true);
        widgets
            .read_more_btn
            .set_label(if expanded { "Show less" } else { "Read more" });
        if expanded {
            widgets.desc_section_spacer.set_visible(false);
            widgets.desc_scroll.set_propagate_natural_height(false);
            widgets.desc_scroll.set_min_content_height(-1);
            widgets
                .desc_scroll
                .set_max_content_height(DESC_EXPANDED_HEIGHT);
            widgets.desc_scroll.set_height_request(DESC_EXPANDED_HEIGHT);
            widgets
                .desc_scroll
                .set_vscrollbar_policy(gtk::PolicyType::Automatic);
        } else {
            widgets.desc_section_spacer.set_visible(true);
            widgets.desc_scroll.set_propagate_natural_height(false);
            widgets.desc_scroll.set_min_content_height(-1);
            widgets
                .desc_scroll
                .set_max_content_height(DESC_PREVIEW_HEIGHT);
            widgets.desc_scroll.set_height_request(-1);
            widgets
                .desc_scroll
                .set_vscrollbar_policy(gtk::PolicyType::Never);
        }
    } else {
        widgets.desc_section.set_height_request(-1);
        widgets.desc_section_spacer.set_visible(false);
        widgets.read_more_btn.set_visible(false);
        widgets.desc_scroll.set_propagate_natural_height(true);
        widgets.desc_scroll.set_min_content_height(-1);
        widgets.desc_scroll.set_max_content_height(-1);
        widgets.desc_scroll.set_height_request(-1);
        widgets
            .desc_scroll
            .set_vscrollbar_policy(gtk::PolicyType::Never);
    }

    for tag in book.tags.iter().take(12) {
        let t = chip(tag, "kalam-chip");
        widgets.tags.insert(&t, -1);
    }
    widgets.tags.set_visible(!book.tags.is_empty());

    let s = sender.clone();
    widgets
        .rating_host
        .append(&star_picker(book.rating, move |v| {
            s.input(BookFloatMsg::SetRating(v))
        }));
    let rating_text = gtk::Label::new(Some(&rating_text(book)));
    rating_text.add_css_class("kalam-float-rating-text");
    rating_text.set_valign(gtk::Align::Center);
    widgets.rating_host.append(&rating_text);

    sync_action_buttons(widgets, model);
}

fn build_cover_display(
    path: Option<&std::path::Path>,
    clickable: bool,
    sender: &ComponentSender<BookFloatModel>,
) -> gtk::Widget {
    let shell = gtk::Overlay::new();
    shell.add_css_class("kalam-float-book-shell");
    shell.set_size_request(COVER_W + 6, COVER_H + 6);
    shell.set_halign(gtk::Align::Center);
    shell.set_valign(gtk::Align::Start);

    let edge = gtk::Box::new(gtk::Orientation::Vertical, 0);
    edge.add_css_class("kalam-float-book-edge");
    edge.set_size_request(COVER_W, COVER_H);
    edge.set_halign(gtk::Align::Start);
    edge.set_valign(gtk::Align::Start);
    edge.set_margin_start(5);
    edge.set_margin_top(5);
    shell.set_child(Some(&edge));

    let cover = cover_widget(path, COVER_W, COVER_H);
    cover.add_css_class("kalam-float-cover");
    cover.set_halign(gtk::Align::Start);
    cover.set_valign(gtk::Align::Start);
    shell.add_overlay(&cover);

    if clickable {
        cover.set_cursor_from_name(Some("pointer"));
        let click = gtk::GestureClick::new();
        let tx = sender.input_sender().clone();
        click.connect_released(move |_, _, _, _| {
            let _ = tx.send(BookFloatMsg::OpenFull);
        });
        shell.add_controller(click);
        shell.set_tooltip_text(Some("Open full details"));
        shell.set_cursor_from_name(Some("pointer"));
    }

    shell.upcast()
}

fn sync_action_buttons(widgets: &BookFloatModelWidgets, model: &BookFloatModel) {
    widgets.tbr_btn.remove_css_class("active");
    widgets.finish_btn.remove_css_class("active");
    widgets.finish_btn.remove_css_class("done-active");

    if model.in_reading_list {
        widgets.tbr_btn.add_css_class("active");
        widgets
            .tbr_btn
            .set_tooltip_text(Some("Remove from reading list"));
    } else {
        widgets
            .tbr_btn
            .set_tooltip_text(Some("Add to reading list"));
    }

    widgets.shelf_btn.set_tooltip_text(Some("Shelves"));
    widgets.edit_btn.set_tooltip_text(Some("Edit metadata"));

    if model.finished {
        widgets.finish_btn.add_css_class("active");
        widgets.finish_btn.add_css_class("done-active");
        widgets.finish_btn.set_tooltip_text(Some("Mark unread"));
    } else {
        widgets.finish_btn.set_tooltip_text(Some("Mark finished"));
    }
}

fn progress_location_text(catalog: &Catalog, book: &Book) -> String {
    if book.progress >= 100 {
        return "Finished".into();
    }
    if let Ok(Some((chapter, _fraction))) = catalog.get_reading_progress(book.id) {
        return format!("Ch. {}", chapter + 1);
    }
    if book.progress > 0 {
        "In progress".into()
    } else {
        "Not started".into()
    }
}

fn blank_dash(text: &str) -> &str {
    let text = text.trim();
    if text.is_empty() {
        "—"
    } else {
        text
    }
}

fn clean_description(book: &Book) -> String {
    let desc = crate::epub::strip_html(&book.description);
    let desc = desc.trim();
    if desc.is_empty() {
        "No description.".into()
    } else {
        desc.to_string()
    }
}

fn description_preview(full: &str, expanded: bool) -> (String, bool) {
    if full == "No description." || full.chars().count() <= DESC_PREVIEW_CHARS {
        return (full.to_string(), false);
    }
    if expanded {
        return (full.to_string(), true);
    }
    (truncate_text(full, DESC_PREVIEW_CHARS), true)
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    let total = text.chars().count();
    if total <= max_chars {
        return text.to_string();
    }
    let mut out = String::new();
    for ch in text.chars().take(max_chars) {
        out.push(ch);
    }
    while out.chars().last().is_some_and(char::is_whitespace) {
        out.pop();
    }
    out.push('…');
    out
}

fn rating_text(book: &Book) -> String {
    match book.rating_stars() {
        Some(v) => format!("{v:.1} / 5"),
        None => "Not rated yet".into(),
    }
}

fn chip(text: &str, class: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.add_css_class(class);
    l
}

fn clear_box(host: &gtk::Box) {
    while let Some(child) = host.first_child() {
        host.remove(&child);
    }
}

fn clear_flowbox(host: &gtk::FlowBox) {
    while let Some(child) = host.first_child() {
        host.remove(&child);
    }
}
