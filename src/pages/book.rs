use crate::models::book_by_id;
use gtk::prelude::*;
use relm4::prelude::*;

#[derive(Debug)]
pub enum BookPageOut {
    Back,
    OpenReader,
}

pub struct BookPageModel {
    #[allow(dead_code)]
    book_id: u64,
}

#[relm4::component(pub)]
impl SimpleComponent for BookPageModel {
    type Init = u64;
    type Input = ();
    type Output = BookPageOut;

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

                // Cover
                gtk::Box {
                    add_css_class: "kalam-detail-cover",
                    set_valign: gtk::Align::Start,
                },

                // Metadata column
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
                set_label: "Reader, highlights, and dictionary land in P2–P3. This page is the permanent home for a book's metadata and actions.",
                add_css_class: "kalam-placeholder",
                set_wrap: true,
                set_margin_top: 8,
            },
        }
    }

    fn init(
        book_id: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = BookPageModel { book_id };
        let widgets = view_output!();

        if let Some(book) = book_by_id(book_id) {
            widgets.title.set_label(&book.title);
            widgets.author.set_label(&book.authors.join(", "));
            if let Some(series) = &book.series {
                widgets.series.set_label(series);
                widgets.series.set_visible(true);
            } else {
                widgets.series.set_visible(false);
            }
            widgets
                .format
                .set_label(&format!("{} · added {}", book.format, book.added));
            widgets
                .progress
                .set_label(&format!("{}% complete", book.progress));
            widgets.path.set_label(book.path);
            widgets.description.set_label(&book.description);

            for tag in &book.tags {
                let chip = gtk::Label::new(Some(tag));
                chip.add_css_class("kalam-chip");
                widgets.tags.append(&chip);
            }
        } else {
            widgets.title.set_label("Unknown book");
            widgets
                .description
                .set_label("This demo id is not in the sample data.");
        }

        ComponentParts { model, widgets }
    }
}
