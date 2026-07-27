use crate::db::{Catalog, SavedWord};
use gtk::prelude::*;
use relm4::prelude::*;
use std::sync::Arc;

#[derive(Debug)]
pub enum SavedWordsOut {
    #[allow(dead_code)]
    OpenBook { book_id: i64 },
}

#[derive(Debug)]
pub enum SavedWordsMsg {
    SearchChanged(String),
    Delete(i64),
    Refresh,
}

pub struct SavedWordsModel {
    catalog: Arc<Catalog>,
    query: String,
    words: Vec<SavedWord>,
    status: String,
}

#[relm4::component(pub)]
impl Component for SavedWordsModel {
    type Init = Arc<Catalog>;
    type Input = SavedWordsMsg;
    type Output = SavedWordsOut;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 12,
            set_hexpand: true,

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 10,
                gtk::Label {
                    set_label: "Saved words",
                    add_css_class: "kalam-page-title",
                    set_halign: gtk::Align::Start,
                    set_hexpand: true,
                },
                gtk::Button {
                    set_label: "↻",
                    add_css_class: "kalam-secondary-btn",
                    connect_clicked => SavedWordsMsg::Refresh,
                },
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 6,
                #[name = "search_entry"]
                gtk::SearchEntry {
                    set_placeholder_text: Some("Search words…"),
                    set_hexpand: true,
                    connect_search_changed[sender] => move |e| {
                        sender.input(SavedWordsMsg::SearchChanged(e.text().to_string()));
                    },
                },
            },
            #[name = "status_label"]
            gtk::Label {
                add_css_class: "kalam-muted",
                set_halign: gtk::Align::Start,
                set_wrap: true,
            },
            #[name = "scroll"]
            gtk::ScrolledWindow {
                set_vexpand: true,
                set_hexpand: true,
                #[name = "list_box"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 10,
                    set_margin_top: 6,
                }
            },
        }
    }

    fn init(
        catalog: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = SavedWordsModel {
            catalog: catalog.clone(),
            query: String::new(),
            words: Vec::new(),
            status: String::new(),
        };
        let widgets = view_output!();
        let mut model = model;
        model.reload();
        rebuild(&widgets.list_box, &model.words, &sender);
        widgets.status_label.set_label(&model.status);
        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            SavedWordsMsg::SearchChanged(q) => {
                self.query = q;
                self.reload();
                rebuild(&widgets.list_box, &self.words, &sender);
                widgets.status_label.set_label(&self.status);
            }
            SavedWordsMsg::Delete(id) => {
                crate::notify::report(
                    self.catalog.delete_saved_word(id),
                    "Could not delete the word",
                );
                self.reload();
                rebuild(&widgets.list_box, &self.words, &sender);
                widgets.status_label.set_label(&self.status);
            }
            SavedWordsMsg::Refresh => {
                self.reload();
                rebuild(&widgets.list_box, &self.words, &sender);
                widgets.status_label.set_label(&self.status);
            }
        }
        self.update_view(widgets, sender);
    }
}

impl SavedWordsModel {
    fn reload(&mut self) {
        match self.catalog.list_saved_words(&self.query) {
            Ok(words) => {
                let n = words.len();
                self.words = words;
                if self.query.trim().is_empty() {
                    self.status = if n == 0 {
                        "No saved words yet — lookup a word in the reader (D or chip Aa) and save it.".into()
                    } else {
                        format!("{n} word{} saved", if n == 1 { "" } else { "s" })
                    };
                } else {
                    self.status = format!(
                        "{n} result{} for \"{}\"",
                        if n == 1 { "" } else { "s" },
                        self.query
                    );
                }
            }
            Err(e) => {
                self.words.clear();
                self.status = format!("DB error: {e}");
            }
        }
    }
}

fn rebuild(list: &gtk::Box, words: &[SavedWord], sender: &ComponentSender<SavedWordsModel>) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    if words.is_empty() {
        let l = gtk::Label::new(Some("No saved words."));
        l.add_css_class("kalam-placeholder");
        list.append(&l);
        return;
    }
    for w in words {
        let row = gtk::Box::new(gtk::Orientation::Vertical, 6);
        row.add_css_class("kalam-word-row");
        row.set_margin_bottom(8);

        let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let word_l = gtk::Label::new(Some(&w.word));
        word_l.add_css_class("kalam-word-title");
        word_l.set_halign(gtk::Align::Start);
        word_l.set_hexpand(true);
        header.append(&word_l);

        if let Some(dict) = &w.dict_name {
            let badge = gtk::Label::new(Some(dict.as_str()));
            badge.add_css_class("kalam-chip");
            header.append(&badge);
        }

        let del = gtk::Button::with_label("✕");
        del.add_css_class("kalam-secondary-btn");
        let id = w.id;
        let s = sender.clone();
        del.connect_clicked(move |_| s.input(SavedWordsMsg::Delete(id)));
        header.append(&del);

        row.append(&header);

        let def_l = gtk::Label::new(Some(&w.definition));
        def_l.add_css_class("kalam-muted");
        def_l.set_wrap(true);
        def_l.set_xalign(0.0);
        row.append(&def_l);

        if let Some(ctx) = &w.context_text {
            if !ctx.trim().is_empty() {
                let ctx_l = gtk::Label::new(Some(&format!("Context: {}", ctx)));
                ctx_l.add_css_class("kalam-placeholder");
                ctx_l.set_wrap(true);
                ctx_l.set_xalign(0.0);
                row.append(&ctx_l);
            }
        }

        let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
        sep.set_margin_top(6);
        row.append(&sep);

        list.append(&row);
    }
}
