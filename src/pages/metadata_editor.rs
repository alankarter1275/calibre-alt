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
use crate::widgets::charts::star_picker;
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
    open_editor_inner(parent, catalog, book_id, Rc::new(on_saved));
}

fn open_editor_inner(
    parent: Option<&gtk::Window>,
    catalog: Rc<Catalog>,
    book_id: i64,
    on_saved: Rc<dyn Fn()>,
) {
    let Ok(Some(book)) = catalog.get_book(book_id) else {
        return;
    };

    let window = gtk::Window::builder()
        .title("Edit metadata")
        .modal(true)
        .default_width(780)
        // Deliberately short: on a 768px-tall laptop a 640px dialog plus window
        // chrome pushed the action bar off-screen. Content scrolls instead.
        .default_height(560)
        .build();
    window.add_css_class("kalam-window");
    if let Some(parent) = parent {
        window.set_transient_for(Some(parent));
    }

    // Outer shell holds the scroller and an always-visible action bar.
    let shell = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    root.set_margin_all(18);

    // ── two columns: fields left, cover + publication right ─────────────
    let top = gtk::Box::new(gtk::Orientation::Horizontal, 20);

    // LEFT — the fields you edit most.
    let fields = gtk::Box::new(gtk::Orientation::Vertical, 8);
    fields.set_hexpand(true);

    let title_entry = labelled_entry(&fields, "TITLE", &book.title);
    let authors_entry = labelled_entry(&fields, "AUTHORS", &book.authors);

    // Series and its position sit on one row, as in Calibre.
    fields.append(&section_label("SERIES"));
    let series_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let series_entry = gtk::Entry::new();
    series_entry.set_text(book.series.as_deref().unwrap_or_default());
    series_entry.set_hexpand(true);
    series_row.append(&series_entry);

    // Fractional steps because novellas are routinely "#2.5".
    let series_index = gtk::SpinButton::with_range(0.0, 999.0, 0.5);
    series_index.set_digits(1);
    series_index.set_value(book.series_index as f64);
    series_index.set_tooltip_text(Some("Position in the series — 0 means unset"));
    series_row.append(&series_index);
    fields.append(&series_row);

    let tags_entry = labelled_entry(&fields, "TAGS / GENRE (COMMA SEPARATED)", &book.tags.join(", "));

    fields.append(&section_label("DESCRIPTION"));
    let desc_view = gtk::TextView::new();
    desc_view.set_wrap_mode(gtk::WrapMode::WordChar);
    desc_view.add_css_class("kalam-desc-view");
    desc_view
        .buffer()
        .set_text(&crate::epub::strip_html(&book.description));
    let desc_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(150)
        .max_content_height(150)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&desc_view)
        .build();
    desc_scroll.add_css_class("kalam-desc-scroll");
    fields.append(&desc_scroll);
    top.append(&fields);

    // RIGHT — rating, publication details, cover.
    let side = gtk::Box::new(gtk::Orientation::Vertical, 8);
    side.set_valign(gtk::Align::Start);
    side.set_size_request(230, -1);
    side.add_css_class("kalam-metadata-side");

    side.append(&section_label("RATING"));
    let rating_host = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let rating_value = Rc::new(std::cell::Cell::new(book.rating));
    {
        let rating_host_inner = rating_host.clone();
        let rating_value = rating_value.clone();
        // Rebuilt on each pick so the filled state follows the click.
        let rebuild: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let rebuild_ref = rebuild.clone();
        let f: Rc<dyn Fn()> = Rc::new(move || {
            while let Some(c) = rating_host_inner.first_child() {
                rating_host_inner.remove(&c);
            }
            let rv = rating_value.clone();
            let again = rebuild_ref.borrow().clone();
            rating_host_inner.append(&star_picker(rating_value.get(), move |v| {
                rv.set(v);
                if let Some(f) = &again {
                    f();
                }
            }));
        });
        *rebuild.borrow_mut() = Some(f.clone());
        f();
    }
    side.append(&rating_host);

    let publisher_entry = labelled_entry(&side, "PUBLISHER", &book.publisher);
    let published_entry = labelled_entry(&side, "PUBLISHED", &book.published);

    side.append(&section_label("COVER"));
    let cover_host = gtk::Box::new(gtk::Orientation::Vertical, 0);
    cover_host.append(&cover_widget(book.cover_path.as_deref(), 150, 240));
    cover_host.set_halign(gtk::Align::Start);
    side.append(&cover_host);

    let cover_btns = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let pick_cover = gtk::Button::with_label("From file…");
    pick_cover.add_css_class("kalam-secondary-btn");
    cover_btns.append(&pick_cover);
    let find_cover = gtk::Button::with_label("🔍 Find");
    find_cover.add_css_class("kalam-secondary-btn");
    find_cover.set_tooltip_text(Some("Search Open Library for a cover"));
    cover_btns.append(&find_cover);
    side.append(&cover_btns);

    top.append(&side);
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
        .min_content_height(140)
        .max_content_height(220)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&results)
        .build();
    root.append(&results_scroll);

    // ── actions: pinned outside the scroller so Save is always reachable ─
    let content_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&root)
        .build();
    shell.append(&content_scroll);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    actions.add_css_class("kalam-dialog-actions");

    // Walk the library without closing the dialog — the point of a bulk
    // clean-up pass. Both save first, so nothing is silently discarded.
    let neighbours = catalog
        .list_books(crate::db::SortKey::Title, "")
        .unwrap_or_default();
    let position = neighbours.iter().position(|b| b.id == book_id);

    let prev_btn = gtk::Button::with_label("← Previous");
    prev_btn.add_css_class("kalam-secondary-btn");
    let next_btn = gtk::Button::with_label("Next →");
    next_btn.add_css_class("kalam-secondary-btn");
    prev_btn.set_sensitive(matches!(position, Some(i) if i > 0));
    next_btn.set_sensitive(matches!(position, Some(i) if i + 1 < neighbours.len()));
    if position.is_some() {
        prev_btn.set_tooltip_text(Some("Save and edit the previous book"));
        next_btn.set_tooltip_text(Some("Save and edit the next book"));
    }
    actions.append(&prev_btn);
    actions.append(&next_btn);

    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    actions.append(&spacer);

    let cancel = gtk::Button::with_label("Cancel");
    cancel.add_css_class("kalam-secondary-btn");
    let save = gtk::Button::with_label("Save");
    save.add_css_class("kalam-primary-btn");
    actions.append(&cancel);
    actions.append(&save);
    shell.append(&actions);

    window.set_child(Some(&shell));

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
        let publisher_entry = publisher_entry.clone();
        let published_entry = published_entry.clone();
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
                            &publisher_entry,
                            &published_entry,
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

    // ── find a cover without touching the other fields ──────────────────
    {
        let title_entry_c = title_entry.clone();
        let authors_entry_c = authors_entry.clone();
        let status = status.clone();
        let tx = tx.clone();
        find_cover.connect_clicked(move |_| {
            let query = format!(
                "{} {}",
                title_entry_c.text().trim(),
                authors_entry_c.text().trim()
            )
            .trim()
            .to_string();
            if query.is_empty() {
                status.set_label("Fill in a title first.");
                return;
            }
            status.set_label("Looking for a cover…");
            let tx = tx.clone();
            std::thread::spawn(move || {
                // Take the first result that actually has cover art.
                let msg = match openlibrary::search(&query, 10) {
                    Ok(list) => match list.iter().find_map(|c| c.cover_id) {
                        Some(id) => match openlibrary::fetch_cover(id, 'L') {
                            Ok(bytes) => FetchMsg::CoverReady(bytes),
                            Err(err) => FetchMsg::CoverFailed(err.to_string()),
                        },
                        None => FetchMsg::CoverFailed("no cover found for that title".into()),
                    },
                    Err(err) => FetchMsg::CoverFailed(err.to_string()),
                };
                let _ = tx.send_blocking(msg);
            });
        });
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
    // Shared by Save and by Previous/Next, which save before moving on.
    // Returns false when the form is invalid, so navigation can abort.
    let commit: Rc<dyn Fn() -> bool> = {
        let catalog = catalog.clone();
        let status = status.clone();
        let pending_cover = pending_cover.clone();
        let on_saved = on_saved.clone();
        let title_entry = title_entry.clone();
        let authors_entry = authors_entry.clone();
        let series_entry = series_entry.clone();
        let series_index = series_index.clone();
        let publisher_entry = publisher_entry.clone();
        let published_entry = published_entry.clone();
        let tags_entry = tags_entry.clone();
        let desc_view = desc_view.clone();
        let rating_value = rating_value.clone();

        Rc::new(move || {
            let title = title_entry.text().trim().to_string();
            if title.is_empty() {
                status.set_label("A book needs a title.");
                return false;
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
                series_index.value() as f32,
                publisher_entry.text().trim(),
                published_entry.text().trim(),
                &description,
                &tags,
            ) {
                status.set_label(&format!("Could not save: {err}"));
                return false;
            }

            let _ = catalog.set_book_rating(book_id, rating_value.get());

            // Re-read so the cover swap sees the current row.
            if let Some(bytes) = pending_cover.borrow_mut().take() {
                if let Ok(Some(fresh)) = catalog.get_book(book_id) {
                    if let Err(err) = crate::epub::replace_cover_bytes(&catalog, &fresh, &bytes) {
                        status.set_label(&format!("Metadata saved, but the cover failed: {err}"));
                        on_saved();
                        return false;
                    }
                }
            }

            on_saved();
            true
        })
    };

    {
        let window = window.clone();
        let commit = commit.clone();
        save.connect_clicked(move |_| {
            if commit() {
                window.close();
            }
        });
    }

    // Previous / Next: commit, close, reopen on the neighbour.
    for (btn, delta) in [(&prev_btn, -1_i64), (&next_btn, 1_i64)] {
        let window = window.clone();
        let catalog = catalog.clone();
        let commit = commit.clone();
        let on_saved = on_saved.clone();
        let neighbours: Vec<i64> = neighbours.iter().map(|b| b.id).collect();
        btn.connect_clicked(move |_| {
            if !commit() {
                return;
            }
            let Some(idx) = neighbours.iter().position(|id| *id == book_id) else {
                return;
            };
            let target = idx as i64 + delta;
            if target < 0 || target as usize >= neighbours.len() {
                return;
            }
            let next_id = neighbours[target as usize];
            let parent = window
                .transient_for()
                .or_else(|| {
                    relm4::main_application()
                        .active_window()
                        .and_then(|w| w.downcast::<gtk::Window>().ok())
                });
            window.close();
            open_editor_inner(parent.as_ref(), catalog.clone(), next_id, on_saved.clone());
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
    publisher_entry: &gtk::Entry,
    published_entry: &gtk::Entry,
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
            let publisher_entry = publisher_entry.clone();
            let published_entry = published_entry.clone();
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
                if !c.publisher.is_empty() {
                    publisher_entry.set_text(&c.publisher);
                }
                if !c.published.is_empty() {
                    published_entry.set_text(&c.published);
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
