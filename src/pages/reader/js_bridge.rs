//! WebKit JavaScript bridge, dictionary lookups, highlight injection, and JS evaluation.

use super::mod_model::ReaderModel;
use super::types::*;
use crate::db::{DictEntry, EntryData, HighlightColor, PhraseLookup};
use relm4::ComponentSender;
use webkit6::prelude::*;

impl ReaderModel {
    pub(crate) fn inject_highlights(&self) {
        if self.chapter_annotations.is_empty() {
            return;
        }
        let simple: Vec<serde_json::Value> = self
            .chapter_annotations
            .iter()
            .map(|a| {
                serde_json::json!({
                    "id": a.id,
                    "start_path": a.start_path,
                    "start_offset": a.start_offset,
                    "end_path": a.end_path,
                    "end_offset": a.end_offset,
                    "color": a.color,
                    "text_excerpt": a.text_excerpt,
                })
            })
            .collect();
        if let Ok(json) = serde_json::to_string(&simple) {
            let escaped = json
                .replace('\\', "\\\\")
                .replace('\'', "\\'")
                .replace('\n', "\\n");
            let script = format!(
                "if (window.kalamInjectHighlights) window.kalamInjectHighlights('{}');",
                escaped
            );
            eval_js(&self.webview, &script);
        }
    }

    pub(crate) fn restore_pending_annotation(&mut self) {
        let Some(id) = self.pending_annotation_jump.take() else {
            return;
        };
        let Some(annotation) = self
            .all_book_annotations
            .iter()
            .find(|annotation| annotation.id == id)
        else {
            return;
        };
        let anchor = serde_json::json!({
            "id": annotation.id,
            "start_path": annotation.start_path,
            "start_offset": annotation.start_offset,
            "end_path": annotation.end_path,
            "end_offset": annotation.end_offset,
            "text_excerpt": annotation.text_excerpt,
        });
        let Ok(anchor_json) = serde_json::to_string(&anchor) else {
            return;
        };
        let script = format!(
            "setTimeout(function() {{ if (window.kalamRevealAnnotation) window.kalamRevealAnnotation({anchor}); }}, 90);",
            anchor = anchor_json,
        );
        eval_js(&self.webview, &script);
    }

    pub(crate) fn lookup_dict(&self, query: &str, limit: usize) -> Vec<DictEntry> {
        crate::timing::span("dict_lookup");
        let results = if query.split_whitespace().count() > 1 {
            match self
                .service
                .catalog()
                .search_phrase(query, limit)
                .unwrap_or(PhraseLookup::Empty)
            {
                PhraseLookup::Phrase(hits) => hits,
                PhraseLookup::Breakdown(parts) => parts
                    .iter()
                    .filter_map(|(_, hits)| hits.first().cloned())
                    .take(limit)
                    .collect(),
                PhraseLookup::Empty => Vec::new(),
            }
        } else {
            match self.service.catalog().search_dict(query, limit) {
                Ok(hits) => hits,
                Err(err) => {
                    crate::notify::error("Dictionary search failed", &err.to_string());
                    Vec::new()
                }
            }
        };
        let _ = self.service.catalog().log_dict_lookup(
            query,
            Some(self.book_id),
            Some(self.chapter as i64),
            self.dict_context.as_deref(),
            !results.is_empty(),
        );
        crate::timing::span_end("dict_lookup");
        results
    }

