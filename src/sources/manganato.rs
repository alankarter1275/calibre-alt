#![allow(dead_code)]

use super::traits::*;
use scraper::{Html, Selector};
use std::time::Duration;
use anyhow::{anyhow, Result};

pub struct ManganatoSource {
    client: ureq::Agent,
}

impl ManganatoSource {
    pub fn new() -> Self {
        Self {
            client: ureq::AgentBuilder::new()
                .timeout_read(Duration::from_secs(15))
                .timeout_write(Duration::from_secs(15))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build(),
        }
    }
}

impl Source for ManganatoSource {
    fn id(&self) -> &'static str {
        "manganato"
    }

    fn name(&self) -> &'static str {
        "Manganato"
    }

    fn base_url(&self) -> &'static str {
        "https://manganato.com"
    }

    fn search(&self, query: &str, _page: u32, _filters: &std::collections::HashMap<String, String>) -> Result<SearchPage> {
        let query_formatted = query.replace(" ", "_");
        let url = format!("{}/search/story/{}", self.base_url(), query_formatted);
        
        let resp = self.client.get(&url).call()?.into_string()?;
        let doc = Html::parse_document(&resp);
        
        let item_sel = Selector::parse(".search-story-item").unwrap();
        let title_sel = Selector::parse("a.item-title").unwrap();
        let author_sel = Selector::parse("span.item-author").unwrap();
        let img_sel = Selector::parse("img").unwrap();

        let mut results = Vec::new();
        for item in doc.select(&item_sel) {
            let mut title = String::new();
            let mut remote_id = String::new();
            if let Some(t) = item.select(&title_sel).next() {
                title = t.text().collect::<Vec<_>>().join(" ").trim().to_string();
                let href = t.value().attr("href").unwrap_or("");
                // Href looks like https://chapmanganato.to/manga-xy123456
                remote_id = href.split('/').last().unwrap_or("").to_string();
            }

            let mut author = String::new();
            if let Some(a) = item.select(&author_sel).next() {
                author = a.text().collect::<Vec<_>>().join(" ").trim().to_string();
            }

            let mut cover_url = None;
            if let Some(img) = item.select(&img_sel).next() {
                if let Some(src) = img.value().attr("src") {
                    cover_url = Some(src.to_string());
                }
            }

            if !remote_id.is_empty() {
                results.push(RemoteBookCard {
                    remote_id,
                    title,
                    author,
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
        let url = format!("https://chapmanganato.to/{}", remote_id);
        let resp = self.client.get(&url).call()?.into_string()?;
        let doc = Html::parse_document(&resp);

        let title_sel = Selector::parse(".story-info-right h1").unwrap();
        let author_sel = Selector::parse("table.variations-tableInfo tbody tr:nth-child(2) td.table-value a").unwrap();
        let desc_sel = Selector::parse("#panel-story-info-description").unwrap();
        let img_sel = Selector::parse(".info-image img").unwrap();

        let title = doc.select(&title_sel).next().map(|n| n.text().collect::<Vec<_>>().join(" ").trim().to_string()).unwrap_or_default();
        let author = doc.select(&author_sel).next().map(|n| n.text().collect::<Vec<_>>().join(" ").trim().to_string()).unwrap_or_default();
        let mut description = doc.select(&desc_sel).next().map(|n| n.text().collect::<Vec<_>>().join(" ").trim().to_string()).unwrap_or_default();
        if description.starts_with("Description :") {
            description = description.replacen("Description :", "", 1).trim().to_string();
        }

        let cover_url = doc.select(&img_sel).next().and_then(|n| n.value().attr("src").map(|s| s.to_string()));

        Ok(RemoteBookDetails {
            remote_id: remote_id.to_string(),
            title,
            author,
            description,
            cover_url,
            tags: vec![],
            status: "Unknown".to_string(),
        })
    }

    fn get_chapters(&self, remote_id: &str) -> Result<Vec<RemoteChapter>> {
        let url = format!("https://chapmanganato.to/{}", remote_id);
        let resp = self.client.get(&url).call()?.into_string()?;
        let doc = Html::parse_document(&resp);

        let chap_sel = Selector::parse("ul.row-content-chapter li a.chapter-name").unwrap();
        
        let mut chapters = Vec::new();
        for item in doc.select(&chap_sel) {
            let title = item.text().collect::<Vec<_>>().join(" ").trim().to_string();
            let href = item.value().attr("href").unwrap_or("");
            
            // Extract chapter number from text (e.g. "Chapter 1")
            let number = title
                .split_whitespace()
                .find(|s| s.parse::<f32>().is_ok())
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.0);

            if !href.is_empty() {
                chapters.push(RemoteChapter {
                    chapter_id: href.to_string(), // use the full URL as chapter_id!
                    title,
                    number,
                    volume: None,
                    url: Some(href.to_string()),
                });
            }
        }
        
        // Manganato lists newest chapters first, so we reverse it to get chronological order
        chapters.reverse();

        Ok(chapters)
    }

    fn get_chapter_content(&self, chapter_id: &str) -> Result<ChapterContent> {
        // chapter_id is actually the full URL to the chapter
        let resp = self.client.get(chapter_id).call()?.into_string()?;
        let doc = Html::parse_document(&resp);

        let img_sel = Selector::parse(".container-chapter-reader img").unwrap();
        
        let mut images = Vec::new();
        for item in doc.select(&img_sel) {
            if let Some(src) = item.value().attr("src") {
                images.push(src.to_string());
            }
        }
        
        if images.is_empty() {
            return Err(anyhow!("No images found in chapter. Manganato might have changed layout."));
        }

        Ok(ChapterContent::Images(images))
    }

    fn fetch_image(&self, url: &str) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.client.get(url)
            .set("Referer", "https://chapmanganato.to/")
            .call()?
            .into_reader()
            .read_to_end(&mut buf)?;
        Ok(buf)
    }
}
