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

/// `~/.local/share/kalam/dictionaries`
pub fn dictionaries_dir() -> PathBuf {
    data_dir().join("dictionaries")
}

fn dirs_next_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

pub fn ensure_data_dirs() -> std::io::Result<()> {
    fs::create_dir_all(data_dir())?;
    fs::create_dir_all(library_dir())?;
    fs::create_dir_all(data_dir().join("cache").join("reader"))?;
    fs::create_dir_all(dictionaries_dir())?;
    fs::create_dir_all(override_covers_dir())?;
    Ok(())
}
