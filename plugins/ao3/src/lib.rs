wit_bindgen::generate!({
    path: "../../wit/kalam.wit",
    world: "plugin",
});

use exports::kalam::plugin::scraper::Guest;
use kalam::plugin::types::*;

struct Ao3Plugin;

impl Guest for Ao3Plugin {
    fn id() -> String {
        "ao3".to_string()
    }

    fn name() -> String {
        "Archive of Our Own".to_string()
    }

    fn base_url() -> String {
        "https://archiveofourown.org".to_string()
    }

    fn get_filter_definitions() -> Vec<FilterDefinition> {
        vec![
            FilterDefinition {
                id: "fandom".to_string(),
                name: "Fandom".to_string(),
                filter_type: FilterType::Text("Fandom name...".to_string()),
                default_value: "".to_string(),
            },
        ]
    }

    fn search(_query: String, _page: u32, _filters: Vec<(String, String)>) -> Result<SearchPage, String> {
        Ok(SearchPage {
            results: vec![],
            has_more: false,
        })
    }

    fn get_details(_remote_id: String) -> Result<RemoteBookDetails, String> {
        Err("not implemented".into())
    }

    fn get_chapters(_remote_id: String) -> Result<Vec<RemoteChapter>, String> {
        Err("not implemented".into())
    }

    fn get_chapter_content(_chapter_id: String) -> Result<ChapterContent, String> {
        Err("not implemented".into())
    }
}

export!(Ao3Plugin);
