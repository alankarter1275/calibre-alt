use super::traits::*;
use scraper::{Html, Selector};
use std::time::Duration;
use anyhow::{anyhow, Result};
use std::sync::Arc;

pub struct WeebCentralSource {
    client: ureq::Agent,
}

impl WeebCentralSource {
    pub fn new() -> Self {
        Self {
            client: ureq::AgentBuilder::new()
                .timeout_read(Duration::from_secs(15))
                .timeout_write(Duration::from_secs(15))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
                .build(),
        }
    }
}

impl Source for WeebCentralSource {
    fn id(&self) -> &'static str {
        "weebcentral"
    }

    fn name(&self) -> &'static str {
        "WeebCentral"
    }

    fn base_url(&self) -> &'static str {
        "https://weebcentral.com"
    }

    fn search(&self, query: &str, _page: u32, _filters: &[SearchFilter]) -> Result<SearchPage> {
        let query_formatted = query.replace(" ", "+");
        let url = format!("{}/search/data?text={}", self.base_url(), query_formatted);
        
        let resp = self.client.get(&url).call()?.into_string()?;
        let doc = Html::parse_document(&resp);
        
        let item_sel = Selector::parse("article.bg-base-300").unwrap();
        let link_sel = Selector::parse("a.link-hover[href*=\"/series/\"]").unwrap();
        let img_sel = Selector::parse("picture source").unwrap();

        let mut results = Vec::new();
        for item in doc.select(&item_sel) {
            let mut title = String::new();
            let mut remote_id = String::new();
            if let Some(a) = item.select(&link_sel).next() {
                let text = a.text().collect::<Vec<_>>().join(" ").trim().to_string();
                if !text.is_empty() {
                    title = text;
                    let href = a.value().attr("href").unwrap_or("");
                    // Href: https://weebcentral.com/series/01J76XY7E827QQQT0ERKCGH4CD/Naruto
                    let parts: Vec<&str> = href.split('/').collect();
                    if parts.len() >= 5 {
                        remote_id = parts[4].to_string();
                    }
                }
            }

            let mut cover_url = None;
            if let Some(img) = item.select(&img_sel).next() {
                if let Some(src) = img.value().attr("srcset") {
                    cover_url = Some(src.to_string());
                }
            }

            if !remote_id.is_empty() && !title.is_empty() {
                results.push(RemoteBookCard {
                    remote_id,
                    title,
                    author: "Unknown".to_string(),
                    cover_url,
                });
            }
        }

        Ok(SearchPage {
            results,
            has_more: false,
        })
    }

    fn get_details(&self, remote_id: &str) -> Result<RemoteBookDetails> {
        // Just return a basic dummy details since we can get everything from the search card if we want,
        // or actually fetch it. For now, fetch to get title.
        let url = format!("{}/series/{}", self.base_url(), remote_id);
        let resp = self.client.get(&url).call()?.into_string()?;
        let doc = Html::parse_document(&resp);

        let title_sel = Selector::parse("h1").unwrap();
        let title = doc.select(&title_sel).next().map(|n| n.text().collect::<Vec<_>>().join(" ").trim().to_string()).unwrap_or_default();
        
        // WeebCentral has a <picture> for the cover here too
        let img_sel = Selector::parse("picture source").unwrap();
        let cover_url = doc.select(&img_sel).next().and_then(|n| n.value().attr("srcset").map(|s| s.to_string()));

        let desc_sel = Selector::parse("p").unwrap(); // rough
        let description = doc.select(&desc_sel).map(|n| n.text().collect::<Vec<_>>().join(" ").trim().to_string()).collect::<Vec<_>>().join("\n");

        Ok(RemoteBookDetails {
            remote_id: remote_id.to_string(),
            title,
            author: "Unknown".to_string(),
            description,
            cover_url,
            tags: vec![],
            status: "Unknown".to_string(),
        })
    }

    fn get_chapters(&self, remote_id: &str) -> Result<Vec<RemoteChapter>> {
        let url = format!("{}/series/{}/full-chapter-list", self.base_url(), remote_id);
        let resp = self.client.get(&url).call()?.into_string()?;
        let doc = Html::parse_document(&resp);

        let link_sel = Selector::parse("a[href*=\"/chapters/\"]").unwrap();
        let name_sel = Selector::parse("span.grow.flex.items-center span").unwrap();
        
        let mut chapters = Vec::new();
        for item in doc.select(&link_sel) {
            let href = item.value().attr("href").unwrap_or("");
            // Href: https://weebcentral.com/chapters/01J8Z7WTD4218AGY0TGYYXZWZ2 or just /chapters/...
            let chapter_id = href.split('/').last().unwrap_or("").to_string();

            let title = if let Some(span) = item.select(&name_sel).next() {
                span.text().collect::<Vec<_>>().join(" ").trim().to_string()
            } else {
                item.text().collect::<Vec<_>>().join(" ").trim().to_string()
            };
            
            let number = title
                .split_whitespace()
                .find(|s| s.parse::<f32>().is_ok())
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.0);

            if !chapter_id.is_empty() {
                chapters.push(RemoteChapter {
                    chapter_id,
                    title,
                    number,
                    volume: None,
                    url: Some(href.to_string()),
                });
            }
        }
        
        // Ensure chronological
        chapters.reverse();

        Ok(chapters)
    }

    fn get_chapter_content(&self, chapter_id: &str) -> Result<ChapterContent> {
        let url = format!("{}/chapters/{}/images?is_prev=False", self.base_url(), chapter_id);
        let resp = self.client.get(&url).call()?.into_string()?;
        let doc = Html::parse_document(&resp);

        let img_sel = Selector::parse("img").unwrap();
        
        let mut images = Vec::new();
        for item in doc.select(&img_sel) {
            if let Some(src) = item.value().attr("src") {
                images.push(src.to_string());
            }
        }
        
        if images.is_empty() {
            return Err(anyhow!("No images found in chapter on WeebCentral."));
        }

        Ok(ChapterContent::Images(images))
    }

    fn fetch_image(&self, url: &str) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.client.get(url)
            .set("Referer", "https://weebcentral.com/")
            .call()?
            .into_reader()
            .read_to_end(&mut buf)?;
        Ok(buf)
    }
}
