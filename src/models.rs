//! Placeholder domain models for P0 navigation.
//! Real persistence arrives with schema v1 / P1.

use std::sync::OnceLock;

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

    /// Simple glyph icons for P0 (no icon theme dependency).
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
    /// Top-level module page (Home, Downloads, …).
    Module(NavItem),
    /// My Library hub subsections.
    LibrarySection(LibrarySection),
    /// All shelves in a 2-column grid.
    ShelvesGrid,
    /// One shelf and the books inside it.
    ShelfDetail { shelf_id: u64 },
    /// Full book page (from shelf, library, home, …).
    BookPage { book_id: u64 },
}

impl Route {
    pub fn title(&self) -> String {
        match self {
            Route::Module(item) => item.label().to_string(),
            Route::LibrarySection(s) => s.label().to_string(),
            Route::ShelvesGrid => "Shelves".into(),
            Route::ShelfDetail { shelf_id } => sample_shelves()
                .iter()
                .find(|s| s.id == *shelf_id)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "Shelf".into()),
            Route::BookPage { book_id } => sample_books()
                .iter()
                .find(|b| b.id == *book_id)
                .map(|b| b.title.clone())
                .unwrap_or_else(|| "Book".into()),
        }
    }

    pub fn subtitle(&self) -> Option<String> {
        match self {
            Route::Module(NavItem::Home) => Some("Continue where you left off".into()),
            Route::Module(NavItem::Library) => Some("Catalog, lists, quotes & more".into()),
            Route::ShelvesGrid => Some("Smart and manual collections".into()),
            Route::ShelfDetail { shelf_id } => sample_shelves()
                .iter()
                .find(|s| s.id == *shelf_id)
                .map(|s| format!("{} · {} books", s.kind_label(), s.book_ids.len())),
            Route::BookPage { book_id } => sample_books()
                .iter()
                .find(|b| b.id == *book_id)
                .map(|b| b.authors.join(", ")),
            Route::LibrarySection(s) => Some(s.blurb().into()),
            _ => None,
        }
    }

    /// Sidebar highlight for this route.
    pub fn sidebar_item(&self) -> NavItem {
        match self {
            Route::Module(item) => *item,
            Route::LibrarySection(_) => NavItem::Library,
            Route::ShelvesGrid | Route::ShelfDetail { .. } => NavItem::Shelves,
            // Book pages keep the context we came from via nav stack;
            // default highlight Library if unknown.
            Route::BookPage { .. } => NavItem::Library,
        }
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

#[derive(Debug, Clone)]
pub struct Shelf {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub smart: bool,
    pub book_ids: Vec<u64>,
    pub accent: String,
}

impl Shelf {
    pub fn kind_label(&self) -> &'static str {
        if self.smart {
            "Smart shelf"
        } else {
            "Manual shelf"
        }
    }
}

#[derive(Debug, Clone)]
pub struct Book {
    pub id: u64,
    pub title: String,
    pub authors: Vec<String>,
    pub series: Option<String>,
    pub tags: Vec<String>,
    pub format: &'static str,
    pub progress: u8,
    pub description: String,
    pub added: &'static str,
    pub path: &'static str,
}

