//! EPUB chapter loading, preloading, CSS generation, and navigation.

use super::mod_model::ReaderModel;
use crate::epub_book::reading_css;
use webkit6::prelude::*;

impl ReaderModel {
    pub(crate) fn css(&self) -> String {
        reading_css(
            self.theme,
            self.font_px,
            self.line_height,
            self.column_px,
            crate::theme::current(self.service.catalog()),
        )
    }

    pub(crate) fn current_chapter_title(&self) -> &str {
        self.open
            .spine
            .get(self.chapter)
            .map(|item| item.title.as_str())
            .unwrap_or("Reading")
    }

    pub(crate) fn go_chapter(&mut self, idx: usize, frac: f64) {
        self.save_progress();
        self.chapter = idx;
        self.fraction = frac.clamp(0.0, 1.0);
        self.loading = true;
        self.reload_annotations();
        self.reload_bookmarks();
        self.reload_saved_words();
        load_chapter(self);
        self.loading = false;
        self.preload_next_chapter();
        self.preload_and_prepend_prev_chapter();
    }

    /// A0 step 5: warm the *next* chapter's file while this one is being read.
    pub(crate) fn preload_next_chapter(&self) {
        if let Some(path) = crate::preload::next_chapter_file(&self.open.spine, self.chapter) {
            crate::preload::warm_chapter_file(path);
        }
    }

    pub(crate) fn preload_and_prepend_prev_chapter(&self) {
        if self.chapter == 0 {
            return;
        }
        let prev_idx = self.chapter - 1;
        if prev_idx >= self.open.chapter_count() {
            return;
        }
        if let Some(item) = self.open.spine.get(prev_idx) {
            let title = item.title.clone();
            let path = item.path.clone();
            if let Ok(body) = self.open.chapter_body(prev_idx) {
                if body.contains("kalam-remote-placeholder") {
                    let source_id = extract_attr(&body, "data-source-id").unwrap_or_default();
                    let chapter_id = extract_attr(&body, "data-chapter-id").unwrap_or_default();
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
                                if let Ok(c_html) = res {
                                    let title_json = serde_json::to_string(&title).unwrap_or_else(|_| "\"\"".into());
                                    let body_json = serde_json::to_string(&c_html).unwrap_or_else(|_| "\"\"".into());
                                    let script = format!("if (window.kalamPrependChapter) window.kalamPrependChapter({prev_idx}, {title_json}, {body_json});");
                                    super::js_bridge::eval_js(&webview, &script);
                                }
                            }
                        );
                        return;
                    }
                }

                let title_json = serde_json::to_string(&title).unwrap_or_else(|_| "\"\"".into());
                let body_json = serde_json::to_string(&body).unwrap_or_else(|_| "\"\"".into());
                let script = format!("if (window.kalamPrependChapter) window.kalamPrependChapter({prev_idx}, {title_json}, {body_json});");
                super::js_bridge::eval_js(&self.webview, &script);
            }
        }
    }
}

pub(crate) fn chapter_label(model: &ReaderModel, chapter_index: usize) -> String {
    if let Some(item) = model.open.spine.get(chapter_index) {
        item.title.clone()
    } else {
        format!("Ch {}", chapter_index + 1)
    }
}

pub(crate) fn extract_attr(html: &str, attr: &str) -> Option<String> {
    let needle = format!("{attr}=\"");
    let start = html.find(&needle)? + needle.len();
    let end = html[start..].find('"')? + start;
    Some(html[start..end].to_string())
}

pub(crate) fn load_chapter(model: &ReaderModel) {
    if model.open.chapter_count() == 0 {
        return;
    }
    crate::timing::span("chapter_load");
    match model
        .open
        .chapter_html(model.chapter, &model.css(), model.fraction)
    {
        Ok(html) => {
            let base = model.open.extract_dir.to_string_lossy();
            let base_uri = if base.starts_with('/') {
                format!("file://{base}/")
            } else {
                format!("file:///{}/", base.replace('\\', "/"))
            };
            if html.contains("kalam-remote-placeholder") {
                if let Some(item) = model.open.spine.get(model.chapter) {
                    let path = item.path.clone();
                    let title = item.title.clone();
                    let source_id = extract_attr(&html, "data-source-id").unwrap_or_default();
                    let chapter_id = extract_attr(&html, "data-chapter-id").unwrap_or_default();
                    if !source_id.is_empty() && !chapter_id.is_empty() {
                        let webview = model.webview.clone();
                        let css = model.css();
                        let fraction = model.fraction;
                        let ch_idx = model.chapter;
                        let total_chapters = model.open.chapter_count();
                        let base_uri_clone = base_uri.clone();
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
                                    quick_xml::escape::escape(&title),
                                    quick_xml::escape::escape(&title),
                                    c_html
                                );
                                let _ = std::fs::write(&path, &xhtml);
                                Ok::<_, anyhow::Error>(xhtml)
                            },
                            |_| {},
                            move |res| {
                                if let Ok(xhtml) = res {
                                    let full = crate::epub_book::inject_reading_shell(&xhtml, &base_uri_clone, &css, fraction, ch_idx, total_chapters);
                                    webview.load_html(&full, Some(&base_uri_clone));
                                }
                            }
                        );
                    }
                }
            }
            model.webview.load_html(&html, Some(&base_uri));
            model.preload_and_prepend_prev_chapter();
        }
        Err(err) => {
            let err_html = format!(
                "<html><body style='padding:2rem;background:#f5f0e8;color:#2c2820;font-family:Georgia,serif'><h1>Could not load chapter</h1><pre>{err:#}</pre></body></html>"
            );
            model.webview.load_html(&err_html, None);
        }
    }
}
