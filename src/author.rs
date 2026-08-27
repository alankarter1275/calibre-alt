use crate::db::{Annotation, AuthorProfile, AuthorWork, Catalog, SortKey};
use crate::metadata::{self, CoverRef};
use crate::models::Book;
use crate::paths::authors_dir;
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::fs;

#[derive(Debug, Clone)]
pub struct AuthorQuote {
    pub book_title: String,
    pub excerpt: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct SeriesProgress {
    pub name: String,
    pub owned: Vec<Book>,
    pub missing: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AuthorSearchResponse {
    #[serde(default)]
    docs: Vec<AuthorSearchDoc>,
}

#[derive(Debug, Deserialize)]
struct AuthorSearchDoc {
    #[serde(default)]
    key: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    alternate_names: Vec<String>,
    #[serde(default)]
    top_subjects: Vec<String>,
    #[serde(default)]
    top_work: String,
    #[serde(default)]
    birth_date: String,
    #[serde(default)]
    death_date: String,
    #[serde(default)]
    work_count: i64,
}

#[derive(Debug, Deserialize)]
struct AuthorResponse {
    #[serde(default)]
    name: String,
    #[serde(default)]
    personal_name: String,
    #[serde(default)]
    birth_date: String,
    #[serde(default)]
    death_date: String,
    #[serde(default)]
    alternate_names: Vec<String>,
    #[serde(default)]
    bio: Option<OpenLibraryText>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum OpenLibraryText {
    Text(String),
    Object { value: String },
}

#[derive(Debug, Deserialize)]
struct AuthorWorksResponse {
    #[serde(default)]
    entries: Vec<AuthorWorkDoc>,
}

#[derive(Debug, Deserialize)]
struct AuthorWorkDoc {
    #[serde(default)]
    key: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    subjects: Vec<String>,
    #[serde(default)]
    covers: Vec<i64>,
    #[serde(default)]
    first_publish_year: Option<i64>,
    #[serde(default)]
    first_publish_date: String,
}

pub fn split_author_names(text: &str) -> Vec<String> {
    let text = collapse_ws(text);
    if text.is_empty() {
        return Vec::new();
    }

    for sep in [";", " & ", " and ", " / ", "\n"] {
        if text.contains(sep) {
            return text
                .split(sep)
                .map(collapse_ws)
                .filter(|part| !part.is_empty())
                .collect();
        }
    }

    vec![text]
}

pub fn display_author_name(name: &str) -> String {
    let name = collapse_ws(name);
    if !looks_like_sort_name(&name) {
        return name;
    }
    let mut parts = name.splitn(2, ',').map(collapse_ws);
    let family = parts.next().unwrap_or_default();
    let rest = parts.next().unwrap_or_default();
    if family.is_empty() || rest.is_empty() {
        name
    } else {
        format!("{rest} {family}")
    }
}

pub fn sort_author_name(name: &str) -> String {
    let name = collapse_ws(name);
    if name.is_empty() || looks_like_sort_name(&name) {
        return name;
    }
    let parts: Vec<_> = name.split_whitespace().collect();
    if parts.len() < 2 {
        return name;
    }
    let family = parts.last().copied().unwrap_or_default();
    let rest = parts[..parts.len() - 1].join(" ");
    format!("{family}, {rest}")
}

pub fn normalize_author_name(name: &str) -> String {
    normalize_bits(&display_author_name(name))
}

pub fn owned_books_for_author(catalog: &Catalog, author_name: &str) -> Vec<Book> {
    let target = normalize_author_name(author_name);
    if target.is_empty() {
        return Vec::new();
    }

    let mut books = catalog.list_books(SortKey::Title, "").unwrap_or_default();
    books.retain(|book| {
        split_author_names(book.authors_display())
            .into_iter()
            .any(|name| normalize_author_name(&name) == target)
            || normalize_author_name(book.authors_display()) == target
    });
    books
}

pub fn saved_quotes_for_books(catalog: &Catalog, books: &[Book], limit: usize) -> Vec<AuthorQuote> {
    let mut quotes = Vec::new();
    for book in books {
        if let Ok(rows) = catalog.get_annotations_for_book(book.id) {
            for ann in rows {
                if !is_saved_quote(&ann) {
                    continue;
                }
                quotes.push(AuthorQuote {
                    book_title: book.title.clone(),
                    excerpt: ann.text_excerpt.trim().to_string(),
                    created_at: ann.created_at,
                });
            }
        }
    }
    quotes.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    quotes.truncate(limit);
    quotes
}

pub fn series_progress(books: &[Book]) -> Vec<SeriesProgress> {
    let mut grouped: BTreeMap<String, Vec<Book>> = BTreeMap::new();
    for book in books {
        let Some(series) = book.series.clone() else {
            continue;
        };
        let series = series.trim();
        if series.is_empty() {
            continue;
        }
        grouped.entry(series.to_string()).or_default().push(book.clone());
    }

    let mut out = Vec::new();
    for (name, mut owned) in grouped {
        owned.sort_by(|a, b| {
            a.series_index
                .partial_cmp(&b.series_index)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.title.cmp(&b.title))
        });
        out.push(SeriesProgress {
            missing: missing_series_slots(&owned),
            name,
            owned,
        });
    }
    out
}

pub fn fetch_and_cache_author(
    catalog: &Catalog,
    requested_name: &str,
    owned_books: &[Book],
) -> Result<AuthorProfile, String> {
    let requested_display = display_author_name(requested_name);
    let requested_key = normalize_author_name(&requested_display);
    if requested_key.is_empty() {
        return Err("This author name is empty.".into());
    }

    let existing = catalog
        .get_author_profile_by_name(requested_name)
        .map_err(|e| e.to_string())?;

    let doc = search_author(&requested_display, owned_books)?
        .ok_or_else(|| format!("Could not find online info for {}.", requested_display))?;

    let mut profile = fetch_author_profile(&doc)?;
    profile.normalized_name = requested_key;
    if profile.canonical_name.trim().is_empty() {
        profile.canonical_name = requested_display.clone();
    }
    if profile.sort_name.trim().is_empty() {
        profile.sort_name = sort_author_name(&profile.canonical_name);
    }

    let mut aliases = profile.aliases.clone();
    aliases.push(requested_name.to_string());
    aliases.push(requested_display);
    if let Some(current) = existing.as_ref() {
        aliases.extend(current.aliases.iter().cloned());
        if profile.photo_file.is_none() {
            profile.photo_file = current.photo_file.clone();
            profile.photo_path = current.photo_path.clone();
        }
    }
    profile.aliases = dedup_names(aliases);

    if let Some(photo_file) = fetch_author_photo(&profile.openlibrary_key)? {
        profile.photo_file = Some(photo_file.clone());
        profile.photo_path = Some(authors_dir().join(photo_file));
    }

    catalog
        .upsert_author_profile(&profile)
        .map_err(|e| e.to_string())?;
    catalog
        .get_author_profile_by_name(requested_name)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Saved the author info, but could not load it back.".into())
}

pub fn works_not_in_library(profile: &AuthorProfile, owned_books: &[Book]) -> Vec<AuthorWork> {
    let owned_titles: HashSet<String> = owned_books
        .iter()
        .map(|book| normalize_title(&book.title))
        .collect();
    let mut works = profile
        .works
        .iter()
        .filter(|work| !work.title.trim().is_empty())
        .filter(|work| !owned_titles.contains(&normalize_title(&work.title)))
        .cloned()
        .collect::<Vec<_>>();
    works.sort_by(|a, b| {
        a.first_publish_year
            .unwrap_or(i64::MAX)
            .cmp(&b.first_publish_year.unwrap_or(i64::MAX))
            .then_with(|| a.title.cmp(&b.title))
    });
    works.truncate(24);
    works
}

pub fn initials(name: &str) -> String {
    let display = display_author_name(name);
    let mut out = String::new();
    for part in display.split_whitespace().take(2) {
        if let Some(ch) = part.chars().find(|c| c.is_alphabetic()) {
            out.push(ch.to_ascii_uppercase());
        }
    }
    if out.is_empty() {
        "?".into()
    } else {
        out
    }
}

pub fn line_text(parts: &[String]) -> String {
    parts
        .iter()
        .filter(|part| !part.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(" · ")
}

pub fn status_counts(books: &[Book]) -> (usize, usize, usize) {
    let finished = books.iter().filter(|book| book.progress >= 100).count();
    let reading = books
        .iter()
        .filter(|book| book.progress > 0 && book.progress < 100)
        .count();
    (books.len(), finished, reading)
}

fn is_saved_quote(ann: &Annotation) -> bool {
    matches!(ann.kind.as_str(), "quote" | "highlight") && !ann.text_excerpt.trim().is_empty()
}

fn missing_series_slots(books: &[Book]) -> Vec<String> {
    let mut nums: Vec<i32> = books
        .iter()
        .filter_map(|book| {
            let idx = book.series_index;
            if idx > 0.0 && idx.fract().abs() < f32::EPSILON {
                Some(idx.round() as i32)
            } else {
                None
            }
        })
        .collect();
    nums.sort_unstable();
    nums.dedup();

    if nums.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    if let Some(first) = nums.first().copied() {
        for n in 1..first {
            out.push(format!("Missing #{n}"));
        }
    }
    for pair in nums.windows(2) {
        let a = pair[0];
        let b = pair[1];
        if b - a > 1 {
            for n in (a + 1)..b {
                out.push(format!("Missing #{n}"));
            }
        }
    }
    out
}

fn search_author(name: &str, owned_books: &[Book]) -> Result<Option<AuthorSearchDoc>, String> {
    let body = metadata::agent()
        .get("https://openlibrary.org/search/authors.json")
        .query("q", name)
        .query("limit", "12")
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())?;

    let parsed: AuthorSearchResponse = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    Ok(pick_best_author(parsed.docs, name, owned_books))
}

fn pick_best_author(
    docs: Vec<AuthorSearchDoc>,
    requested_name: &str,
    owned_books: &[Book],
) -> Option<AuthorSearchDoc> {
    let target = normalize_author_name(requested_name);
    let owned_titles: HashSet<String> = owned_books
        .iter()
        .map(|book| normalize_title(&book.title))
        .collect();

    docs.into_iter().max_by_key(|doc| {
        let mut score = 0_i64;
        if normalize_author_name(&doc.name) == target {
            score += 500;
        }
        if doc
            .alternate_names
            .iter()
            .any(|name| normalize_author_name(name) == target)
        {
            score += 380;
        }
        if !doc.top_work.trim().is_empty() && owned_titles.contains(&normalize_title(&doc.top_work)) {
            score += 260;
        }
        score += doc.work_count.min(60);
        if !doc.birth_date.trim().is_empty() {
            score += 10;
        }
        if !doc.key.trim().is_empty() {
            score += 10;
        }
        score
    })
}

fn fetch_author_profile(doc: &AuthorSearchDoc) -> Result<AuthorProfile, String> {
    let author_key = author_path(&doc.key);
    let body = metadata::agent()
        .get(&format!("https://openlibrary.org{author_key}.json"))
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())?;
    let parsed: AuthorResponse = serde_json::from_str(&body).map_err(|e| e.to_string())?;

    let works_body = metadata::agent()
        .get(&format!("https://openlibrary.org{author_key}/works.json"))
        .query("limit", "40")
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())?;
    let works_parsed: AuthorWorksResponse =
        serde_json::from_str(&works_body).map_err(|e| e.to_string())?;

