//! P5 — edit metadata, replace the cover, fetch from Open Library.
//!
//! A standalone `gtk::Window` rather than a Relm4 component, matching
//! `shelf_editor`: the Open Library results list is built and rebuilt
//! dynamically, which is far simpler with direct widget handling.
//!
//! Network work runs on a worker thread and reports back through a channel on
//! the main context, so the dialog never blocks the UI. Fetched values are
//! staged into the form for review — nothing is written to the catalog until
//! you press Save.

use crate::db::Catalog;
use crate::openlibrary::{self, Candidate};
use crate::widgets::book_row::cover_widget;
use gtk::prelude::*;
use relm4::RelmWidgetExt;
use std::cell::RefCell;
use std::rc::Rc;

/// What the worker thread sends back to the UI.
enum FetchMsg {
    Results(Vec<Candidate>),
    Failed(String),
    CoverReady(Vec<u8>),
    CoverFailed(String),
}

/// Open the editor for `book_id`. `on_saved` runs after a successful write.
pub fn open_metadata_editor(
    parent: Option<&gtk::Window>,
    catalog: Rc<Catalog>,
    book_id: i64,
    on_saved: impl Fn() + 'static,
) {
    let Ok(Some(book)) = catalog.get_book(book_id) else {
        return;
    };

    let window = gtk::Window::builder()
        .title("Edit metadata")
        .modal(true)
        .default_width(760)
        .default_height(640)
        .build();
    window.add_css_class("kalam-window");
    if let Some(parent) = parent {
        window.set_transient_for(Some(parent));
    }

    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    root.set_margin_all(18);

    // ── top: cover on the left, fields on the right ─────────────────────
    let top = gtk::Box::new(gtk::Orientation::Horizontal, 18);

    let cover_col = gtk::Box::new(gtk::Orientation::Vertical, 8);
    cover_col.set_valign(gtk::Align::Start);
    let cover_host = gtk::Box::new(gtk::Orientation::Vertical, 0);
    cover_host.append(&cover_widget(book.cover_path.as_deref(), 150, 240));
    cover_col.append(&cover_host);

    let pick_cover = gtk::Button::with_label("Replace cover…");
    pick_cover.add_css_class("kalam-secondary-btn");
    cover_col.append(&pick_cover);
    top.append(&cover_col);

    let fields = gtk::Box::new(gtk::Orientation::Vertical, 8);
    fields.set_hexpand(true);

    let title_entry = labelled_entry(&fields, "TITLE", &book.title);
    let authors_entry = labelled_entry(&fields, "AUTHORS", &book.authors);
    let series_entry = labelled_entry(
        &fields,
        "SERIES",
        book.series.as_deref().unwrap_or_default(),
    );
    let tags_entry = labelled_entry(&fields, "TAGS (COMMA SEPARATED)", &book.tags.join(", "));

    let desc_label = section_label("DESCRIPTION");
    fields.append(&desc_label);
    let desc_view = gtk::TextView::new();
    desc_view.set_wrap_mode(gtk::WrapMode::WordChar);
    desc_view.add_css_class("kalam-desc-view");
    desc_view
        .buffer()
        .set_text(&crate::epub::strip_html(&book.description));
    let desc_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(110)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&desc_view)
        .build();
    desc_scroll.add_css_class("kalam-desc-scroll");
    fields.append(&desc_scroll);

    top.append(&fields);
    root.append(&top);

    // ── Open Library ────────────────────────────────────────────────────
    root.append(&section_label("FETCH FROM OPEN LIBRARY"));

    let search_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let search_entry = gtk::Entry::new();
    search_entry.set_hexpand(true);
    search_entry.set_placeholder_text(Some("Title and author…"));
    // Seed with what we already know so one click usually suffices.
    search_entry.set_text(
        &format!("{} {}", book.title, book.authors)
            .trim()
            .to_string(),
    );
    search_row.append(&search_entry);

    let search_btn = gtk::Button::with_label("Search");
    search_btn.add_css_class("kalam-secondary-btn");
    search_row.append(&search_btn);
    root.append(&search_row);

    let status = gtk::Label::new(Some(
        "Searching sends the text above to openlibrary.org. Nothing is changed until you save.",
    ));
    status.add_css_class("kalam-muted");
    status.set_halign(gtk::Align::Start);
    status.set_wrap(true);
    status.set_xalign(0.0);
    root.append(&status);

    let results = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let results_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(150)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&results)
        .build();
    root.append(&results_scroll);

    // ── actions ─────────────────────────────────────────────────────────
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    actions.set_halign(gtk::Align::End);
    actions.set_margin_top(4);
    let cancel = gtk::Button::with_label("Cancel");
    cancel.add_css_class("kalam-secondary-btn");
    let save = gtk::Button::with_label("Save");
    save.add_css_class("kalam-primary-btn");
    actions.append(&cancel);
    actions.append(&save);
    root.append(&actions);

    window.set_child(Some(&root));

    // Cover bytes fetched from Open Library, written only on Save.
    let pending_cover: Rc<RefCell<Option<Vec<u8>>>> = Rc::new(RefCell::new(None));

    // ── worker channel ──────────────────────────────────────────────────
    let (tx, rx) = async_channel::unbounded::<FetchMsg>();

    {
        let status = status.clone();
        let results = results.clone();
        let title_entry = title_entry.clone();
        let authors_entry = authors_entry.clone();
        let series_entry = series_entry.clone();
        let tags_entry = tags_entry.clone();
        let desc_view = desc_view.clone();
        let cover_host = cover_host.clone();
        let pending_cover = pending_cover.clone();
        let tx_inner = tx.clone();

        gtk::glib::spawn_future_local(async move {
            while let Ok(msg) = rx.recv().await {
                match msg {
                    FetchMsg::Results(list) => {
                        rebuild_results(
                            &results,
                            &list,
                            &status,
                            &title_entry,
                            &authors_entry,
                            &series_entry,
                            &tags_entry,
                            &desc_view,
                            &tx_inner,
                        );
                        status.set_label(&if list.is_empty() {
                            "No matches. Try a different title or author.".to_string()
                        } else {
                            format!(
                                "{} match{} — “Use this” fills the form for review.",
                                list.len(),
                                if list.len() == 1 { "" } else { "es" }
                            )
                        });
                    }
                    FetchMsg::Failed(err) => {
                        status.set_label(&format!("Lookup failed. {err}"));
                    }
                    FetchMsg::CoverReady(bytes) => {
                        // Preview immediately; the file is written on Save.
                        if let Some(texture) = texture_from_bytes(&bytes) {
                            while let Some(c) = cover_host.first_child() {
                                cover_host.remove(&c);
                            }
                            let pic = gtk::Picture::for_paintable(&texture);
                            pic.set_size_request(150, 240);
                            pic.set_content_fit(gtk::ContentFit::Fill);
                            cover_host.append(&pic);
                        }
                        *pending_cover.borrow_mut() = Some(bytes);
                        status.set_label("Cover downloaded — press Save to keep it.");
                    }
                    FetchMsg::CoverFailed(err) => {
                        status.set_label(&format!("Could not download that cover. {err}"));
                    }
                }
            }
        });
    }

    // ── search ──────────────────────────────────────────────────────────
    {
        let entry = search_entry.clone();
        let status = status.clone();
        let tx = tx.clone();
        // Rc so both the button and Enter can trigger the same logic; a plain
        // move closure capturing widgets is not Clone.
        let run: Rc<dyn Fn()> = Rc::new(move || {
            let query = entry.text().to_string();
            if query.trim().is_empty() {
                status.set_label("Type something to search for.");
                return;
            }
            status.set_label("Searching Open Library…");
            let tx = tx.clone();
            // Blocking HTTP on a worker thread keeps the dialog responsive.
            std::thread::spawn(move || {
                let msg = match openlibrary::search(&query, 10) {
                    Ok(list) => FetchMsg::Results(list),
                    Err(err) => FetchMsg::Failed(err.to_string()),
                };
                let _ = tx.send_blocking(msg);
            });
        });
        let run2 = run.clone();
        search_btn.connect_clicked(move |_| run());
        search_entry.connect_activate(move |_| run2());
    }

    // ── replace cover from disk ─────────────────────────────────────────
    {
        let window = window.clone();
        let cover_host = cover_host.clone();
        let pending_cover = pending_cover.clone();
        let status = status.clone();
        pick_cover.connect_clicked(move |_| {
            let dialog = gtk::FileDialog::builder()
                .title("Choose a cover image")
                .modal(true)
                .build();
            let filter = gtk::FileFilter::new();
            filter.set_name(Some("Images"));
            for suffix in ["png", "jpg", "jpeg", "webp", "gif"] {
                filter.add_suffix(suffix);
            }
            let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&filter);
            dialog.set_filters(Some(&filters));

            let cover_host = cover_host.clone();
            let pending_cover = pending_cover.clone();
            let status = status.clone();
            dialog.open(Some(&window), gtk::gio::Cancellable::NONE, move |res| {
                let Ok(file) = res else { return };
                let Some(path) = file.path() else { return };
                match std::fs::read(&path) {
                    Ok(bytes) if !bytes.is_empty() => {
                        if let Some(texture) = texture_from_bytes(&bytes) {
                            while let Some(c) = cover_host.first_child() {
                                cover_host.remove(&c);
                            }
                            let pic = gtk::Picture::for_paintable(&texture);
                            pic.set_size_request(150, 240);
                            pic.set_content_fit(gtk::ContentFit::Fill);
                            cover_host.append(&pic);
                        }
                        *pending_cover.borrow_mut() = Some(bytes);
                        status.set_label("Cover selected — press Save to keep it.");
                    }
                    Ok(_) => status.set_label("That image file is empty."),
                    Err(err) => status.set_label(&format!("Could not read that file: {err}")),
                }
            });
        });
    }

    {
        let window = window.clone();
        cancel.connect_clicked(move |_| window.close());
    }

    // ── save ────────────────────────────────────────────────────────────
    {
        let window = window.clone();
        let catalog = catalog.clone();
        let status = status.clone();
        let pending_cover = pending_cover.clone();
        let on_saved = Rc::new(on_saved);

        save.connect_clicked(move |_| {
            let title = title_entry.text().trim().to_string();
            if title.is_empty() {
                status.set_label("A book needs a title.");
                return;
            }

            let buffer = desc_view.buffer();
            let description = buffer
                .text(&buffer.start_iter(), &buffer.end_iter(), false)
                .to_string();
            let tags: Vec<String> = tags_entry
                .text()
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();
            let series = series_entry.text().trim().to_string();

            if let Err(err) = catalog.update_book_metadata(
                book_id,
                &title,
                authors_entry.text().trim(),
                if series.is_empty() {
                    None
                } else {
                    Some(series.as_str())
                },
                &description,
                &tags,
            ) {
                status.set_label(&format!("Could not save: {err}"));
                return;
            }

            // Re-read so the cover swap sees the current row.
            if let Some(bytes) = pending_cover.borrow_mut().take() {
                if let Ok(Some(fresh)) = catalog.get_book(book_id) {
                    if let Err(err) = crate::epub::replace_cover_bytes(&catalog, &fresh, &bytes) {
                        status.set_label(&format!("Metadata saved, but the cover failed: {err}"));
                        on_saved();
                        return;
                    }
                }
            }

            on_saved();
            window.close();
        });
    }

    let key = gtk::EventControllerKey::new();
    {
        let window = window.clone();
        key.connect_key_pressed(move |_, keyval, _, _| {
            if keyval == gtk::gdk::Key::Escape {
                window.close();
                return gtk::glib::Propagation::Stop;
            }
            gtk::glib::Propagation::Proceed
        });
    }
    window.add_controller(key);

    window.present();
}

