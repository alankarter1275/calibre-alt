//! P4 — History: the append-only reading event log.

use crate::db::{Catalog, EventKind, ReadingEvent};
use gtk::prelude::*;
use relm4::prelude::*;
use std::rc::Rc;

const PAGE_LIMIT: usize = 300;

#[derive(Debug)]
pub enum HistoryOut {
    OpenBook { book_id: i64 },
}

#[derive(Debug)]
pub enum HistoryMsg {
    SearchChanged(String),
    FilterChanged(Option<EventKind>),
    Clear,
    Refresh,
}

pub struct HistoryModel {
    catalog: Rc<Catalog>,
    events: Vec<ReadingEvent>,
    query: String,
    filter: Option<EventKind>,
}

#[relm4::component(pub)]
impl Component for HistoryModel {
    type Init = Rc<Catalog>;
    type Input = HistoryMsg;
    type Output = HistoryOut;
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

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,

                    gtk::Label {
                        set_label: "History",
                        add_css_class: "kalam-page-title",
                        set_halign: gtk::Align::Start,
                    },
                    gtk::Label {
                        #[watch]
                        set_label: &status_line(model.events.len()),
                        add_css_class: "kalam-page-sub",
                        set_halign: gtk::Align::Start,
                    },
                },

                gtk::Button {
                    set_label: "↻",
                    add_css_class: "kalam-secondary-btn",
                    set_valign: gtk::Align::Center,
                    set_tooltip_text: Some("Reload history"),
                    connect_clicked => HistoryMsg::Refresh,
                },
                gtk::Button {
                    set_label: "Clear history",
                    add_css_class: "kalam-secondary-btn",
                    set_valign: gtk::Align::Center,
                    connect_clicked => HistoryMsg::Clear,
                },
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 8,

                gtk::SearchEntry {
                    set_hexpand: true,
                    set_placeholder_text: Some("Search by title or author…"),
                    connect_search_changed[sender] => move |e| {
                        sender.input(HistoryMsg::SearchChanged(e.text().to_string()));
                    },
                },

                #[name = "filter_box"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 4,
                },
            },

            gtk::ScrolledWindow {
                set_vexpand: true,
                set_hexpand: true,
                set_hscrollbar_policy: gtk::PolicyType::Never,

                #[name = "list"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 6,
                },
            },
        }
    }

    fn init(
        catalog: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let events = catalog
            .list_events(None, "", PAGE_LIMIT)
            .unwrap_or_default();
        let model = HistoryModel {
            catalog,
            events,
            query: String::new(),
            filter: None,
        };
        let widgets = view_output!();

        let filters: [(&str, Option<EventKind>); 4] = [
            ("All", None),
            ("Opened", Some(EventKind::Opened)),
            ("Finished", Some(EventKind::Finished)),
            ("Imported", Some(EventKind::Imported)),
        ];
        for (label, kind) in filters {
            let btn = gtk::ToggleButton::with_label(label);
            btn.add_css_class("kalam-secondary-btn");
            if kind.is_none() {
                btn.set_active(true);
            }
            let s = sender.clone();
            btn.connect_toggled(move |b| {
                if b.is_active() {
                    s.input(HistoryMsg::FilterChanged(kind));
                }
            });
            widgets.filter_box.append(&btn);
        }
        group_toggles(&widgets.filter_box);

        rebuild(&widgets.list, &model.events, &sender);
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
            HistoryMsg::SearchChanged(q) => {
                self.query = q;
                self.reload();
            }
            HistoryMsg::FilterChanged(kind) => {
                self.filter = kind;
                self.reload();
            }
            HistoryMsg::Clear => {
                let _ = self.catalog.clear_history();
                self.reload();
            }
            HistoryMsg::Refresh => self.reload(),
        }
        rebuild(&widgets.list, &self.events, &sender);
        self.update_view(widgets, sender);
    }
}

impl HistoryModel {
    fn reload(&mut self) {
        self.events = self
            .catalog
            .list_events(self.filter, &self.query, PAGE_LIMIT)
            .unwrap_or_default();
    }
}

