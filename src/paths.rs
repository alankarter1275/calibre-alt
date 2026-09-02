//! XDG paths for Kalam data and config.

use std::fs;
use std::path::PathBuf;

/// `~/.local/share/kalam`
pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs_next_home()
                .map(|h| h.join(".local/share"))
                .unwrap_or_else(|| PathBuf::from("."))
        });
    base.join("kalam")
}

/// `~/.local/share/kalam/catalog.db`
pub fn catalog_db() -> PathBuf {
    data_dir().join("catalog.db")
}

/// `~/.local/share/kalam/library`
pub fn library_dir() -> PathBuf {
    data_dir().join("library")
}

/// `~/.local/share/kalam/library/<uuid>`
pub fn book_dir(uuid: &str) -> PathBuf {
    library_dir().join(uuid)
}

/// Extracted EPUB cache for the reader.
pub fn reader_cache_dir(uuid: &str) -> PathBuf {
    data_dir().join("cache").join("reader").join(uuid)
}

/// `~/.local/share/kalam/covers` — covers kept for remembered metadata.
///
/// Separate from `library/<uuid>/` because that directory is deleted with the
/// book; these must outlive it so a re-import can restore the chosen cover.
pub fn override_covers_dir() -> PathBuf {
    data_dir().join("covers")
}

/// `~/.local/share/kalam/authors` — cached author photos.
pub fn authors_dir() -> PathBuf {
    data_dir().join("authors")
}

/// `~/.local/share/kalam/series-covers` — covers for remote series works,
/// fetched with the series cache and named by their Open Library cover id.
pub fn series_covers_dir() -> PathBuf {
    data_dir().join("series-covers")
}

/// `~/.local/share/kalam/dictionaries`
pub fn dictionaries_dir() -> PathBuf {
    data_dir().join("dictionaries")
}

/// `~/.local/share/kalam/cache/thumbs` — persistent cover thumbnails (A0 step 3).
///
/// Unlike the in-memory `COVER_CACHE` (which dies at relaunch), these stay on
/// disk so a relaunched library grid decodes a tiny 256×408 PNG instead of the
/// full cover on every open.
pub fn thumbs_dir() -> PathBuf {
    data_dir().join("cache").join("thumbs")
}

/// Thumbnail path for a book's uuid.
pub fn thumbnail_path(uuid: &str) -> PathBuf {
    thumbs_dir().join(format!("{uuid}.png"))
}

/// Derive the thumbnail path for a *library* cover path.
///
/// Covers live at `library/<uuid>/cover.ext`, so the parent directory name is
/// the uuid. Returns `None` for any path that is not a library cover (e.g. a
/// stashed override or a remote series cover) — those simply decode full.
pub fn thumbnail_for_cover(cover: &Path) -> Option<PathBuf> {
    let uuid = cover.parent()?.file_name()?.to_str()?;
    Some(thumbnail_path(uuid))
}

fn dirs_next_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

pub fn ensure_data_dirs() -> std::io::Result<()> {
    fs::create_dir_all(data_dir())?;
    fs::create_dir_all(library_dir())?;
    fs::create_dir_all(data_dir().join("cache").join("reader"))?;
    fs::create_dir_all(thumbs_dir())?;
    fs::create_dir_all(dictionaries_dir())?;
    fs::create_dir_all(override_covers_dir())?;
    fs::create_dir_all(authors_dir())?;
    fs::create_dir_all(series_covers_dir())?;
    Ok(())
}

/// `~/.local/share/kalam/cache/reader`
pub fn reader_cache_root() -> PathBuf {
    data_dir().join("cache").join("reader")
}

/// Total bytes held by extracted-EPUB caches.
pub fn reader_cache_size() -> u64 {
    dir_size(&reader_cache_root())
}

/// Delete every extracted book cache. They are rebuilt on next open.
pub fn clear_reader_cache() -> (usize, u64) {
    let root = reader_cache_root();
    let freed = dir_size(&root);
    let mut removed = 0;
    if let Ok(entries) = fs::read_dir(&root) {
        for entry in entries.flatten() {
            if fs::remove_dir_all(entry.path()).is_ok() {
                removed += 1;
            }
        }
    }
    (removed, freed)
}

/// Drop caches for books no longer in the library, and any older than
/// `max_age_days`. Called at startup so the cache cannot grow forever.
pub fn prune_reader_cache(live_uuids: &[String], max_age_days: u64) -> u64 {
    use std::time::{Duration, SystemTime};

    let root = reader_cache_root();
    let Ok(entries) = fs::read_dir(&root) else {
        return 0;
    };
    let max_age = Duration::from_secs(max_age_days * 24 * 60 * 60);
    let now = SystemTime::now();
    let mut freed = 0;

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        // A cache for a deleted book is dead weight regardless of age.
        let orphaned = !live_uuids.iter().any(|u| u == name);
        let stale = entry
            .metadata()
            .and_then(|m| m.accessed().or_else(|_| m.modified()))
            .ok()
            .and_then(|t| now.duration_since(t).ok())
            .is_some_and(|age| age > max_age);

        if orphaned || stale {
            let size = dir_size(&path);
            if fs::remove_dir_all(&path).is_ok() {
                freed += size;
            }
        }
    }
    freed
}

fn dir_size(path: &std::path::Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|e| match e.metadata() {
            Ok(m) if m.is_dir() => dir_size(&e.path()),
            Ok(m) => m.len(),
            Err(_) => 0,
        })
        .sum()
}
