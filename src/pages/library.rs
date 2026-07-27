//! My Library — a single at-a-glance dashboard.
//!
//! Deliberately *not* a menu of buttons: every section shows real content
//! (covers, quotes, shelves, tags) and the whole page is visible by scrolling.
//! Section headers double as the way in — click a header or any card to drill
//! down. Nothing here requires a click just to find out what is behind it.

use crate::db::{Catalog, LibraryStats, SortKey};
use crate::models::{Book, LibrarySection};
use crate::widgets::book_row::{build_book_card, cover_widget, CARD_H, CARD_W};
use gtk::prelude::*;
use relm4::prelude::*;
use std::rc::Rc;

#[derive(Debug)]
pub enum LibraryOut {
    OpenSection(LibrarySection),
    OpenBook { book_id: i64 },
    OpenBookDialog { book_id: i64 },
    OpenTag { tag: String },
}

pub struct LibraryPageModel {
    catalog: Rc<Catalog>,
}

#[relm4::component(pub)]
impl SimpleComponent for LibraryPageModel {
    type Init = Rc<Catalog>;
    type Input = ();
    type Output = LibraryOut;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 18,
            set_hexpand: true,

            #[name = "body"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 22,
                set_hexpand: true,
            },
        }
    }

    fn init(
        catalog: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = LibraryPageModel { catalog };
        let widgets = view_output!();
        build_dashboard(&widgets.body, &model.catalog, &sender);
        ComponentParts { model, widgets }
    }
}

