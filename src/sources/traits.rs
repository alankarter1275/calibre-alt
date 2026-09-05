
/// What a chapter contains.
/// This enum is the crucial seam that unifies Manga (P9) and Fiction (P7).
#[derive(Debug, Clone)]
pub enum ChapterContent {
    /// Manga returns a list of image URLs to stream.
    Images(Vec<String>),
    /// Fiction returns an HTML string.
    Html(String),
}

/// Metadata for a remote book discovered via search or browse.
#[derive(Debug, Clone, Default)]
pub struct RemoteBookCard {
    pub remote_id: String,
    pub title: String,
    pub author: String,
    pub cover_url: Option<String>,
}

/// Detailed metadata for a remote book.
#[derive(Debug, Clone, Default)]
pub struct RemoteBookDetails {
    pub remote_id: String,
    pub title: String,
    pub author: String,
    pub description: String,
    pub cover_url: Option<String>,
    pub tags: Vec<String>,
    pub status: String,
}

/// Metadata for a remote chapter.
#[derive(Debug, Clone, Default)]
pub struct RemoteChapter {
    pub chapter_id: String,
    pub title: String,
    pub number: f32, // Float to support 12.5 etc.
    pub volume: Option<f32>,
    pub url: Option<String>,
}

/// A filter for searching.
#[derive(Debug, Clone)]
pub enum SearchFilter {
    TagsInclude(Vec<String>),
    TagsExclude(Vec<String>),
    OngoingOnly(bool),
    OrderBy(String),
}

/// A paginated page of search results.
#[derive(Debug, Clone)]
pub struct SearchPage {
    pub results: Vec<RemoteBookCard>,
    pub has_more: bool,
}

/// The unified Source trait.
/// All methods are synchronous; they should be called from `crate::tasks::spawn` workers.
pub trait Source: Send + Sync {
    /// Internal unique ID of this source (e.g., "mangadex").
    fn id(&self) -> &'static str;
    
    /// Human-readable name of this source.
    fn name(&self) -> &'static str;
    
    /// Base URL for this source, if applicable.
    fn base_url(&self) -> &'static str;

    /// Search for books.
    fn search(
        &self,
        query: &str,
        page: u32,
        filters: &[SearchFilter],
    ) -> anyhow::Result<SearchPage>;

    /// Fetch detailed metadata for a book.
    fn get_details(&self, remote_id: &str) -> anyhow::Result<RemoteBookDetails>;

    /// Fetch the list of chapters for a book.
    fn get_chapters(&self, remote_id: &str) -> anyhow::Result<Vec<RemoteChapter>>;

    /// Fetch the content of a specific chapter.
    fn get_chapter_content(&self, chapter_id: &str) -> anyhow::Result<ChapterContent>;

    /// Fetch raw bytes for a specific URL, optionally injecting referer/auth headers.
    /// This is used by the ComicsReader's ImageProvider to stream images.
    fn fetch_image(&self, url: &str) -> anyhow::Result<Vec<u8>>;
}
