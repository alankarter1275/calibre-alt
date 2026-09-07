use crate::sources::traits::{
    ChapterContent, FilterDefinition, FilterType, RemoteBookCard, RemoteBookDetails, RemoteChapter,
    SearchPage, Source,
};
use anyhow::Result;
use scraper::{Html, Selector};
use std::collections::HashMap;

pub struct ArchiveOfOurOwnSource;

impl ArchiveOfOurOwnSource {
    pub fn new() -> Self {
        Self
    }
}

impl Source for ArchiveOfOurOwnSource {
    fn id(&self) -> &'static str {
        "ao3"
    }

    fn name(&self) -> &'static str {
        "Archive of Our Own"
    }

    fn base_url(&self) -> &'static str {
        "https://archiveofourown.org"
    }

    fn get_filter_definitions(&self) -> Vec<FilterDefinition> {
        vec![
            FilterDefinition {
                id: "fandom".to_string(),
                name: "Fandom".to_string(),
                filter_type: FilterType::Text {
                    placeholder: "e.g. Harry Potter, Marvel".to_string(),
                },
                default_value: "".to_string(),
            },
            FilterDefinition {
                id: "rating".to_string(),
                name: "Rating".to_string(),
                filter_type: FilterType::Select {
                    options: vec![
                        ("Any Rating".to_string(), "".to_string()),
                        ("General Audiences".to_string(), "10".to_string()),
                        ("Teen And Up Audiences".to_string(), "11".to_string()),
                        ("Mature".to_string(), "12".to_string()),
                        ("Explicit".to_string(), "13".to_string()),
                        ("Not Rated".to_string(), "9".to_string()),
                    ],
                },
                default_value: "".to_string(),
            },
            FilterDefinition {
                id: "complete".to_string(),
                name: "Completion Status".to_string(),
                filter_type: FilterType::Select {
                    options: vec![
                        ("All Works".to_string(), "".to_string()),
                        ("Complete Only".to_string(), "T".to_string()),
                        ("Incomplete Only".to_string(), "F".to_string()),
                    ],
                },
                default_value: "".to_string(),
            },
            FilterDefinition {
                id: "word_count".to_string(),
                name: "Word Count".to_string(),
                filter_type: FilterType::Text {
                    placeholder: "e.g. >5000 or 1000-5000".to_string(),
                },
                default_value: "".to_string(),
            },
            FilterDefinition {
                id: "sort".to_string(),
                name: "Sort By".to_string(),
                filter_type: FilterType::Sort {
                    options: vec![
                        ("Best Match".to_string(), "_score".to_string()),
                        ("Kudos".to_string(), "kudos_count".to_string()),
                        ("Hits".to_string(), "hits".to_string()),
                        ("Bookmarks".to_string(), "bookmarks_count".to_string()),
                        ("Date Updated".to_string(), "revised_at".to_string()),
                        ("Word Count".to_string(), "word_count".to_string()),
                    ],
                },
                default_value: "_score".to_string(),
            },
        ]
    }

    fn search(
        &self,
        query: &str,
        page: u32,
        filters: &HashMap<String, String>,
    ) -> Result<SearchPage> {
        let mut url = format!("https://archiveofourown.org/works/search?page={}", page);
        if !query.trim().is_empty() {
            url.push_str(&format!("&work_search[query]={}", urlencoding::encode(query.trim())));
        }

        if let Some(fandom) = filters.get("fandom") {
            if !fandom.trim().is_empty() {
                url.push_str(&format!("&work_search[fandom_names]={}", urlencoding::encode(fandom.trim())));
            }
        }
        if let Some(rating) = filters.get("rating") {
            if !rating.is_empty() {
                url.push_str(&format!("&work_search[rating_ids]={}", urlencoding::encode(rating)));
            }
        }
        if let Some(complete) = filters.get("complete") {
            if !complete.is_empty() {
                url.push_str(&format!("&work_search[complete]={}", urlencoding::encode(complete)));
            }
        }
        if let Some(wc) = filters.get("word_count") {
            if !wc.trim().is_empty() {
                url.push_str(&format!("&work_search[word_count]={}", urlencoding::encode(wc.trim())));
            }
        }
        if let Some(sort) = filters.get("sort") {
            if !sort.is_empty() {
                url.push_str(&format!("&work_search[sort_column]={}", urlencoding::encode(sort)));
            }
        }

        let resp = ureq::get(&url)
            .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) Kalam/1.0")
            .call()?
            .into_string()?;

        let doc = Html::parse_document(&resp);
        let work_sel = Selector::parse("li.work").unwrap();
        let title_sel = Selector::parse("h4.heading a[href^=\"/works/\"]").unwrap();
        let author_sel = Selector::parse("h4.heading a[rel=\"author\"]").unwrap();

        let mut results = Vec::new();
        for item in doc.select(&work_sel) {
            if let Some(title_el) = item.select(&title_sel).next() {
                let title = title_el.text().collect::<Vec<_>>().join("").trim().to_string();
                let href = title_el.value().attr("href").unwrap_or("");
                let remote_id = href.trim_start_matches("/works/").to_string();

                let author = item
                    .select(&author_sel)
                    .next()
                    .map(|e| e.text().collect::<Vec<_>>().join("").trim().to_string())
                    .unwrap_or_else(|| "Anonymous".to_string());

                if !remote_id.is_empty() {
                    results.push(RemoteBookCard {
                        remote_id,
                        title,
                        author,
                        cover_url: None, // AO3 fics don't have covers
                    });
                }
            }
        }

        let next_sel = Selector::parse("ol.pagination li.next a").unwrap();
        let has_more = doc.select(&next_sel).next().is_some();

        Ok(SearchPage { results, has_more })
    }

    fn get_details(&self, remote_id: &str) -> Result<RemoteBookDetails> {
        let url = format!("https://archiveofourown.org/works/{}?view_adult=true", remote_id);
        let resp = ureq::get(&url)
            .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) Kalam/1.0")
            .call()?
            .into_string()?;

        let doc = Html::parse_document(&resp);
        let title_sel = Selector::parse("h2.title").unwrap();
        let author_sel = Selector::parse("h3.byline a[rel=\"author\"]").unwrap();
        let summary_sel = Selector::parse("blockquote.userstuff").unwrap();

        let title = doc
            .select(&title_sel)
            .next()
            .map(|e| e.text().collect::<Vec<_>>().join("").trim().to_string())
            .unwrap_or_else(|| "Untitled".to_string());

        let author = doc
            .select(&author_sel)
            .next()
            .map(|e| e.text().collect::<Vec<_>>().join("").trim().to_string())
            .unwrap_or_else(|| "Anonymous".to_string());

        let description = doc
            .select(&summary_sel)
            .next()
            .map(|e| e.html())
            .unwrap_or_default();

        Ok(RemoteBookDetails {
            remote_id: remote_id.to_string(),
            title,
            author,
            description,
            cover_url: None,
            tags: Vec::new(),
            status: "Completed".to_string(),
        })
    }

    fn get_chapters(&self, remote_id: &str) -> Result<Vec<RemoteChapter>> {
        let nav_url = format!("https://archiveofourown.org/works/{}/navigate?view_adult=true", remote_id);
        let resp = ureq::get(&nav_url)
            .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) Kalam/1.0")
            .call()?;

        // If navigate page exists, parse chapter list
        if resp.status() == 200 {
            let body = resp.into_string()?;
            let doc = Html::parse_document(&body);
            let item_sel = Selector::parse("ol.chapter.index li").unwrap();
            let link_sel = Selector::parse("a[href^=\"/works/\"]").unwrap();

            let mut chapters = Vec::new();
            for (idx, item) in doc.select(&item_sel).enumerate() {
                if let Some(link) = item.select(&link_sel).next() {
                    let title = link.text().collect::<Vec<_>>().join("").trim().to_string();
                    let href = link.value().attr("href").unwrap_or("");
                    // href is /works/12345/chapters/67890
                    let parts: Vec<&str> = href.split("/chapters/").collect();
                    let ch_id = if parts.len() > 1 { parts[1] } else { href };

                    chapters.push(RemoteChapter {
                        chapter_id: ch_id.to_string(),
                        title,
                        number: (idx + 1) as f32,
                        volume: None,
                        url: Some(format!("https://archiveofourown.org{}", href)),
                    });
                }
            }

            if !chapters.is_empty() {
                return Ok(chapters);
            }
        }

        // Single chapter work fallback
        Ok(vec![RemoteChapter {
            chapter_id: remote_id.to_string(),
            title: "Full Work".to_string(),
            number: 1.0,
            volume: None,
            url: Some(format!("https://archiveofourown.org/works/{}", remote_id)),
        }])
    }

    fn get_chapter_content(&self, chapter_id: &str) -> Result<ChapterContent> {
        let url = if chapter_id.contains('/') {
            format!("https://archiveofourown.org/{}?view_adult=true", chapter_id)
        } else {
            format!("https://archiveofourown.org/chapters/{}?view_adult=true", chapter_id)
        };

        let resp = ureq::get(&url)
            .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) Kalam/1.0")
            .call()?
            .into_string()?;

        let doc = Html::parse_document(&resp);
        let userstuff_sel = Selector::parse("div.userstuff").unwrap();

        if let Some(content) = doc.select(&userstuff_sel).next() {
            Ok(ChapterContent::Html(content.html()))
        } else {
            Err(anyhow::anyhow!("Chapter content not found"))
        }
    }

    fn fetch_image(&self, _url: &str) -> Result<Vec<u8>> {
        Err(anyhow::anyhow!("AO3 does not support image fetching"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ao3_filter_definitions() {
        let src = ArchiveOfOurOwnSource::new();
        let defs = src.get_filter_definitions();
        assert!(!defs.is_empty());
        assert!(defs.iter().any(|d| d.id == "fandom"));
        assert!(defs.iter().any(|d| d.id == "rating"));
        assert!(defs.iter().any(|d| d.id == "sort"));
    }
}