fn build_dashboard(
    body: &gtk::Box,
    catalog: &Rc<Catalog>,
    sender: &ComponentSender<LibraryPageModel>,
) {
    let stats = catalog.library_stats().unwrap_or_default();

    // ── header ──────────────────────────────────────────────────────────
    let head = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let title = gtk::Label::new(Some("My Library"));
    title.add_css_class("kalam-page-title");
    title.set_halign(gtk::Align::Start);
    head.append(&title);

    let sub = gtk::Label::new(Some(&format!(
        "{} book{} · {} reading · {} finished · {} unread",
        stats.total_books,
        if stats.total_books == 1 { "" } else { "s" },
        stats.reading,
        stats.finished,
        stats.unread
    )));
    sub.add_css_class("kalam-page-sub");
    sub.set_halign(gtk::Align::Start);
    head.append(&sub);
    body.append(&head);

    if stats.total_books == 0 {
        let empty = gtk::Label::new(Some(concat!(
            "Your library is empty.\n\n",
            "Import an EPUB from All books to get started — this page fills up ",
            "with your shelves, quotes and reading progress as you go."
        )));
        empty.add_css_class("kalam-placeholder");
        empty.set_wrap(true);
        empty.set_halign(gtk::Align::Start);
        body.append(&empty);
        body.append(&section(
            "ALL BOOKS",
            None,
            LibrarySection::AllBooks,
            sender,
            gtk::Label::new(Some("Import your first book here.")).upcast(),
        ));
        return;
    }

    // ── continue reading: covers with progress bars ─────────────────────
    let mut continuing = catalog.recently_opened(6).unwrap_or_default();
    if continuing.is_empty() {
        continuing = catalog
            .list_books(SortKey::Added, "")
            .unwrap_or_default()
            .into_iter()
            .filter(|b| b.progress > 0 && b.progress < 100)
            .take(6)
            .collect();
    }
    if !continuing.is_empty() {
        body.append(&section(
            "CONTINUE READING",
            Some(&format!("{} in progress", continuing.len())),
            LibrarySection::History,
            sender,
            progress_strip(&continuing, sender),
        ));
    }

    // ── reading list ────────────────────────────────────────────────────
    let tbr = catalog.list_reading_list().unwrap_or_default();
    if !tbr.is_empty() {
        let strip: Vec<Book> = tbr.iter().take(6).map(|e| e.book.clone()).collect();
        body.append(&section(
            "UP NEXT",
            Some(&format!("{} queued", tbr.len())),
            LibrarySection::ReadingList,
            sender,
            cover_strip(&strip, sender),
        ));
    }

    // ── shelves ─────────────────────────────────────────────────────────
    let shelves = catalog.list_shelves().unwrap_or_default();
    if !shelves.is_empty() {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        row.set_homogeneous(true);
        for shelf in shelves.iter().take(4) {
            let card = gtk::Box::new(gtk::Orientation::Vertical, 4);
            card.add_css_class("kalam-shelf-mini");

            let badge = gtk::Label::new(Some(shelf.kind.label()));
            badge.add_css_class("kalam-card-badge");
            badge.set_halign(gtk::Align::Start);
            card.append(&badge);

            let name = gtk::Label::new(Some(&shelf.name));
            name.add_css_class("kalam-card-title");
            name.set_halign(gtk::Align::Start);
            name.set_xalign(0.0);
            name.set_ellipsize(gtk::pango::EllipsizeMode::End);
            card.append(&name);

            let count = gtk::Label::new(Some(&format!(
                "{} book{}",
                shelf.book_count,
                if shelf.book_count == 1 { "" } else { "s" }
            )));
            count.add_css_class("kalam-card-meta");
            count.set_halign(gtk::Align::Start);
            card.append(&count);
            row.append(&card);
        }
        body.append(&plain_section("SHELVES", Some("Smart & manual"), row.upcast()));
    }

    // ── saved quotes ────────────────────────────────────────────────────
    let quotes = catalog.list_all_quotes("").unwrap_or_default();
    if !quotes.is_empty() {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.set_homogeneous(true);
        for anno in quotes.iter().take(2) {
            let card = gtk::Box::new(gtk::Orientation::Vertical, 6);
            card.add_css_class("kalam-quote-card");

            let text = anno.text_excerpt.trim();
            let shown = if text.chars().count() > 180 {
                let cut: String = text.chars().take(180).collect();
                format!("{cut}…")
            } else {
                text.to_string()
            };
            let quote = gtk::Label::new(Some(&shown));
            quote.add_css_class("kalam-quote-text");
            quote.set_wrap(true);
            quote.set_xalign(0.0);
            quote.set_halign(gtk::Align::Start);
            card.append(&quote);

            if let Ok(Some(book)) = catalog.get_book(anno.book_id) {
                let src = gtk::Label::new(Some(&format!("— {}", book.title)));
                src.add_css_class("kalam-quote-source");
                src.set_halign(gtk::Align::Start);
                src.set_xalign(0.0);
                src.set_ellipsize(gtk::pango::EllipsizeMode::End);
                card.append(&src);
            }
            row.append(&card);
        }
        body.append(&section(
            "QUOTES",
            Some(&format!("{} saved", quotes.len())),
            LibrarySection::SavedQuotes,
            sender,
            row.upcast(),
        ));
    }

    // ── tags ────────────────────────────────────────────────────────────
    let tags = catalog.list_tags_with_counts().unwrap_or_default();
    if !tags.is_empty() {
        let flow = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .max_children_per_line(8)
            .column_spacing(8)
            .row_spacing(8)
            .halign(gtk::Align::Start)
            .build();
        for (name, count) in tags.iter().take(12) {
            let chip = gtk::Button::new();
            chip.add_css_class("kalam-tag-chip");
            let inner = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            inner.append(&gtk::Label::new(Some(name)));
            let badge = gtk::Label::new(Some(&count.to_string()));
            badge.add_css_class("kalam-tag-count");
            inner.append(&badge);
            chip.set_child(Some(&inner));

            let tag = name.clone();
            let s = sender.clone();
            chip.connect_clicked(move |_| {
                s.output(LibraryOut::OpenTag { tag: tag.clone() }).ok();
            });
            flow.insert(&chip, -1);
        }
        body.append(&section(
            "TAGS",
            Some(&format!("{} in use", tags.len())),
            LibrarySection::Tags,
            sender,
            flow.upcast(),
        ));
    }

    // ── recently added ──────────────────────────────────────────────────
    let recent = catalog.list_books(SortKey::Added, "").unwrap_or_default();
    if !recent.is_empty() {
        let strip: Vec<Book> = recent.iter().take(6).cloned().collect();
        body.append(&section(
            "RECENTLY ADDED",
            Some(&format!("{} total", recent.len())),
            LibrarySection::AllBooks,
            sender,
            cover_strip(&strip, sender),
        ));
    }

    // ── vocabulary + analytics summary ──────────────────────────────────
    body.append(&stat_footer(&stats, sender));
}

/// Section with a clickable header that routes to the full page.
fn section(
    title: &str,
    meta: Option<&str>,
    target: LibrarySection,
    sender: &ComponentSender<LibraryPageModel>,
    content: gtk::Widget,
) -> gtk::Box {
    let wrap = gtk::Box::new(gtk::Orientation::Vertical, 10);

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    header.add_css_class("kalam-section-header");

    let label = gtk::Label::new(Some(title));
    label.add_css_class("kalam-section-label");
    label.set_halign(gtk::Align::Start);
    header.append(&label);

    if let Some(meta) = meta {
        let m = gtk::Label::new(Some(meta));
        m.add_css_class("kalam-section-meta");
        m.set_halign(gtk::Align::Start);
        m.set_hexpand(true);
        header.append(&m);
    } else {
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        header.append(&spacer);
    }

    let arrow = gtk::Label::new(Some("›"));
    arrow.add_css_class("kalam-section-arrow");
    header.append(&arrow);

    // The whole header is the affordance — no separate "Show all" button.
    let click = gtk::GestureClick::new();
    click.set_button(1);
    let s = sender.clone();
    click.connect_released(move |_, _, _, _| {
        s.output(LibraryOut::OpenSection(target)).ok();
    });
    header.add_controller(click);
    header.set_cursor_from_name(Some("pointer"));
    header.set_tooltip_text(Some(&format!("Open {}", title.to_lowercase())));

    wrap.append(&header);
    wrap.append(&content);
    wrap
}