fn status_line(n: usize) -> String {
    if n == 0 {
        "Opened and finished books show up here as you read.".into()
    } else {
        format!("{n} event{}, newest first", if n == 1 { "" } else { "s" })
    }
}

fn group_toggles(box_: &gtk::Box) {
    let mut leader: Option<gtk::ToggleButton> = None;
    let mut child = box_.first_child();
    while let Some(w) = child {
        let next = w.next_sibling();
        if let Ok(btn) = w.downcast::<gtk::ToggleButton>() {
            match &leader {
                Some(l) => btn.set_group(Some(l)),
                None => leader = Some(btn),
            }
        }
        child = next;
    }
}

fn rebuild(list: &gtk::Box, events: &[ReadingEvent], sender: &ComponentSender<HistoryModel>) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    if events.is_empty() {
        let empty = gtk::Label::new(Some(
            "Nothing here yet — open a book and it will be recorded.",
        ));
        empty.add_css_class("kalam-placeholder");
        empty.set_wrap(true);
        empty.set_halign(gtk::Align::Start);
        list.append(&empty);
        return;
    }

    // Group by calendar day so the log reads like a diary.
    let mut current_day = String::new();
    for event in events {
        let day = event.at.get(..10).unwrap_or("").to_string();
        if day != current_day {
            let header = gtk::Label::new(Some(&pretty_day(&day)));
            header.add_css_class("kalam-section-label");
            header.set_halign(gtk::Align::Start);
            header.set_margin_top(if current_day.is_empty() { 0 } else { 10 });
            list.append(&header);
            current_day = day;
        }
        list.append(&build_row(event, sender));
    }
}

fn build_row(event: &ReadingEvent, sender: &ComponentSender<HistoryModel>) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.add_css_class("kalam-list-row");

    let icon = gtk::Label::new(Some(event.kind.icon()));
    icon.add_css_class("kalam-event-icon");
    icon.add_css_class(match event.kind {
        EventKind::Finished => "kalam-event-finished",
        EventKind::Imported => "kalam-event-imported",
        _ => "kalam-event-opened",
    });
    icon.set_valign(gtk::Align::Center);
    icon.set_width_chars(2);
    row.append(&icon);

    let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text.set_hexpand(true);

    let title = gtk::Label::new(Some(&event.book_title));
    title.add_css_class("kalam-card-title");
    title.set_halign(gtk::Align::Start);
    title.set_xalign(0.0);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    text.append(&title);

    let detail = if event.detail == "auto" && event.kind == EventKind::Finished {
        format!(
            "{} automatically · {}",
            event.kind.label(),
            event.book_authors
        )
    } else {
        format!("{} · {}", event.kind.label(), event.book_authors)
    };
    let meta = gtk::Label::new(Some(&detail));
    meta.add_css_class("kalam-card-meta");
    meta.set_halign(gtk::Align::Start);
    meta.set_xalign(0.0);
    meta.set_ellipsize(gtk::pango::EllipsizeMode::End);
    text.append(&meta);
    row.append(&text);

    let time = gtk::Label::new(Some(event.at.get(11..16).unwrap_or("")));
    time.add_css_class("kalam-muted");
    time.set_valign(gtk::Align::Center);
    row.append(&time);

    let book_id = event.book_id;
    let click = gtk::GestureClick::new();
    click.set_button(1);
    let s = sender.clone();
    click.connect_released(move |_, _, _, _| {
        s.output(HistoryOut::OpenBook { book_id }).ok();
    });
    row.add_controller(click);
    row.set_cursor_from_name(Some("pointer"));

    row
}

/// `2026-07-27` → `27 Jul 2026`; today and yesterday get friendly names.
fn pretty_day(day: &str) -> String {
    if day.len() < 10 {
        return day.to_string();
    }
    let today = crate::db::iso_days_ago(0);
    let yesterday = crate::db::iso_days_ago(1);
    if today.starts_with(day) {
        return "Today".into();
    }
    if yesterday.starts_with(day) {
        return "Yesterday".into();
    }

    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let month: usize = day[5..7].parse().unwrap_or(1);
    let month = MONTHS.get(month.saturating_sub(1)).copied().unwrap_or("");
    format!("{} {} {}", &day[8..10], month, &day[0..4])
}
