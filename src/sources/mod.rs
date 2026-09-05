pub mod mangadex;
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

        // Register MangaDex first so it is always the default source.
        manager.register(Arc::new(mangadex::MangaDexSource::new()));

        // Generic scrapers via TOML. Only added if the TOML parses correctly.
        // MangaBall is kept as a config example but disabled since its domain is dead.
        // Uncomment and update the base_url if you find a working mirror:
        //
        // if let Ok(src) = scraper::GenericScraperSource::new(include_str!("scrapers/mangaball.toml")) {
        //     manager.register(Arc::new(src));
        // }

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
