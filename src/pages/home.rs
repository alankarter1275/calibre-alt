use crate::db::Catalog;
use crate::widgets::book_row::{build_book_card, CARD_H, CARD_W};
use gtk::prelude::*;
use relm4::prelude::*;
use std::sync::Arc;

#[derive(Debug)]
pub enum HomeOut {
    OpenBook { book_id: i64 },
    OpenBookDialog { book_id: i64 },
}

pub struct HomePageModel;

#[relm4::component(pub)]
impl SimpleComponent for HomePageModel {
    type Init = Arc<Catalog>;
    type Input = ();
    type Output = HomeOut;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 16,
            set_hexpand: true,
            set_vexpand: false,
            set_margin_all: 0,

            gtk::Label {
                set_label: "Home",
                add_css_class: "kalam-page-title",
                set_halign: gtk::Align::Start,
            },
            gtk::Label {
                set_label: "Continue reading and recently added · click = float · Ctrl+click = full page",
                add_css_class: "kalam-page-sub",
                set_halign: gtk::Align::Start,
            },

            #[name = "counts_host"]
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 12,
                set_homogeneous: true,
                set_margin_bottom: 4,
            },

            gtk::Label {
                set_label: "CONTINUE",
                add_css_class: "kalam-section-label",
                set_halign: gtk::Align::Start,
            },

            #[name = "continue_host"]
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 16,
                set_halign: gtk::Align::Start,
                set_valign: gtk::Align::Start,
                set_hexpand: true,
                set_vexpand: false,
                set_margin_bottom: 8,
            },

            #[name = "tbr_label"]
            gtk::Label {
                set_label: "UP NEXT",
                add_css_class: "kalam-section-label",
                set_halign: gtk::Align::Start,
            },

            #[name = "tbr_host"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 6,
                set_margin_bottom: 8,
            },

            gtk::Label {
                set_label: "RECENTLY ADDED",
                add_css_class: "kalam-section-label",
                set_halign: gtk::Align::Start,
            },

            #[name = "recent_host"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_halign: gtk::Align::Fill,
                set_hexpand: true,
                set_vexpand: false,
                set_spacing: 0,
            },
        }
    }

    fn init(
        catalog: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = HomePageModel;
        let widgets = view_output!();

        // Bounded: Home shows a dozen covers, not the whole library.
        let books = catalog.recent_books(12).unwrap_or_default();

        // ── counts strip ────────────────────────────────────────────────
        let stats = catalog.library_stats().unwrap_or_default();
        for (label, value) in [
            ("Books", stats.total_books.to_string()),
            ("Reading", stats.reading.to_string()),
            ("Finished", stats.finished.to_string()),
            ("Up next", stats.reading_list.to_string()),
        ] {
            let tile = gtk::Box::new(gtk::Orientation::Vertical, 2);
            tile.add_css_class("kalam-stat-tile");
            let v = gtk::Label::new(Some(&value));
            v.add_css_class("kalam-stat-value");
            v.set_halign(gtk::Align::Start);
            tile.append(&v);
            let l = gtk::Label::new(Some(label));
            l.add_css_class("kalam-stat-label");
            l.set_halign(gtk::Align::Start);
            tile.append(&l);
            widgets.counts_host.append(&tile);
        }

        // ── continue: most recently opened, newest first ────────────────
        let mut cont: Vec<_> = catalog.recently_opened(4).unwrap_or_default();
        if cont.is_empty() {
            // Fall back to anything part-read, then to the newest import.
            cont = books
                .iter()
                .filter(|b| b.progress > 0 && b.progress < 100)
                .take(4)
                .cloned()
                .collect();
        }
        if cont.is_empty() {
            if let Some(first) = books.first() {
                cont.push(first.clone());
            }
        }

        if cont.is_empty() {
            let empty = gtk::Label::new(Some(
                "Nothing to continue — import books from My Library → All books.",
            ));
            empty.add_css_class("kalam-placeholder");
            empty.set_wrap(true);
            empty.set_halign(gtk::Align::Start);
            widgets.continue_host.append(&empty);
        } else {
            for book in &cont {
                let id = book.id;
                let s1 = sender.clone();
                let s2 = sender.clone();
                let card = build_book_card(
                    book,
                    move || {
                        s1.output(HomeOut::OpenBook { book_id: id }).ok();
                    },
                    move || {
                        s2.output(HomeOut::OpenBookDialog { book_id: id }).ok();
                    },
                );
                widgets.continue_host.append(&card);
            }
        }

        // ── reading list peek ───────────────────────────────────────────
        let tbr = catalog.list_reading_list().unwrap_or_default();
        if tbr.is_empty() {
            widgets.tbr_label.set_visible(false);
            widgets.tbr_host.set_visible(false);
        } else {
            for (i, entry) in tbr.iter().take(3).enumerate() {
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                row.add_css_class("kalam-list-row");

                let ordinal = gtk::Label::new(Some(&format!("{}", i + 1)));
                ordinal.add_css_class("kalam-list-ordinal");
                ordinal.set_width_chars(2);
                row.append(&ordinal);

                let title = gtk::Label::new(Some(&entry.book.title));
                title.add_css_class("kalam-card-title");
                title.set_halign(gtk::Align::Start);
                title.set_hexpand(true);
                title.set_xalign(0.0);
                title.set_ellipsize(gtk::pango::EllipsizeMode::End);
                row.append(&title);

                let author = gtk::Label::new(Some(entry.book.authors_display()));
                author.add_css_class("kalam-card-meta");
                row.append(&author);

                let id = entry.book.id;
                let s = sender.clone();
                let click = gtk::GestureClick::new();
                click.set_button(1);
                click.connect_released(move |_, _, _, _| {
                    s.output(HomeOut::OpenBook { book_id: id }).ok();
                });
                row.add_controller(click);
                row.set_cursor_from_name(Some("pointer"));

                widgets.tbr_host.append(&row);
            }
        }

        let recent: Vec<_> = books.iter().take(12).cloned().collect();
        if recent.is_empty() {
            let empty = gtk::Label::new(Some("Your library is empty."));
            empty.add_css_class("kalam-muted");
            empty.set_halign(gtk::Align::Start);
            widgets.recent_host.append(&empty);
        } else {
            // FlowBox left-aligned to avoid centered covers
            let flow = gtk::FlowBox::builder()
                .max_children_per_line(6)
                .min_children_per_line(2)
                .selection_mode(gtk::SelectionMode::None)
                .column_spacing(16)
                .row_spacing(20)
                .halign(gtk::Align::Start)
                .valign(gtk::Align::Start)
                .hexpand(true)
                .vexpand(false)
                .build();
            flow.add_css_class("kalam-book-grid");
            flow.add_css_class("kalam-home-flow");

            for book in &recent {
                let id = book.id;
                let s1 = sender.clone();
                let s2 = sender.clone();
                let card = build_book_card(
                    book,
                    {
                        let s = s1.clone();
                        move || {
                            s.output(HomeOut::OpenBook { book_id: id }).ok();
                        }
                    },
                    {
                        let s = s2.clone();
                        move || {
                            s.output(HomeOut::OpenBookDialog { book_id: id }).ok();
                        }
                    },
                );
                // Wrap card in fixed cell to keep uniform size in FlowBox
                let cell = gtk::Box::new(gtk::Orientation::Vertical, 0);
                cell.set_size_request(CARD_W, CARD_H);
                cell.set_halign(gtk::Align::Start);
                cell.set_valign(gtk::Align::Start);
                cell.append(&card);
                flow.insert(&cell, -1);
            }
            widgets.recent_host.append(&flow);
        }

        ComponentParts { model, widgets }
    }
}
