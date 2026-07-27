//! P5 — Open Library metadata lookup.
//!
//! Deliberately small and synchronous: callers run it off the UI thread via
//! Relm4's `oneshot_command`. No API key is needed and Open Library asks only
//! that clients identify themselves, which we do with a User-Agent.
//!
//! Nothing here touches the catalog. A lookup returns candidates; applying one
//! is a separate, explicit step so a fetch can never silently overwrite the
//! metadata you already have.

use serde::Deserialize;
use std::io::Read;
use std::time::Duration;

const SEARCH_URL: &str = "https://openlibrary.org/search.json";
const COVER_URL: &str = "https://covers.openlibrary.org/b/id";
const USER_AGENT: &str = concat!(
    "Kalam/",
    env!("CARGO_PKG_VERSION"),
    " (personal ebook manager; +https://github.com/alankarter1275/calibre-alt)"
);

/// Network calls are best-effort; the UI shows the message and moves on.
#[derive(Debug)]
pub enum FetchError {
    Network(String),
    Parse(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Network(m) => write!(f, "Network error: {m}"),
            FetchError::Parse(m) => write!(f, "Could not read the response: {m}"),
        }
    }
}

/// One candidate result, flattened into just what the edit dialog needs.
#[derive(Debug, Clone, Default)]
pub struct Candidate {
    pub title: String,
    pub authors: String,
    pub series: Option<String>,
    pub description: String,
    pub tags: Vec<String>,
    pub first_year: Option<i64>,
    pub cover_id: Option<i64>,
    /// `/works/OL…W`, used to fetch the description lazily.
    pub work_key: Option<String>,
}

impl Candidate {
    /// One-line summary for the results list.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if !self.authors.is_empty() {
            parts.push(self.authors.clone());
        }
        if let Some(year) = self.first_year {
            parts.push(year.to_string());
        }
        if self.cover_id.is_some() {
            parts.push("has cover".into());
        }
        parts.join(" · ")
    }

    pub fn cover_url(&self, size: char) -> Option<String> {
        self.cover_id
            .map(|id| format!("{COVER_URL}/{id}-{size}.jpg"))
    }
}

// ---------------------------------------------------------------------------
// Wire format
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    docs: Vec<SearchDoc>,
}

#[derive(Debug, Deserialize)]
struct SearchDoc {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    author_name: Option<Vec<String>>,
    #[serde(default)]
    first_publish_year: Option<i64>,
    #[serde(default)]
    cover_i: Option<i64>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    subject: Option<Vec<String>>,
    #[serde(default)]
    series: Option<Vec<String>>,
}

/// `description` is maddeningly polymorphic: sometimes a string, sometimes
/// `{ "value": "…" }`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Description {
    Text(String),
    Object { value: String },
}

#[derive(Debug, Deserialize)]
struct WorkResponse {
    #[serde(default)]
    description: Option<Description>,
}

// ---------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(20))
        .user_agent(USER_AGENT)
        .build()
}

/// Search by free text, or by title/author when both are known.
pub fn search(query: &str, limit: usize) -> Result<Vec<Candidate>, FetchError> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let body = agent()
        .get(SEARCH_URL)
        .query("q", query)
        .query("limit", &limit.clamp(1, 20).to_string())
        // Ask only for the fields we use — the default payload is enormous.
        .query(
            "fields",
            "title,author_name,first_publish_year,cover_i,key,subject,series",
        )
        .call()
        .map_err(|e| FetchError::Network(e.to_string()))?
        .into_string()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let parsed: SearchResponse =
        serde_json::from_str(&body).map_err(|e| FetchError::Parse(e.to_string()))?;

    Ok(parsed.docs.into_iter().map(doc_to_candidate).collect())
}

