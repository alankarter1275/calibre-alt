//! A0 step 2 — `LibraryService`: the seam between pages and the database.
//!
//! # What problem this solves
//!
//! Pages used to call `Catalog` directly, several times each, and swallow the
//! errors individually:
//!
//! ```ignore
//! let stats = catalog.library_stats().unwrap_or_default();
//! let books = catalog.recent_books(12).unwrap_or_default();
//! let tbr   = catalog.list_reading_list().unwrap_or_default();
//! ```
//!
//! Three problems with that. Every page decides for itself what a failed query
//! means (26 `unwrap_or_default()`s and 16 `.ok().flatten()`s across `pages/`),
//! so a broken database renders as an empty library rather than an error.
//! There is no single place to put caching. And moving queries off the UI
//! thread would mean editing every page.
//!
//! The service answers a page's whole data question in **one call returning one
//! owned snapshot**:
//!
//! ```ignore
//! let snap = service.home();   // one call, everything Home draws
//! ```
//!
//! # Why snapshots, not just wrapped getters
//!
//! This shape is the point of the step, not decoration. A snapshot is a plain
//! owned `Send` struct, so the same call can later run on a worker thread and
//! be handed back to the UI — without touching the page. Wrapping each getter
//! one-for-one would have moved the calls but kept N round trips per page, and
//! made each of them a separate future to sequence. `snapshots_are_send()`
//! below asserts the `Send` property at compile time so a future edit cannot
//! quietly break the promise by putting an `Rc` in a snapshot.
//!
//! # Error policy, in one place
//!
//! A read failure degrades to the empty value **and records the reason** in
//! `errors`, which pages surface. Previously the reason was dropped on the
//! floor. The service deliberately does not call `notify` itself: it must stay
//! callable from a worker thread, and `notify` is UI-thread-only (thread-local
//! toast host). Reporting is the caller's job.
//!
//! # Status
//!
//! Home, Analytics and Tags are converted. The remaining pages still hold an
//! `Arc<Catalog>`; both styles coexist on purpose, because `LibraryService`
//! borrows the same `Arc` rather than replacing it. Converting a page is:
//! add a snapshot method here, swap the page's field, delete its
//! `unwrap_or_default()`s. See the roadmap's A0 step 2 entry.

use crate::db::{Catalog, LibraryStats, ReadingListEntry, SortKey};
use crate::models::Book;
use std::sync::Arc;

/// Reads the catalog on behalf of the UI.
///
/// Cheap to clone (one `Arc` bump) and holds no state of its own, so a page can
/// keep one for its lifetime.
#[derive(Clone)]
pub struct LibraryService {
    catalog: Arc<Catalog>,
}

/// Collected read failures. Empty in the normal case, so the happy path costs
/// no allocation.
pub type Errors = Vec<String>;

/// Everything the Home page draws, fetched together.
#[derive(Debug, Default)]
pub struct HomeSnapshot {
    pub stats: LibraryStats,
    /// Recently added — bounded; Home shows a dozen covers, not the library.
    pub recent: Vec<Book>,
    /// "Continue reading", already resolved through Home's fallback chain
    /// (recently opened → in-progress → first book) so the page does not have
    /// to know the rule.
    pub continue_reading: Vec<Book>,
    pub reading_list: Vec<ReadingListEntry>,
    pub errors: Errors,
}

/// Everything the Analytics page draws.
#[derive(Debug, Default)]
pub struct AnalyticsSnapshot {
    pub stats: LibraryStats,
    pub goal: i64,
    pub finished_this_year: i64,
    /// Which of the last 7 days had any reading.
    pub week: [bool; 7],
    pub errors: Errors,
}

/// The tag cloud: every tag with how many books carry it.
#[derive(Debug, Default)]
pub struct TagsSnapshot {
    pub tags: Vec<(String, i64)>,
    pub errors: Errors,
}

/// Books carrying one tag.
#[derive(Debug, Default)]
pub struct TagBooksSnapshot {
    pub books: Vec<Book>,
    pub errors: Errors,
}

impl LibraryService {
    pub fn new(catalog: Arc<Catalog>) -> Self {
        Self { catalog }
    }

    /// Escape hatch for code not yet converted (imports, writes, the reader's
    /// own session handling). Kept deliberately visible: every call site is a
    /// place the service does not cover yet.
    pub fn catalog(&self) -> &Arc<Catalog> {
        &self.catalog
    }

    /// Changes-counter passthrough, used by the app's page cache.
    pub fn change_token(&self) -> i64 {
        self.catalog.change_token()
    }

    // -- snapshots ---------------------------------------------------------

