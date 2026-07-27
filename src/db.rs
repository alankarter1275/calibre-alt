//! SQLite catalog access — P1 books + P2 progress + P3 annotations & dictionary.
#![allow(dead_code)]

use crate::models::{Book, BookFormat};
use crate::paths::{book_dir, catalog_db, ensure_data_dirs};
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, DbError>;

/// Bumped whenever `migrate()` learns new tables/columns.
/// v3 = P3 annotations & dictionary · v4 = P4 shelves, lists, history, sessions
/// · v5 = ratings + reading goals · v6 = publisher/published/series index
/// · v7 = remembered metadata edits, keyed by file hash.
pub const SCHEMA_VERSION: i64 = 7;

/// Process-wide DB handle (GTK app is single-threaded for UI; imports run sync on UI for P1).
pub struct Catalog {
    conn: Mutex<Connection>,
    /// `library_stats()` runs ~20 aggregates and is called on Home, the
    /// Library dashboard and Analytics. The result is memoised against
    /// SQLite's total_changes() counter: any write anywhere bumps it, so the
    /// cache cannot go stale and no write path has to remember to clear it.
    stats_cache: Mutex<Option<(i64, LibraryStats)>>,
    /// Set while restoring remembered metadata.
    ///
    /// restore_overrides applies the saved values through the normal edit
    /// path, which re-stashes as it goes — and at that moment the book still
    /// has its freshly-imported cover, so the stash was being overwritten with
    /// the EPUB default before the real cover could be copied back.
    restoring: std::sync::atomic::AtomicBool,
}

