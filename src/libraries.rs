//! P6.5 — which library we are looking at, and where it lives.
//!
//! Kalam used to keep exactly one library in a fixed place. This module is the
//! piece that lets you choose the folder and keep several. Everything else in
//! the app is unchanged: `paths::data_dir()` asks this module where the active
//! library is, and the ~30 helpers built on `data_dir()` follow automatically.
//!
//! **Why the list cannot live inside a library.** A setting stored in a
//! library cannot be read before you know which library to open — the
//! chicken-and-egg. So the registry is a small file in the *config* directory,
//! outside every library. That is the one genuinely machine-specific piece of
//! state in the app, and correctly so: it describes *this computer*, not the
//! books.
//!
//! **What travels and what does not.** A library folder holds books, covers
//! and its own `catalog.db` — everything about the books. Dictionaries, the
//! theme and app preferences stay global; otherwise you would reinstall
//! dictionaries every time you switched library.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One library the user knows about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryEntry {
    /// What the user calls it. Shown in the switcher.
    pub name: String,
    /// Where it lives. Absolute.
    pub path: PathBuf,
}

/// The registry file: every library, and which one is open.
///
/// Deliberately plain data with no behaviour, so it can be serialised, tested
/// and reasoned about without touching the filesystem.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryRegistry {
    #[serde(default)]
    pub libraries: Vec<LibraryEntry>,
    /// Index into `libraries`. `None` = nothing chosen yet.
    #[serde(default)]
    pub active: Option<usize>,
}

impl LibraryRegistry {
    /// The library currently open, if the registry is coherent.
    ///
    /// Returns `None` rather than panicking when `active` points nowhere — a
    /// hand-edited or truncated file must not take the app down, and the
    /// caller falls back to the legacy location.
    pub fn active(&self) -> Option<&LibraryEntry> {
        self.libraries.get(self.active?)
    }

    /// Add a library, or select it if that path is already known.
    ///
    /// Matching on path, not name: two entries pointing at the same folder
    /// would be two views of one database, and edits through one would appear
    /// to corrupt the other.
    pub fn add_or_select(&mut self, name: &str, path: &Path) -> usize {
        if let Some(i) = self.libraries.iter().position(|l| l.path == path) {
            self.active = Some(i);
            return i;
        }
        self.libraries.push(LibraryEntry {
            name: name.to_string(),
            path: path.to_path_buf(),
        });
        let i = self.libraries.len() - 1;
        self.active = Some(i);
        i
    }

    /// Forget a library. Does **not** touch the files on disk.
    ///
    /// Returns false if the index is out of range. Keeping the books is the
    /// only safe default: "remove from the list" and "delete my library" are
    /// different intentions and must never be the same button.
    pub fn forget(&mut self, index: usize) -> bool {
        if index >= self.libraries.len() {
            return false;
        }
        self.libraries.remove(index);

        // Keep `active` pointing at the same *library*, not the same slot.
        // Removing an earlier entry shifts everything after it down by one,
        // and an off-by-one here silently opens the wrong library.
        self.active = match self.active {
            Some(a) if a == index => None,
            Some(a) if a > index => Some(a - 1),
            other => other,
        };
        true
    }

    /// Select by index. False if out of range, leaving the choice unchanged.
    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.libraries.len() {
            return false;
        }
        self.active = Some(index);
        true
    }
}

// ---------------------------------------------------------------------------
// Reading and writing the registry
// ---------------------------------------------------------------------------

/// `~/.config/kalam` — settings about *this machine*, not about books.
pub fn config_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            crate::paths::home_dir()
                .map(|h| h.join(".config"))
                .unwrap_or_else(|| PathBuf::from("."))
        });
    base.join("kalam")
}

/// `~/.config/kalam/libraries.json`
pub fn registry_path() -> PathBuf {
    config_dir().join("libraries.json")
}

/// Load the registry, or an empty one.
///
/// Every failure returns the default rather than an error. A missing file is
/// the normal first-run state, and a corrupt one must not stop the app
/// starting — the fallback is the legacy fixed location, which still has the
/// user's books in it. Losing the *list* is recoverable; refusing to launch is
/// not.
pub fn load_registry() -> LibraryRegistry {
    let path = registry_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return LibraryRegistry::default();
    };
    match serde_json::from_str(&text) {
        Ok(reg) => reg,
        Err(e) => {
            eprintln!("kalam: {} is unreadable ({e}); ignoring it", path.display());
            LibraryRegistry::default()
        }
    }
}

