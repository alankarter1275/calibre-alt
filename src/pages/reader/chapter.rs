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
    }

    /// A0 step 5: warm the *next* chapter's file while this one is being read.
    pub(crate) fn preload_next_chapter(&self) {
        if let Some(path) = crate::preload::next_chapter_file(&self.open.spine, self.chapter) {
            crate::preload::warm_chapter_file(path);
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
            model.webview.load_html(&html, Some(&base_uri));
        }
        Err(err) => {
            let err_html = format!(
                "<html><body style='padding:2rem;background:#f5f0e8;color:#2c2820;font-family:Georgia,serif'><h1>Could not load chapter</h1><pre>{err:#}</pre></body></html>"
            );
            model.webview.load_html(&err_html, None);
        }
    }
}
