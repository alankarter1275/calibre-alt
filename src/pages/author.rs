use crate::author::{
    self, AuthorQuote, SeriesProgress, display_author_name, initials, line_text, status_counts,
    works_not_in_library,
};
use crate::db::{AuthorProfile, Catalog};
use crate::models::Book;
use crate::widgets::book_row::build_book_grid;
use gtk::prelude::*;
use relm4::prelude::*;
use std::sync::Arc;

#[derive(Debug)]
pub enum AuthorPageOut {
    OpenBook { book_id: i64 },
    OpenBookDialog { book_id: i64 },
}

#[derive(Debug)]
pub enum AuthorPageMsg {
    Refresh,
    Fetched(Result<AuthorProfile, String>),
}

pub struct AuthorPageModel {
    catalog: Arc<Catalog>,
    requested_name: String,
    profile: Option<AuthorProfile>,
    owned_books: Vec<Book>,
    series: Vec<SeriesProgress>,
    quotes: Vec<AuthorQuote>,
    loading: bool,
    error: Option<String>,
}

#[relm4::component(pub)]
impl Component for AuthorPageModel {
    type Init = (Arc<Catalog>, String);
    type Input = AuthorPageMsg;
    type Output = AuthorPageOut;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 16,
            set_hexpand: true,
            set_vexpand: false,

            #[name = "page_title"]
            gtk::Label {
                add_css_class: "kalam-page-title",
                set_halign: gtk::Align::Start,
                set_wrap: true,
                set_xalign: 0.0,
            },

            #[name = "page_sub"]
            gtk::Label {
                add_css_class: "kalam-page-sub",
                set_halign: gtk::Align::Start,
                set_wrap: true,
                set_xalign: 0.0,
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 12,
                set_halign: gtk::Align::Start,

                #[name = "refresh_btn"]
                gtk::Button {
                    set_label: "Refresh author info",
                    add_css_class: "kalam-secondary-btn",
                    connect_clicked => AuthorPageMsg::Refresh,
                },

                #[name = "status_label"]
                gtk::Label {
                    add_css_class: "kalam-muted",
                    set_halign: gtk::Align::Start,
                    set_valign: gtk::Align::Center,
                    set_wrap: true,
                    set_xalign: 0.0,
                },
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 16,
                set_hexpand: true,

                #[name = "hero_main"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    add_css_class: "kalam-author-hero",
                    set_spacing: 10,
                    set_hexpand: true,
                },

                #[name = "hero_side"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 12,
                    set_hexpand: false,
                },
            },

            gtk::Label {
                set_label: "YOUR BOOKS",
                add_css_class: "kalam-section-label",
                set_halign: gtk::Align::Start,
            },

            #[name = "owned_host"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 8,
            },

            gtk::Label {
                set_label: "SERIES TRACKER",
                add_css_class: "kalam-section-label",
                set_halign: gtk::Align::Start,
            },

            #[name = "series_host"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 8,
            },

            gtk::Label {
                set_label: "MORE BY THIS AUTHOR",
                add_css_class: "kalam-section-label",
                set_halign: gtk::Align::Start,
            },

            #[name = "works_host"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 8,
            },

            gtk::Label {
                set_label: "YOUR SAVED QUOTES",
                add_css_class: "kalam-section-label",
                set_halign: gtk::Align::Start,
            },

            #[name = "quotes_host"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 8,
                set_margin_bottom: 12,
            },
        }
    }

    fn init(
        (catalog, author_name): Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let owned_books = author::owned_books_for_author(&catalog, &author_name);
        let profile = catalog.get_author_profile_by_name(&author_name).ok().flatten();
        let quotes = author::saved_quotes_for_books(&catalog, &owned_books, 6);
        let series = author::series_progress(&owned_books);
        let loading = profile.is_none();

        let model = AuthorPageModel {
            catalog,
            requested_name: author_name,
            profile,
            owned_books,
            series,
            quotes,
            loading,
            error: None,
        };
        let mut widgets = view_output!();
        fill_author_page(&mut widgets, &model, &sender);
        if model.profile.is_none() {
            spawn_author_fetch(
                model.catalog.clone(),
                model.requested_name.clone(),
                model.owned_books.clone(),
                &sender,
            );
        }
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
            AuthorPageMsg::Refresh => {
                self.loading = true;
                self.error = None;
                spawn_author_fetch(
                    self.catalog.clone(),
                    self.requested_name.clone(),
                    self.owned_books.clone(),
                    &sender,
                );
            }
            AuthorPageMsg::Fetched(result) => {
                self.loading = false;
                match result {
                    Ok(profile) => {
                        self.profile = Some(profile);
                        self.error = None;
                    }
                    Err(err) => {
                        self.error = Some(err);
                    }
                }
                self.owned_books =
                    author::owned_books_for_author(&self.catalog, &self.requested_name);
                self.series = author::series_progress(&self.owned_books);
                self.quotes = author::saved_quotes_for_books(&self.catalog, &self.owned_books, 6);
            }
        }

        fill_author_page(widgets, self, &sender);
        self.update_view(widgets, sender);
    }
}