#[allow(clippy::too_many_arguments)]
fn rebuild_results(
    host: &gtk::Box,
    candidates: &[Candidate],
    status: &gtk::Label,
    title_entry: &gtk::Entry,
    authors_entry: &gtk::Entry,
    series_entry: &gtk::Entry,
    tags_entry: &gtk::Entry,
    desc_view: &gtk::TextView,
    tx: &async_channel::Sender<FetchMsg>,
) {
    while let Some(child) = host.first_child() {
        host.remove(&child);
    }

    for candidate in candidates {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        row.add_css_class("kalam-list-row");

        let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
        text.set_hexpand(true);

        let title = gtk::Label::new(Some(&candidate.title));
        title.add_css_class("kalam-card-title");
        title.set_halign(gtk::Align::Start);
        title.set_xalign(0.0);
        title.set_ellipsize(gtk::pango::EllipsizeMode::End);
        text.append(&title);

        let meta = gtk::Label::new(Some(&candidate.summary()));
        meta.add_css_class("kalam-card-meta");
        meta.set_halign(gtk::Align::Start);
        meta.set_xalign(0.0);
        meta.set_ellipsize(gtk::pango::EllipsizeMode::End);
        text.append(&meta);
        row.append(&text);

        let use_btn = gtk::Button::with_label("Use this");
        use_btn.add_css_class("kalam-mini-btn");
        use_btn.set_valign(gtk::Align::Center);
        {
            let c = candidate.clone();
            let title_entry = title_entry.clone();
            let authors_entry = authors_entry.clone();
            let series_entry = series_entry.clone();
            let tags_entry = tags_entry.clone();
            let desc_view = desc_view.clone();
            let status = status.clone();
            let tx = tx.clone();
            use_btn.connect_clicked(move |_| {
                // Only overwrite fields the result actually has, so a sparse
                // match cannot blank out good local data.
                if !c.title.is_empty() {
                    title_entry.set_text(&c.title);
                }
                if !c.authors.is_empty() {
                    authors_entry.set_text(&c.authors);
                }
                if let Some(series) = &c.series {
                    series_entry.set_text(series);
                }
                if !c.tags.is_empty() {
                    tags_entry.set_text(&c.tags.join(", "));
                }
                status.set_label("Filled from Open Library — review, then Save.");

                // Descriptions need a second request.
                if let Some(key) = c.work_key.clone() {
                    let desc_view = desc_view.clone();
                    let status2 = status.clone();
                    let (dtx, drx) = async_channel::bounded::<String>(1);
                    std::thread::spawn(move || {
                        if let Ok(text) = openlibrary::fetch_description(&key) {
                            let _ = dtx.send_blocking(text);
                        }
                    });
                    gtk::glib::spawn_future_local(async move {
                        if let Ok(text) = drx.recv().await {
                            if !text.trim().is_empty() {
                                desc_view.buffer().set_text(&text);
                                status2.set_label("Description filled — review, then Save.");
                            }
                        }
                    });
                }

                if let Some(id) = c.cover_id {
                    let tx = tx.clone();
                    std::thread::spawn(move || {
                        let msg = match openlibrary::fetch_cover(id, 'L') {
                            Ok(bytes) => FetchMsg::CoverReady(bytes),
                            Err(err) => FetchMsg::CoverFailed(err.to_string()),
                        };
                        let _ = tx.send_blocking(msg);
                    });
                }
            });
        }
        row.append(&use_btn);
        host.append(&row);
    }
}

fn section_label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("kalam-detail-section-title");
    label.set_halign(gtk::Align::Start);
    label
}

fn labelled_entry(parent: &gtk::Box, label: &str, value: &str) -> gtk::Entry {
    parent.append(&section_label(label));
    let entry = gtk::Entry::new();
    entry.set_text(value);
    entry.set_hexpand(true);
    parent.append(&entry);
    entry
}

fn texture_from_bytes(bytes: &[u8]) -> Option<gtk::gdk::Texture> {
    let glib_bytes = gtk::glib::Bytes::from(bytes);
    gtk::gdk::Texture::from_bytes(&glib_bytes).ok()
}