    /// Home: counts strip, continue row, reading-list peek, recently added.
    pub fn home(&self) -> HomeSnapshot {
        let mut errors = Errors::new();
        let stats = take(self.catalog.library_stats(), "library stats", &mut errors);
        // Bounded on purpose: Home shows a dozen covers, not the whole library.
        let recent = take(self.catalog.recent_books(12), "recent books", &mut errors);
        let opened = take(
            self.catalog.recently_opened(4),
            "recently opened",
            &mut errors,
        );
        let reading_list = take(
            self.catalog.list_reading_list(),
            "reading list",
            &mut errors,
        );

        HomeSnapshot {
            continue_reading: continue_row(opened, &recent),
            stats,
            recent,
            reading_list,
            errors,
        }
    }

    /// Analytics: totals, streaks, goal progress, this week's activity.
    pub fn analytics(&self) -> AnalyticsSnapshot {
        let mut errors = Errors::new();
        AnalyticsSnapshot {
            stats: take(self.catalog.library_stats(), "library stats", &mut errors),
            // These three already return plain values (they default internally),
            // so there is no error to collect from them.
            goal: self.catalog.reading_goal(),
            finished_this_year: self.catalog.finished_this_year(),
            week: self.catalog.week_activity(),
            errors,
        }
    }

    /// The tag cloud.
    pub fn tags(&self) -> TagsSnapshot {
        let mut errors = Errors::new();
        TagsSnapshot {
            tags: take(self.catalog.list_tags_with_counts(), "tags", &mut errors),
            errors,
        }
    }

    /// Books carrying `tag`, in `sort` order.
    pub fn tag_books(&self, tag: &str, sort: SortKey) -> TagBooksSnapshot {
        let mut errors = Errors::new();
        TagBooksSnapshot {
            books: take(
                self.catalog.books_with_tag(tag, sort),
                "books for tag",
                &mut errors,
            ),
            errors,
        }
    }
}

/// Unwrap a query result, or record why it failed and fall back to the empty
/// value. This is the one place the "a failed read shows as empty" policy
/// lives; it used to be repeated, silently, in every page.
fn take<T: Default>(result: crate::db::Result<T>, what: &str, errors: &mut Errors) -> T {
    match result {
        Ok(value) => value,
        Err(err) => {
            errors.push(format!("{what}: {err}"));
            T::default()
        }
    }
}