fn spawn_author_fetch(
    catalog: Arc<Catalog>,
    author_name: String,
    owned_books: Vec<Book>,
    sender: &ComponentSender<AuthorPageModel>,
) {
    let tx = sender.input_sender().clone();
    std::thread::spawn(move || {
        let result = author::fetch_and_cache_author(&catalog, &author_name, &owned_books);
        let _ = tx.send(AuthorPageMsg::Fetched(result));
    });
}

fn fill_author_page(
    widgets: &mut AuthorPageModelWidgets,
    model: &AuthorPageModel,
    sender: &ComponentSender<AuthorPageModel>,
) {
    let display_name = model
        .profile
        .as_ref()
        .map(|profile| profile.canonical_name.clone())
        .unwrap_or_else(|| display_author_name(&model.requested_name));
    widgets.page_title.set_label(&display_name);
    widgets.page_sub.set_label("Books you own, saved quotes, and more works kept offline after fetch.");
    widgets.refresh_btn.set_sensitive(!model.loading);

    let mut status_bits = Vec::new();
    if model.loading {
        status_bits.push("Loading online author info…".to_string());
    } else if let Some(profile) = &model.profile {
        if !profile.fetched_at.trim().is_empty() {
            let day = profile
                .fetched_at
                .split('T')
                .next()
                .unwrap_or(profile.fetched_at.as_str());
            status_bits.push(format!("Last updated {day}"));
        }
        if !profile.source_url.trim().is_empty() {
            status_bits.push("Source: Open Library".to_string());
        }
    }
    if let Some(err) = &model.error {
        status_bits.push(err.clone());
    }
    widgets.status_label.set_label(&line_text(&status_bits));

    rebuild_hero(&widgets.hero_main, &widgets.hero_side, model);
    rebuild_owned_books(&widgets.owned_host, &model.owned_books, sender);
    rebuild_series(&widgets.series_host, &model.series);
    rebuild_works(&widgets.works_host, model);
    rebuild_quotes(&widgets.quotes_host, &model.quotes);
}

fn rebuild_hero(main_host: &gtk::Box, side_host: &gtk::Box, model: &AuthorPageModel) {
    clear_box(main_host);
    clear_box(side_host);

    let display_name = model
        .profile
        .as_ref()
        .map(|profile| profile.canonical_name.clone())
        .unwrap_or_else(|| display_author_name(&model.requested_name));
    let title = gtk::Label::new(Some(&display_name));
    title.add_css_class("kalam-author-name");
    title.set_halign(gtk::Align::Start);
    title.set_wrap(true);
    title.set_xalign(0.0);
    main_host.append(&title);

    let facts = hero_facts(model);
    if !facts.is_empty() {
        let meta = gtk::Label::new(Some(&facts));
        meta.add_css_class("kalam-author-meta");
        meta.set_halign(gtk::Align::Start);
        meta.set_wrap(true);
        meta.set_xalign(0.0);
        main_host.append(&meta);
    }

    let bio = model
        .profile
        .as_ref()
        .map(|profile| profile.bio.trim())
        .filter(|bio| !bio.is_empty())
        .unwrap_or("No online bio yet. You can still browse your books and saved quotes below.");
    let bio_label = gtk::Label::new(Some(bio));
    bio_label.add_css_class("kalam-author-bio");
    bio_label.set_halign(gtk::Align::Start);
    bio_label.set_wrap(true);
    bio_label.set_xalign(0.0);
    main_host.append(&bio_label);

    if let Some(profile) = &model.profile {
        if !profile.top_subjects.is_empty() {
            let chips = gtk::FlowBox::builder()
                .selection_mode(gtk::SelectionMode::None)
                .column_spacing(6)
                .row_spacing(6)
                .halign(gtk::Align::Start)
                .build();
            for subject in profile.top_subjects.iter().take(8) {
                let chip = gtk::Label::new(Some(subject));
                chip.add_css_class("kalam-chip");
                chips.insert(&chip, -1);
            }
            main_host.append(&chips);
        }
    }

    let stats = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    stats.set_homogeneous(true);
    let (owned, finished, reading) = status_counts(&model.owned_books);
    for (label, value) in [
        ("Owned", owned.to_string()),
        ("Finished", finished.to_string()),
        ("Reading", reading.to_string()),
        ("Quotes", model.quotes.len().to_string()),
    ] {
        let tile = gtk::Box::new(gtk::Orientation::Vertical, 2);
        tile.add_css_class("kalam-author-stat");
        let value_label = gtk::Label::new(Some(&value));
        value_label.add_css_class("kalam-author-stat-value");
        value_label.set_halign(gtk::Align::Start);
        tile.append(&value_label);
        let label_widget = gtk::Label::new(Some(label));
        label_widget.add_css_class("kalam-author-stat-label");
        label_widget.set_halign(gtk::Align::Start);
        tile.append(&label_widget);
        stats.append(&tile);
    }
    main_host.append(&stats);

    let photo_card = gtk::Box::new(gtk::Orientation::Vertical, 10);
    photo_card.add_css_class("kalam-author-side-card");
    photo_card.set_size_request(220, -1);
    let photo = author_photo(model);
    photo_card.append(&photo);

    if let Some(profile) = &model.profile {
        if !profile.top_work.trim().is_empty() {
            let top_work_title = gtk::Label::new(Some("Top work"));
            top_work_title.add_css_class("kalam-detail-section-title");
            top_work_title.set_halign(gtk::Align::Start);
            photo_card.append(&top_work_title);

            let top_work = gtk::Label::new(Some(&profile.top_work));
            top_work.add_css_class("kalam-card-title");
            top_work.set_halign(gtk::Align::Start);
            top_work.set_wrap(true);
            top_work.set_xalign(0.0);
            photo_card.append(&top_work);
        }
    }

    let side_note = gtk::Label::new(Some(side_note_text(model)));
    side_note.add_css_class("kalam-author-side-note");
    side_note.set_halign(gtk::Align::Start);
    side_note.set_wrap(true);
    side_note.set_xalign(0.0);
    photo_card.append(&side_note);

    side_host.append(&photo_card);
}