fn doc_to_candidate(doc: SearchDoc) -> Candidate {
    Candidate {
        title: doc.title.unwrap_or_default(),
        authors: doc.author_name.unwrap_or_default().join(", "),
        series: doc.series.and_then(|s| s.into_iter().next()),
        description: String::new(),
        // Open Library subjects are long-tailed; keep a usable handful.
        tags: doc
            .subject
            .unwrap_or_default()
            .into_iter()
            .filter(|s| s.len() < 40)
            .take(8)
            .collect(),
        first_year: doc.first_publish_year,
        cover_id: doc.cover_i,
        work_key: doc.key,
    }
}

/// Fetch a work's description. Absent descriptions are common and not an error.
pub fn fetch_description(work_key: &str) -> Result<String, FetchError> {
    let key = work_key.trim_start_matches('/');
    let url = format!("https://openlibrary.org/{key}.json");

    let body = agent()
        .get(&url)
        .call()
        .map_err(|e| FetchError::Network(e.to_string()))?
        .into_string()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let parsed: WorkResponse =
        serde_json::from_str(&body).map_err(|e| FetchError::Parse(e.to_string()))?;

    Ok(match parsed.description {
        Some(Description::Text(t)) => t,
        Some(Description::Object { value }) => value,
        None => String::new(),
    })
}

/// Download cover bytes. `size` is 'S', 'M' or 'L'.
pub fn fetch_cover(cover_id: i64, size: char) -> Result<Vec<u8>, FetchError> {
    let url = format!("{COVER_URL}/{cover_id}-{size}.jpg");
    let resp = agent()
        .get(&url)
        .call()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let mut bytes = Vec::new();
    resp.into_reader()
        .take(8 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .map_err(|e| FetchError::Network(e.to_string()))?;

    if bytes.len() < 512 {
        // Open Library serves a tiny 1x1 placeholder when a cover is missing.
        return Err(FetchError::Network("no cover available".into()));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_short_circuits() {
        assert!(search("   ", 5).unwrap().is_empty());
    }

    #[test]
    fn parses_search_payload() {
        let json = r#"{"docs":[
            {"title":"Dune","author_name":["Frank Herbert"],
             "first_publish_year":1965,"cover_i":123,"key":"/works/OL1W",
             "subject":["Science fiction","Desert"]}
        ]}"#;
        let parsed: SearchResponse = serde_json::from_str(json).unwrap();
        let c = doc_to_candidate(parsed.docs.into_iter().next().unwrap());
        assert_eq!(c.title, "Dune");
        assert_eq!(c.authors, "Frank Herbert");
        assert_eq!(c.first_year, Some(1965));
        assert_eq!(c.cover_id, Some(123));
        assert_eq!(c.tags.len(), 2);
    }

    #[test]
    fn tolerates_missing_fields() {
        let parsed: SearchResponse = serde_json::from_str(r#"{"docs":[{}]}"#).unwrap();
        let c = doc_to_candidate(parsed.docs.into_iter().next().unwrap());
        assert!(c.title.is_empty());
        assert!(c.authors.is_empty());
        assert!(c.cover_id.is_none());
    }

    #[test]
    fn description_accepts_both_shapes() {
        let a: WorkResponse = serde_json::from_str(r#"{"description":"plain"}"#).unwrap();
        assert!(matches!(a.description, Some(Description::Text(_))));

        let b: WorkResponse =
            serde_json::from_str(r#"{"description":{"value":"nested"}}"#).unwrap();
        assert!(matches!(b.description, Some(Description::Object { .. })));

        let c: WorkResponse = serde_json::from_str("{}").unwrap();
        assert!(c.description.is_none());
    }

    #[test]
    fn long_subjects_are_dropped() {
        let long = "x".repeat(60);
        let json = format!(r#"{{"docs":[{{"subject":["short","{long}"]}}]}}"#);
        let parsed: SearchResponse = serde_json::from_str(&json).unwrap();
        let c = doc_to_candidate(parsed.docs.into_iter().next().unwrap());
        assert_eq!(c.tags, vec!["short"]);
    }
}