    let canonical_name = if !parsed.name.trim().is_empty() {
        parsed.name.trim().to_string()
    } else if !parsed.personal_name.trim().is_empty() {
        parsed.personal_name.trim().to_string()
    } else {
        display_author_name(&doc.name)
    };
    let bio = match parsed.bio {
        Some(OpenLibraryText::Text(text)) => text,
        Some(OpenLibraryText::Object { value }) => value,
        None => String::new(),
    };
    let works = works_parsed
        .entries
        .into_iter()
        .filter(|entry| !entry.title.trim().is_empty())
        .map(|entry| AuthorWork {
            title: entry.title.trim().to_string(),
            first_publish_year: entry
                .first_publish_year
                .or_else(|| extract_year(&entry.first_publish_date)),
            subjects: entry
                .subjects
                .into_iter()
                .filter(|subject| subject.len() <= 32)
                .take(4)
                .collect(),
            cover_id: entry.covers.into_iter().next(),
            work_key: entry.key,
        })
        .collect::<Vec<_>>();

    let mut aliases = parsed.alternate_names;
    aliases.push(canonical_name.clone());
    aliases.push(sort_author_name(&canonical_name));
    aliases.push(display_author_name(&doc.name));

    Ok(AuthorProfile {
        id: 0,
        canonical_name: canonical_name.clone(),
        sort_name: sort_author_name(&canonical_name),
        normalized_name: normalize_author_name(&canonical_name),
        bio: bio.trim().to_string(),
        birth_date: if parsed.birth_date.trim().is_empty() {
            doc.birth_date.trim().to_string()
        } else {
            parsed.birth_date.trim().to_string()
        },
        death_date: if parsed.death_date.trim().is_empty() {
            doc.death_date.trim().to_string()
        } else {
            parsed.death_date.trim().to_string()
        },
        top_work: doc.top_work.trim().to_string(),
        top_subjects: doc.top_subjects.into_iter().take(8).collect(),
        openlibrary_key: author_key,
        photo_file: None,
        photo_path: None,
        work_count: doc.work_count.max(works.len() as i64),
        works,
        aliases: dedup_names(aliases),
        fetched_at: String::new(),
        source_url: format!("https://openlibrary.org{author_key}"),
    })
}

