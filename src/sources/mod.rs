pub mod mangadex;
pub mod scraper;
pub mod traits;

pub use traits::*;

use std::collections::HashMap;
use std::sync::Arc;

/// Global registry of all available sources.
pub struct SourceManager {
    sources: HashMap<&'static str, Arc<dyn Source>>,
}

impl SourceManager {
    /// Create a new SourceManager and register all built-in sources.
    pub fn new() -> Self {
        let mut manager = Self {
            sources: HashMap::new(),
        };
        
        // Register built-in sources
        manager.register(Arc::new(mangadex::MangaDexSource::new()));

        // Register generic scrapers via TOML
        if let Ok(mangaball) = scraper::GenericScraperSource::new(include_str!("scrapers/mangaball.toml")) {
            manager.register(Arc::new(mangaball));
        }

        manager
    }

    /// Register a source in the manager.
    pub fn register(&mut self, source: Arc<dyn Source>) {
        self.sources.insert(source.id(), source);
    }

    /// Get a source by its ID.
    pub fn get(&self, id: &str) -> Option<Arc<dyn Source>> {
        self.sources.get(id).cloned()
    }

    /// Get all registered sources.
    pub fn all(&self) -> Vec<Arc<dyn Source>> {
        self.sources.values().cloned().collect()
    }
}

impl Default for SourceManager {
    fn default() -> Self {
        Self::new()
    }
}
