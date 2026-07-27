//! Google Books — broad coverage, especially for recent and non-English titles
//! where Open Library thins out.
//!
//! Works without an API key, but keyless requests draw on a quota shared by
//! every anonymous client worldwide, so `429` happens unpredictably. A free
//! personal key (Google Cloud console → enable the Books API) lifts that to a
//! private allowance, and can be set in Settings. Descriptions and thumbnails
//! arrive inline, so no second request is needed.

use super::{agent, Candidate, CoverRef, FetchError, MetadataSource, SourceId};
use serde::Deserialize;

const SEARCH_URL: &str = "https://www.googleapis.com/books/v1/volumes";

pub struct GoogleBooks {
    /// Empty means "no key" — still works, just shares the global quota.
    pub api_key: String,
}

impl MetadataSource for GoogleBooks {
    fn id(&self) -> SourceId {
        SourceId::GoogleBooks
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<Candidate>, FetchError> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let mut req = agent()
            .get(SEARCH_URL)
            .query("q", query)
            .query("maxResults", &limit.clamp(1, 40).to_string())
            .query("printType", "books");
        if !self.api_key.trim().is_empty() {
            req = req.query("key", self.api_key.trim());
        }

        let body = match req.call() {
            Ok(resp) => resp
                .into_string()
                .map_err(|e| FetchError::Network(e.to_string()))?,
            // 429 is the common failure without a key; say so plainly rather
            // than surfacing a bare status code.
            Err(ureq::Error::Status(429, _)) => {
                return Err(FetchError::Limited(
                    "Google Books is rate limited right now. Add a free API key in Settings to \
                     get your own allowance."
                        .into(),
                ))
            }
            Err(ureq::Error::Status(400, _)) if !self.api_key.trim().is_empty() => {
                return Err(FetchError::Limited(
                    "Google Books rejected the API key in Settings.".into(),
                ))
            }
            Err(err) => return Err(FetchError::Network(err.to_string())),
        };

        let parsed: VolumesResponse =
            serde_json::from_str(&body).map_err(|e| FetchError::Parse(e.to_string()))?;

        Ok(parsed
            .items
            .unwrap_or_default()
            .into_iter()
            .map(volume_to_candidate)
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Wire format
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct VolumesResponse {
    #[serde(default)]
    items: Option<Vec<Volume>>,
}

#[derive(Debug, Deserialize)]
struct Volume {
    #[serde(rename = "volumeInfo", default)]
    volume_info: Option<VolumeInfo>,
}

#[derive(Debug, Deserialize)]
struct VolumeInfo {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    subtitle: Option<String>,
    #[serde(default)]
    authors: Option<Vec<String>>,
    #[serde(default)]
    publisher: Option<String>,
    /// `2012`, `2012-02` or `2012-02-15` — Google is inconsistent.
    #[serde(rename = "publishedDate", default)]
    published_date: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    categories: Option<Vec<String>>,
    #[serde(rename = "imageLinks", default)]
    image_links: Option<ImageLinks>,
}

#[derive(Debug, Deserialize)]
struct ImageLinks {
    #[serde(default)]
    thumbnail: Option<String>,
    #[serde(rename = "smallThumbnail", default)]
    small_thumbnail: Option<String>,
}

fn volume_to_candidate(volume: Volume) -> Candidate {
    let info = volume.volume_info.unwrap_or(VolumeInfo {
        title: None,
        subtitle: None,
        authors: None,
        publisher: None,
        published_date: None,
        description: None,
        categories: None,
        image_links: None,
    });

    // Google splits "Title: Subtitle" across two fields; rejoin so the value
    // matches what is printed on the book.
    let title = match (info.title, info.subtitle) {
        (Some(t), Some(s)) if !s.trim().is_empty() => format!("{t}: {s}"),
        (Some(t), _) => t,
        (None, _) => String::new(),
    };

    let published = info.published_date.unwrap_or_default();
    let first_year = published
        .get(..4)
        .and_then(|y| y.parse::<i64>().ok())
        .filter(|y| *y > 0);

    let cover = info.image_links.and_then(|links| {
        links
            .thumbnail
            .or(links.small_thumbnail)
            // Thumbnails are served over plain HTTP in some responses, and the
            // zoom parameter caps them small; ask for a larger image instead.
            .map(|url| {
                url.replace("http://", "https://")
                    .replace("&edge=curl", "")
                    .replace("zoom=1", "zoom=2")
            })
            .map(CoverRef::Url)
    });

    Candidate {
        title,
        authors: info.authors.unwrap_or_default().join(", "),
        // Google Books has no series concept in this payload.
        series: None,
        tags: info
            .categories
            .unwrap_or_default()
            .into_iter()
            // Categories arrive as "Fiction / Science Fiction / Space Opera";
            // the trailing segment is the useful one.
            .flat_map(|c| {
                c.split('/')
                    .map(|part| part.trim().to_string())
                    .collect::<Vec<_>>()
            })
            .filter(|c| !c.is_empty() && c.len() < 40)
            .take(8)
            .collect(),
        first_year,
        publisher: info.publisher.unwrap_or_default(),
        published,
        cover,
        // Descriptions come back inline, so no follow-up request.
        description: info.description.unwrap_or_default(),
        detail_key: None,
        source: Some(SourceId::GoogleBooks),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> Vec<Candidate> {
        let parsed: VolumesResponse = serde_json::from_str(json).unwrap();
        parsed
            .items
            .unwrap_or_default()
            .into_iter()
            .map(volume_to_candidate)
            .collect()
    }

    #[test]
    fn parses_a_volume() {
        let out = parse(
            r#"{"items":[{"volumeInfo":{
                "title":"Dune","authors":["Frank Herbert"],
                "publisher":"Ace","publishedDate":"1965-06-01",
                "description":"Desert planet.",
                "categories":["Fiction / Science Fiction / Space Opera"],
                "imageLinks":{"thumbnail":"http://books.google.com/x?zoom=1&edge=curl"}
            }}]}"#,
        );
        assert_eq!(out.len(), 1);
        let c = &out[0];
        assert_eq!(c.title, "Dune");
        assert_eq!(c.authors, "Frank Herbert");
        assert_eq!(c.publisher, "Ace");
        assert_eq!(c.first_year, Some(1965));
        assert_eq!(c.description, "Desert planet.");
        assert_eq!(c.source, Some(SourceId::GoogleBooks));
        // Categories are split on '/' and trimmed.
        assert!(c.tags.contains(&"Space Opera".to_string()));
    }