/// Section header with no drill-down target.
fn plain_section(title: &str, meta: Option<&str>, content: gtk::Widget) -> gtk::Box {
    let wrap = gtk::Box::new(gtk::Orientation::Vertical, 10);
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);

    let label = gtk::Label::new(Some(title));
    label.add_css_class("kalam-section-label");
    label.set_halign(gtk::Align::Start);
    header.append(&label);

    if let Some(meta) = meta {
        let m = gtk::Label::new(Some(meta));
        m.add_css_class("kalam-section-meta");
        m.set_hexpand(true);
        m.set_halign(gtk::Align::Start);
        header.append(&m);
    }
    wrap.append(&header);
    wrap.append(&content);
    wrap
}

/// Horizontal run of cover cards.
fn cover_strip(books: &[Book], sender: &ComponentSender<LibraryPageModel>) -> gtk::Widget {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    row.set_halign(gtk::Align::Start);

    for book in books {
        let id = book.id;
        let s1 = sender.clone();
        let s2 = sender.clone();
        let card = build_book_card(
            book,
            move || {
                s1.output(LibraryOut::OpenBook { book_id: id }).ok();
            },
            move || {
                s2.output(LibraryOut::OpenBookDialog { book_id: id }).ok();
            },
        );
        let cell = gtk::Box::new(gtk::Orientation::Vertical, 0);
        cell.set_size_request(CARD_W, CARD_H);
        cell.append(&card);
        row.append(&cell);
    }
    row.upcast()
}

/// Covers with a progress bar and percentage underneath, like the reference.
fn progress_strip(books: &[Book], sender: &ComponentSender<LibraryPageModel>) -> gtk::Widget {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    row.set_halign(gtk::Align::Start);

    for book in books {
        let cell = gtk::Box::new(gtk::Orientation::Vertical, 6);
        cell.add_css_class("kalam-progress-cell");
        cell.set_size_request(CARD_W, -1);

        let cover = cover_widget(book.cover_path.as_deref(), CARD_W, 168);
        cell.append(&cover);

        let bar = gtk::ProgressBar::new();
        bar.set_fraction((book.progress as f64 / 100.0).clamp(0.0, 1.0));
        bar.add_css_class("kalam-mini-progress");
        cell.append(&bar);

        let pct = gtk::Label::new(Some(&format!("{}%", book.progress)));
        pct.add_css_class("kalam-progress");
        pct.set_halign(gtk::Align::End);
        cell.append(&pct);

        let title = gtk::Label::new(Some(&book.title));
        title.add_css_class("kalam-book-card-title");
        title.set_ellipsize(gtk::pango::EllipsizeMode::End);
        title.set_max_width_chars(1);
        title.set_width_chars(1);
        title.set_xalign(0.0);
        cell.append(&title);

        let id = book.id;
        let s = sender.clone();
        let click = gtk::GestureClick::new();
        click.set_button(1);
        click.connect_released(move |_, _, _, _| {
            s.output(LibraryOut::OpenBookDialog { book_id: id }).ok();
        });
        cell.add_controller(click);
        cell.set_cursor_from_name(Some("pointer"));
        cell.set_tooltip_text(Some(&book.title));

        row.append(&cell);
    }
    row.upcast()
}

/// Closing strip: vocabulary and analytics, each routing to its page.
fn stat_footer(stats: &LibraryStats, sender: &ComponentSender<LibraryPageModel>) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.set_homogeneous(true);

    let tiles: [(&str, String, LibrarySection); 4] = [
        (
            "Saved words",
            stats.saved_words.to_string(),
            LibrarySection::SavedWords,
        ),
        (
            "Highlights",
            stats.highlights.to_string(),
            LibrarySection::SavedQuotes,
        ),
        (
            "Time read",
            human_hours(stats.total_seconds),
            LibrarySection::Analytics,
        ),
        (
            "Day streak",
            stats.current_streak_days.to_string(),
            LibrarySection::Analytics,
        ),
    ];

    for (label, value, target) in tiles {
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

        let s = sender.clone();
        let click = gtk::GestureClick::new();
        click.set_button(1);
        click.connect_released(move |_, _, _, _| {
            s.output(LibraryOut::OpenSection(target)).ok();
        });
        tile.add_controller(click);
        tile.set_cursor_from_name(Some("pointer"));

        row.append(&tile);
    }
    row
}

fn human_hours(seconds: i64) -> String {
    if seconds <= 0 {
        return "—".into();
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        format!("{minutes}m")
    } else {
        format!("{}h", minutes / 60)
    }
}
