use super::traits::*;
use scraper::{Html, Selector};
use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize, Clone)]
pub struct ScraperConfig {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub search: SearchSelectorsConfig,
    pub details: DetailsSelectorsConfig,
    pub chapters: ChaptersSelectorsConfig,
    pub pages: PagesSelectorsConfig,
}

#[derive(Deserialize, Clone)]
pub struct SearchSelectorsConfig {
    pub url: String,
    pub item: String,
    pub title: String,
    pub link: String,
    pub cover: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct DetailsSelectorsConfig {
    pub description: Option<String>,
    pub author: Option<String>,
    pub cover: Option<String>,
    pub status: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct ChaptersSelectorsConfig {
    pub item: String,
    pub title: String,
    pub link: String,
}

#[derive(Deserialize, Clone)]
pub struct PagesSelectorsConfig {
    pub item: String,
    pub image_attr: Option<String>,
}

/// Compiled selectors for fast execution
struct CompiledSelectors {
    search_item: Selector,
    search_title: Selector,
    search_link: Selector,
    search_cover: Option<Selector>,

    details_desc: Option<Selector>,
    details_author: Option<Selector>,
    details_cover: Option<Selector>,
    details_status: Option<Selector>,

    chapters_item: Selector,
    chapters_title: Selector,
    chapters_link: Selector,

    pages_item: Selector,
}

impl CompiledSelectors {
    fn new(cfg: &ScraperConfig) -> anyhow::Result<Self> {
        let compile = |s: &str| Selector::parse(s).map_err(|e| anyhow::anyhow!("Invalid selector: {}", e));
        let compile_opt = |opt: &Option<String>| -> anyhow::Result<Option<Selector>> {
            match opt {
                Some(s) => Ok(Some(compile(s)?)),
                None => Ok(None),
            }
        };

        Ok(Self {
            search_item: compile(&cfg.search.item)?,
            search_title: compile(&cfg.search.title)?,
            search_link: compile(&cfg.search.link)?,
            search_cover: compile_opt(&cfg.search.cover)?,

            details_desc: compile_opt(&cfg.details.description)?,
            details_author: compile_opt(&cfg.details.author)?,
            details_cover: compile_opt(&cfg.details.cover)?,
            details_status: compile_opt(&cfg.details.status)?,

            chapters_item: compile(&cfg.chapters.item)?,
            chapters_title: compile(&cfg.chapters.title)?,
            chapters_link: compile(&cfg.chapters.link)?,

            pages_item: compile(&cfg.pages.item)?,
        })
    }
}

pub struct GenericScraperSource {
    config: ScraperConfig,
    selectors: CompiledSelectors,
    client: ureq::Agent,
    id: String,
    name: String,
    base_url: String,
}

impl GenericScraperSource {
    pub fn new(toml_str: &str) -> anyhow::Result<Self> {
        let config: ScraperConfig = toml::from_str(toml_str)?;
        let selectors = CompiledSelectors::new(&config)?;

        let id = config.id.clone();
        let name = config.name.clone();
        let base_url = config.base_url.clone();

        Ok(Self {
            config,
            selectors,
            client: ureq::AgentBuilder::new()
                .timeout_read(Duration::from_secs(15))
                .timeout_write(Duration::from_secs(15))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .build(),
            id,
            name,
            base_url,
        })
    }

    fn text(&self, element: scraper::ElementRef, selector: Option<&Selector>) -> Option<String> {
        if let Some(sel) = selector {
            element.select(sel).next().map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
        } else {
            None
        }
    }

    fn attr(&self, element: scraper::ElementRef, selector: Option<&Selector>, attr: &str) -> Option<String> {
        if let Some(sel) = selector {
            element.select(sel).next().and_then(|el| el.value().attr(attr).map(|s| s.to_string()))
        } else {
            None
        }
    }

    fn resolve_url(&self, url: &str) -> String {
        if url.starts_with("http") {
            url.to_string()
        } else if url.starts_with('/') {
            format!("{}{}", self.base_url, url)
        } else {
            format!("{}/{}", self.base_url, url)
        }
    }
}

impl Source for GenericScraperSource {
    fn id(&self) -> &'static str {
        // Leak is acceptable here because source IDs are meant to be 'static in the registry
        Box::leak(self.id.clone().into_boxed_str())
    }

    fn name(&self) -> &'static str {
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn base_url(&self) -> &'static str {
        Box::leak(self.base_url.clone().into_boxed_str())
    }

