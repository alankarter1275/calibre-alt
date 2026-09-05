use super::traits::*;
use std::time::Duration;
use serde::Deserialize;

pub struct MangaDexSource {
    client: ureq::Agent,
}

impl MangaDexSource {
    pub fn new() -> Self {
        Self {
            client: ureq::AgentBuilder::new()
                .timeout_read(Duration::from_secs(15))
                .timeout_write(Duration::from_secs(15))
                .user_agent("Kalam/0.1.0 (calibre-alt)")
                .build(),
        }
    }
}

// Structs for parsing MangaDex JSON API
#[derive(Deserialize)]
struct MdListResponse<T> {
    data: Vec<T>,
}

#[derive(Deserialize)]
struct MdItemResponse<T> {
    data: T,
}

#[derive(Deserialize)]
struct MdManga {
    id: String,
    attributes: MdMangaAttributes,
    relationships: Vec<MdRelationship>,
}

#[derive(Deserialize)]
struct MdMangaAttributes {
    title: std::collections::HashMap<String, String>,
    description: std::collections::HashMap<String, String>,
    status: Option<String>,
}

#[derive(Deserialize)]
struct MdRelationship {
    id: String,
    #[serde(rename = "type")]
    rel_type: String,
    attributes: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct MdChapter {
    id: String,
    attributes: MdChapterAttributes,
}

#[derive(Deserialize)]
struct MdChapterAttributes {
    title: Option<String>,
    chapter: Option<String>,
    volume: Option<String>,
}

#[derive(Deserialize)]
struct MdAtHomeResponse {
    #[serde(rename = "baseUrl")]
    base_url: String,
    chapter: MdAtHomeChapter,
}

#[derive(Deserialize)]
struct MdAtHomeChapter {
    hash: String,
    data: Vec<String>,
}

impl Source for MangaDexSource {
    fn id(&self) -> &'static str {
        "mangadex"
    }

    fn name(&self) -> &'static str {
        "MangaDex"
    }

    fn base_url(&self) -> &'static str {
        "https://api.mangadex.org"
    }

    fn search(
        &self,
        query: &str,
        page: u32,
        _filters: &[SearchFilter],
    ) -> anyhow::Result<SearchPage> {
        let limit = 20;
        let offset = page.saturating_sub(1) * limit;

        // Note: For cover URLs we need `includes[]=cover_art` and `includes[]=author`.
        let url = format!("{}/manga", self.base_url());

        let req = self.client.get(&url)
            .query("title", query)
            .query("limit", &limit.to_string())
            .query("offset", &offset.to_string())
            .query("includes[]", "cover_art")
            .query("includes[]", "author");

        let resp = req.call()?;
        let resp: MdListResponse<MdManga> = serde_json::from_reader(resp.into_reader())?;

        let mut results = Vec::new();
        for manga in resp.data {
            let title = manga
                .attributes
                .title
                .get("en")
                .or_else(|| manga.attributes.title.values().next())
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());

            let mut author = "Unknown".to_string();
            let mut cover_url = None;

            for rel in &manga.relationships {
                if rel.rel_type == "author" {
                    if let Some(attrs) = &rel.attributes {
                        if let Some(name) = attrs.get("name").and_then(|n| n.as_str()) {
                            author = name.to_string();
                        }
                    }
                } else if rel.rel_type == "cover_art" {
                    if let Some(attrs) = &rel.attributes {
                        if let Some(file_name) = attrs.get("fileName").and_then(|f| f.as_str()) {
                            cover_url = Some(format!(
                                "https://uploads.mangadex.org/covers/{}/{}",
                                manga.id, file_name
                            ));
                        }
                    }
                }
            }

            results.push(RemoteBookCard {
                remote_id: manga.id,
                title,
                author,
                cover_url,
            });
        }

        Ok(SearchPage {
            has_more: results.len() == limit as usize,
            results,
        })
    }

    fn get_details(&self, remote_id: &str) -> anyhow::Result<RemoteBookDetails> {
        let url = format!(
            "{}/manga/{}?includes[]=cover_art&includes[]=author",
            self.base_url(),
            remote_id
        );

        let resp: MdItemResponse<MdManga> = serde_json::from_reader(self.client.get(&url).call()?.into_reader())?;
        let manga = resp.data;

        let title = manga
            .attributes
            .title
            .get("en")
            .or_else(|| manga.attributes.title.values().next())
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());

        let description = manga
            .attributes
            .description
            .get("en")
            .or_else(|| manga.attributes.description.values().next())
            .cloned()
            .unwrap_or_default();

        let mut author = "Unknown".to_string();
        let mut cover_url = None;

        for rel in &manga.relationships {
            if rel.rel_type == "author" {
                if let Some(attrs) = &rel.attributes {
                    if let Some(name) = attrs.get("name").and_then(|n| n.as_str()) {
                        author = name.to_string();
                    }
                }
            } else if rel.rel_type == "cover_art" {
                if let Some(attrs) = &rel.attributes {
                    if let Some(file_name) = attrs.get("fileName").and_then(|f| f.as_str()) {
                        cover_url = Some(format!(
                            "https://uploads.mangadex.org/covers/{}/{}",
                            manga.id, file_name
                        ));
                    }
                }
            }
        }

        Ok(RemoteBookDetails {
            remote_id: manga.id,
            title,
            author,
            description,
            cover_url,
            tags: vec![],
            status: manga.attributes.status.unwrap_or_else(|| "Ongoing".to_string()),
        })
    }

    fn get_chapters(&self, remote_id: &str) -> anyhow::Result<Vec<RemoteChapter>> {
        let mut all_chapters = Vec::new();
        let mut offset = 0;
        let limit = 100;

        loop {
            // Fetch English chapters ordered by ascending chapter number
            let url = format!(
                "{}/manga/{}/feed?limit={}&offset={}&translatedLanguage[]=en&order[chapter]=asc",
                self.base_url(),
                remote_id,
                limit,
                offset
            );

            let resp: MdListResponse<MdChapter> = serde_json::from_reader(self.client.get(&url).call()?.into_reader())?;
            let count = resp.data.len();

            for ch in resp.data {
                all_chapters.push(RemoteChapter {
                    chapter_id: ch.id,
                    title: ch.attributes.title.unwrap_or_default(),
                    number: ch.attributes.chapter.and_then(|s| s.parse().ok()).unwrap_or(0.0),
                    volume: ch.attributes.volume.and_then(|s| s.parse().ok()),
                    url: None,
                });
            }

            if count < limit {
                break;
            }
            offset += limit;
        }

        Ok(all_chapters)
    }

    fn get_chapter_content(&self, chapter_id: &str) -> anyhow::Result<ChapterContent> {
        // MangaDex requires asking the "at-home" server for the correct shard
        let url = format!("{}/at-home/server/{}", self.base_url(), chapter_id);
        let resp: MdAtHomeResponse = serde_json::from_reader(self.client.get(&url).call()?.into_reader())?;

        let mut image_urls = Vec::new();
        for file in resp.chapter.data {
            image_urls.push(format!("{}/data/{}/{}", resp.base_url, resp.chapter.hash, file));
        }

        Ok(ChapterContent::Images(image_urls))
    }

    fn fetch_image(&self, url: &str) -> anyhow::Result<Vec<u8>> {
        // MangaDex images don't strictly require headers, but it's good practice
        let mut buf = Vec::new();
        self.client.get(url).call()?.into_reader().read_to_end(&mut buf)?;
        Ok(buf)
    }
}