// ---------------------------------------------------------------------------
// Domain structs for P3
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Annotation {
    pub id: i64,
    pub book_id: i64,
    pub kind: String, // highlight | quote | note
    pub chapter_index: i64,
    pub start_path: String,
    pub start_offset: i64,
    pub end_path: String,
    pub end_offset: i64,
    pub color: String,
    pub text_excerpt: String,
    pub note: String,
    pub cfi: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SavedWord {
    pub id: i64,
    pub word: String,
    pub definition: String,
    pub dict_name: Option<String>,
    pub book_id: Option<i64>,
    pub chapter_index: Option<i64>,
    pub context_text: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Dictionary {
    pub id: i64,
    pub name: String,
    pub lang: Option<String>,
    pub entry_count: i64,
    pub added_at: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DictEntry {
    pub id: i64,
    pub dict_id: i64,
    pub word: String,
    pub definition: String,
}

// ---------------------------------------------------------------------------
// Domain structs for P4
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShelfKind {
    Manual,
    Smart,
}

impl ShelfKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ShelfKind::Manual => "manual",
            ShelfKind::Smart => "smart",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ShelfKind::Manual => "Manual",
            ShelfKind::Smart => "Smart",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "smart" => ShelfKind::Smart,
            _ => ShelfKind::Manual,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Shelf {
    pub id: i64,
    pub name: String,
    pub kind: ShelfKind,
    pub description: String,
    /// JSON rule document; empty for manual shelves.
    pub rules: String,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
    /// Live count, filled in by `list_shelves`.
    pub book_count: usize,
}

impl Shelf {
    pub fn rule_set(&self) -> crate::shelf_rules::RuleSet {
        crate::shelf_rules::RuleSet::parse(&self.rules)
    }

    /// Card subtitle: rule summary for smart shelves, description otherwise.
    pub fn summary(&self) -> String {
        match self.kind {
            ShelfKind::Smart => self.rule_set().describe(),
            ShelfKind::Manual => {
                if self.description.trim().is_empty() {
                    "Hand-picked books".to_string()
                } else {
                    self.description.clone()
                }
            }
        }
    }
}

/// A row in the ordered to-be-read list.
#[derive(Debug, Clone)]
pub struct ReadingListEntry {
    pub book: Book,
    pub position: i64,
    pub note: String,
    pub added_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Opened,
    Finished,
    Unfinished,
    Imported,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EventKind::Opened => "opened",
            EventKind::Finished => "finished",
            EventKind::Unfinished => "unfinished",
            EventKind::Imported => "imported",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            EventKind::Opened => "Opened",
            EventKind::Finished => "Finished",
            EventKind::Unfinished => "Marked unread",
            EventKind::Imported => "Imported",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            EventKind::Opened => "◷",
            EventKind::Finished => "✓",
            EventKind::Unfinished => "↺",
            EventKind::Imported => "+",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "finished" => EventKind::Finished,
            "unfinished" => EventKind::Unfinished,
            "imported" => EventKind::Imported,
            _ => EventKind::Opened,
        }
    }
}

/// A history row joined with its book title for display.
#[derive(Debug, Clone)]
pub struct ReadingEvent {
    pub id: i64,
    pub book_id: i64,
    pub kind: EventKind,
    pub at: String,
    pub detail: String,
    pub book_title: String,
    pub book_authors: String,
}

/// Aggregated numbers for the Analytics page.
#[derive(Debug, Clone, Default)]
pub struct LibraryStats {
    pub total_books: i64,
    pub finished: i64,
    pub reading: i64,
    pub unread: i64,
    pub highlights: i64,
    pub quotes: i64,
    pub saved_words: i64,
    pub shelves: i64,
    pub reading_list: i64,
    pub total_seconds: i64,
    pub seconds_last_7: i64,
    pub seconds_last_30: i64,
    pub sessions: i64,
    pub finished_last_30: i64,
    pub added_last_30: i64,
    pub current_streak_days: i64,
    pub longest_streak_days: i64,
    /// Mean minutes per day across days that had any reading (last 30 days).
    pub avg_minutes_per_active_day: i64,
    /// (label, count) — newest month last.
    pub added_by_month: Vec<(String, i64)>,
    /// (label, seconds) — last 14 days, oldest first.
    pub minutes_by_day: Vec<(String, i64)>,
    pub top_tags: Vec<(String, i64)>,
    pub top_authors: Vec<(String, i64)>,
    pub most_read: Vec<(String, i64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightColor {
    Yellow,
    Green,
    Blue,
    Pink,
    Orange,
}

impl HighlightColor {
    pub fn as_str(self) -> &'static str {
        match self {
            HighlightColor::Yellow => "yellow",
            HighlightColor::Green => "green",
            HighlightColor::Blue => "blue",
            HighlightColor::Pink => "pink",
            HighlightColor::Orange => "orange",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "green" => HighlightColor::Green,
            "blue" => HighlightColor::Blue,
            "pink" | "rose" => HighlightColor::Pink,
            "orange" => HighlightColor::Orange,
            _ => HighlightColor::Yellow,
        }
    }

    pub const ALL: &'static [HighlightColor] = &[
        HighlightColor::Yellow,
        HighlightColor::Green,
        HighlightColor::Blue,
        HighlightColor::Pink,
        HighlightColor::Orange,
    ];
}

impl Catalog {
    /// Take the connection lock, recovering from poisoning.
    ///
    /// A panic on any thread while holding this lock used to make every later
    /// `expect("db lock")` abort the whole app. SQLite itself is unharmed by
    /// the panic, so carrying on with the data is strictly better than dying.
    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|poisoned| {
            eprintln!("kalam: recovered a poisoned database lock");
            poisoned.into_inner()
        })
    }

    pub fn open() -> Result<Self> {
        ensure_data_dirs()?;
        let path = catalog_db();
        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            -- WAL already survives crashes; full fsync per commit is the
            -- single biggest cost on spinning disks and cheap SSDs.
            PRAGMA synchronous = NORMAL;
            -- 64 MB page cache and memory temp tables: the catalog is small
            -- enough to sit in RAM, which removes most read latency.
            PRAGMA cache_size = -64000;
            PRAGMA temp_store = MEMORY;
            PRAGMA mmap_size = 268435456;
            ",
        )?;
        let cat = Self {
            conn: Mutex::new(conn),
            stats_cache: Mutex::new(None),
            restoring: std::sync::atomic::AtomicBool::new(false),
        };
        cat.migrate()?;
        Ok(cat)
    }

    /// In-memory catalog with the full schema applied — tests only.
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        let cat = Self {
            conn: Mutex::new(conn),
            stats_cache: Mutex::new(None),
            restoring: std::sync::atomic::AtomicBool::new(false),
        };
        cat.migrate()?;
        Ok(cat)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS books (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                uuid          TEXT    NOT NULL UNIQUE,
                title         TEXT    NOT NULL,
                sort_title    TEXT    NOT NULL,
                authors       TEXT    NOT NULL DEFAULT '',
                series        TEXT,
                description   TEXT    NOT NULL DEFAULT '',
                format        TEXT    NOT NULL,
                file_name     TEXT    NOT NULL,
                file_hash     TEXT    NOT NULL,
                cover_name    TEXT,
                added_at      TEXT    NOT NULL,
                progress      INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS tags (
                id   INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE COLLATE NOCASE
            );

            CREATE TABLE IF NOT EXISTS book_tags (
                book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
                tag_id  INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
                PRIMARY KEY (book_id, tag_id)
            );

            CREATE INDEX IF NOT EXISTS idx_books_title ON books(sort_title);
            CREATE INDEX IF NOT EXISTS idx_books_added ON books(added_at);
            CREATE INDEX IF NOT EXISTS idx_books_hash ON books(file_hash);

            CREATE TABLE IF NOT EXISTS reading_progress (
                book_id       INTEGER PRIMARY KEY REFERENCES books(id) ON DELETE CASCADE,
                chapter_index INTEGER NOT NULL DEFAULT 0,
                fraction      REAL    NOT NULL DEFAULT 0.0,
                updated_at    TEXT    NOT NULL
            );

            -- P3 tables
            CREATE TABLE IF NOT EXISTS annotations (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                book_id       INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
                kind          TEXT    NOT NULL,
                chapter_index INTEGER NOT NULL,
                start_path    TEXT    NOT NULL,
                start_offset  INTEGER NOT NULL,
                end_path      TEXT    NOT NULL,
                end_offset    INTEGER NOT NULL,
                color         TEXT    NOT NULL DEFAULT 'yellow',
                text_excerpt  TEXT    NOT NULL DEFAULT '',
                note          TEXT    NOT NULL DEFAULT '',
                cfi           TEXT,
                created_at    TEXT    NOT NULL,
                updated_at    TEXT    NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_annotations_book ON annotations(book_id);
            CREATE INDEX IF NOT EXISTS idx_annotations_book_chapter ON annotations(book_id, chapter_index);
            CREATE INDEX IF NOT EXISTS idx_annotations_kind ON annotations(kind);

            CREATE TABLE IF NOT EXISTS saved_words (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                word          TEXT    NOT NULL,
                definition    TEXT    NOT NULL,
                dict_name     TEXT,
                book_id       INTEGER REFERENCES books(id) ON DELETE SET NULL,
                chapter_index INTEGER,
                context_text  TEXT,
                created_at    TEXT    NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_saved_words_word ON saved_words(word COLLATE NOCASE);

            CREATE TABLE IF NOT EXISTS dictionaries (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                name        TEXT    NOT NULL UNIQUE,
                lang        TEXT,
                entry_count INTEGER NOT NULL DEFAULT 0,
                added_at    TEXT    NOT NULL
            );

            CREATE TABLE IF NOT EXISTS dict_entries (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                dict_id    INTEGER NOT NULL REFERENCES dictionaries(id) ON DELETE CASCADE,
                word       TEXT    NOT NULL COLLATE NOCASE,
                definition TEXT    NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_dict_entries_word ON dict_entries(word COLLATE NOCASE);
            CREATE INDEX IF NOT EXISTS idx_dict_entries_dict ON dict_entries(dict_id);

            -- P4 tables
            CREATE TABLE IF NOT EXISTS shelves (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                name        TEXT    NOT NULL UNIQUE COLLATE NOCASE,
                kind        TEXT    NOT NULL DEFAULT 'manual',  -- manual | smart
                description TEXT    NOT NULL DEFAULT '',
                rules       TEXT    NOT NULL DEFAULT '',        -- JSON for smart shelves
                position    INTEGER NOT NULL DEFAULT 0,
                created_at  TEXT    NOT NULL,
                updated_at  TEXT    NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_shelves_position ON shelves(position);

            CREATE TABLE IF NOT EXISTS shelf_books (
                shelf_id INTEGER NOT NULL REFERENCES shelves(id) ON DELETE CASCADE,
                book_id  INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
                position INTEGER NOT NULL DEFAULT 0,
                added_at TEXT    NOT NULL,
                PRIMARY KEY (shelf_id, book_id)
            );
            CREATE INDEX IF NOT EXISTS idx_shelf_books_shelf ON shelf_books(shelf_id, position);
            CREATE INDEX IF NOT EXISTS idx_shelf_books_book ON shelf_books(book_id);

            CREATE TABLE IF NOT EXISTS reading_list (
                book_id  INTEGER PRIMARY KEY REFERENCES books(id) ON DELETE CASCADE,
                position INTEGER NOT NULL DEFAULT 0,
                note     TEXT    NOT NULL DEFAULT '',
                added_at TEXT    NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_reading_list_position ON reading_list(position);

            -- Append-only history log: opened | finished | unfinished | imported
            CREATE TABLE IF NOT EXISTS reading_events (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                book_id    INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
                kind       TEXT    NOT NULL,
                at         TEXT    NOT NULL,
                detail     TEXT    NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_reading_events_book ON reading_events(book_id);
            CREATE INDEX IF NOT EXISTS idx_reading_events_at ON reading_events(at DESC);
            CREATE INDEX IF NOT EXISTS idx_reading_events_kind ON reading_events(kind);

            -- One row per reader visit; closed out when the reader shuts down.
            CREATE TABLE IF NOT EXISTS reading_sessions (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                book_id     INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
                started_at  TEXT    NOT NULL,
                ended_at    TEXT,
                seconds     INTEGER NOT NULL DEFAULT 0,
                start_pct   INTEGER NOT NULL DEFAULT 0,
                end_pct     INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_reading_sessions_book ON reading_sessions(book_id);
            CREATE INDEX IF NOT EXISTS idx_reading_sessions_started ON reading_sessions(started_at DESC);

            -- P4.1: tiny key/value store for UI preferences that must survive
            -- restarts (reader theme, font size, ...).
            CREATE TABLE IF NOT EXISTS app_prefs (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            -- v7: metadata you edited by hand, remembered against the file's
            -- content hash so it survives removing and re-importing the book.
            -- Deliberately NOT cascaded from books: the whole point is that it
            -- outlives the catalog row.
            CREATE TABLE IF NOT EXISTS metadata_overrides (
                file_hash    TEXT PRIMARY KEY,
                title        TEXT,
                authors      TEXT,
                series       TEXT,
                series_index REAL,
                publisher    TEXT,
                published    TEXT,
                description  TEXT,
                tags         TEXT,
                rating       INTEGER,
                cover_name   TEXT,
                updated_at   TEXT NOT NULL
            );
            "#,
        )?;

        // books.last_opened_at / finished_at were added in v4; ALTER is the only
        // way to extend an existing table created by v1–v3.
        add_column_if_missing(&conn, "books", "last_opened_at", "TEXT")?;
        add_column_if_missing(&conn, "books", "finished_at", "TEXT")?;
        // v5: half-star ratings stored as 0..=10 (i.e. tenths of the 5-star
        // scale x2) so "3.5 stars" is an integer 7 and needs no float compare.
        add_column_if_missing(&conn, "books", "rating", "INTEGER NOT NULL DEFAULT 0")?;
        // v6: publication details. `series_index` is REAL because half-numbers
        // ("book 2.5") are common in series, and 0 means "not set".
        add_column_if_missing(&conn, "books", "publisher", "TEXT NOT NULL DEFAULT ''")?;
        add_column_if_missing(&conn, "books", "published", "TEXT NOT NULL DEFAULT ''")?;
        add_column_if_missing(&conn, "books", "series_index", "REAL NOT NULL DEFAULT 0")?;

        // Indexed *after* the ALTERs above, since these columns do not exist in
        // the CREATE TABLE that older databases were built from.
        // Stats and the dashboard filter on them on every visit.
        conn.execute_batch(
            "
            CREATE INDEX IF NOT EXISTS idx_books_progress ON books(progress);
            CREATE INDEX IF NOT EXISTS idx_books_last_opened ON books(last_opened_at DESC);
            CREATE INDEX IF NOT EXISTS idx_books_finished ON books(finished_at);
            ",
        )?;

        let version: Option<i64> = conn
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |r| {
                r.get(0)
            })
            .optional()?;
        if version.is_none() {
            conn.execute(
                "INSERT INTO schema_version (version) VALUES (?1)",
                params![SCHEMA_VERSION],
            )?;
        } else if let Some(v) = version {
            if v < SCHEMA_VERSION {
                conn.execute(
                    "UPDATE schema_version SET version = ?1",
                    params![SCHEMA_VERSION],
                )?;
            }
        }
        Ok(())
    }

    /// Every book's uuid — used to spot orphaned reader caches.
    pub fn all_uuids(&self) -> Result<Vec<String>> {
        let conn = self.conn();
        let mut stmt = conn.prepare_cached("SELECT uuid FROM books")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        Ok(rows.flatten().collect())
    }

    pub fn count_books(&self) -> Result<usize> {
        let conn = self.conn();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM books", [], |r| r.get(0))?;
        Ok(n as usize)
    }

    pub fn list_books(&self, sort: SortKey, query: &str) -> Result<Vec<Book>> {
        let conn = self.conn();
        let order = match sort {
            SortKey::Title => "sort_title COLLATE NOCASE ASC",
            SortKey::Author => "authors COLLATE NOCASE ASC, sort_title COLLATE NOCASE ASC",
            SortKey::Added => "added_at DESC",
        };

        let q = query.trim();
        let mut books = if q.is_empty() {
            let sql = format!("SELECT {BOOK_COLUMNS} FROM books ORDER BY {order}");
            let mut stmt = conn.prepare_cached(&sql)?;
            let rows = stmt.query_map([], row_to_book)?;
            rows.collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            let like = format!("%{}%", escape_like(q));
            let sql = format!(
                "SELECT {BOOK_COLUMNS}
                 FROM books
                 WHERE books.title LIKE ?1 ESCAPE '\\'
                    OR books.authors LIKE ?1 ESCAPE '\\'
                    OR IFNULL(books.series,'') LIKE ?1 ESCAPE '\\'
                 ORDER BY {order}"
            );
            let mut stmt = conn.prepare_cached(&sql)?;
            let rows = stmt.query_map(params![like], row_to_book)?;
            rows.collect::<std::result::Result<Vec<_>, _>>()?
        };

        hydrate_books(&conn, &mut books)?;
        Ok(books)
    }

    /// Newest books, capped. Pages that show a handful of covers were calling
    /// `list_books` and loading the entire library to display six of them.
    pub fn recent_books(&self, limit: usize) -> Result<Vec<Book>> {
        let conn = self.conn();
        let sql = format!("SELECT {BOOK_COLUMNS} FROM books ORDER BY books.added_at DESC LIMIT ?1");
        let mut stmt = conn.prepare_cached(&sql)?;
        let rows = stmt.query_map(params![limit as i64], row_to_book)?;
        let mut books = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        hydrate_books(&conn, &mut books)?;
        Ok(books)
    }

    pub fn get_book(&self, id: i64) -> Result<Option<Book>> {
        let conn = self.conn();
        let mut book = conn
            .query_row(
                &format!("SELECT {BOOK_COLUMNS} FROM books WHERE books.id = ?1"),
                params![id],
                row_to_book,
            )
            .optional()?;
        if let Some(ref mut b) = book {
            b.tags = tags_for_book(&conn, b.id)?;
            b.cover_path = b
                .cover_name
                .as_ref()
                .map(|name| book_dir(&b.uuid).join(name));
            b.file_path = book_dir(&b.uuid).join(&b.file_name);
        }
        Ok(book)
    }

    pub fn find_by_hash(&self, hash: &str) -> Result<Option<i64>> {
        let conn = self.conn();
        let id = conn
            .query_row(
                "SELECT id FROM books WHERE file_hash = ?1 LIMIT 1",
                params![hash],
                |r| r.get(0),
            )
            .optional()?;
        Ok(id)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_book(
        &self,
        uuid: &str,
        title: &str,
        authors: &str,
        series: Option<&str>,
        description: &str,
        format: BookFormat,
        file_name: &str,
        file_hash: &str,
        cover_name: Option<&str>,
        tags: &[String],
    ) -> Result<i64> {
        let conn = self.conn();
        let added = chrono_like_now();
        let sort_title = title.to_lowercase();
        conn.execute(
            "INSERT INTO books
                (uuid, title, sort_title, authors, series, description, format,
                 file_name, file_hash, cover_name, added_at, progress)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,0)",
            params![
                uuid,
                title,
                sort_title,
                authors,
                series,
                description,
                format.as_str(),
                file_name,
                file_hash,
                cover_name,
                added,
            ],
        )?;
        let id = conn.last_insert_rowid();
        for tag in tags {
            let tag = tag.trim();
            if tag.is_empty() {
                continue;
            }
            conn.execute(
                "INSERT OR IGNORE INTO tags (name) VALUES (?1)",
                params![tag],
            )?;
            let tag_id: i64 = conn.query_row(
                "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE",
                params![tag],
                |r| r.get(0),
            )?;
            conn.execute(
                "INSERT OR IGNORE INTO book_tags (book_id, tag_id) VALUES (?1, ?2)",
                params![id, tag_id],
            )?;
        }
        Ok(id)
    }

    pub fn delete_book(&self, id: i64) -> Result<()> {
        let book = match self.get_book(id)? {
            Some(b) => b,
            None => return Ok(()),
        };
        // Capture any hand-edited metadata first: the row is about to go, and
        // re-importing the same file should not lose your work.
        let _ = self.remember_overrides(id);
        {
            let conn = self.conn();
            conn.execute("DELETE FROM books WHERE id = ?1", params![id])?;
        }
        let dir = book_dir(&book.uuid);
        if dir.exists() {
            let _ = fs::remove_dir_all(&dir);
        }
        Ok(())
    }

    /// Detailed reading position (chapter + in-chapter fraction).
    pub fn get_reading_progress(&self, book_id: i64) -> Result<Option<(usize, f64)>> {
        let conn = self.conn();
        let row = conn
            .query_row(
                "SELECT chapter_index, fraction FROM reading_progress WHERE book_id = ?1",
                params![book_id],
                |r| Ok((r.get::<_, i64>(0)? as usize, r.get::<_, f64>(1)?)),
            )
            .optional()?;
        Ok(row)
    }

    /// Save position and update the books.progress percent (0–100).
    pub fn set_reading_progress(
        &self,
        book_id: i64,
        chapter_index: usize,
        fraction: f64,
        chapter_count: usize,
    ) -> Result<()> {
        let conn = self.conn();
        let frac = fraction.clamp(0.0, 1.0);
        let now = chrono_like_now();
        conn.execute(
            "INSERT INTO reading_progress (book_id, chapter_index, fraction, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(book_id) DO UPDATE SET
               chapter_index = excluded.chapter_index,
               fraction = excluded.fraction,
               updated_at = excluded.updated_at",
            params![book_id, chapter_index as i64, frac, now],
        )?;
        // Overall percent across chapters.
        let overall = if chapter_count == 0 {
            0.0
        } else {
            ((chapter_index as f64) + frac) / (chapter_count as f64) * 100.0
        };
        let pct = overall.round().clamp(0.0, 100.0) as i64;
        conn.execute(
            "UPDATE books SET progress = ?1 WHERE id = ?2",
            params![pct, book_id],
        )?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // P3: Annotations
    // -----------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub fn insert_annotation(
        &self,
        book_id: i64,
        kind: &str,
        chapter_index: i64,
        start_path: &str,
        start_offset: i64,
        end_path: &str,
        end_offset: i64,
        color: &str,
        text_excerpt: &str,
        note: &str,
    ) -> Result<i64> {
        let conn = self.conn();
        let now = chrono_like_now();
        conn.execute(
            "INSERT INTO annotations
                (book_id, kind, chapter_index, start_path, start_offset, end_path, end_offset,
                 color, text_excerpt, note, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
            params![
                book_id,
                kind,
                chapter_index,
                start_path,
                start_offset,
                end_path,
                end_offset,
                color,
                text_excerpt,
                note,
                now
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_annotations_for_book(&self, book_id: i64) -> Result<Vec<Annotation>> {
        let conn = self.conn();
        let mut stmt = conn.prepare_cached(
            "SELECT id, book_id, kind, chapter_index, start_path, start_offset, end_path, end_offset,
                    color, text_excerpt, note, cfi, created_at, updated_at
             FROM annotations WHERE book_id = ?1 ORDER BY chapter_index ASC, created_at ASC",
        )?;
        let rows = stmt.query_map(params![book_id], row_to_annotation)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn get_annotations_for_chapter(
        &self,
        book_id: i64,
        chapter_index: i64,
    ) -> Result<Vec<Annotation>> {
        let conn = self.conn();
        let mut stmt = conn.prepare_cached(
            "SELECT id, book_id, kind, chapter_index, start_path, start_offset, end_path, end_offset,
                    color, text_excerpt, note, cfi, created_at, updated_at
             FROM annotations
             WHERE book_id = ?1 AND chapter_index = ?2
             ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![book_id, chapter_index], row_to_annotation)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// A few recent quotes with their book titles, in one query.
    /// The dashboard previously fetched 500 rows and then a book per card.
    pub fn recent_quotes(&self, limit: usize) -> Result<Vec<(Annotation, String)>> {
        let conn = self.conn();
        let mut stmt = conn.prepare_cached(
            "SELECT a.id, a.book_id, a.kind, a.chapter_index, a.start_path, a.start_offset,
                    a.end_path, a.end_offset, a.color, a.text_excerpt, a.note, a.cfi,
                    a.created_at, a.updated_at, books.title
             FROM annotations a
             JOIN books ON books.id = a.book_id
             WHERE a.kind IN ('quote','highlight') AND TRIM(a.text_excerpt) <> ''
             ORDER BY a.created_at DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |r| {
            Ok((row_to_annotation(r)?, r.get::<_, String>(14)?))
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// Total saved quotes, for counts that do not need the rows themselves.
    pub fn count_quotes(&self) -> Result<i64> {
        let conn = self.conn();
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM annotations WHERE kind IN ('quote','highlight')",
            [],
            |r| r.get(0),
        )?;
        Ok(n)
    }

    pub fn list_all_quotes(&self, query: &str) -> Result<Vec<Annotation>> {
        let conn = self.conn();
        let q = query.trim();
        let mut stmt = if q.is_empty() {
            conn.prepare_cached(
                "SELECT id, book_id, kind, chapter_index, start_path, start_offset, end_path, end_offset,
                        color, text_excerpt, note, cfi, created_at, updated_at
                 FROM annotations WHERE kind IN ('quote','highlight')
                 ORDER BY created_at DESC LIMIT 500",
            )?
        } else {
            conn.prepare_cached(
                "SELECT id, book_id, kind, chapter_index, start_path, start_offset, end_path, end_offset,
                        color, text_excerpt, note, cfi, created_at, updated_at
                 FROM annotations
                 WHERE kind IN ('quote','highlight')
                   AND (text_excerpt LIKE ?1 ESCAPE '\\' OR note LIKE ?1 ESCAPE '\\')
                 ORDER BY created_at DESC LIMIT 500",
            )?
        };
        let like = format!("%{}%", escape_like(q));
        let rows = if q.is_empty() {
            stmt.query_map([], row_to_annotation)?
        } else {
            stmt.query_map(params![like], row_to_annotation)?
        };
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn delete_annotation(&self, id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute("DELETE FROM annotations WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn update_annotation_note(&self, id: i64, note: &str) -> Result<()> {
        let conn = self.conn();
        let now = chrono_like_now();
        conn.execute(
            "UPDATE annotations SET note = ?1, updated_at = ?2 WHERE id = ?3",
            params![note, now, id],
        )?;
        Ok(())
    }

    pub fn update_annotation_color(&self, id: i64, color: &str) -> Result<()> {
        let conn = self.conn();
        let now = chrono_like_now();
        conn.execute(
            "UPDATE annotations SET color = ?1, updated_at = ?2 WHERE id = ?3",
            params![color, now, id],
        )?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // P3: Saved words
    // -----------------------------------------------------------------------

    pub fn insert_saved_word(
        &self,
        word: &str,
        definition: &str,
        dict_name: Option<&str>,
        book_id: Option<i64>,
        chapter_index: Option<i64>,
        context_text: Option<&str>,
    ) -> Result<i64> {
        let conn = self.conn();
        let now = chrono_like_now();
        conn.execute(
            "INSERT INTO saved_words (word, definition, dict_name, book_id, chapter_index, context_text, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                word,
                definition,
                dict_name,
                book_id,
                chapter_index,
                context_text,
                now
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_saved_words(&self, query: &str) -> Result<Vec<SavedWord>> {
        let conn = self.conn();
        let q = query.trim();
        if q.is_empty() {
            let mut stmt = conn.prepare_cached(
                "SELECT id, word, definition, dict_name, book_id, chapter_index, context_text, created_at
                 FROM saved_words ORDER BY created_at DESC LIMIT 500",
            )?;
            let rows = stmt.query_map([], row_to_saved_word)?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(Into::into)
        } else {
            let like = format!("%{}%", escape_like(q));
            let mut stmt = conn.prepare_cached(
                "SELECT id, word, definition, dict_name, book_id, chapter_index, context_text, created_at
                 FROM saved_words
                 WHERE word LIKE ?1 ESCAPE '\\' OR definition LIKE ?1 ESCAPE '\\'
                 ORDER BY created_at DESC LIMIT 500",
            )?;
            let rows = stmt.query_map(params![like], row_to_saved_word)?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(Into::into)
        }
    }

    pub fn delete_saved_word(&self, id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute("DELETE FROM saved_words WHERE id = ?1", params![id])?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // P3: Dictionaries
    // -----------------------------------------------------------------------

    pub fn list_dictionaries(&self) -> Result<Vec<Dictionary>> {
        let conn = self.conn();
        let mut stmt = conn.prepare_cached(
            "SELECT id, name, lang, entry_count, added_at FROM dictionaries ORDER BY name ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Dictionary {
                id: r.get(0)?,
                name: r.get(1)?,
                lang: r.get(2)?,
                entry_count: r.get(3)?,
                added_at: r.get(4)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn insert_dictionary(
        &self,
        name: &str,
        lang: Option<&str>,
        entry_count: i64,
    ) -> Result<i64> {
        let conn = self.conn();
        let now = chrono_like_now();
        conn.execute(
            "INSERT INTO dictionaries (name, lang, entry_count, added_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(name) DO UPDATE SET lang=excluded.lang, entry_count=excluded.entry_count",
            params![name, lang, entry_count, now],
        )?;
        let id = conn.query_row(
            "SELECT id FROM dictionaries WHERE name = ?1",
            params![name],
            |r| r.get(0),
        )?;
        Ok(id)
    }

    pub fn delete_dictionary(&self, id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute("DELETE FROM dictionaries WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn set_dictionary_entry_count(&self, id: i64, count: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE dictionaries SET entry_count = ?1 WHERE id = ?2",
            params![count, id],
        )?;
        Ok(())
    }

    pub fn insert_dict_entry(&self, dict_id: i64, word: &str, definition: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO dict_entries (dict_id, word, definition) VALUES (?1, ?2, ?3)",
            params![dict_id, word, definition],
        )?;
        Ok(())
    }

    pub fn batch_insert_dict_entries(
        &self,
        dict_id: i64,
        entries: &[(String, String)],
    ) -> Result<()> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO dict_entries (dict_id, word, definition) VALUES (?1, ?2, ?3)",
            )?;
            for (w, d) in entries {
                stmt.execute(params![dict_id, w, d])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn clear_dict_entries(&self, dict_id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM dict_entries WHERE dict_id = ?1",
            params![dict_id],
        )?;
        Ok(())
    }

    pub fn search_dict(&self, word: &str, limit: usize) -> Result<Vec<DictEntry>> {
        let conn = self.conn();
        let clean = word.trim();
        if clean.is_empty() {
            return Ok(Vec::new());
        }
        // Exact match first, then prefix, then LIKE fallback
        let mut stmt = conn.prepare_cached(
            "SELECT id, dict_id, word, definition FROM dict_entries
             WHERE word = ?1 COLLATE NOCASE
             ORDER BY word ASC LIMIT ?2",
        )?;
        let mut out = Vec::new();
        let lim = limit as i64;
        for r in stmt.query_map(params![clean, lim], |r| {
            Ok(DictEntry {
                id: r.get(0)?,
                dict_id: r.get(1)?,
                word: r.get(2)?,
                definition: r.get(3)?,
            })
        })? {
            out.push(r?);
        }
        if !out.is_empty() {
            return Ok(out);
        }

        // Prefix search
        let like = format!("{}%", escape_like(clean));
        let mut stmt2 = conn.prepare_cached(
            "SELECT id, dict_id, word, definition FROM dict_entries
             WHERE word LIKE ?1 ESCAPE '\\' COLLATE NOCASE
             ORDER BY LENGTH(word) ASC, word ASC LIMIT ?2",
        )?;
        for r in stmt2.query_map(params![like, lim], |r| {
            Ok(DictEntry {
                id: r.get(0)?,
                dict_id: r.get(1)?,
                word: r.get(2)?,
                definition: r.get(3)?,
            })
        })? {
            out.push(r?);
        }
        if !out.is_empty() {
            return Ok(out);
        }

        // Substring fallback
        let like2 = format!("%{}%", escape_like(clean));
        let mut stmt3 = conn.prepare_cached(
            "SELECT id, dict_id, word, definition FROM dict_entries
             WHERE word LIKE ?1 ESCAPE '\\' COLLATE NOCASE
                OR definition LIKE ?1 ESCAPE '\\'
             ORDER BY LENGTH(word) ASC LIMIT ?2",
        )?;
        for r in stmt3.query_map(params![like2, lim], |r| {
            Ok(DictEntry {
                id: r.get(0)?,
                dict_id: r.get(1)?,
                word: r.get(2)?,
                definition: r.get(3)?,
            })
        })? {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn dict_entry_count(&self) -> Result<i64> {
        let conn = self.conn();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM dict_entries", [], |r| r.get(0))?;
        Ok(n)
    }

    // -----------------------------------------------------------------------
    // P5: metadata editing
    // -----------------------------------------------------------------------

    /// Overwrite the editable metadata fields. Tags are replaced wholesale,
    /// which matches what the edit dialog presents (one comma-separated box).
    #[allow(clippy::too_many_arguments)]
    pub fn update_book_metadata(
        &self,
        book_id: i64,
        title: &str,
        authors: &str,
        series: Option<&str>,
        series_index: f32,
        publisher: &str,
        published: &str,
        description: &str,
        tags: &[String],
    ) -> Result<()> {
        let conn = self.conn();
        let title = title.trim();
        // sort_title mirrors insert_book so ordering stays consistent.
        conn.execute(
            "UPDATE books
             SET title = ?2, sort_title = ?3, authors = ?4, series = ?5,
                 series_index = ?6, publisher = ?7, published = ?8, description = ?9
             WHERE id = ?1",
            params![
                book_id,
                title,
                title.to_lowercase(),
                authors.trim(),
                series.map(|s| s.trim()).filter(|s| !s.is_empty()),
                series_index.max(0.0) as f64,
                publisher.trim(),
                published.trim(),
                description,
            ],
        )?;

        conn.execute("DELETE FROM book_tags WHERE book_id = ?1", params![book_id])?;
        for tag in tags {
            let tag = tag.trim();
            if tag.is_empty() {
                continue;
            }
            conn.execute(
                "INSERT OR IGNORE INTO tags (name) VALUES (?1)",
                params![tag],
            )?;
            let tag_id: i64 = conn.query_row(
                "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE",
                params![tag],
                |r| r.get(0),
            )?;
            conn.execute(
                "INSERT OR IGNORE INTO book_tags (book_id, tag_id) VALUES (?1, ?2)",
                params![book_id, tag_id],
            )?;
        }

        // Tags orphaned by this edit would otherwise clutter the tag browser.
        conn.execute(
            "DELETE FROM tags WHERE id NOT IN (SELECT tag_id FROM book_tags)",
            [],
        )?;
        drop(conn);

        // Remember the result so removing and re-importing this file does not
        // silently discard the edit.
        self.remember_overrides(book_id)?;
        Ok(())
    }

    /// Remember the current metadata against the file's hash, so it can be
    /// restored if the book is removed and imported again.
    fn remember_overrides(&self, book_id: i64) -> Result<()> {
        // A restore is not an edit: writing the freshly-imported state back
        // over the saved copy is exactly what we are trying to avoid.
        if self.restoring.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(());
        }
        let Some(book) = self.get_book(book_id)? else {
            return Ok(());
        };
        if book.file_hash.trim().is_empty() {
            return Ok(());
        }

        // The cover lives in library/<uuid>/, which is deleted along with the
        // book, so remembering only its name would leave a dangling pointer.
        // Keep a copy outside that directory, named after the file hash.
        let stashed_cover = book.cover_path.as_ref().and_then(|src| {
            if !src.is_file() {
                return None;
            }
            let ext = src
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("jpg")
                .to_ascii_lowercase();
            let dest_name = format!("{}.{ext}", book.file_hash);
            let dest = crate::paths::override_covers_dir().join(&dest_name);
            let _ = fs::create_dir_all(crate::paths::override_covers_dir());
            match fs::copy(src, &dest) {
                Ok(_) => Some(dest_name),
                Err(_) => None,
            }
        });

        let conn = self.conn();
        conn.execute(
            "INSERT INTO metadata_overrides
                (file_hash, title, authors, series, series_index, publisher,
                 published, description, tags, rating, cover_name, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
             ON CONFLICT(file_hash) DO UPDATE SET
                title = excluded.title,
                authors = excluded.authors,
                series = excluded.series,
                series_index = excluded.series_index,
                publisher = excluded.publisher,
                published = excluded.published,
                description = excluded.description,
                tags = excluded.tags,
                rating = excluded.rating,
                cover_name = excluded.cover_name,
                updated_at = excluded.updated_at",
            params![
                book.file_hash,
                book.title,
                book.authors,
                book.series,
                book.series_index as f64,
                book.publisher,
                book.published,
                book.description,
                book.tags.join(", "),
                book.rating as i64,
                stashed_cover,
                chrono_like_now(),
            ],
        )?;
        Ok(())
    }

    /// Re-apply remembered edits to a freshly imported book. Returns true when
    /// something was restored.
    pub fn restore_overrides(&self, book_id: i64, file_hash: &str) -> Result<bool> {
        // A named struct rather than a ten-element tuple: the tuple needed a
        // type annotation that tripped clippy::type_complexity, and this is
        // easier to read besides.
        struct Saved {
            title: String,
            authors: String,
            series: Option<String>,
            series_index: f64,
            publisher: String,
            published: String,
            description: String,
            tags: String,
            rating: i64,
            cover_name: Option<String>,
        }

        let saved: Option<Saved> = {
            let conn = self.conn();
            conn.query_row(
                "SELECT IFNULL(title,''), IFNULL(authors,''), series,
                        IFNULL(series_index,0), IFNULL(publisher,''),
                        IFNULL(published,''), IFNULL(description,''),
                        IFNULL(tags,''), IFNULL(rating,0), cover_name
                 FROM metadata_overrides WHERE file_hash = ?1",
                params![file_hash],
                |r| {
                    Ok(Saved {
                        title: r.get(0)?,
                        authors: r.get(1)?,
                        series: r.get(2)?,
                        series_index: r.get(3)?,
                        publisher: r.get(4)?,
                        published: r.get(5)?,
                        description: r.get(6)?,
                        tags: r.get(7)?,
                        rating: r.get(8)?,
                        cover_name: r.get(9)?,
                    })
                },
            )
            .optional()?
        };

        let Some(saved) = saved else {
            return Ok(false);
        };

        // Suppress re-stashing for the duration; the Drop impl clears the flag
        // even if a step below fails.
        struct RestoreGuard<'a>(&'a std::sync::atomic::AtomicBool);
        impl Drop for RestoreGuard<'_> {
            fn drop(&mut self) {
                self.0.store(false, std::sync::atomic::Ordering::Relaxed);
            }
        }
        self.restoring
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let _guard = RestoreGuard(&self.restoring);

        let tags: Vec<String> = saved
            .tags
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();

        self.update_book_metadata(
            book_id,
            &saved.title,
            &saved.authors,
            saved.series.as_deref(),
            saved.series_index as f32,
            &saved.publisher,
            &saved.published,
            &saved.description,
            &tags,
        )?;

        if saved.rating > 0 {
            self.set_book_rating(book_id, saved.rating.clamp(0, 10) as u8)?;
        }

        // Copy the stashed cover back into the new book's directory. Failing
        // here is not fatal — the text metadata is already restored, and the
        // book keeps whatever cover the EPUB supplied.
        if let (Some(stashed), Some(book)) = (saved.cover_name, self.get_book(book_id)?) {
            let src = crate::paths::override_covers_dir().join(&stashed);
            if src.is_file() {
                let ext = src
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("jpg")
                    .to_ascii_lowercase();
                let dest_name = format!("cover-restored.{ext}");
                let dest = book_dir(&book.uuid).join(&dest_name);
                if fs::create_dir_all(book_dir(&book.uuid)).is_ok() && fs::copy(&src, &dest).is_ok()
                {
                    self.set_cover_name_quiet(book_id, Some(&dest_name))?;
                }
            }
        }
        Ok(true)
    }

    /// Forget remembered edits for a file — used by "import fresh".
    pub fn forget_overrides(&self, file_hash: &str) -> Result<()> {
        // Drop the stashed cover too, otherwise the covers directory grows
        // forever with images nothing references.
        let stashed: Option<String> = {
            let conn = self.conn();
            conn.query_row(
                "SELECT IFNULL(cover_name, '') FROM metadata_overrides WHERE file_hash = ?1",
                params![file_hash],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .filter(|n| !n.is_empty())
        };
        if let Some(name) = stashed {
            let path = crate::paths::override_covers_dir().join(name);
            let _ = fs::remove_file(path);
        }

        let conn = self.conn();
        conn.execute(
            "DELETE FROM metadata_overrides WHERE file_hash = ?1",
            params![file_hash],
        )?;
        Ok(())
    }

    /// Re-point a book (and its remembered edits) at a new content hash.
    ///
    /// Writing metadata into the EPUB changes the file's bytes, so the stored
    /// hash goes stale. Left alone that would break duplicate detection on
    /// re-import and orphan the override row, so both move together.
    pub fn rehash_book(&self, book_id: i64, old_hash: &str, new_hash: &str) -> Result<()> {
        if old_hash == new_hash {
            return Ok(());
        }
        let conn = self.conn();
        conn.execute(
            "UPDATE books SET file_hash = ?2 WHERE id = ?1",
            params![book_id, new_hash],
        )?;
        // Drop any override already filed under the new hash, then move ours.
        conn.execute(
            "DELETE FROM metadata_overrides WHERE file_hash = ?1",
            params![new_hash],
        )?;
        conn.execute(
            "UPDATE metadata_overrides SET file_hash = ?2 WHERE file_hash = ?1",
            params![old_hash, new_hash],
        )?;
        Ok(())
    }

    /// Point the book at a new cover file inside its own directory.
    pub fn set_cover_name(&self, book_id: i64, cover_name: Option<&str>) -> Result<()> {
        {
            let conn = self.conn();
            conn.execute(
                "UPDATE books SET cover_name = ?2 WHERE id = ?1",
                params![book_id, cover_name],
            )?;
        }
        // Re-stash: a replaced cover is an edit, and the remembered copy would
        // otherwise still be the jacket from before the swap.
        self.remember_overrides(book_id)?;
        Ok(())
    }

    /// Like `set_cover_name`, but does not touch the remembered override.
    /// Used when *restoring* a cover, where re-stashing would be circular.
    fn set_cover_name_quiet(&self, book_id: i64, cover_name: Option<&str>) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE books SET cover_name = ?2 WHERE id = ?1",
            params![book_id, cover_name],
        )?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // P4.2: ratings & reading goals
    // -----------------------------------------------------------------------

    /// Ratings are stored as 0..=10 half-stars (7 == 3.5 stars); 0 == unrated.
    pub fn set_book_rating(&self, book_id: i64, half_stars: u8) -> Result<()> {
        {
            let conn = self.conn();
            conn.execute(
                "UPDATE books SET rating = ?2 WHERE id = ?1",
                params![book_id, half_stars.min(10) as i64],
            )?;
        }
        self.remember_overrides(book_id)?;
        Ok(())
    }

    /// Yearly target, e.g. "read 24 books this year". 0 disables the goal.
    pub fn reading_goal(&self) -> i64 {
        self.get_pref_i64("goal.books_per_year", 0)
    }

    pub fn set_reading_goal(&self, books: i64) {
        self.set_pref("goal.books_per_year", &books.max(0).to_string());
    }

    /// Books finished since 1 January of the current year.
    pub fn finished_this_year(&self) -> i64 {
        let year = &chrono_like_now()[..4];
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return 0,
        };
        conn.query_row(
            "SELECT COUNT(*) FROM books
             WHERE finished_at IS NOT NULL AND substr(finished_at, 1, 4) = ?1",
            params![year],
            |r| r.get(0),
        )
        .unwrap_or(0)
    }

    /// Which of the last 7 days had any reading — powers the streak strip.
    /// Index 0 is six days ago, index 6 is today.
    pub fn week_activity(&self) -> [bool; 7] {
        let mut out = [false; 7];
        let Ok(conn) = self.conn.lock() else {
            return out;
        };
        let cutoff = iso_days_ago(6);
        let Ok(mut stmt) = conn.prepare_cached(
            "SELECT DISTINCT substr(started_at, 1, 10) FROM reading_sessions
             WHERE started_at >= ?1 AND seconds > 0",
        ) else {
            return out;
        };
        let Ok(rows) = stmt.query_map(params![cutoff], |r| r.get::<_, String>(0)) else {
            return out;
        };
        let days: Vec<String> = rows.flatten().collect();
        for (i, slot) in out.iter_mut().enumerate() {
            let key = iso_days_ago(6 - i as i64)[..10].to_string();
            *slot = days.iter().any(|d| *d == key);
        }
        out
    }

    // -----------------------------------------------------------------------
    // P4.1: preferences
    // -----------------------------------------------------------------------

    pub fn get_pref(&self, key: &str) -> Option<String> {
        let conn = self.conn.lock().ok()?;
        conn.query_row(
            "SELECT value FROM app_prefs WHERE key = ?1",
            params![key],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten()
    }

    pub fn set_pref(&self, key: &str, value: &str) {
        if let Ok(conn) = self.conn.lock() {
            let _ = conn.execute(
                "INSERT INTO app_prefs (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            );
        }
    }

    /// Convenience for numeric prefs; falls back when unset or unparsable.
    pub fn get_pref_i64(&self, key: &str, default: i64) -> i64 {
        self.get_pref(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }

    // -----------------------------------------------------------------------
    // P4: Shelves
    // -----------------------------------------------------------------------

    /// All shelves ordered by position, each with a live book count.
    pub fn list_shelves(&self) -> Result<Vec<Shelf>> {
        let mut shelves = {
            let conn = self.conn();
            let mut stmt = conn.prepare_cached(
                "SELECT id, name, kind, description, rules, position, created_at, updated_at
                 FROM shelves
                 ORDER BY position ASC, name COLLATE NOCASE ASC",
            )?;
            let rows = stmt.query_map([], row_to_shelf)?;
            rows.collect::<std::result::Result<Vec<_>, _>>()?
        };

        // Manual counts come back in a single grouped query; only smart
        // shelves need their rules compiled and counted individually.
        let manual_counts: std::collections::HashMap<i64, usize> = {
            let conn = self.conn();
            let mut stmt = conn
                .prepare_cached("SELECT shelf_id, COUNT(*) FROM shelf_books GROUP BY shelf_id")?;
            let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?;
            rows.filter_map(|r| r.ok())
                .map(|(id, n)| (id, n as usize))
                .collect()
        };

        for shelf in &mut shelves {
            shelf.book_count = match shelf.kind {
                ShelfKind::Manual => manual_counts.get(&shelf.id).copied().unwrap_or(0),
                ShelfKind::Smart => self.shelf_book_count(shelf).unwrap_or(0),
            };
        }
        Ok(shelves)
    }

    pub fn get_shelf(&self, id: i64) -> Result<Option<Shelf>> {
        let shelf = {
            let conn = self.conn();
            conn.query_row(
                "SELECT id, name, kind, description, rules, position, created_at, updated_at
                 FROM shelves WHERE id = ?1",
                params![id],
                row_to_shelf,
            )
            .optional()?
        };
        let Some(mut shelf) = shelf else {
            return Ok(None);
        };
        shelf.book_count = self.shelf_book_count(&shelf).unwrap_or(0);
        Ok(Some(shelf))
    }

    pub fn create_shelf(
        &self,
        name: &str,
        kind: ShelfKind,
        description: &str,
        rules: &str,
    ) -> Result<i64> {
        let conn = self.conn();
        let now = chrono_like_now();
        let next_pos: i64 = conn
            .query_row(
                "SELECT IFNULL(MAX(position), -1) + 1 FROM shelves",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        conn.execute(
            "INSERT INTO shelves (name, kind, description, rules, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![
                name.trim(),
                kind.as_str(),
                description,
                rules,
                next_pos,
                now
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_shelf(&self, id: i64, name: &str, description: &str, rules: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE shelves SET name = ?2, description = ?3, rules = ?4, updated_at = ?5
             WHERE id = ?1",
            params![id, name.trim(), description, rules, chrono_like_now()],
        )?;
        Ok(())
    }

    pub fn delete_shelf(&self, id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute("DELETE FROM shelves WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// True when a shelf with this name already exists (case-insensitive).
    /// `except_id` lets the edit dialog ignore the shelf being renamed.
    pub fn shelf_name_taken(&self, name: &str, except_id: Option<i64>) -> Result<bool> {
        let conn = self.conn();
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM shelves
             WHERE name = ?1 COLLATE NOCASE AND id <> IFNULL(?2, -1)",
            params![name.trim(), except_id],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// Books on a shelf: membership rows for manual, compiled rules for smart.
    pub fn shelf_books(&self, shelf: &Shelf, sort: SortKey, query: &str) -> Result<Vec<Book>> {
        match shelf.kind {
            ShelfKind::Manual => self.manual_shelf_books(shelf.id, sort, query),
            ShelfKind::Smart => self.smart_shelf_books(shelf, sort, query),
        }
    }

    fn manual_shelf_books(&self, shelf_id: i64, sort: SortKey, query: &str) -> Result<Vec<Book>> {
        let conn = self.conn();
        // Manual shelves keep hand-sorted order unless the user picks a sort.
        let order = match sort {
            SortKey::Title => "books.sort_title COLLATE NOCASE ASC",
            SortKey::Author => {
                "books.authors COLLATE NOCASE ASC, books.sort_title COLLATE NOCASE ASC"
            }
            SortKey::Added => "sb.position ASC, sb.added_at ASC",
        };
        let q = query.trim();
        let sql = format!(
            "SELECT {BOOK_COLUMNS}
             FROM books
             JOIN shelf_books sb ON sb.book_id = books.id
             WHERE sb.shelf_id = ?1
               AND (?2 = '' OR books.title LIKE ?3 ESCAPE '\\'
                            OR books.authors LIKE ?3 ESCAPE '\\')
             ORDER BY {order}"
        );
        let like = format!("%{}%", escape_like(q));
        let mut stmt = conn.prepare_cached(&sql)?;
        let rows = stmt.query_map(params![shelf_id, q, like], row_to_book)?;
        let mut books = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        hydrate_books(&conn, &mut books)?;
        Ok(books)
    }

    fn smart_shelf_books(&self, shelf: &Shelf, sort: SortKey, query: &str) -> Result<Vec<Book>> {
        let (where_sql, rule_params) = shelf.rule_set().to_sql();
        let conn = self.conn();
        let order = match sort {
            SortKey::Title => "books.sort_title COLLATE NOCASE ASC",
            SortKey::Author => {
                "books.authors COLLATE NOCASE ASC, books.sort_title COLLATE NOCASE ASC"
            }
            SortKey::Added => "books.added_at DESC",
        };
        let q = query.trim();
        // Compiled rules use anonymous `?` placeholders, so the whole statement
        // must stay positional — mixing `?N` here would collide with them.
        let sql = format!(
            "SELECT {BOOK_COLUMNS}
             FROM books
             WHERE ({where_sql})
               AND (? = '' OR books.title LIKE ? ESCAPE '\\'
                           OR books.authors LIKE ? ESCAPE '\\')
             ORDER BY {order}"
        );

        let like = format!("%{}%", escape_like(q));
        // Bind order follows placeholder order in the SQL text: rules first.
        let mut bound: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(rule_params.len() + 3);
        for p in &rule_params {
            bound.push(p);
        }
        bound.push(&q);
        bound.push(&like);
        bound.push(&like);

        // Not cached: the WHERE clause is generated from this shelf's rules.
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(bound.as_slice(), row_to_book)?;
        let mut books = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        hydrate_books(&conn, &mut books)?;
        Ok(books)
    }

    fn shelf_book_count(&self, shelf: &Shelf) -> Result<usize> {
        let conn = self.conn();
        let n: i64 = match shelf.kind {
            ShelfKind::Manual => conn.query_row(
                "SELECT COUNT(*) FROM shelf_books WHERE shelf_id = ?1",
                params![shelf.id],
                |r| r.get(0),
            )?,
            ShelfKind::Smart => {
                let (where_sql, rule_params) = shelf.rule_set().to_sql();
                // Rule-derived SQL differs per shelf, so it is not cached.
                let sql = format!("SELECT COUNT(*) FROM books WHERE {where_sql}");
                let bound: Vec<&dyn rusqlite::ToSql> = rule_params
                    .iter()
                    .map(|p| p as &dyn rusqlite::ToSql)
                    .collect();
                conn.query_row(&sql, bound.as_slice(), |r| r.get(0))?
            }
        };
        Ok(n as usize)
    }

    /// Count matches for an unsaved rule set — powers the live count in the editor.
    pub fn count_matching_rules(&self, rules: &crate::shelf_rules::RuleSet) -> Result<usize> {
        let (where_sql, rule_params) = rules.to_sql();
        let conn = self.conn();
        let sql = format!("SELECT COUNT(*) FROM books WHERE {where_sql}");
        let bound: Vec<&dyn rusqlite::ToSql> = rule_params
            .iter()
            .map(|p| p as &dyn rusqlite::ToSql)
            .collect();
        let n: i64 = conn.query_row(&sql, bound.as_slice(), |r| r.get(0))?;
        Ok(n as usize)
    }

    pub fn add_book_to_shelf(&self, shelf_id: i64, book_id: i64) -> Result<()> {
        let conn = self.conn();
        let next_pos: i64 = conn
            .query_row(
                "SELECT IFNULL(MAX(position), -1) + 1 FROM shelf_books WHERE shelf_id = ?1",
                params![shelf_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        conn.execute(
            "INSERT OR IGNORE INTO shelf_books (shelf_id, book_id, position, added_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![shelf_id, book_id, next_pos, chrono_like_now()],
        )?;
        Ok(())
    }

    pub fn remove_book_from_shelf(&self, shelf_id: i64, book_id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM shelf_books WHERE shelf_id = ?1 AND book_id = ?2",
            params![shelf_id, book_id],
        )?;
        Ok(())
    }

    /// Manual shelves this book belongs to (id, name) — for the book page chips.
    pub fn shelves_for_book(&self, book_id: i64) -> Result<Vec<(i64, String)>> {
        let conn = self.conn();
        let mut stmt = conn.prepare_cached(
            "SELECT s.id, s.name FROM shelves s
             JOIN shelf_books sb ON sb.shelf_id = s.id
             WHERE sb.book_id = ?1
             ORDER BY s.name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map(params![book_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// Move a manual shelf entry up/down by swapping positions with its neighbour.
    pub fn move_shelf_book(&self, shelf_id: i64, book_id: i64, delta: i64) -> Result<()> {
        let conn = self.conn();
        let ids: Vec<i64> = {
            let mut stmt = conn.prepare_cached(
                "SELECT book_id FROM shelf_books WHERE shelf_id = ?1
                 ORDER BY position ASC, added_at ASC",
            )?;
            let rows = stmt.query_map(params![shelf_id], |r| r.get(0))?;
            rows.collect::<std::result::Result<Vec<_>, _>>()?
        };
        let Some(idx) = ids.iter().position(|id| *id == book_id) else {
            return Ok(());
        };
        let target = idx as i64 + delta;
        if target < 0 || target as usize >= ids.len() {
            return Ok(());
        }
        let mut reordered = ids.clone();
        reordered.swap(idx, target as usize);
        for (pos, id) in reordered.iter().enumerate() {
            conn.execute(
                "UPDATE shelf_books SET position = ?3 WHERE shelf_id = ?1 AND book_id = ?2",
                params![shelf_id, id, pos as i64],
            )?;
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // P4: Reading list
    // -----------------------------------------------------------------------

    pub fn list_reading_list(&self) -> Result<Vec<ReadingListEntry>> {
        let conn = self.conn();
        let sql = format!(
            "SELECT {BOOK_COLUMNS}, rl.position, rl.note, rl.added_at
             FROM books
             JOIN reading_list rl ON rl.book_id = books.id
             ORDER BY rl.position ASC, rl.added_at ASC"
        );
        let mut stmt = conn.prepare_cached(&sql)?;
        let rows = stmt.query_map([], |row| {
            let book = row_to_book(row)?;
            Ok(ReadingListEntry {
                book,
                position: row.get(12)?,
                note: row.get(13)?,
                added_at: row.get(14)?,
            })
        })?;
        let mut entries = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        let mut books: Vec<Book> = entries.iter().map(|e| e.book.clone()).collect();
        hydrate_books(&conn, &mut books)?;
        for (entry, book) in entries.iter_mut().zip(books) {
            entry.book = book;
        }
        Ok(entries)
    }

    pub fn is_in_reading_list(&self, book_id: i64) -> Result<bool> {
        let conn = self.conn();
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM reading_list WHERE book_id = ?1",
            params![book_id],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    pub fn add_to_reading_list(&self, book_id: i64) -> Result<()> {
        let conn = self.conn();
        let next_pos: i64 = conn
            .query_row(
                "SELECT IFNULL(MAX(position), -1) + 1 FROM reading_list",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        conn.execute(
            "INSERT OR IGNORE INTO reading_list (book_id, position, note, added_at)
             VALUES (?1, ?2, '', ?3)",
            params![book_id, next_pos, chrono_like_now()],
        )?;
        Ok(())
    }

    pub fn remove_from_reading_list(&self, book_id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM reading_list WHERE book_id = ?1",
            params![book_id],
        )?;
        Ok(())
    }

    /// Move an entry up (`delta = -1`) or down (`delta = 1`).
    pub fn move_reading_list_entry(&self, book_id: i64, delta: i64) -> Result<()> {
        let conn = self.conn();
        let ids: Vec<i64> = {
            let mut stmt = conn
                .prepare("SELECT book_id FROM reading_list ORDER BY position ASC, added_at ASC")?;
            let rows = stmt.query_map([], |r| r.get(0))?;
            rows.collect::<std::result::Result<Vec<_>, _>>()?
        };
        let Some(idx) = ids.iter().position(|id| *id == book_id) else {
            return Ok(());
        };
        let target = idx as i64 + delta;
        if target < 0 || target as usize >= ids.len() {
            return Ok(());
        }
        let mut reordered = ids.clone();
        reordered.swap(idx, target as usize);
        for (pos, id) in reordered.iter().enumerate() {
            conn.execute(
                "UPDATE reading_list SET position = ?2 WHERE book_id = ?1",
                params![id, pos as i64],
            )?;
        }
        Ok(())
    }

    pub fn set_reading_list_note(&self, book_id: i64, note: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE reading_list SET note = ?2 WHERE book_id = ?1",
            params![book_id, note],
        )?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // P4: History (append-only event log)
    // -----------------------------------------------------------------------

    pub fn log_event(&self, book_id: i64, kind: EventKind, detail: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO reading_events (book_id, kind, at, detail) VALUES (?1, ?2, ?3, ?4)",
            params![book_id, kind.as_str(), chrono_like_now(), detail],
        )?;
        Ok(())
    }

    /// Stamp `books.last_opened_at` and log an `opened` event — but collapse
    /// repeat opens within the same hour so flipping in and out of the reader
    /// doesn't flood History.
    pub fn mark_book_opened(&self, book_id: i64) -> Result<()> {
        let conn = self.conn();
        let now = chrono_like_now();
        conn.execute(
            "UPDATE books SET last_opened_at = ?2 WHERE id = ?1",
            params![book_id, now],
        )?;

        let recent: Option<String> = conn
            .query_row(
                "SELECT at FROM reading_events
                 WHERE book_id = ?1 AND kind = 'opened'
                 ORDER BY at DESC LIMIT 1",
                params![book_id],
                |r| r.get(0),
            )
            .optional()?;

        // Timestamps are ISO-8601 UTC, so a 13-char prefix is the same hour.
        let same_hour = recent
            .as_deref()
            .map(|prev| prev.len() >= 13 && now.len() >= 13 && prev[..13] == now[..13])
            .unwrap_or(false);

        if !same_hour {
            conn.execute(
                "INSERT INTO reading_events (book_id, kind, at, detail) VALUES (?1, 'opened', ?2, '')",
                params![book_id, now],
            )?;
        }
        Ok(())
    }

    /// Mark finished/unfinished by hand. Finishing also pins progress to 100%
    /// and drops the book off the reading list.
    pub fn set_book_finished(&self, book_id: i64, finished: bool) -> Result<()> {
        {
            let conn = self.conn();
            let now = chrono_like_now();
            if finished {
                conn.execute(
                    "UPDATE books SET finished_at = ?2, progress = 100 WHERE id = ?1",
                    params![book_id, now],
                )?;
            } else {
                conn.execute(
                    "UPDATE books SET finished_at = NULL WHERE id = ?1",
                    params![book_id],
                )?;
            }
        }
        if finished {
            self.remove_from_reading_list(book_id)?;
        }
        self.log_event(
            book_id,
            if finished {
                EventKind::Finished
            } else {
                EventKind::Unfinished
            },
            "",
        )
    }

    /// Auto-finish when progress crosses the threshold, once per book.
    /// Returns true if this call flipped the book to finished.
    pub fn auto_finish_if_complete(&self, book_id: i64, progress_pct: i64) -> Result<bool> {
        if progress_pct < 99 {
            return Ok(false);
        }
        let already: Option<String> = {
            let conn = self.conn();
            conn.query_row(
                "SELECT finished_at FROM books WHERE id = ?1",
                params![book_id],
                |r| r.get(0),
            )
            .optional()?
            .flatten()
        };
        if already.is_some() {
            return Ok(false);
        }
        {
            let conn = self.conn();
            conn.execute(
                "UPDATE books SET finished_at = ?2 WHERE id = ?1",
                params![book_id, chrono_like_now()],
            )?;
        }
        self.remove_from_reading_list(book_id)?;
        self.log_event(book_id, EventKind::Finished, "auto")?;
        Ok(true)
    }

    pub fn book_finished_at(&self, book_id: i64) -> Result<Option<String>> {
        let conn = self.conn();
        let v: Option<Option<String>> = conn
            .query_row(
                "SELECT finished_at FROM books WHERE id = ?1",
                params![book_id],
                |r| r.get(0),
            )
            .optional()?;
        Ok(v.flatten())
    }

    /// Newest-first history, optionally filtered by kind and free text.
    pub fn list_events(
        &self,
        kind: Option<EventKind>,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ReadingEvent>> {
        let conn = self.conn();
        let q = query.trim();
        let like = format!("%{}%", escape_like(q));
        let kind_str = kind.map(|k| k.as_str().to_string());
        let mut stmt = conn.prepare_cached(
            "SELECT e.id, e.book_id, e.kind, e.at, e.detail, books.title, books.authors
             FROM reading_events e
             JOIN books ON books.id = e.book_id
             WHERE (?1 IS NULL OR e.kind = ?1)
               AND (?2 = '' OR books.title LIKE ?3 ESCAPE '\\'
                            OR books.authors LIKE ?3 ESCAPE '\\')
             ORDER BY e.at DESC, e.id DESC
             LIMIT ?4",
        )?;
        let rows = stmt.query_map(params![kind_str, q, like, limit as i64], |r| {
            let kind: String = r.get(2)?;
            Ok(ReadingEvent {
                id: r.get(0)?,
                book_id: r.get(1)?,
                kind: EventKind::from_str_lossy(&kind),
                at: r.get(3)?,
                detail: r.get(4)?,
                book_title: r.get(5)?,
                book_authors: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn clear_history(&self) -> Result<()> {
        let conn = self.conn();
        conn.execute("DELETE FROM reading_events", [])?;
        Ok(())
    }

    /// Books ordered by most recently opened — powers Home → Continue.
    pub fn recently_opened(&self, limit: usize) -> Result<Vec<Book>> {
        let conn = self.conn();
        let sql = format!(
            "SELECT {BOOK_COLUMNS}
             FROM books
             WHERE books.last_opened_at IS NOT NULL
               AND IFNULL(books.finished_at, '') = ''
             ORDER BY books.last_opened_at DESC
             LIMIT ?1"
        );
        let mut stmt = conn.prepare_cached(&sql)?;
        let rows = stmt.query_map(params![limit as i64], row_to_book)?;
        let mut books = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        hydrate_books(&conn, &mut books)?;
        Ok(books)
    }

    // -----------------------------------------------------------------------
    // P4: Reading sessions (time tracking)
    // -----------------------------------------------------------------------

    /// Open a session row when the reader mounts; returns its id.
    pub fn start_reading_session(&self, book_id: i64, start_pct: i64) -> Result<i64> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO reading_sessions (book_id, started_at, ended_at, seconds, start_pct, end_pct)
             VALUES (?1, ?2, NULL, 0, ?3, ?3)",
            params![book_id, chrono_like_now(), start_pct],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Close a session. Absurd durations (laptop suspended with the reader
    /// open) are clamped so one forgotten window can't claim 14 hours read.
    pub fn end_reading_session(&self, session_id: i64, seconds: i64, end_pct: i64) -> Result<()> {
        let conn = self.conn();
        let clamped = seconds.clamp(0, MAX_SESSION_SECONDS);
        conn.execute(
            "UPDATE reading_sessions SET ended_at = ?2, seconds = ?3, end_pct = ?4 WHERE id = ?1",
            params![session_id, chrono_like_now(), clamped, end_pct],
        )?;
        Ok(())
    }

    pub fn total_reading_seconds(&self, book_id: i64) -> Result<i64> {
        let conn = self.conn();
        let n: i64 = conn.query_row(
            "SELECT IFNULL(SUM(seconds), 0) FROM reading_sessions WHERE book_id = ?1",
            params![book_id],
            |r| r.get(0),
        )?;
        Ok(n)
    }

    // -----------------------------------------------------------------------
    // P4: Tags browse
    // -----------------------------------------------------------------------

    /// (tag name, book count), most used first.
    pub fn list_tags_with_counts(&self) -> Result<Vec<(String, i64)>> {
        let conn = self.conn();
        let mut stmt = conn.prepare_cached(
            "SELECT t.name, COUNT(bt.book_id) AS n
             FROM tags t
             LEFT JOIN book_tags bt ON bt.tag_id = t.id
             GROUP BY t.id
             HAVING n > 0
             ORDER BY n DESC, t.name COLLATE NOCASE ASC",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn books_with_tag(&self, tag: &str, sort: SortKey) -> Result<Vec<Book>> {
        let conn = self.conn();
        let order = match sort {
            SortKey::Title => "books.sort_title COLLATE NOCASE ASC",
            SortKey::Author => "books.authors COLLATE NOCASE ASC",
            SortKey::Added => "books.added_at DESC",
        };
        let sql = format!(
            "SELECT {BOOK_COLUMNS}
             FROM books
             JOIN book_tags bt ON bt.book_id = books.id
             JOIN tags t ON t.id = bt.tag_id
             WHERE t.name = ?1 COLLATE NOCASE
             ORDER BY {order}"
        );
        let mut stmt = conn.prepare_cached(&sql)?;
        let rows = stmt.query_map(params![tag], row_to_book)?;
        let mut books = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        hydrate_books(&conn, &mut books)?;
        Ok(books)
    }

    // -----------------------------------------------------------------------
    // P4: Analytics
    // -----------------------------------------------------------------------

    /// Write a consistent copy of the catalog to `dest`.
    ///
    /// Uses SQLite's own VACUUM INTO, so the result is a defragmented, valid
    /// database even while the app is running — unlike copying the file, which
    /// can catch a half-written WAL.
    pub fn backup_to(&self, dest: &Path) -> Result<u64> {
        if dest.exists() {
            fs::remove_file(dest)?;
        }
        let conn = self.conn();
        // The path is interpolated because VACUUM INTO does not take a bound
        // parameter; single quotes are escaped to keep it safe.
        let escaped = dest.to_string_lossy().replace('\'', "''");
        conn.execute_batch(&format!("VACUUM INTO '{escaped}'"))?;
        drop(conn);
        Ok(fs::metadata(dest).map(|m| m.len()).unwrap_or(0))
    }

    /// SQLite's write counter. Any INSERT/UPDATE/DELETE bumps it, so callers
    /// can cheaply tell whether the catalog changed since they last looked.
    pub fn change_token(&self) -> i64 {
        self.conn
            .lock()
            .map(|c| c.total_changes() as i64)
            .unwrap_or(0)
    }

    pub fn library_stats(&self) -> Result<LibraryStats> {
        // Cheap: total_changes() is an in-memory counter, not a query.
        let version = {
            let conn = self.conn();
            conn.total_changes() as i64
        };

        if let Ok(cache) = self.stats_cache.lock() {
            if let Some((cached_version, stats)) = cache.as_ref() {
                if *cached_version == version {
                    return Ok(stats.clone());
                }
            }
        }

        let stats = self.compute_library_stats()?;
        if let Ok(mut cache) = self.stats_cache.lock() {
            *cache = Some((version, stats.clone()));
        }
        Ok(stats)
    }

    fn compute_library_stats(&self) -> Result<LibraryStats> {
        let conn = self.conn();

        let one =
            |sql: &str| -> i64 { conn.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap_or(0) };

        let mut s = LibraryStats {
            total_books: one("SELECT COUNT(*) FROM books"),
            finished: one("SELECT COUNT(*) FROM books
                 WHERE IFNULL(finished_at,'') <> '' OR progress >= 100"),
            reading: one("SELECT COUNT(*) FROM books
                 WHERE progress > 0 AND progress < 100 AND IFNULL(finished_at,'') = ''"),
            unread: one("SELECT COUNT(*) FROM books
                 WHERE progress <= 0 AND IFNULL(finished_at,'') = ''"),
            highlights: one("SELECT COUNT(*) FROM annotations WHERE kind = 'highlight'"),
            quotes: one("SELECT COUNT(*) FROM annotations WHERE kind = 'quote'"),
            saved_words: one("SELECT COUNT(*) FROM saved_words"),
            shelves: one("SELECT COUNT(*) FROM shelves"),
            reading_list: one("SELECT COUNT(*) FROM reading_list"),
            total_seconds: one("SELECT IFNULL(SUM(seconds), 0) FROM reading_sessions"),
            sessions: one("SELECT COUNT(*) FROM reading_sessions WHERE seconds > 0"),
            ..LibraryStats::default()
        };

        let cutoff_7 = iso_days_ago(7);
        let cutoff_30 = iso_days_ago(30);
        s.seconds_last_7 = conn
            .query_row(
                "SELECT IFNULL(SUM(seconds), 0) FROM reading_sessions WHERE started_at >= ?1",
                params![cutoff_7],
                |r| r.get(0),
            )
            .unwrap_or(0);
        s.seconds_last_30 = conn
            .query_row(
                "SELECT IFNULL(SUM(seconds), 0) FROM reading_sessions WHERE started_at >= ?1",
                params![cutoff_30],
                |r| r.get(0),
            )
            .unwrap_or(0);
        s.finished_last_30 = conn
            .query_row(
                "SELECT COUNT(*) FROM reading_events WHERE kind = 'finished' AND at >= ?1",
                params![cutoff_30],
                |r| r.get(0),
            )
            .unwrap_or(0);
        s.added_last_30 = conn
            .query_row(
                "SELECT COUNT(*) FROM books WHERE added_at >= ?1",
                params![cutoff_30],
                |r| r.get(0),
            )
            .unwrap_or(0);

        // Books added per month, last 6 calendar months present in the data.
        {
            let mut stmt = conn.prepare_cached(
                "SELECT substr(added_at, 1, 7) AS ym, COUNT(*)
                 FROM books
                 GROUP BY ym
                 ORDER BY ym DESC
                 LIMIT 6",
            )?;
            let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
            let mut months = rows.collect::<std::result::Result<Vec<_>, _>>()?;
            months.reverse();
            s.added_by_month = months;
        }

        // Reading minutes per day for the last 14 days (zero-filled).
        {
            let mut stmt = conn.prepare_cached(
                "SELECT substr(started_at, 1, 10) AS d, IFNULL(SUM(seconds), 0)
                 FROM reading_sessions
                 WHERE started_at >= ?1
                 GROUP BY d",
            )?;
            let cutoff_14 = iso_days_ago(13);
            let rows = stmt.query_map(params![cutoff_14], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })?;
            let found = rows.collect::<std::result::Result<Vec<_>, _>>()?;
            let mut series = Vec::with_capacity(14);
            for back in (0..14).rev() {
                let day = iso_days_ago(back);
                let key = day[..10].to_string();
                let secs = found
                    .iter()
                    .find(|(d, _)| *d == key)
                    .map(|(_, v)| *v)
                    .unwrap_or(0);
                series.push((key, secs));
            }
            s.minutes_by_day = series;
        }

        // Average over *active* days only — dividing by 30 when you read on 3 of
        // them reports a demoralising and fairly meaningless number.
        {
            let (total, days): (i64, i64) = conn
                .query_row(
                    "SELECT IFNULL(SUM(seconds), 0),
                            COUNT(DISTINCT substr(started_at, 1, 10))
                     FROM reading_sessions
                     WHERE started_at >= ?1 AND seconds > 0",
                    params![cutoff_30],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap_or((0, 0));
            s.avg_minutes_per_active_day = if days > 0 { total / days / 60 } else { 0 };
        }

        {
            let mut stmt = conn.prepare_cached(
                "SELECT t.name, COUNT(bt.book_id) AS n
                 FROM tags t JOIN book_tags bt ON bt.tag_id = t.id
                 GROUP BY t.id ORDER BY n DESC, t.name COLLATE NOCASE LIMIT 8",
            )?;
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            s.top_tags = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        }

        {
            let mut stmt = conn.prepare_cached(
                "SELECT authors, COUNT(*) AS n FROM books
                 WHERE TRIM(authors) <> ''
                 GROUP BY authors COLLATE NOCASE
                 ORDER BY n DESC, authors COLLATE NOCASE LIMIT 8",
            )?;
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            s.top_authors = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        }

        {
            let mut stmt = conn.prepare_cached(
                "SELECT books.title, SUM(rs.seconds) AS n
                 FROM reading_sessions rs JOIN books ON books.id = rs.book_id
                 GROUP BY rs.book_id
                 HAVING n > 0
                 ORDER BY n DESC LIMIT 5",
            )?;
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            s.most_read = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        }

        // Streaks over distinct days that have any reading session.
        {
            let mut stmt = conn.prepare_cached(
                "SELECT DISTINCT substr(started_at, 1, 10) FROM reading_sessions
                 WHERE seconds > 0 ORDER BY 1 DESC",
            )?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            let days = rows.collect::<std::result::Result<Vec<_>, _>>()?;
            let (current, longest) = streaks(&days);
            s.current_streak_days = current;
            s.longest_streak_days = longest;
        }

        Ok(s)
    }
}

/// Shared projection so every book query returns the same column order.
const BOOK_COLUMNS: &str = "books.id, books.uuid, books.title, books.authors, books.series, \
     books.description, books.format, books.file_name, books.file_hash, books.cover_name, \
     books.added_at, books.progress, books.rating, books.publisher, books.published, \
     books.series_index";

/// Sessions longer than this are almost certainly an idle window.
const MAX_SESSION_SECONDS: i64 = 6 * 60 * 60;

/// Fill tags + resolved paths for a freshly queried batch of books.
fn hydrate_books(conn: &Connection, books: &mut [Book]) -> Result<()> {
    if books.is_empty() {
        return Ok(());
    }

    // One query for every book's tags, rather than one query per book. With a
    // few hundred books the old loop was the dominant cost of opening any page
    // that showed a list.
    let ids: Vec<String> = books.iter().map(|b| b.id.to_string()).collect();
    let sql = format!(
        "SELECT bt.book_id, t.name FROM tags t
         JOIN book_tags bt ON bt.tag_id = t.id
         WHERE bt.book_id IN ({})
         ORDER BY t.name COLLATE NOCASE",
        ids.join(",")
    );

    let mut by_book: std::collections::HashMap<i64, Vec<String>> = std::collections::HashMap::new();
    {
        // Plain prepare: the id list makes this SQL unique per call, so caching
        // it would grow the statement cache without bound.
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows {
            let (book_id, tag) = row?;
            by_book.entry(book_id).or_default().push(tag);
        }
    }

    for book in books.iter_mut() {
        book.tags = by_book.remove(&book.id).unwrap_or_default();
        book.cover_path = book
            .cover_name
            .as_ref()
            .map(|name| book_dir(&book.uuid).join(name));
        book.file_path = book_dir(&book.uuid).join(&book.file_name);
    }
    Ok(())
}

fn row_to_shelf(row: &rusqlite::Row<'_>) -> rusqlite::Result<Shelf> {
    let kind: String = row.get(2)?;
    Ok(Shelf {
        id: row.get(0)?,
        name: row.get(1)?,
        kind: ShelfKind::from_str_lossy(&kind),
        description: row.get(3)?,
        rules: row.get(4)?,
        position: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        book_count: 0,
    })
}

/// `ALTER TABLE ... ADD COLUMN` guarded by a PRAGMA lookup, so migrations stay
/// idempotent on databases created by older versions.
fn add_column_if_missing(conn: &Connection, table: &str, column: &str, ty: &str) -> Result<()> {
    let mut stmt = conn.prepare_cached(&format!("PRAGMA table_info({table})"))?;
    let existing: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if !existing.iter().any(|c| c.eq_ignore_ascii_case(column)) {
        conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {ty}"))?;
    }
    Ok(())
}

/// Longest and current run of consecutive days, given day keys (`YYYY-MM-DD`)
/// sorted newest first.
fn streaks(days_desc: &[String]) -> (i64, i64) {
    if days_desc.is_empty() {
        return (0, 0);
    }
    let nums: Vec<i64> = days_desc.iter().filter_map(|d| days_from_iso(d)).collect();
    if nums.is_empty() {
        return (0, 0);
    }

    let today = days_from_iso(&chrono_like_now()[..10]).unwrap_or(nums[0]);
    // A streak is "current" if the newest day is today or yesterday.
    let mut current = 0;
    if today - nums[0] <= 1 {
        current = 1;
        for pair in nums.windows(2) {
            if pair[0] - pair[1] == 1 {
                current += 1;
            } else {
                break;
            }
        }
    }

    let mut longest = 1;
    let mut run = 1;
    for pair in nums.windows(2) {
        if pair[0] - pair[1] == 1 {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 1;
        }
    }
    (current, longest)
}

fn days_from_iso(s: &str) -> Option<i64> {
    let bytes = s.as_bytes();
    if bytes.len() < 10 {
        return None;
    }
    let y: i64 = s[0..4].parse().ok()?;
    let m: i64 = s[5..7].parse().ok()?;
    let d: i64 = s[8..10].parse().ok()?;
    Some(days_from_civil(y, m, d))
}

/// Howard Hinnant's `days_from_civil` — inverse of `civil_from_days` below.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Whole days since the Unix epoch, for weekday arithmetic.
pub fn days_since_epoch() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_secs() / 86400) as i64)
        .unwrap_or(0)
}

/// ISO-8601 UTC timestamp for midnight `days` ago — used by date-window rules.
pub fn iso_days_ago(days: i64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let then = (now - days.max(0) * 86400).max(0);
    // Snap to midnight so "last 7 days" means 7 whole days.
    let midnight = then - (then % 86400);
    format_unix_utc(midnight as u64)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Title,
    Author,
    Added,
}

impl SortKey {
    pub fn label(self) -> &'static str {
        match self {
            SortKey::Title => "Title",
            SortKey::Author => "Author",
            SortKey::Added => "Added",
        }
    }

    pub const ALL: &'static [SortKey] = &[SortKey::Title, SortKey::Author, SortKey::Added];
}

fn row_to_book(row: &rusqlite::Row<'_>) -> rusqlite::Result<Book> {
    let format_str: String = row.get(6)?;
    Ok(Book {
        id: row.get(0)?,
        uuid: row.get(1)?,
        title: row.get(2)?,
        authors: row.get(3)?,
        series: row.get(4)?,
        description: row.get(5)?,
        format: BookFormat::from_str_lossy(&format_str),
        file_name: row.get(7)?,
        file_hash: row.get(8)?,
        cover_name: row.get(9)?,
        added_at: row.get(10)?,
        progress: row.get::<_, i64>(11)? as u8,
        // Older rows predate the column; treat a read failure as unrated.
        rating: row.get::<_, i64>(12).unwrap_or(0) as u8,
        publisher: row.get::<_, String>(13).unwrap_or_default(),
        published: row.get::<_, String>(14).unwrap_or_default(),
        series_index: row.get::<_, f64>(15).unwrap_or(0.0) as f32,
        tags: Vec::new(),
        cover_path: None,
        file_path: PathBuf::new(),
    })
}

fn row_to_annotation(row: &rusqlite::Row<'_>) -> rusqlite::Result<Annotation> {
    Ok(Annotation {
        id: row.get(0)?,
        book_id: row.get(1)?,
        kind: row.get(2)?,
        chapter_index: row.get(3)?,
        start_path: row.get(4)?,
        start_offset: row.get(5)?,
        end_path: row.get(6)?,
        end_offset: row.get(7)?,
        color: row.get(8)?,
        text_excerpt: row.get(9)?,
        note: row.get(10)?,
        cfi: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn row_to_saved_word(row: &rusqlite::Row<'_>) -> rusqlite::Result<SavedWord> {
    Ok(SavedWord {
        id: row.get(0)?,
        word: row.get(1)?,
        definition: row.get(2)?,
        dict_name: row.get(3)?,
        book_id: row.get(4)?,
        chapter_index: row.get(5)?,
        context_text: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn tags_for_book(conn: &Connection, book_id: i64) -> Result<Vec<String>> {
    let mut stmt = conn.prepare_cached(
        "SELECT t.name FROM tags t
         JOIN book_tags bt ON bt.tag_id = t.id
         WHERE bt.book_id = ?1
         ORDER BY t.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map(params![book_id], |r| r.get(0))?;
    let mut tags = Vec::new();
    for t in rows {
        tags.push(t?);
    }
    Ok(tags)
}

fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_unix_utc(secs)
}

fn format_unix_utc(secs: u64) -> String {
    let days = (secs / 86400) as i64;
    let tod = secs % 86400;
    let (y, m, d) = civil_from_days(days);
    let hh = tod / 3600;
    let mm = (tod % 3600) / 60;
    let ss = tod % 60;
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

pub fn hash_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shelf_rules::{MatchMode, Rule, RuleField, RuleOp, RuleSet};

    fn seed(cat: &Catalog, title: &str, authors: &str, tags: &[&str]) -> i64 {
        let uuid = format!("uuid-{title}");
        let tags: Vec<String> = tags.iter().map(|t| t.to_string()).collect();
        cat.insert_book(
            &uuid,
            title,
            authors,
            None,
            "",
            BookFormat::Epub,
            "book.epub",
            &format!("hash-{title}"),
            None,
            &tags,
        )
        .expect("insert")
    }

    #[test]
    fn migrations_land_on_current_version() {
        let cat = Catalog::open_in_memory().unwrap();
        let conn = cat.conn.lock().unwrap();
        let v: i64 = conn
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(v, SCHEMA_VERSION);
    }

    #[test]
    fn manual_shelf_membership_round_trips() {
        let cat = Catalog::open_in_memory().unwrap();
        let a = seed(&cat, "Dune", "Frank Herbert", &["scifi"]);
        let b = seed(&cat, "Emma", "Jane Austen", &["classic"]);
        let shelf_id = cat
            .create_shelf("Favourites", ShelfKind::Manual, "", "")
            .unwrap();

        cat.add_book_to_shelf(shelf_id, a).unwrap();
        cat.add_book_to_shelf(shelf_id, b).unwrap();
        // Duplicate add must not create a second row.
        cat.add_book_to_shelf(shelf_id, a).unwrap();

        let shelf = cat.get_shelf(shelf_id).unwrap().unwrap();
        assert_eq!(shelf.book_count, 2);

        cat.remove_book_from_shelf(shelf_id, b).unwrap();
        let shelf = cat.get_shelf(shelf_id).unwrap().unwrap();
        assert_eq!(shelf.book_count, 1);
        assert_eq!(cat.shelves_for_book(a).unwrap().len(), 1);
    }

    #[test]
    fn smart_shelf_filters_by_tag_and_progress() {
        let cat = Catalog::open_in_memory().unwrap();
        let dune = seed(&cat, "Dune", "Frank Herbert", &["scifi", "classic"]);
        seed(&cat, "Emma", "Jane Austen", &["classic"]);
        seed(&cat, "Neuromancer", "William Gibson", &["scifi"]);

        // Dune is half read; the others are untouched.
        cat.set_reading_progress(dune, 5, 0.5, 10).unwrap();

        let mut rules = RuleSet::default();
        rules
            .rules
            .push(Rule::new(RuleField::Tag, RuleOp::Is, "scifi"));
        rules
            .rules
            .push(Rule::new(RuleField::Progress, RuleOp::Is, "unread"));

        let id = cat
            .create_shelf("Unread scifi", ShelfKind::Smart, "", &rules.to_json())
            .unwrap();
        let shelf = cat.get_shelf(id).unwrap().unwrap();
        let books = cat.shelf_books(&shelf, SortKey::Title, "").unwrap();
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].title, "Neuromancer");

        // Same rules with ANY should widen the result.
        let mut any = rules.clone();
        any.set_mode(MatchMode::Any);
        assert_eq!(cat.count_matching_rules(&any).unwrap(), 3);
    }

    #[test]
    fn smart_shelf_search_binds_alongside_rules() {
        let cat = Catalog::open_in_memory().unwrap();
        seed(&cat, "Dune", "Frank Herbert", &["scifi"]);
        seed(&cat, "Neuromancer", "William Gibson", &["scifi"]);

        let mut rules = RuleSet::default();
        rules
            .rules
            .push(Rule::new(RuleField::Tag, RuleOp::Is, "scifi"));
        let id = cat
            .create_shelf("Scifi", ShelfKind::Smart, "", &rules.to_json())
            .unwrap();
        let shelf = cat.get_shelf(id).unwrap().unwrap();

        // Regression: rule params and the search term share one statement.
        let hits = cat.shelf_books(&shelf, SortKey::Title, "neuro").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "Neuromancer");
    }

    #[test]
    fn empty_smart_shelf_matches_nothing() {
        let cat = Catalog::open_in_memory().unwrap();
        seed(&cat, "Dune", "Frank Herbert", &["scifi"]);
        let id = cat.create_shelf("Empty", ShelfKind::Smart, "", "").unwrap();
        let shelf = cat.get_shelf(id).unwrap().unwrap();
        assert_eq!(shelf.book_count, 0);
    }

    #[test]
    fn reading_list_reorders() {
        let cat = Catalog::open_in_memory().unwrap();
        let a = seed(&cat, "A", "x", &[]);
        let b = seed(&cat, "B", "x", &[]);
        let c = seed(&cat, "C", "x", &[]);
        for id in [a, b, c] {
            cat.add_to_reading_list(id).unwrap();
        }

        let titles: Vec<String> = cat
            .list_reading_list()
            .unwrap()
            .iter()
            .map(|e| e.book.title.clone())
            .collect();
        assert_eq!(titles, vec!["A", "B", "C"]);

        cat.move_reading_list_entry(c, -1).unwrap();
        let titles: Vec<String> = cat
            .list_reading_list()
            .unwrap()
            .iter()
            .map(|e| e.book.title.clone())
            .collect();
        assert_eq!(titles, vec!["A", "C", "B"]);

        // Moving past the edge is a no-op, not an error.
        cat.move_reading_list_entry(a, -1).unwrap();
        assert_eq!(cat.list_reading_list().unwrap().len(), 3);
    }

    #[test]
    fn finishing_a_book_clears_it_from_the_reading_list() {
        let cat = Catalog::open_in_memory().unwrap();
        let a = seed(&cat, "A", "x", &[]);
        cat.add_to_reading_list(a).unwrap();
        assert!(cat.is_in_reading_list(a).unwrap());

        cat.set_book_finished(a, true).unwrap();
        assert!(!cat.is_in_reading_list(a).unwrap());
        assert!(cat.book_finished_at(a).unwrap().is_some());

        cat.set_book_finished(a, false).unwrap();
        assert!(cat.book_finished_at(a).unwrap().is_none());
    }

    #[test]
    fn auto_finish_fires_once() {
        let cat = Catalog::open_in_memory().unwrap();
        let a = seed(&cat, "A", "x", &[]);
        assert!(!cat.auto_finish_if_complete(a, 50).unwrap());
        assert!(cat.auto_finish_if_complete(a, 100).unwrap());
        assert!(!cat.auto_finish_if_complete(a, 100).unwrap());
    }

    #[test]
    fn opening_twice_in_an_hour_logs_one_event() {
        let cat = Catalog::open_in_memory().unwrap();
        let a = seed(&cat, "A", "x", &[]);
        cat.mark_book_opened(a).unwrap();
        cat.mark_book_opened(a).unwrap();
        let events = cat.list_events(Some(EventKind::Opened), "", 50).unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn sessions_clamp_absurd_durations() {
        let cat = Catalog::open_in_memory().unwrap();
        let a = seed(&cat, "A", "x", &[]);
        let sid = cat.start_reading_session(a, 0).unwrap();
        cat.end_reading_session(sid, 99_999_999, 10).unwrap();
        assert_eq!(cat.total_reading_seconds(a).unwrap(), MAX_SESSION_SECONDS);
    }

    #[test]
    fn tags_browse_counts_books() {
        let cat = Catalog::open_in_memory().unwrap();
        seed(&cat, "Dune", "Frank Herbert", &["scifi", "classic"]);
        seed(&cat, "Emma", "Jane Austen", &["classic"]);

        let tags = cat.list_tags_with_counts().unwrap();
        assert_eq!(tags[0], ("classic".to_string(), 2));
        assert_eq!(
            cat.books_with_tag("scifi", SortKey::Title).unwrap().len(),
            1
        );
    }

    #[test]
    fn stats_reflect_library_state() {
        let cat = Catalog::open_in_memory().unwrap();
        let a = seed(&cat, "A", "x", &["t"]);
        seed(&cat, "B", "y", &[]);
        cat.set_reading_progress(a, 1, 0.5, 10).unwrap();

        let stats = cat.library_stats().unwrap();
        assert_eq!(stats.total_books, 2);
        assert_eq!(stats.reading, 1);
        assert_eq!(stats.unread, 1);
        assert_eq!(stats.minutes_by_day.len(), 14);
    }

    #[test]
    fn edits_survive_delete_and_reimport() {
        let cat = Catalog::open_in_memory().unwrap();
        let id = seed(&cat, "Nyxia", "S. Reintgen", &["scifi"]);
        let hash = cat.get_book(id).unwrap().unwrap().file_hash;

        cat.update_book_metadata(
            id,
            "Nyxia Uprising",
            "Scott Reintgen",
            Some("The Nyxia Triad"),
            3.0,
            "Random House",
            "2019",
            "Edited by hand.",
            &["scifi".into(), "ya".into()],
        )
        .unwrap();
        cat.set_book_rating(id, 9).unwrap();

        // Removing the book must not discard the edits.
        cat.delete_book(id).unwrap();
        assert!(cat.get_book(id).unwrap().is_none());

        // Re-import: same file, so the same hash.
        let new_id = cat
            .insert_book(
                "uuid-again",
                "Nyxia",
                "S. Reintgen",
                None,
                "",
                BookFormat::Epub,
                "book.epub",
                &hash,
                None,
                &[],
            )
            .unwrap();
        assert!(cat.restore_overrides(new_id, &hash).unwrap());

        let restored = cat.get_book(new_id).unwrap().unwrap();
        assert_eq!(restored.title, "Nyxia Uprising");
        assert_eq!(restored.authors, "Scott Reintgen");
        assert_eq!(restored.series.as_deref(), Some("The Nyxia Triad"));
        assert_eq!(restored.series_index, 3.0);
        assert_eq!(restored.publisher, "Random House");
        assert_eq!(restored.rating, 9);
        assert_eq!(restored.tags.len(), 2);
    }

    #[test]
    fn a_restored_book_keeps_its_cover() {
        let cat = Catalog::open_in_memory().unwrap();
        let id = seed(&cat, "A", "x", &[]);
        let book = cat.get_book(id).unwrap().unwrap();
        let hash = book.file_hash.clone();

        // Put a real file where the cover is expected.
        let dir = crate::paths::book_dir(&book.uuid);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("cover.png"), b"not-really-a-png-but-long-enough").unwrap();
        cat.set_cover_name(id, Some("cover.png")).unwrap();

        cat.delete_book(id).unwrap();

        let new_id = cat
            .insert_book(
                "uuid-restored-cover",
                "A",
                "x",
                None,
                "",
                BookFormat::Epub,
                "book.epub",
                &hash,
                None,
                &[],
            )
            .unwrap();
        assert!(cat.restore_overrides(new_id, &hash).unwrap());

        let restored = cat.get_book(new_id).unwrap().unwrap();
        assert!(restored.cover_name.is_some(), "cover was not restored");
        assert!(
            restored.cover_path.as_ref().is_some_and(|p| p.is_file()),
            "cover file missing on disk"
        );

        // Tidy up so the test does not leave files in the real data dir.
        let _ = std::fs::remove_dir_all(crate::paths::book_dir(&restored.uuid));
        let _ = cat.forget_overrides(&hash);
    }

    #[test]
    fn replacing_a_cover_updates_the_remembered_copy() {
        // The bug: editing metadata stashed the cover, then the new cover was
        // written afterwards, so the override kept the *previous* jacket and a
        // re-import restored the wrong image.
        let cat = Catalog::open_in_memory().unwrap();
        let id = seed(&cat, "A", "x", &[]);
        let book = cat.get_book(id).unwrap().unwrap();
        let hash = book.file_hash.clone();
        let dir = crate::paths::book_dir(&book.uuid);
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("old.png"), vec![b'o'; 600]).unwrap();
        cat.set_cover_name(id, Some("old.png")).unwrap();

        // Swap in a different cover, as the metadata editor does.
        std::fs::write(dir.join("new.png"), vec![b'n'; 600]).unwrap();
        cat.set_cover_name(id, Some("new.png")).unwrap();

        cat.delete_book(id).unwrap();
        let new_id = cat
            .insert_book(
                "uuid-cover-swap",
                "A",
                "x",
                None,
                "",
                BookFormat::Epub,
                "book.epub",
                &hash,
                None,
                &[],
            )
            .unwrap();
        assert!(cat.restore_overrides(new_id, &hash).unwrap());

        let restored = cat.get_book(new_id).unwrap().unwrap();
        let bytes = std::fs::read(restored.cover_path.as_ref().unwrap()).unwrap();
        assert_eq!(bytes[0], b'n', "restored the pre-swap cover");

        let _ = std::fs::remove_dir_all(crate::paths::book_dir(&restored.uuid));
        let _ = std::fs::remove_dir_all(dir);
        let _ = cat.forget_overrides(&hash);
    }

    #[test]
    fn restoring_does_not_clobber_the_stashed_cover() {
        // Regression: restore_overrides applies metadata through the normal
        // edit path, which re-stashes. At that moment the book still has the
        // freshly-imported cover, so the saved image was overwritten with the
        // EPUB default a moment before it was due to be copied back.
        let cat = Catalog::open_in_memory().unwrap();
        let id = seed(&cat, "A", "x", &[]);
        let book = cat.get_book(id).unwrap().unwrap();
        let hash = book.file_hash.clone();
        let dir = crate::paths::book_dir(&book.uuid);
        std::fs::create_dir_all(&dir).unwrap();

        // A distinctive custom cover.
        std::fs::write(dir.join("custom.png"), vec![b'C'; 800]).unwrap();
        cat.set_cover_name(id, Some("custom.png")).unwrap();
        cat.delete_book(id).unwrap();

        // Re-import: the new book arrives with a *different* cover on disk,
        // standing in for whatever the EPUB supplies.
        let new_id = cat
            .insert_book(
                "uuid-clobber",
                "A",
                "x",
                None,
                "",
                BookFormat::Epub,
                "book.epub",
                &hash,
                Some("epub-default.png"),
                &[],
            )
            .unwrap();
        let fresh_dir = crate::paths::book_dir("uuid-clobber");
        std::fs::create_dir_all(&fresh_dir).unwrap();
        std::fs::write(fresh_dir.join("epub-default.png"), vec![b'E'; 800]).unwrap();

        assert!(cat.restore_overrides(new_id, &hash).unwrap());

        let restored = cat.get_book(new_id).unwrap().unwrap();
        let bytes = std::fs::read(restored.cover_path.as_ref().unwrap()).unwrap();
        assert_eq!(
            bytes[0], b'C',
            "restored the EPUB default instead of the saved cover"
        );

        let _ = std::fs::remove_dir_all(&fresh_dir);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = cat.forget_overrides(&hash);
    }

    #[test]
    fn a_different_file_gets_no_overrides() {
        let cat = Catalog::open_in_memory().unwrap();
        let id = seed(&cat, "A", "x", &[]);
        cat.update_book_metadata(id, "Edited", "x", None, 0.0, "", "", "", &[])
            .unwrap();

        // A book whose bytes differ has a different hash and must be untouched.
        let other = seed(&cat, "B", "y", &[]);
        let other_hash = cat.get_book(other).unwrap().unwrap().file_hash;
        assert!(!cat.restore_overrides(other, &other_hash).unwrap());
        assert_eq!(cat.get_book(other).unwrap().unwrap().title, "B");
    }

    #[test]
    fn forgetting_overrides_gives_a_clean_import() {
        let cat = Catalog::open_in_memory().unwrap();
        let id = seed(&cat, "A", "x", &[]);
        let hash = cat.get_book(id).unwrap().unwrap().file_hash;
        cat.update_book_metadata(id, "Edited", "x", None, 0.0, "", "", "", &[])
            .unwrap();

        cat.forget_overrides(&hash).unwrap();
        assert!(!cat.restore_overrides(id, &hash).unwrap());
    }

    #[test]
    fn stats_cache_refreshes_after_a_write() {
        let cat = Catalog::open_in_memory().unwrap();
        seed(&cat, "A", "x", &[]);
        assert_eq!(cat.library_stats().unwrap().total_books, 1);

        // A second call must hit the cache and still be correct.
        assert_eq!(cat.library_stats().unwrap().total_books, 1);

        // Any write bumps total_changes(), so the next read recomputes.
        seed(&cat, "B", "y", &[]);
        assert_eq!(cat.library_stats().unwrap().total_books, 2);
    }

    #[test]
    fn recent_books_is_bounded_and_newest_first() {
        let cat = Catalog::open_in_memory().unwrap();
        for name in ["A", "B", "C"] {
            seed(&cat, name, "x", &["t"]);
        }
        let books = cat.recent_books(2).unwrap();
        assert_eq!(books.len(), 2);
        // Tags must still be hydrated by the batched lookup.
        assert_eq!(books[0].tags, vec!["t".to_string()]);
    }

    #[test]
    fn batched_tag_hydration_matches_per_book() {
        let cat = Catalog::open_in_memory().unwrap();
        seed(&cat, "Dune", "Herbert", &["scifi", "classic"]);
        seed(&cat, "Emma", "Austen", &["classic"]);
        seed(&cat, "Bare", "Nobody", &[]);

        let books = cat.list_books(SortKey::Title, "").unwrap();
        let find = |t: &str| books.iter().find(|b| b.title == t).unwrap().tags.clone();
        assert_eq!(find("Dune").len(), 2);
        assert_eq!(find("Emma"), vec!["classic".to_string()]);
        // A book with no tags must come back empty, not missing.
        assert!(find("Bare").is_empty());
    }

    #[test]
    fn recent_quotes_carries_its_book_title() {
        let cat = Catalog::open_in_memory().unwrap();
        let id = seed(&cat, "Dune", "Herbert", &[]);
        cat.insert_annotation(
            id,
            "quote",
            0,
            "p",
            0,
            "p",
            9,
            "yellow",
            "Fear is the mind-killer",
            "",
        )
        .unwrap();

        let quotes = cat.recent_quotes(5).unwrap();
        assert_eq!(quotes.len(), 1);
        assert_eq!(quotes[0].1, "Dune");
        assert_eq!(cat.count_quotes().unwrap(), 1);
    }

    #[test]
    fn metadata_edit_round_trips_new_fields() {
        let cat = Catalog::open_in_memory().unwrap();
        let id = seed(&cat, "Dune", "Frank Herbert", &["scifi"]);

        cat.update_book_metadata(
            id,
            "Dune Messiah",
            "Frank Herbert",
            Some("Dune"),
            2.5,
            "Ace",
            "1969",
            "Sequel.",
            &["scifi".into(), "classic".into()],
        )
        .unwrap();

        let b = cat.get_book(id).unwrap().unwrap();
        assert_eq!(b.title, "Dune Messiah");
        assert_eq!(b.series.as_deref(), Some("Dune"));
        assert_eq!(b.series_index, 2.5);
        assert_eq!(b.publisher, "Ace");
        assert_eq!(b.published, "1969");
        assert_eq!(b.tags.len(), 2);
        // Fractional indexes must not render as "2.5.0".
        assert_eq!(b.series_display().as_deref(), Some("Dune #2.5"));
    }

    #[test]
    fn whole_series_numbers_drop_the_decimal() {
        let cat = Catalog::open_in_memory().unwrap();
        let id = seed(&cat, "A", "x", &[]);
        cat.update_book_metadata(id, "A", "x", Some("Trilogy"), 3.0, "", "", "", &[])
            .unwrap();
        let b = cat.get_book(id).unwrap().unwrap();
        assert_eq!(b.series_display().as_deref(), Some("Trilogy #3"));
    }

    #[test]
    fn editing_tags_prunes_orphans() {
        let cat = Catalog::open_in_memory().unwrap();
        let id = seed(&cat, "A", "x", &["temporary"]);
        cat.update_book_metadata(id, "A", "x", None, 0.0, "", "", "", &["kept".into()])
            .unwrap();
        let tags = cat.list_tags_with_counts().unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].0, "kept");
    }

    #[test]
    fn shelf_name_collisions_are_detected() {
        let cat = Catalog::open_in_memory().unwrap();
        let id = cat
            .create_shelf("Favourites", ShelfKind::Manual, "", "")
            .unwrap();
        assert!(cat.shelf_name_taken("favourites", None).unwrap());
        // The shelf being edited doesn't collide with itself.
        assert!(!cat.shelf_name_taken("Favourites", Some(id)).unwrap());
        assert!(!cat.shelf_name_taken("Other", None).unwrap());
    }

    #[test]
    fn streak_helpers_handle_gaps() {
        assert_eq!(streaks(&[]), (0, 0));
        let days = vec![
            "2026-01-10".to_string(),
            "2026-01-09".to_string(),
            "2026-01-05".to_string(),
        ];
        // Not adjacent to today, so current is 0 but the run of 2 is longest.
        let (_, longest) = streaks(&days);
        assert_eq!(longest, 2);
    }

    #[test]
    fn iso_days_ago_is_midnight_aligned() {
        let s = iso_days_ago(3);
        assert!(s.ends_with("T00:00:00Z"), "{s}");
    }
}
