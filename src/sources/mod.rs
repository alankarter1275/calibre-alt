pub mod mangadex;
pub mod manganato;
pub mod scraper;
pub mod traits;

pub use traits::*;

use std::sync::Arc;

/// Global registry of all available sources.
/// Uses a Vec to maintain registration order — the first source is the default.
pub struct SourceManager {
    sources: Vec<Arc<dyn Source>>,
}

impl SourceManager {
    /// Create a new SourceManager and register all built-in sources.
    pub fn new() -> Self {
        let mut manager = Self {
            sources: Vec::new(),
        };

        // Register Manganato first so it is the default source.
        // It provides raw images for official chapters.
        manager.register(Arc::new(manganato::ManganatoSource::new()));

        // Register MangaDex second.
        manager.register(Arc::new(mangadex::MangaDexSource::new()));

        manager
    }

    /// Register a source, appending it to the list.
    pub fn register(&mut self, source: Arc<dyn Source>) {
        self.sources.push(source);
    }

    /// Get a source by its ID.
    pub fn get(&self, id: &str) -> Option<Arc<dyn Source>> {
        self.sources.iter().find(|s| s.id() == id).cloned()
    }

    /// Get all registered sources, in registration order.
    pub fn all(&self) -> Vec<Arc<dyn Source>> {
        self.sources.clone()
    }
}

impl Default for SourceManager {
    fn default() -> Self {
        Self::new()
    }
}