fn fetch_author_photo(author_key: &str) -> Result<Option<String>, String> {
    let olid = author_key.rsplit('/').next().unwrap_or(author_key).trim();
    if olid.is_empty() {
        return Ok(None);
    }
    let cover = CoverRef::Url(format!("https://covers.openlibrary.org/a/olid/{olid}-L.jpg"));
    let bytes = match metadata::fetch_cover(&cover) {
        Ok(bytes) if !bytes.is_empty() => bytes,
        Ok(_) => return Ok(None),
        Err(_) => return Ok(None),
    };
    fs::create_dir_all(authors_dir()).map_err(|e| e.to_string())?;
    let file_name = format!("{olid}.jpg");
    fs::write(authors_dir().join(&file_name), bytes).map_err(|e| e.to_string())?;
    Ok(Some(file_name))
}

fn author_path(key: &str) -> String {
    let key = key.trim();
    if key.starts_with("/authors/") {
        key.to_string()
    } else if key.starts_with("OL") {
        format!("/authors/{key}")
    } else {
        format!("/authors/{}", key.trim_start_matches('/'))
    }
}

fn dedup_names(names: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for name in names {
        let display = display_author_name(&name);
        let key = normalize_author_name(&display);
        if key.is_empty() || !seen.insert(key) {
            continue;
        }
        out.push(display);
    }
    out
}

fn looks_like_sort_name(name: &str) -> bool {
    let mut parts = name.split(',').map(str::trim).filter(|part| !part.is_empty());
    let Some(first) = parts.next() else {
        return false;
    };
    let Some(second) = parts.next() else {
        return false;
    };
    parts.next().is_none() && !first.is_empty() && !second.is_empty() && second.split_whitespace().count() <= 6
}

fn extract_year(text: &str) -> Option<i64> {
    let digits = text
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.len() < 4 {
        None
    } else {
        digits[..4].parse().ok()
    }
}

fn collapse_ws(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_bits(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .collect()
}

fn normalize_title(title: &str) -> String {
    normalize_bits(title)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_sort_name_to_same_key() {
        assert_eq!(
            normalize_author_name("Martin, George R. R."),
            normalize_author_name("George R. R. Martin")
        );
    }

    #[test]
    fn splits_common_multi_author_formats() {
        assert_eq!(
            split_author_names("Alice Smith & Bob Jones"),
            vec!["Alice Smith", "Bob Jones"]
        );
        assert_eq!(
            split_author_names("Alice Smith; Bob Jones"),
            vec!["Alice Smith", "Bob Jones"]
        );
    }
}
