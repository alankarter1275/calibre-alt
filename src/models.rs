//! Domain models and navigation routes.

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavItem {
    Home,
    Library,
    Shelves,
    Downloads,
    Comics,
    Ao3,
    Fanfiction,
    Settings,
}

impl NavItem {
    pub const ALL: &'static [NavItem] = &[
        NavItem::Home,
        NavItem::Library,
        NavItem::Shelves,
        NavItem::Downloads,
        NavItem::Comics,
        NavItem::Ao3,
        NavItem::Fanfiction,
        NavItem::Settings,
    ];

    pub fn label(self) -> &'static str {
        match self {
            NavItem::Home => "Home",
            NavItem::Library => "Library",
            NavItem::Shelves => "Shelves",
            NavItem::Downloads => "Downloads",
            NavItem::Comics => "Comics",
            NavItem::Ao3 => "AO3",
            NavItem::Fanfiction => "Fanfic",
            NavItem::Settings => "Settings",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            NavItem::Home => "⌂",
            NavItem::Library => "☰",
            NavItem::Shelves => "▦",
            NavItem::Downloads => "↓",
            NavItem::Comics => "▤",
            NavItem::Ao3 => "A3",
            NavItem::Fanfiction => "✎",
            NavItem::Settings => "⚙",
        }
    }

    pub fn is_bottom(self) -> bool {
        matches!(self, NavItem::Settings)
    }
}

/// Where we are inside the main content stack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Module(NavItem),
    LibrarySection(LibrarySection),
    ShelvesGrid,
    ShelfDetail {
        shelf_id: i64,
    },
    /// Books carrying one tag (P4 tag browse).
    TagBooks {
        tag: String,
    },
    BookPage {
        book_id: i64,
    },
    /// Immersive EPUB reader.
    Reader {
        book_id: i64,
    },
}

impl Route {
    pub fn title(&self) -> String {
        match self {
            Route::Module(item) => item.label().to_string(),
            Route::LibrarySection(s) => s.label().to_string(),
            Route::ShelvesGrid => "Shelves".into(),
            Route::ShelfDetail { .. } => "Shelf".into(),
            Route::TagBooks { tag } => tag.clone(),
            Route::BookPage { .. } => "Book".into(),
            Route::Reader { .. } => "Reading".into(),
        }
    }

    pub fn subtitle(&self) -> Option<String> {
        match self {
            Route::Module(NavItem::Home) => Some("Continue where you left off".into()),
            Route::Module(NavItem::Library) => Some("Catalog, lists, quotes & more".into()),
            Route::ShelvesGrid => Some("Smart and manual collections".into()),
            Route::LibrarySection(s) => Some(s.blurb().into()),
            Route::Module(NavItem::Settings) => Some("Paths and preferences".into()),
            Route::TagBooks { .. } => Some("Every book with this tag".into()),
            Route::Reader { .. } => Some("Esc back · T TOC · N/P chapter · A+/A−".into()),
            _ => None,
        }
    }

    pub fn sidebar_item(&self) -> NavItem {
        match self {
            Route::Module(item) => *item,
            Route::LibrarySection(_) => NavItem::Library,
            Route::ShelvesGrid | Route::ShelfDetail { .. } => NavItem::Shelves,
            Route::TagBooks { .. } => NavItem::Library,
            Route::BookPage { .. } | Route::Reader { .. } => NavItem::Library,
        }
    }

    pub fn is_reader(&self) -> bool {
        matches!(self, Route::Reader { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibrarySection {
    AllBooks,
    ReadingList,
    History,
    SavedQuotes,
    SavedWords,
    Tags,
    Analytics,
}

impl LibrarySection {
    /// Kept for the section pickers that will return with the definitive
    /// layout; the dashboard now routes via content sections instead of a
    /// generated tile grid.
    #[allow(dead_code)]
    pub const ALL: &'static [LibrarySection] = &[
        LibrarySection::AllBooks,
        LibrarySection::ReadingList,
        LibrarySection::History,
        LibrarySection::SavedQuotes,
        LibrarySection::SavedWords,
        LibrarySection::Tags,
        LibrarySection::Analytics,
    ];

    pub fn label(self) -> &'static str {
        match self {
            LibrarySection::AllBooks => "All books",
            LibrarySection::ReadingList => "Reading list",
            LibrarySection::History => "History",
            LibrarySection::SavedQuotes => "Saved quotes",
            LibrarySection::SavedWords => "Saved words",
            LibrarySection::Tags => "Tags",
            LibrarySection::Analytics => "Analytics",
        }
    }

    #[allow(dead_code)]
    pub fn icon(self) -> &'static str {
        match self {
            LibrarySection::AllBooks => "📚",
            LibrarySection::ReadingList => "📌",
            LibrarySection::History => "◷",
            LibrarySection::SavedQuotes => "❝",
            LibrarySection::SavedWords => "Aa",
            LibrarySection::Tags => "#",
            LibrarySection::Analytics => "◔",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            LibrarySection::AllBooks => "Everything in your library",
            LibrarySection::ReadingList => "Up next / to be read",
            LibrarySection::History => "Recently opened and finished",
            LibrarySection::SavedQuotes => "Lines you saved while reading",
            LibrarySection::SavedWords => "Vocabulary from the dictionary",
            LibrarySection::Tags => "Browse by tag",
            LibrarySection::Analytics => "Light reading stats",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookFormat {
    Epub,
    Pdf,
    Cbz,
    Cbr,
    Other,
}

impl BookFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            BookFormat::Epub => "EPUB",
            BookFormat::Pdf => "PDF",
            BookFormat::Cbz => "CBZ",
            BookFormat::Cbr => "CBR",
            BookFormat::Other => "OTHER",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_ascii_uppercase().as_str() {
            "EPUB" => BookFormat::Epub,
            "PDF" => BookFormat::Pdf,
            "CBZ" => BookFormat::Cbz,
            "CBR" => BookFormat::Cbr,
            _ => BookFormat::Other,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Book {
    pub id: i64,
    pub uuid: String,
    pub title: String,
    /// Comma-separated for P1 simplicity.
    pub authors: String,
    pub series: Option<String>,
    pub description: String,
    pub format: BookFormat,
    pub file_name: String,
    #[allow(dead_code)]
    pub file_hash: String,
    pub cover_name: Option<String>,
    pub added_at: String,
    pub progress: u8,
    pub tags: Vec<String>,
    pub cover_path: Option<PathBuf>,
    pub file_path: PathBuf,
}

impl Book {
    pub fn authors_display(&self) -> &str {
        if self.authors.trim().is_empty() {
            "Unknown"
        } else {
            self.authors.as_str()
        }
    }
}