/// Write the registry.
///
/// Written to a temporary file and renamed, so an interrupted write cannot
/// leave a half-written registry behind — the same verify-then-rename rule
/// `epub_write.rs` follows. A truncated file here would lose the list of every
/// library the user has.
pub fn save_registry(reg: &LibraryRegistry) -> std::io::Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;
    let final_path = registry_path();
    let tmp = final_path.with_extension("json.tmp");

    let json = serde_json::to_string_pretty(reg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, &final_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg_with(names: &[&str]) -> LibraryRegistry {
        let mut reg = LibraryRegistry::default();
        for n in names {
            reg.add_or_select(n, Path::new(&format!("/books/{n}")));
        }
        reg
    }

    #[test]
    fn a_fresh_install_has_no_libraries_and_no_active_one() {
        // Drives the fallback to the legacy location, so an existing install
        // keeps working with no migration.
        let reg = LibraryRegistry::default();
        assert!(reg.libraries.is_empty());
        assert_eq!(reg.active, None);
        assert_eq!(reg.active(), None);
    }

    #[test]
    fn adding_a_library_selects_it() {
        let mut reg = LibraryRegistry::default();
        reg.add_or_select("Fiction", Path::new("/books/Fiction"));
        assert_eq!(reg.active().map(|l| l.name.as_str()), Some("Fiction"));
    }

    #[test]
    fn the_same_folder_is_never_added_twice() {
        // Two entries for one folder would be two views of one database, and
        // edits through one would look like corruption in the other.
        let mut reg = reg_with(&["Fiction"]);
        let again = reg.add_or_select("A different name", Path::new("/books/Fiction"));
        assert_eq!(reg.libraries.len(), 1, "the folder was added twice");
        assert_eq!(again, 0, "should have selected the existing entry");
        assert_eq!(
            reg.libraries[0].name, "Fiction",
            "re-adding must not rename the existing library"
        );
    }

    #[test]
    fn forgetting_an_earlier_library_keeps_the_right_one_open() {
        // The bug this guards against: `active` is an index, so removing an
        // entry *before* it shifts every later entry down by one. Off by one
        // here means the app silently opens somebody else's library.
        let mut reg = reg_with(&["A", "B", "C"]);
        assert!(reg.select(2)); // C is open
        assert_eq!(reg.active().map(|l| l.name.as_str()), Some("C"));

        assert!(reg.forget(0)); // forget A
        assert_eq!(
            reg.active().map(|l| l.name.as_str()),
            Some("C"),
            "still C, at its new index"
        );
    }

    #[test]
    fn forgetting_a_later_library_does_not_move_the_open_one() {
        let mut reg = reg_with(&["A", "B", "C"]);
        assert!(reg.select(0));
        assert!(reg.forget(2));
        assert_eq!(reg.active().map(|l| l.name.as_str()), Some("A"));
    }

    #[test]
    fn forgetting_the_open_library_leaves_none_open() {
        // Deliberately not "fall back to the first one": the app should ask
        // rather than silently open a library the user did not choose.
        let mut reg = reg_with(&["A", "B"]);
        assert!(reg.select(1));
        assert!(reg.forget(1));
        assert_eq!(reg.active, None);
        assert_eq!(reg.active(), None);
    }

    #[test]
    fn out_of_range_operations_are_refused_not_obeyed() {
        let mut reg = reg_with(&["A"]);
        assert!(!reg.forget(9), "forget past the end must fail");
        assert!(!reg.select(9), "select past the end must fail");
        assert_eq!(reg.libraries.len(), 1);
        assert_eq!(
            reg.active().map(|l| l.name.as_str()),
            Some("A"),
            "a refused operation must not change what is open"
        );
    }

    #[test]
    fn a_nonsense_active_index_is_ignored_rather_than_fatal() {
        // A hand-edited or truncated registry must not take the app down; the
        // caller falls back to the legacy location, where the books still are.
        let reg = LibraryRegistry {
            libraries: vec![LibraryEntry {
                name: "A".into(),
                path: "/books/A".into(),
            }],
            active: Some(7),
        };
        assert_eq!(reg.active(), None);
    }

    #[test]
    fn the_registry_survives_a_round_trip_through_json() {
        // It is the only record of where a user's libraries are.
        let reg = reg_with(&["Fiction", "Research"]);
        let json = serde_json::to_string(&reg).unwrap();
        let back: LibraryRegistry = serde_json::from_str(&json).unwrap();
        assert_eq!(reg, back);
    }

    #[test]
    fn an_older_registry_without_every_field_still_loads() {
        // `#[serde(default)]` on both fields. A registry written by a future
        // version that gained a field must not brick an older build, and vice
        // versa -- the alternative is a user who cannot open their books.
        let back: LibraryRegistry = serde_json::from_str("{}").unwrap();
        assert_eq!(back, LibraryRegistry::default());

        let partial = r#"{"libraries":[{"name":"A","path":"/books/A"}]}"#;
        let back: LibraryRegistry = serde_json::from_str(partial).unwrap();
        assert_eq!(back.libraries.len(), 1);
        assert_eq!(back.active, None, "no active field means nothing is open");
    }
}
