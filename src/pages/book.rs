use crate::db::Catalog;
use crate::models::Book;
use crate::widgets::book_row::cover_widget;
use gtk::prelude::*;
use relm4::prelude::*;
use std::rc::Rc;

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
}

pub struct BookPageModel {
    catalog: Rc<Catalog>,
    book: Option<Book>,
}

#[relm4::component(pub)]
impl Component for BookPageModel {
    type Init = (Rc<Catalog>, i64);
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
                        gtk::Button {
                            set_label: "Edit metadata",
                            add_css_class: "kalam-secondary-btn",
                            set_sensitive: false,
                            set_tooltip_text: Some("Coming in a later phase"),
                        },
                        gtk::Button {
                            set_label: "Remove",
                            add_css_class: "kalam-secondary-btn",
                            connect_clicked => BookPageMsg::Delete,
                        },
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
                set_label: concat!(
                    "Reader lands in P2. This page is the home for a book's ",
                    "metadata and library actions.",
                ),
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
        let model = BookPageModel { catalog, book };
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
        let _ = sender;
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
            BookPageMsg::Delete => {
                if let Some(book) = &self.book {
                    let id = book.id;
                    if self.catalog.delete_book(id).is_ok() {
                        self.book = None;
                        sender.output(BookPageOut::Deleted { book_id: id }).ok();
                    }
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
        self.update_view(widgets, sender);
    }
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
        let cover_h = (cover_w as f64 * 1.6) as i32;
        let cover = cover_widget(book.cover_path.as_deref(), cover_w, cover_h);
        cover.add_css_class("kalam-detail-cover");
        cover.set_hexpand(false);
        cover.set_vexpand(false);
        cover_host.append(&cover);

        title.set_label(&book.title);
        author.set_label(book.authors_display());
        if let Some(s) = &book.series {
            series.set_label(s);
            series.set_visible(true);
        } else {
            series.set_visible(false);
        }
        format.set_label(&format!(
            "{} · added {}",
            book.format.as_str(),
            book.added_at
        ));
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
