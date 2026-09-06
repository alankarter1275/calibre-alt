use crate::sources::traits::{
    ChapterContent, RemoteBookCard, RemoteBookDetails, RemoteChapter, SearchFilter, SearchPage, Source,
};
use anyhow::Result;
use scraper::{Html, Selector};

pub struct RoyalRoadSource;

impl RoyalRoadSource {
    pub fn new() -> Self {
        Self
    }
}

impl Source for RoyalRoadSource {
    fn id(&self) -> &'static str {
        "royalroad"
    }

    fn name(&self) -> &'static str {
        "RoyalRoad"
    }
    
    fn base_url(&self) -> &'static str {
        "https://www.royalroad.com"
    }

    fn search(&self, query: &str, page: u32, filters: &[SearchFilter]) -> Result<SearchPage> {
        let mut url = format!("https://www.royalroad.com/fictions/search?page={}", page);
        if !query.trim().is_empty() {
            url.push_str(&format!("&title={}", urlencoding::encode(query.trim())));
        }
        for f in filters {
            match f {
                SearchFilter::TagsInclude(tags) => {
                    for t in tags {
                        url.push_str(&format!("&tagsAdd={}", urlencoding::encode(&t.to_lowercase())));
                    }
                }
                SearchFilter::OrderBy(order) => {
                    url.push_str(&format!("&orderBy={}", urlencoding::encode(order)));
                }
                _ => {}
            }
        }
        let resp = ureq::get(&url).set("User-Agent", "Kalam/1.0").call()?.into_string()?;
        let doc = Html::parse_document(&resp);

        let item_sel = Selector::parse(".fiction-list-item").unwrap();
        let title_sel = Selector::parse(".fiction-title").unwrap();
        let link_sel = Selector::parse("a[href^=\"/fiction/\"]").unwrap();
        let img_sel = Selector::parse("img").unwrap();
        let author_sel = Selector::parse(".author").unwrap();

        let mut results = Vec::new();
        for item in doc.select(&item_sel) {
            let title = item.select(&title_sel).next().map(|e| e.text().collect::<Vec<_>>().join("").trim().to_string()).unwrap_or_default();
            let author = item.select(&author_sel).next().map(|e| e.text().collect::<Vec<_>>().join("").trim().to_string()).unwrap_or_default();
            
            let mut href = String::new();
            if let Some(a) = item.select(&link_sel).next() {
                href = a.value().attr("href").unwrap_or("").to_string();
            }
            if href.is_empty() { continue; }

            let mut cover_url = None;
            if let Some(img) = item.select(&img_sel).next() {
                if let Some(src) = img.value().attr("src") {
                    if src.starts_with('/') {
                        cover_url = Some(format!("https://www.royalroad.com{}", src));
                    } else {
                        cover_url = Some(src.to_string());
                    }
                }
            }

            results.push(RemoteBookCard {
                remote_id: href,
                title,
                author,
                cover_url,
            });
        }

        Ok(SearchPage {
            results,
            has_more: true,
        })
    }

    fn get_details(&self, remote_id: &str) -> Result<RemoteBookDetails> {
        let url = format!("https://www.royalroad.com{}", remote_id);
        let resp = ureq::get(&url).set("User-Agent", "Kalam/1.0").call()?.into_string()?;
        let doc = Html::parse_document(&resp);

        let title_sel = Selector::parse("h1.font-white").unwrap();
        let author_sel = Selector::parse("h4.font-white a").unwrap();
        let desc_sel = Selector::parse(".description").unwrap();
        let status_sel = Selector::parse(".fiction-info .label").unwrap();
        let cover_sel = Selector::parse(".fiction-info img").unwrap();

        let title = doc.select(&title_sel).next().map(|e| e.text().collect::<Vec<_>>().join("").trim().to_string()).unwrap_or_default();
        let author = doc.select(&author_sel).next().map(|e| e.text().collect::<Vec<_>>().join("").trim().to_string()).unwrap_or_default();
        let description = doc.select(&desc_sel).next().map(|e| e.text().collect::<Vec<_>>().join("\n").trim().to_string()).unwrap_or_default();
        let status = doc.select(&status_sel).next().map(|e| e.text().collect::<Vec<_>>().join("").trim().to_string()).unwrap_or_default();
        
        let mut cover_url = None;
        if let Some(img) = doc.select(&cover_sel).next() {
            if let Some(src) = img.value().attr("src") {
                if src.starts_with('/') {
                    cover_url = Some(format!("https://www.royalroad.com{}", src));
                } else {
                    cover_url = Some(src.to_string());
                }
            }
        }

        Ok(RemoteBookDetails {
            remote_id: remote_id.to_string(),
            title,
            author,
            description,
            cover_url,
            status,
            tags: vec![],
        })
    }
    
    fn get_chapters(&self, remote_id: &str) -> Result<Vec<RemoteChapter>> {
        let url = format!("https://www.royalroad.com{}", remote_id);
        let resp = ureq::get(&url).set("User-Agent", "Kalam/1.0").call()?.into_string()?;
        let doc = Html::parse_document(&resp);
        
        let row_sel = Selector::parse("#chapters tbody tr").unwrap();
        let link_sel = Selector::parse("a[href^=\"/fiction/\"]").unwrap();
        
        let mut chapters = Vec::new();
        for (i, row) in doc.select(&row_sel).enumerate() {
            if let Some(a) = row.select(&link_sel).next() {
                let chapter_id = a.value().attr("href").unwrap_or("").to_string();
                let chapter_title = a.text().collect::<Vec<_>>().join("").trim().to_string();
                if !chapter_id.is_empty() {
                    chapters.push(RemoteChapter {
                        chapter_id,
                        title: chapter_title,
                        number: i as f32 + 1.0,
                        volume: None,
                        url: None,
                    });
                }
            }
        }

        Ok(chapters)
    }

    fn get_chapter_content(&self, chapter_id: &str) -> Result<ChapterContent> {
        let url = format!("https://www.royalroad.com{}", chapter_id);
        let resp = ureq::get(&url).set("User-Agent", "Kalam/1.0").call()?.into_string()?;
        let doc = Html::parse_document(&resp);

        let content_sel = Selector::parse(".chapter-content").unwrap();
        if let Some(content) = doc.select(&content_sel).next() {
            return Ok(ChapterContent::Html(content.html()));
        }

        anyhow::bail!("Could not find chapter content on RoyalRoad")
    }

    fn fetch_image(&self, url: &str) -> Result<Vec<u8>> {
        let mut reader = ureq::get(url).set("User-Agent", "Kalam/1.0").call()?.into_reader();
        let mut bytes = Vec::new();
        std::io::copy(&mut reader, &mut bytes)?;
        Ok(bytes)
    }
}