    pub(crate) fn show_dict_in_webview(
        &self,
        query: &str,
        rect_json: Option<String>,
        hint_index: Option<usize>,
    ) {
        let data = self
            .service
            .catalog()
            .lookup_entry(query)
            .unwrap_or_else(|_| EntryData {
                word: query.trim().to_string(),
                ..Default::default()
            });
        let saved = self
            .service
            .catalog()
            .saved_word_exists(&data.word, self.book_id)
            .unwrap_or(false);
        let pronunciation = crate::db::pronunciation_for(&data.word).map(|p| format!("/{p}"));
        let payload = serde_json::json!({
            "word": data.word,
            "pos": data.pos,
            "senses": data.senses.iter().map(|s| serde_json::json!({
                "number": s.number,
                "pos": s.pos,
                "def": s.def.chars().take(2000).collect::<String>(),
                "example": s.example,
            })).collect::<Vec<_>>(),
            "synonyms": data.synonyms,
            "antonyms": data.antonyms,
            "idioms": data.idioms.iter().map(|(phrase, def)| {
                serde_json::json!({ "phrase": phrase, "def": def })
            }).collect::<Vec<_>>(),
            "suggestions": data.suggestions,
            "saved": saved,
            "hint": hint_index,
            "pronunciation": pronunciation,
        });
        let payload_json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into());
        let rect_part = rect_json
            .as_deref()
            .map(|rect| serde_json::to_string(rect).unwrap_or_else(|_| "null".into()))
            .unwrap_or_else(|| "null".into());
        let script = format!(
            "if (window.kalamShowDict) window.kalamShowDict({}, {});",
            payload_json, rect_part
        );
        eval_js(&self.webview, &script);
    }

    pub(crate) fn handle_js_payload(&mut self, payload: JsPayload, sender: ComponentSender<Self>) {
        match payload.kind.as_str() {
            "progress" => {
                if let Some(ch) = payload.chapter {
                    if ch < self.open.chapter_count() {
                        self.chapter = ch;
                    }
                }
                if let Some(f) = payload.fraction {
                    self.fraction = f.clamp(0.0, 1.0);
                }
                self.save_progress();
                self.checkpoint_session();
            }
            "chapter-changed" => {
                if let Some(ch) = payload.chapter {
                    if ch < self.open.chapter_count() && ch != self.chapter {
                        self.chapter = ch;
                        self.fraction = 0.0;
                        self.save_progress();
                        self.reload_annotations();
                        self.reload_bookmarks();
                        self.reload_saved_words();
                        sender.input(ReaderMsg::AnnotationsReload);
                        self.preload_next_chapter();
                    }
                }
            }
            "request-next-chapter" => {
                let and_scroll = payload.and_scroll_to.unwrap_or(false);
                if let Some(next_idx) = payload.next {
                    if next_idx < self.open.chapter_count() {
                        if let Some(item) = self.open.spine.get(next_idx) {
                            let title = item.title.clone();
                            let path = item.path.clone();
                            if let Ok(body) = self.open.chapter_body(next_idx) {
                                if body.contains("kalam-remote-placeholder") {
                                    let source_id = super::chapter::extract_attr(&body, "data-source-id").unwrap_or_default();
                                    let chapter_id = super::chapter::extract_attr(&body, "data-chapter-id").unwrap_or_default();
                                    if !source_id.is_empty() && !chapter_id.is_empty() {
                                        let webview = self.webview.clone();
                                        let chap_title = title.clone();
                                        crate::tasks::spawn(
                                            move |_| {
                                                let source_mgr = crate::sources::global_source_manager();
                                                let source = source_mgr.get(&source_id).ok_or_else(|| anyhow::anyhow!("Source not found"))?;
                                                let chap_content = source.get_chapter_content(&chapter_id)?;
                                                let c_html = match chap_content {
                                                    crate::sources::ChapterContent::Html(h) => h,
                                                    _ => return Err(anyhow::anyhow!("Expected HTML content")),
                                                };
                                                let xhtml = format!(
                                                    r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>{}</title></head>
<body>
<h1>{}</h1>
{}
</body>
</html>"#,
                                                    quick_xml::escape::escape(&chap_title),
                                                    quick_xml::escape::escape(&chap_title),
                                                    c_html
                                                );
                                                let _ = std::fs::write(&path, &xhtml);
                                                Ok::<_, anyhow::Error>(c_html)
                                            },
                                            |_| {},
                                            move |res| {
                                                match res {
                                                    Ok(c_html) => {
                                                        let title_json = serde_json::to_string(&title).unwrap_or_else(|_| "\"\"".into());
                                                        let body_json = serde_json::to_string(&c_html).unwrap_or_else(|_| "\"\"".into());
                                                        let script = format!("if (window.kalamAppendChapter) window.kalamAppendChapter({next_idx}, {title_json}, {body_json}, {and_scroll});");
                                                        eval_js(&webview, &script);
                                                    }
                                                    Err(e) => {
                                                        eprintln!("Failed to fetch next remote chapter {next_idx}: {e}");
                                                        eval_js(&webview, "if (window.kalam) window.kalam._loadingNext = false;");
                                                    }
                                                }
                                            }
                                        );
                                        return;
                                    }
                                }

                                let title_json = serde_json::to_string(&title).unwrap_or_else(|_| "\"\"".into());
                                let body_json = serde_json::to_string(&body).unwrap_or_else(|_| "\"\"".into());
                                let script = format!("if (window.kalamAppendChapter) window.kalamAppendChapter({next_idx}, {title_json}, {body_json}, {and_scroll});");
                                eval_js(&self.webview, &script);
                            }
                        }
                    } else {
                        eval_js(&self.webview, "if (window.kalam) window.kalam._loadingNext = false;");
                    }
                }
            }
            "request-prev-chapter" => {
                let and_scroll = payload.and_scroll_to.unwrap_or(false);
                if let Some(prev_idx) = payload.prev {
                    if prev_idx < self.open.chapter_count() {
                        if let Some(item) = self.open.spine.get(prev_idx) {
                            let title = item.title.clone();
                            let path = item.path.clone();
                            if let Ok(body) = self.open.chapter_body(prev_idx) {
                                if body.contains("kalam-remote-placeholder") {
                                    let source_id = super::chapter::extract_attr(&body, "data-source-id").unwrap_or_default();
                                    let chapter_id = super::chapter::extract_attr(&body, "data-chapter-id").unwrap_or_default();
                                    if !source_id.is_empty() && !chapter_id.is_empty() {
                                        let webview = self.webview.clone();
                                        let chap_title = title.clone();
                                        crate::tasks::spawn(
                                            move |_| {
                                                let source_mgr = crate::sources::global_source_manager();
                                                let source = source_mgr.get(&source_id).ok_or_else(|| anyhow::anyhow!("Source not found"))?;
                                                let chap_content = source.get_chapter_content(&chapter_id)?;
                                                let c_html = match chap_content {
                                                    crate::sources::ChapterContent::Html(h) => h,
                                                    _ => return Err(anyhow::anyhow!("Expected HTML content")),
                                                };
                                                let xhtml = format!(
                                                    r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>{}</title></head>
<body>
<h1>{}</h1>
{}
</body>
</html>"#,
                                                    quick_xml::escape::escape(&chap_title),
                                                    quick_xml::escape::escape(&chap_title),
                                                    c_html
                                                );
                                                let _ = std::fs::write(&path, &xhtml);
                                                Ok::<_, anyhow::Error>(c_html)
                                            },
                                            |_| {},
                                            move |res| {
                                                match res {
                                                    Ok(c_html) => {
                                                        let title_json = serde_json::to_string(&title).unwrap_or_else(|_| "\"\"".into());
                                                        let body_json = serde_json::to_string(&c_html).unwrap_or_else(|_| "\"\"".into());
                                                        let script = format!("if (window.kalamPrependChapter) window.kalamPrependChapter({prev_idx}, {title_json}, {body_json}, {and_scroll});");
                                                        eval_js(&webview, &script);
                                                    }
                                                    Err(e) => {
                                                        eprintln!("Failed to fetch prev remote chapter {prev_idx}: {e}");
                                                        eval_js(&webview, "if (window.kalam) window.kalam._loadingPrev = false;");
                                                    }
                                                }
                                            }
                                        );
                                        return;
                                    }
                                }

                                let title_json = serde_json::to_string(&title).unwrap_or_else(|_| "\"\"".into());
                                let body_json = serde_json::to_string(&body).unwrap_or_else(|_| "\"\"".into());
                                let script = format!("if (window.kalamPrependChapter) window.kalamPrependChapter({prev_idx}, {title_json}, {body_json}, {and_scroll});");
                                eval_js(&self.webview, &script);
                            }
                        }
                    } else {
                        eval_js(&self.webview, "if (window.kalam) window.kalam._loadingPrev = false;");
                    }
                }
            }
            "link-click" => {
                if let Some(href) = payload.href {
                    if href.starts_with("http://") || href.starts_with("https://") {
                        let launcher = gtk::UriLauncher::new(&href);
                        launcher.launch(None::<&gtk::Window>, gtk::gio::Cancellable::NONE, |_| {});
                    } else if let Some(spine_idx) = self.open.spine_index_for(&href) {
                        let script = format!("if (window.kalamNavigateChapter) window.kalamNavigateChapter({spine_idx});");
                        eval_js(&self.webview, &script);
                    }
                }
            }
            "jump-to-chapter" => {
                // JS couldn't find the chapter in DOM — feed it directly without any page reload.
                if let Some(target_idx) = payload.target {
                    if target_idx < self.open.chapter_count() {
                        self.chapter = target_idx;
                        self.fraction = 0.0;
                        self.save_progress();
                        self.reload_annotations();
                        self.reload_bookmarks();
                        self.reload_saved_words();
                        sender.input(ReaderMsg::AnnotationsReload);
                        self.preload_next_chapter();

                        if let Some(item) = self.open.spine.get(target_idx) {
                            let title = item.title.clone();
                            let path = item.path.clone();
                            if let Ok(body) = self.open.chapter_body(target_idx) {
                                if body.contains("kalam-remote-placeholder") {
                                    let source_id = super::chapter::extract_attr(&body, "data-source-id").unwrap_or_default();
                                    let chapter_id = super::chapter::extract_attr(&body, "data-chapter-id").unwrap_or_default();
                                    if !source_id.is_empty() && !chapter_id.is_empty() {
                                        let webview = self.webview.clone();
                                        let chap_title = title.clone();
                                        crate::tasks::spawn(
                                            move |_| {
                                                let source_mgr = crate::sources::global_source_manager();
                                                let source = source_mgr.get(&source_id).ok_or_else(|| anyhow::anyhow!("Source not found"))?;
                                                let chap_content = source.get_chapter_content(&chapter_id)?;
                                                let c_html = match chap_content {
                                                    crate::sources::ChapterContent::Html(h) => h,
                                                    _ => return Err(anyhow::anyhow!("Expected HTML content")),
                                                };
                                                let xhtml = format!(
                                                    r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>{}</title></head>
<body>
<h1>{}</h1>
{}
</body>
</html>"#,
                                                    quick_xml::escape::escape(&chap_title),
                                                    quick_xml::escape::escape(&chap_title),
                                                    c_html
                                                );
                                                let _ = std::fs::write(&path, &xhtml);
                                                Ok::<_, anyhow::Error>((chap_title, c_html))
                                            },
                                            |_| {},
                                            move |res| {
                                                if let Ok((chap_title, c_html)) = res {
                                                    let title_json = serde_json::to_string(&chap_title).unwrap_or_else(|_| "\"\"".into());
                                                    let body_json = serde_json::to_string(&c_html).unwrap_or_else(|_| "\"\"".into());
                                                    let script = format!("if (window.kalamJumpToChapter) window.kalamJumpToChapter({target_idx}, {title_json}, {body_json}, 0.0);");
                                                    eval_js(&webview, &script);
                                                }
                                            }
                                        );
                                    }
                                } else {
                                    let title_json = serde_json::to_string(&title).unwrap_or_else(|_| "\"\"".into());
                                    let body_json = serde_json::to_string(&body).unwrap_or_else(|_| "\"\"".into());
                                    let script = format!("if (window.kalamJumpToChapter) window.kalamJumpToChapter({target_idx}, {title_json}, {body_json}, 0.0);");
                                    eval_js(&self.webview, &script);
                                }
                            }
                        }
                    }
                }
            }
            "next" => {
                sender.input(ReaderMsg::NextChapter);
            }
            "selection" => {
                self.last_selection = payload.text;
            }
            "highlight" => {
                let color = payload.color.unwrap_or_else(|| "yellow".into());
                let text = payload.text.unwrap_or_default();
                let sp = payload.start_path.unwrap_or_default();
                let so = payload.start_offset.unwrap_or(0);
                let ep = payload.end_path.unwrap_or_default();
                let eo = payload.end_offset.unwrap_or(0);
                let tmp_id = payload
                    .tmp_id
                    .unwrap_or_else(|| format!("tmp_{}", chrono_now()));
                if sp.is_empty() || ep.is_empty() || text.trim().is_empty() {
                    return;
                }
                let col = HighlightColor::from_str_lossy(&color).as_str().to_string();
                match self.service.catalog().insert_annotation(
                    self.book_id,
                    "highlight",
                    self.chapter as i64,
                    &sp,
                    so,
                    &ep,
                    eo,
                    &col,
                    &text,
                    "",
                ) {
                    Ok(real_id) => {
                        let script = format!(
                            "try {{ var nodes = document.querySelectorAll('span[data-annotation-id=\\\"{tmp}\\\"]'); nodes.forEach(function(n){{ n.dataset.annotationId='{real}'; }}); }} catch(e) {{}}",
                            tmp = tmp_id.replace('\'', "\\'"),
                            real = real_id,
                        );
                        eval_js(&self.webview, &script);
                        self.reload_annotations();
                        sender.input(ReaderMsg::AnnotationsReload);
                    }
                    Err(e) => crate::notify::error("Could not save the highlight", &e.to_string()),
                }
            }
            "quote" => {
                let text = payload.text.unwrap_or_default();
                let sp = payload.start_path.unwrap_or_default();
                let so = payload.start_offset.unwrap_or(0);
                let ep = payload.end_path.unwrap_or_default();
                let eo = payload.end_offset.unwrap_or(0);
                if sp.is_empty() || ep.is_empty() || text.trim().is_empty() {
                    return;
                }
                match self.service.catalog().insert_annotation(
                    self.book_id,
                    "quote",
                    self.chapter as i64,
                    &sp,
                    so,
                    &ep,
                    eo,
                    "yellow",
                    &text,
                    "",
                ) {
                    Ok(_) => {
                        crate::notify::compact("Quote saved", "");
                        self.reload_annotations();
                        sender.input(ReaderMsg::AnnotationsReload);
                    }
                    Err(e) => crate::notify::error("Could not save the quote", &e.to_string()),
                }
            }
            "dict-lookup" => {
                let word = payload.word.unwrap_or_default();
                let context = payload.context;
                let rect_json = payload.rect.map(|v| v.to_string());
                if word.trim().is_empty() {
                    return;
                }
                self.dict_context = context.clone();
                self.dict_lookup_rect_json = rect_json.clone();
                let data = self
                    .service
                    .catalog()
                    .lookup_entry(&word)
                    .unwrap_or_else(|_| EntryData {
                        word: word.clone(),
                        ..Default::default()
                    });
                self.dict_lookup_word = Some(data.word.clone());
                self.dict_lookup_def = data
                    .senses
                    .first()
                    .map(|s| s.def.clone())
                    .or_else(|| (!data.suggestions.is_empty()).then(|| data.word.clone()));
                let hint_index = context.as_deref().and_then(|sentence| {
                    if self.service.catalog().get_pref_i64("dict_sense_hint", 1) == 0 {
                        return None;
                    }
                    if !data.senses.iter().any(|s| s.pos.is_some()) {
                        return None;
                    }
                    crate::db::likely_sense_index(sentence, &data.word, &data.senses)
                });
                let _ = self.service.catalog().log_dict_lookup(
                    &word,
                    Some(self.book_id),
                    Some(self.chapter as i64),
                    context.as_deref(),
                    !data.senses.is_empty(),
                );
                self.show_dict_in_webview(&word, rect_json, hint_index);
            }
            "save-word" => {
                let word = payload.word.unwrap_or_default();
                let def = payload.definition.unwrap_or_default();
                if !word.trim().is_empty() && !def.trim().is_empty() {
                    match self.service.catalog().insert_saved_word(
                        &word,
                        &def,
                        None,
                        Some(self.book_id),
                        Some(self.chapter as i64),
                        payload.context.as_deref().or(self.dict_context.as_deref()),
                    ) {
                        Ok(_) => {
                            crate::notify::compact("Word saved", &word);
                            self.reload_saved_words();
                            self.right_tab = RightSidebarTab::Words;
                            self.right_sidebar_open = true;
                        }
                        Err(e) => crate::notify::error("Could not save the word", &e.to_string()),
                    }
                }
            }
            "unsave-word" => {
                let word = payload.word.unwrap_or_default();
                if !word.trim().is_empty() {
                    match self
                        .service
                        .catalog()
                        .delete_saved_word_by_word(&word, self.book_id)
                    {
                        Ok(_) => {
                            crate::notify::compact("Word removed", &word);
                            self.reload_saved_words();
                        }
                        Err(e) => crate::notify::error("Could not remove the word", &e.to_string()),
                    }
                }
            }
            "dict-shortcut" => {
                self.right_tab = RightSidebarTab::Words;
                self.right_sidebar_open = true;
            }
            "search-in-book" => {
                let word = payload.word.unwrap_or_default();
                if !word.trim().is_empty() {
                    let word_json = serde_json::to_string(&word).unwrap_or_else(|_| "\"\"".into());
                    let script = format!("window.kalamSearchInBook({word_json});");
                    eval_js(&self.webview, &script);
                }
            }
            "search-in-book-done" => match payload.count.unwrap_or(0) {
                0 => crate::notify::compact("No matches in this chapter", ""),
                1 => crate::notify::compact("1 match in this chapter", ""),
                n => crate::notify::compact(&format!("{n} matches in this chapter"), ""),
            },
            "reader-ui-hide" => {
                self.show_back_button = false;
                self.show_bottom_pill = false;
            }
            "reader-ui-show-back" => {
                self.show_back_button = true;
                self.show_bottom_pill = false;
            }
            "reader-ui-show-pill" => {
                self.show_bottom_pill = true;
                self.show_back_button = false;
            }
            "reader-ui-show-all" => {
                self.show_back_button = true;
                self.show_bottom_pill = true;
            }
            _ => {}
        }
    }
}