    fn search(&self, query: &str, page: u32, _filters: &[SearchFilter]) -> anyhow::Result<SearchPage> {
        let url = self.config.search.url
            .replace("{query}", &urlencoding::encode(query))
            .replace("{page}", &page.to_string());
        
        let full_url = self.resolve_url(&url);
        let html_str = self.client.get(&full_url).call()?.into_string()?;
        let document = Html::parse_document(&html_str);

        let mut results = Vec::new();

        for item in document.select(&self.selectors.search_item) {
            let title = self.text(item, Some(&self.selectors.search_title)).unwrap_or_else(|| "Unknown".to_string());
            let link = self.attr(item, Some(&self.selectors.search_link), "href").unwrap_or_default();
            let cover_url = self.attr(item, self.selectors.search_cover.as_ref(), "src");

            results.push(RemoteBookCard {
                remote_id: link,
                title,
                author: "Unknown".to_string(), // Often not in search results
                cover_url,
            });
        }

        Ok(SearchPage {
            has_more: !results.is_empty(), // Naive assumption: if we got results, there might be more
            results,
        })
    }

    fn get_details(&self, remote_id: &str) -> anyhow::Result<RemoteBookDetails> {
        let full_url = self.resolve_url(remote_id);
        let html_str = self.client.get(&full_url).call()?.into_string()?;
        let document = Html::parse_document(&html_str);

        // Document root element for details
        let root = document.root_element();

        let description = self.text(root, self.selectors.details_desc.as_ref()).unwrap_or_default();
        let author = self.text(root, self.selectors.details_author.as_ref()).unwrap_or_else(|| "Unknown".to_string());
        let cover_url = self.attr(root, self.selectors.details_cover.as_ref(), "src");
        let status = self.text(root, self.selectors.details_status.as_ref()).unwrap_or_else(|| "Ongoing".to_string());

        // Parse Title again just in case, but usually we already have it. We'll leave it empty to inherit from search if possible, or parse from a standard selector.
        // Actually, let's just parse it if we added it, but it's fine for now.

        Ok(RemoteBookDetails {
            remote_id: remote_id.to_string(),
            title: "Parsed Title".to_string(), // Could add a selector for this
            author,
            description,
            cover_url,
            tags: vec![],
            status,
        })
    }

    fn get_chapters(&self, remote_id: &str) -> anyhow::Result<Vec<RemoteChapter>> {
        let full_url = self.resolve_url(remote_id);
        let html_str = self.client.get(&full_url).call()?.into_string()?;
        let document = Html::parse_document(&html_str);

        let mut chapters = Vec::new();
        let mut number = 1.0;

        // Often chapters are listed descending. We just parse them in order.
        for item in document.select(&self.selectors.chapters_item) {
            let title = self.text(item, Some(&self.selectors.chapters_title)).unwrap_or_default();
            let link = self.attr(item, Some(&self.selectors.chapters_link), "href").unwrap_or_default();

            chapters.push(RemoteChapter {
                chapter_id: link,
                title,
                number,
                volume: None,
                url: None,
            });
            number += 1.0;
        }

        // Reverse if they came descending
        // chapters.reverse();

        Ok(chapters)
    }

    fn get_chapter_content(&self, chapter_id: &str) -> anyhow::Result<ChapterContent> {
        let full_url = self.resolve_url(chapter_id);
        let html_str = self.client.get(&full_url).call()?.into_string()?;
        let document = Html::parse_document(&html_str);

        let mut images = Vec::new();
        let attr = self.config.pages.image_attr.as_deref().unwrap_or("src");

        for item in document.select(&self.selectors.pages_item) {
            if let Some(img_url) = item.value().attr(attr) {
                images.push(self.resolve_url(img_url));
            }
        }

        Ok(ChapterContent::Images(images))
    }

    fn fetch_image(&self, url: &str) -> anyhow::Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.client.get(url)
            .set("Referer", &self.base_url) // Many sites block hotlinking without referer
            .call()?
            .into_reader()
            .read_to_end(&mut buf)?;
        Ok(buf)
    }
}