/// Home's "continue reading" rule, extracted so it is testable and so the page
/// does not carry query logic: prefer genuinely recently-opened books, else any
/// book in progress, else the newest book so the row is never empty in a
/// freshly-imported library.
fn continue_row(recently_opened: Vec<Book>, recent: &[Book]) -> Vec<Book> {
    if !recently_opened.is_empty() {
        return recently_opened;
    }
    let in_progress: Vec<Book> = recent
        .iter()
        .filter(|b| b.progress > 0 && b.progress < 100)
        .take(4)
        .cloned()
        .collect();
    if !in_progress.is_empty() {
        return in_progress;
    }
    recent.first().cloned().into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::BookFormat;

    /// The reason snapshots exist: they must be able to cross a thread
    /// boundary, so that moving a query onto a worker is a change *here* and
    /// not in every page. Asserted at compile time — putting an `Rc` or a GTK
    /// widget in a snapshot will fail this rather than being noticed later.
    #[test]
    fn snapshots_are_send() {
        fn assert_send<T: Send>() {}
        assert_send::<HomeSnapshot>();
        assert_send::<AnalyticsSnapshot>();
        assert_send::<TagsSnapshot>();
        assert_send::<TagBooksSnapshot>();
        // The service itself must be Send too, or it cannot be moved onto the
        // worker that would run those queries.
        assert_send::<LibraryService>();
    }

    fn seed(cat: &Catalog, title: &str, tags: &[&str]) -> i64 {
        let tags: Vec<String> = tags.iter().map(|t| t.to_string()).collect();
        cat.insert_book(
            &format!("uuid-{title}"),
            title,
            "An Author",
            None,
            "",
            BookFormat::Epub,
            "book.epub",
            &format!("hash-{title}"),
            None,
            &tags,
        )
        .expect("seed book")
    }

    /// A `Book` with only the fields the continue-row rule reads. Building one
    /// by hand keeps those tests free of the database entirely.
    fn book(title: &str, progress: u8) -> Book {
        Book {
            id: 0,
            uuid: String::new(),
            title: title.into(),
            authors: String::new(),
            series: None,
            description: String::new(),
            format: BookFormat::Epub,
            file_name: String::new(),
            file_hash: String::new(),
            cover_name: None,
            added_at: String::new(),
            progress,
            rating: 0,
            publisher: String::new(),
            published: String::new(),
            series_index: 0.0,
            tags: Vec::new(),
            cover_path: None,
            file_path: std::path::PathBuf::new(),
        }
    }

    #[test]
    fn home_reports_counts_and_recent_books() {
        let cat = Catalog::open_in_memory().unwrap();
        seed(&cat, "Dune", &["scifi"]);
        seed(&cat, "Emma", &["classic"]);
        let svc = LibraryService::new(Arc::new(cat));

        let snap = svc.home();
        assert!(
            snap.errors.is_empty(),
            "unexpected errors: {:?}",
            snap.errors
        );
        assert_eq!(snap.stats.total_books, 2);
        assert_eq!(snap.recent.len(), 2);
    }

    #[test]
    fn continue_row_falls_back_to_the_newest_book() {
        // Nothing opened and nothing part-read: the row would be empty, which
        // reads as "this app is broken" in a freshly imported library.
        let cat = Catalog::open_in_memory().unwrap();
        seed(&cat, "Dune", &[]);
        let svc = LibraryService::new(Arc::new(cat));

        let snap = svc.home();
        assert_eq!(snap.continue_reading.len(), 1);
        assert_eq!(snap.continue_reading[0].title, "Dune");
    }

    #[test]
    fn continue_row_prefers_in_progress_over_newest() {
        // `recent` is newest-first, so the untouched book leads the list; the
        // rule must still pick the one actually being read.
        let row = continue_row(Vec::new(), &[book("Untouched", 0), book("Half read", 42)]);
        assert_eq!(row.len(), 1);
        assert_eq!(row[0].title, "Half read");
    }

    #[test]
    fn continue_row_ignores_finished_books() {
        // 100% is done, not "continue". Without the upper bound the row would
        // keep offering a book the user has already finished.
        let row = continue_row(Vec::new(), &[book("Finished", 100)]);
        assert_eq!(row.len(), 1, "falls back to the newest book");
        assert_eq!(
            row[0].title, "Finished",
            "the fallback is allowed to show it, but not as 'in progress'"
        );

        let row = continue_row(Vec::new(), &[book("Finished", 100), book("Reading", 10)]);
        assert_eq!(row.len(), 1);
        assert_eq!(row[0].title, "Reading");
    }

    #[test]
    fn continue_row_keeps_recently_opened_when_present() {
        let row = continue_row(
            vec![book("Opened last night", 5)],
            &[book("Newer import", 0)],
        );
        assert_eq!(row.len(), 1);
        assert_eq!(row[0].title, "Opened last night");
    }

    #[test]
    fn tags_snapshot_counts_books_per_tag() {
        let cat = Catalog::open_in_memory().unwrap();
        seed(&cat, "Dune", &["scifi"]);
        seed(&cat, "Neuromancer", &["scifi"]);
        seed(&cat, "Emma", &["classic"]);
        let svc = LibraryService::new(Arc::new(cat));

        let snap = svc.tags();
        assert!(snap.errors.is_empty());
        let scifi = snap
            .tags
            .iter()
            .find(|(name, _)| name == "scifi")
            .expect("scifi tag present");
        assert_eq!(scifi.1, 2);
    }

    #[test]
    fn tag_books_returns_only_that_tag() {
        let cat = Catalog::open_in_memory().unwrap();
        seed(&cat, "Dune", &["scifi"]);
        seed(&cat, "Emma", &["classic"]);
        let svc = LibraryService::new(Arc::new(cat));

        let snap = svc.tag_books("scifi", SortKey::Title);
        assert!(snap.errors.is_empty());
        assert_eq!(snap.books.len(), 1);
        assert_eq!(snap.books[0].title, "Dune");
    }

    #[test]
    fn analytics_snapshot_reads_the_goal_back() {
        let cat = Catalog::open_in_memory().unwrap();
        cat.set_reading_goal(24);
        let svc = LibraryService::new(Arc::new(cat));

        let snap = svc.analytics();
        assert!(snap.errors.is_empty());
        assert_eq!(snap.goal, 24);
    }

    #[test]
    fn a_failed_read_is_recorded_rather_than_silently_empty() {
        // The whole point of the error policy: the page can say "database
        // error", where before every page turned this into an empty list.
        let mut errors = Errors::new();
        let value: Vec<Book> = take(
            Err(crate::db::DbError::Sqlite(
                rusqlite::Error::QueryReturnedNoRows,
            )),
            "recent books",
            &mut errors,
        );
        assert!(value.is_empty());
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0].starts_with("recent books:"),
            "the message must name the query: {:?}",
            errors[0]
        );
    }
}