pub(crate) fn truncate_def(s: &str, n: usize) -> String {
    let mut chars = s.chars();
    let head: String = chars.by_ref().take(n).collect();
    if chars.next().is_none() {
        head
    } else {
        format!("{head}…")
    }
}

pub(crate) fn eval_js(webview: &webkit6::WebView, script: &str) {
    webview.evaluate_javascript(script, None, None, Option::<&gio::Cancellable>::None, |res| {
        if let Err(err) = res {
            eprintln!("kalam js eval error: {err}");
        }
    });
}

pub(crate) fn url_decode(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next();
            let h2 = chars.next();
            if let (Some(a), Some(b)) = (h1, h2) {
                let hex = format!("{a}{b}");
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    out.push(byte as char);
                    continue;
                }
                out.push('%');
                out.push(a);
                out.push(b);
            } else {
                out.push('%');
                if let Some(a) = h1 {
                    out.push(a);
                }
                if let Some(b) = h2 {
                    out.push(b);
                }
            }
        } else if c == '+' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

pub(crate) fn chrono_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_def_keeps_short_text_untouched() {
        assert_eq!(truncate_def("short", 180), "short");
        assert_eq!(truncate_def("abcde", 5), "abcde");
    }

    #[test]
    fn truncate_def_cuts_long_text_and_marks_it() {
        assert_eq!(truncate_def("abcdef", 5), "abcde…");
    }

    #[test]
    fn truncate_def_does_not_panic_on_multibyte_text() {
        let cases = [
            "café — a small restaurant serving coffee",
            "\u{2018}bank\u{2019} the side of a river",
            "/ˈbæŋk/ pronunciation of the headword",
            "銀行 — a financial institution",
            "ααααααααααααααααααααααααααααα",
        ];
        for case in cases {
            for n in 0..12 {
                let out = truncate_def(case, n);
                let expected: String = case.chars().take(n).collect();
                assert!(
                    out.starts_with(&expected),
                    "truncate_def({case:?}, {n}) = {out:?} lost the prefix"
                );
                assert!(out.chars().count() <= n + 1, "cut {n} produced {out:?}");
            }
        }
    }

    #[test]
    fn truncate_def_counts_characters_not_bytes() {
        assert_eq!(truncate_def("ααααα", 5), "ααααα");
        assert_eq!(truncate_def("αααααα", 5), "ααααα…");
    }
}