fn build_sample_books() -> Vec<Book> {
    vec![
        Book {
            id: 1,
            title: "The Name of the Wind".into(),
            authors: vec!["Patrick Rothfuss".into()],
            series: Some("The Kingkiller Chronicle #1".into()),
            tags: vec!["fantasy".into(), "favorites".into()],
            format: "EPUB",
            progress: 42,
            description: "A legendary figure recounts the story of his life \
                to a chronicler — demo blurb for Kalam's book page."
                .into(),
            added: "2026-01-12",
            path: "library/0001/book.epub",
        },
        Book {
            id: 2,
            title: "Project Hail Mary".into(),
            authors: vec!["Andy Weir".into()],
            series: None,
            tags: vec!["scifi".into(), "space".into()],
            format: "EPUB",
            progress: 88,
            description: "A lone astronaut wakes up with amnesia and a mission \
                to save Earth. Placeholder copy for P0."
                .into(),
            added: "2026-02-03",
            path: "library/0002/book.epub",
        },
        Book {
            id: 3,
            title: "Circe".into(),
            authors: vec!["Madeline Miller".into()],
            series: None,
            tags: vec!["mythology".into(), "literary".into()],
            format: "EPUB",
            progress: 15,
            description: "The witch of Aiaia, retold. Demo book for shelf grids.".into(),
            added: "2026-03-20",
            path: "library/0003/book.epub",
        },
        Book {
            id: 4,
            title: "The Left Hand of Darkness".into(),
            authors: vec!["Ursula K. Le Guin".into()],
            series: Some("Hainish Cycle".into()),
            tags: vec!["scifi".into(), "classics".into()],
            format: "EPUB",
            progress: 0,
            description: "Gender, politics, and ice on Gethen. Sample entry.".into(),
            added: "2026-04-01",
            path: "library/0004/book.epub",
        },
        Book {
            id: 5,
            title: "Gödel, Escher, Bach".into(),
            authors: vec!["Douglas Hofstadter".into()],
            series: None,
            tags: vec!["nonfiction".into(), "math".into()],
            format: "PDF",
            progress: 5,
            description: "A metaphorical fugue on minds and machines. Demo PDF.".into(),
            added: "2025-11-18",
            path: "library/0005/book.pdf",
        },
        Book {
            id: 6,
            title: "The Hobbit".into(),
            authors: vec!["J. R. R. Tolkien".into()],
            series: None,
            tags: vec!["fantasy".into(), "classics".into()],
            format: "EPUB",
            progress: 100,
            description: "There and back again — finished book sample.".into(),
            added: "2025-08-02",
            path: "library/0006/book.epub",
        },
    ]
}

fn build_sample_shelves() -> Vec<Shelf> {
    vec![
        Shelf {
            id: 1,
            name: "Currently reading".into(),
            description: "progress > 0 AND progress < 100".into(),
            smart: true,
            book_ids: vec![1, 2, 3, 5],
            accent: "#7c9cff".into(),
        },
        Shelf {
            id: 2,
            name: "Unread sci‑fi".into(),
            description: "tag:scifi AND progress = 0".into(),
            smart: true,
            book_ids: vec![4],
            accent: "#7cffc3".into(),
        },
        Shelf {
            id: 3,
            name: "Favorites".into(),
            description: "Pinned by hand".into(),
            smart: false,
            book_ids: vec![1, 6],
            accent: "#ffc37c".into(),
        },
        Shelf {
            id: 4,
            name: "Classics".into(),
            description: "tag:classics".into(),
            smart: true,
            book_ids: vec![4, 6],
            accent: "#c37cff".into(),
        },
        Shelf {
            id: 5,
            name: "Short stack".into(),
            description: "Manual short-reads pile".into(),
            smart: false,
            book_ids: vec![3],
            accent: "#ff7c8a".into(),
        },
        Shelf {
            id: 6,
            name: "PDF inbox".into(),
            description: "format:PDF".into(),
            smart: true,
            book_ids: vec![5],
            accent: "#7ce0ff".into(),
        },
    ]
}

pub fn sample_books() -> &'static [Book] {
    static BOOKS: OnceLock<Vec<Book>> = OnceLock::new();
    BOOKS.get_or_init(build_sample_books)
}

pub fn sample_shelves() -> &'static [Shelf] {
    static SHELVES: OnceLock<Vec<Shelf>> = OnceLock::new();
    SHELVES.get_or_init(build_sample_shelves)
}

pub fn book_by_id(id: u64) -> Option<&'static Book> {
    sample_books().iter().find(|b| b.id == id)
}

pub fn shelf_by_id(id: u64) -> Option<&'static Shelf> {
    sample_shelves().iter().find(|s| s.id == id)
}

pub fn books_on_shelf(shelf_id: u64) -> Vec<&'static Book> {
    shelf_by_id(shelf_id)
        .map(|s| s.book_ids.iter().filter_map(|id| book_by_id(*id)).collect())
        .unwrap_or_default()
}