fn rebuild_owned_books(
    host: &gtk::Box,
    books: &[Book],
    sender: &ComponentSender<AuthorPageModel>,
) {
    clear_box(host);
    if books.is_empty() {
        host.append(&placeholder(
            "No books by this author are in your library yet.",
        ));
        return;
    }
    let summary = gtk::Label::new(Some(&format!(
        "{} book{} in your library. Click for the floating details panel. Ctrl+click for the full page.",
        books.len(),
        if books.len() == 1 { "" } else { "s" }
    )));
    summary.add_css_class("kalam-muted");
    summary.set_halign(gtk::Align::Start);
    host.append(&summary);

    let s1 = sender.clone();
    let s2 = sender.clone();
    host.append(&build_book_grid(
        books,
        move |book_id| {
            s1.output(AuthorPageOut::OpenBook { book_id }).ok();
        },
        move |book_id| {
            s2.output(AuthorPageOut::OpenBookDialog { book_id }).ok();
        },
    ));
}

fn rebuild_series(host: &gtk::Box, series: &[SeriesProgress]) {
    clear_box(host);
    if series.is_empty() {
        host.append(&placeholder(
            "No series info yet from the books you own by this author.",
        ));
        return;
    }

    for entry in series {
        let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
        card.add_css_class("kalam-card");

        let name = gtk::Label::new(Some(&entry.name));
        name.add_css_class("kalam-card-title");
        name.set_halign(gtk::Align::Start);
        card.append(&name);

        let owned = entry
            .owned
            .iter()
            .map(|book| {
                if book.series_index > 0.0 {
                    format!("#{} {}", trim_series_index(book.series_index), book.title)
                } else {
                    book.title.clone()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let owned_label = gtk::Label::new(Some(&owned));
        owned_label.add_css_class("kalam-muted");
        owned_label.set_halign(gtk::Align::Start);
        owned_label.set_wrap(true);
        owned_label.set_xalign(0.0);
        card.append(&owned_label);

        if entry.missing.is_empty() {
            let ok = gtk::Label::new(Some("No gap seen in the series numbers you already own."));
            ok.add_css_class("kalam-card-meta");
            ok.set_halign(gtk::Align::Start);
            card.append(&ok);
        } else {
            let chips = gtk::FlowBox::builder()
                .selection_mode(gtk::SelectionMode::None)
                .column_spacing(6)
                .row_spacing(6)
                .halign(gtk::Align::Start)
                .build();
            for missing in &entry.missing {
                let chip = gtk::Label::new(Some(missing));
                chip.add_css_class("kalam-card-badge");
                chips.insert(&chip, -1);
            }
            card.append(&chips);
        }

        host.append(&card);
    }
}

fn rebuild_works(host: &gtk::Box, model: &AuthorPageModel) {
    clear_box(host);
    let Some(profile) = &model.profile else {
        host.append(&placeholder("No online works have been loaded yet."));
        return;
    };
    let works = works_not_in_library(profile, &model.owned_books);
    if works.is_empty() {
        host.append(&placeholder(
            "No extra works to show yet, or all fetched works are already in your library.",
        ));
        return;
    }

    let flow = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .column_spacing(12)
        .row_spacing(12)
        .halign(gtk::Align::Start)
        .build();

    for work in works {
        let card = gtk::Box::new(gtk::Orientation::Vertical, 6);
        card.add_css_class("kalam-card");
        card.add_css_class("kalam-author-work-card");

        let title = gtk::Label::new(Some(&work.title));
        title.add_css_class("kalam-card-title");
        title.set_halign(gtk::Align::Start);
        title.set_wrap(true);
        title.set_xalign(0.0);
        card.append(&title);

        let mut bits = Vec::new();
        if let Some(year) = work.first_publish_year {
            bits.push(year.to_string());
        }
        if !work.subjects.is_empty() {
            bits.push(work.subjects.join(" · "));
        }
        let meta = gtk::Label::new(Some(&line_text(&bits)));
        meta.add_css_class("kalam-card-meta");
        meta.set_halign(gtk::Align::Start);
        meta.set_wrap(true);
        meta.set_xalign(0.0);
        card.append(&meta);

        flow.insert(&card, -1);
    }

    host.append(&flow);
}

fn rebuild_quotes(host: &gtk::Box, quotes: &[AuthorQuote]) {
    clear_box(host);
    if quotes.is_empty() {
        host.append(&placeholder(
            "No saved quotes yet from the books you own by this author.",
        ));
        return;
    }

    for quote in quotes {
        let card = gtk::Box::new(gtk::Orientation::Vertical, 6);
        card.add_css_class("kalam-card");

        let text = gtk::Label::new(Some(&format!("“{}”", quote.excerpt)));
        text.add_css_class("kalam-author-quote");
        text.set_halign(gtk::Align::Start);
        text.set_wrap(true);
        text.set_xalign(0.0);
        card.append(&text);

        let meta = gtk::Label::new(Some(&quote.book_title));
        meta.add_css_class("kalam-card-meta");
        meta.set_halign(gtk::Align::Start);
        meta.set_xalign(0.0);
        card.append(&meta);

        host.append(&card);
    }
}

fn hero_facts(model: &AuthorPageModel) -> String {
    let Some(profile) = &model.profile else {
        return String::new();
    };
    let mut bits = Vec::new();
    match (profile.birth_date.trim(), profile.death_date.trim()) {
        ("", "") => {}
        (birth, "") => bits.push(format!("Born {birth}")),
        ("", death) => bits.push(format!("Died {death}")),
        (birth, death) => bits.push(format!("{birth}–{death}")),
    }
    if profile.work_count > 0 {
        bits.push(format!("{} works found", profile.work_count));
    }
    if !profile.aliases.is_empty() {
        bits.push(format!("{} known name forms", profile.aliases.len()));
    }
    line_text(&bits)
}

fn side_note_text(model: &AuthorPageModel) -> &'static str {
    if model.quotes.is_empty() {
        "When you save quotes from this author’s books, they will show up here too."
    } else {
        "This page mixes your own library with fetched author info and keeps it locally for later."
    }
}

fn author_photo(model: &AuthorPageModel) -> gtk::Widget {
    if let Some(profile) = &model.profile {
        if let Some(path) = profile.photo_path.as_deref() {
            if path.is_file() {
                let photo = crate::widgets::book_row::cover_widget(Some(path), 220, 260);
                photo.add_css_class("kalam-author-photo");
                return photo;
            }
        }
    }

    let placeholder = gtk::Box::new(gtk::Orientation::Vertical, 0);
    placeholder.add_css_class("kalam-author-photo-placeholder");
    placeholder.set_size_request(220, 260);
    placeholder.set_halign(gtk::Align::Center);
    placeholder.set_valign(gtk::Align::Start);

    let label = gtk::Label::new(Some(&initials(
        model
            .profile
            .as_ref()
            .map(|profile| profile.canonical_name.as_str())
            .unwrap_or(model.requested_name.as_str()),
    )));
    label.add_css_class("kalam-author-photo-initials");
    label.set_halign(gtk::Align::Center);
    label.set_valign(gtk::Align::Center);
    placeholder.set_vexpand(false);
    placeholder.append(&label);
    placeholder.upcast()
}

fn trim_series_index(value: f32) -> String {
    if value.fract().abs() < f32::EPSILON {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

fn placeholder(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("kalam-placeholder");
    label.set_halign(gtk::Align::Start);
    label.set_wrap(true);
    label.set_xalign(0.0);
    label
}

fn clear_box(host: &gtk::Box) {
    while let Some(child) = host.first_child() {
        host.remove(&child);
    }
}