    #[test]
    fn cover_urls_are_upgraded() {
        let out = parse(
            r#"{"items":[{"volumeInfo":{"title":"A",
                "imageLinks":{"thumbnail":"http://x/y?zoom=1&edge=curl"}}}]}"#,
        );
        match &out[0].cover {
            Some(CoverRef::Url(url)) => {
                assert!(url.starts_with("https://"), "{url}");
                assert!(!url.contains("edge=curl"), "{url}");
                assert!(url.contains("zoom=2"), "{url}");
            }
            other => panic!("expected a URL cover, got {other:?}"),
        }
    }

    #[test]
    fn subtitle_is_rejoined() {
        let out = parse(r#"{"items":[{"volumeInfo":{"title":"Dune","subtitle":"Book One"}}]}"#);
        assert_eq!(out[0].title, "Dune: Book One");
    }

    #[test]
    fn missing_fields_are_tolerated() {
        let out = parse(r#"{"items":[{}]}"#);
        assert_eq!(out.len(), 1);
        assert!(out[0].title.is_empty());
        assert!(out[0].cover.is_none());
    }

    #[test]
    fn empty_payload_is_not_an_error() {
        assert!(parse(r#"{}"#).is_empty());
    }

    #[test]
    fn partial_dates_still_yield_a_year() {
        let out = parse(r#"{"items":[{"volumeInfo":{"title":"A","publishedDate":"2012"}}]}"#);
        assert_eq!(out[0].first_year, Some(2012));
    }
}
