//! P4 — Analytics: honest numbers derived from the catalog and session log.

use crate::db::{Catalog, LibraryStats};
use gtk::prelude::*;
use relm4::prelude::*;
use std::rc::Rc;

#[derive(Debug)]
pub enum AnalyticsMsg {
    Refresh,
}

pub struct AnalyticsModel {
    catalog: Rc<Catalog>,
    stats: LibraryStats,
}

#[relm4::component(pub)]
impl Component for AnalyticsModel {
    type Init = Rc<Catalog>;
    type Input = AnalyticsMsg;
    type Output = ();
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
                        set_label: "Analytics",
                        add_css_class: "kalam-page-title",
                        set_halign: gtk::Align::Start,
                    },
                    gtk::Label {
                        set_label: "Everything below is measured locally — nothing leaves your machine.",
                        add_css_class: "kalam-page-sub",
                        set_halign: gtk::Align::Start,
                    },
                },

                gtk::Button {
                    set_label: "↻",
                    add_css_class: "kalam-secondary-btn",
                    set_valign: gtk::Align::Center,
                    connect_clicked => AnalyticsMsg::Refresh,
                },
            },

            gtk::ScrolledWindow {
                set_vexpand: true,
                set_hexpand: true,
                set_hscrollbar_policy: gtk::PolicyType::Never,

                #[name = "body"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 18,
                },
            },
        }
    }

    fn init(
        catalog: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let stats = catalog.library_stats().unwrap_or_default();
        let model = AnalyticsModel { catalog, stats };
        let widgets = view_output!();
        rebuild(&widgets.body, &model.stats);
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
            AnalyticsMsg::Refresh => {
                self.stats = self.catalog.library_stats().unwrap_or_default();
            }
        }
        rebuild(&widgets.body, &self.stats);
        self.update_view(widgets, sender);
    }
}

fn rebuild(body: &gtk::Box, stats: &LibraryStats) {
    while let Some(child) = body.first_child() {
        body.remove(&child);
    }

    if stats.total_books == 0 {
        let empty = gtk::Label::new(Some(
            "Nothing to measure yet — import a book and start reading.",
        ));
        empty.add_css_class("kalam-placeholder");
        empty.set_halign(gtk::Align::Start);
        body.append(&empty);
        return;
    }

    // ── headline tiles ──────────────────────────────────────────────────
    body.append(&section_label("LIBRARY"));
    body.append(&tile_row(&[
        ("Books", stats.total_books.to_string()),
        ("Finished", stats.finished.to_string()),
        ("Reading", stats.reading.to_string()),
        ("Unread", stats.unread.to_string()),
    ]));

    body.append(&section_label("TIME READ"));
    body.append(&tile_row(&[
        ("All time", human_duration(stats.total_seconds)),
        ("Last 7 days", human_duration(stats.seconds_last_7)),
        ("Last 30 days", human_duration(stats.seconds_last_30)),
        ("Sessions", stats.sessions.to_string()),
    ]));

    body.append(&tile_row(&[
        (
            "Current streak",
            format!(
                "{} day{}",
                stats.current_streak_days,
                if stats.current_streak_days == 1 { "" } else { "s" }
            ),
        ),
        (
            "Longest streak",
            format!(
                "{} day{}",
                stats.longest_streak_days,
                if stats.longest_streak_days == 1 { "" } else { "s" }
            ),
        ),
        ("Finished (30d)", stats.finished_last_30.to_string()),
        ("Added (30d)", stats.added_last_30.to_string()),
    ]));

    body.append(&section_label("MARKUP"));
    body.append(&tile_row(&[
        ("Highlights", stats.highlights.to_string()),
        ("Quotes", stats.quotes.to_string()),
        ("Saved words", stats.saved_words.to_string()),
        ("Shelves", stats.shelves.to_string()),
    ]));

    // ── daily reading bars ──────────────────────────────────────────────
    if stats.minutes_by_day.iter().any(|(_, s)| *s > 0) {
        body.append(&section_label("LAST 14 DAYS"));
        body.append(&bar_chart(
            &stats
                .minutes_by_day
                .iter()
                .map(|(day, secs)| (short_day(day), *secs / 60))
                .collect::<Vec<_>>(),
            "min",
        ));
    }

    if !stats.added_by_month.is_empty() {
        body.append(&section_label("BOOKS ADDED PER MONTH"));
        body.append(&bar_chart(
            &stats
                .added_by_month
                .iter()
                .map(|(ym, n)| (short_month(ym), *n))
                .collect::<Vec<_>>(),
            "",
        ));
    }

    // ── leaderboards ────────────────────────────────────────────────────
    let columns = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    columns.set_homogeneous(true);

    if !stats.most_read.is_empty() {
        columns.append(&ranked_list(
            "MOST READ",
            &stats
                .most_read
                .iter()
                .map(|(title, secs)| (title.clone(), human_duration(*secs)))
                .collect::<Vec<_>>(),
        ));
    }
    if !stats.top_tags.is_empty() {
        columns.append(&ranked_list(
            "TOP TAGS",
            &stats
                .top_tags
                .iter()
                .map(|(name, n)| (name.clone(), n.to_string()))
                .collect::<Vec<_>>(),
        ));
    }
    if !stats.top_authors.is_empty() {
        columns.append(&ranked_list(
            "TOP AUTHORS",
            &stats
                .top_authors
                .iter()
                .map(|(name, n)| (name.clone(), n.to_string()))
                .collect::<Vec<_>>(),
        ));
    }
    if columns.first_child().is_some() {
        body.append(&columns);
    }
}

fn section_label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("kalam-section-label");
    label.set_halign(gtk::Align::Start);
    label
}

fn tile_row(items: &[(&str, String)]) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.set_homogeneous(true);
    for (label, value) in items {
        let tile = gtk::Box::new(gtk::Orientation::Vertical, 4);
        tile.add_css_class("kalam-stat-tile");

        let v = gtk::Label::new(Some(value));
        v.add_css_class("kalam-stat-value");
        v.set_halign(gtk::Align::Start);
        tile.append(&v);

        let l = gtk::Label::new(Some(label));
        l.add_css_class("kalam-stat-label");
        l.set_halign(gtk::Align::Start);
        tile.append(&l);

        row.append(&tile);
    }
    row
}

/// Simple CSS-drawn bar chart — no extra drawing dependency.
fn bar_chart(data: &[(String, i64)], unit: &str) -> gtk::Box {
    let chart = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    chart.add_css_class("kalam-bar-chart");
    chart.set_homogeneous(true);

    let max = data.iter().map(|(_, v)| *v).max().unwrap_or(1).max(1);

    for (label, value) in data {
        let col = gtk::Box::new(gtk::Orientation::Vertical, 4);
        col.set_valign(gtk::Align::End);

        let value_label = gtk::Label::new(Some(&if *value > 0 {
            format!("{value}{unit}")
        } else {
            String::new()
        }));
        value_label.add_css_class("kalam-bar-value");
        col.append(&value_label);

        // Height is proportional; 4px keeps empty days visible as a stub.
        let bar = gtk::Box::new(gtk::Orientation::Vertical, 0);
        bar.add_css_class("kalam-bar");
        if *value == 0 {
            bar.add_css_class("kalam-bar-empty");
        }
        let height = if *value == 0 {
            4
        } else {
            ((*value as f64 / max as f64) * 96.0).round().max(6.0) as i32
        };
        bar.set_size_request(-1, height);
        bar.set_valign(gtk::Align::End);
        col.append(&bar);

        let name = gtk::Label::new(Some(label));
        name.add_css_class("kalam-bar-label");
        col.append(&name);

        chart.append(&col);
    }
    chart
}

fn ranked_list(title: &str, rows: &[(String, String)]) -> gtk::Box {
    let col = gtk::Box::new(gtk::Orientation::Vertical, 6);
    col.append(&section_label(title));

    for (name, value) in rows {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.add_css_class("kalam-rank-row");

        let n = gtk::Label::new(Some(name));
        n.set_halign(gtk::Align::Start);
        n.set_hexpand(true);
        n.set_xalign(0.0);
        n.set_ellipsize(gtk::pango::EllipsizeMode::End);
        row.append(&n);

        let v = gtk::Label::new(Some(value));
        v.add_css_class("kalam-progress");
        row.append(&v);

        col.append(&row);
    }
    col
}

/// `4530` → `1h 15m`; small values stay in minutes so short sessions register.
fn human_duration(seconds: i64) -> String {
    if seconds <= 0 {
        return "—".into();
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{}m", minutes.max(1));
    }
    let hours = minutes / 60;
    let rem = minutes % 60;
    if rem == 0 {
        format!("{hours}h")
    } else {
        format!("{hours}h {rem}m")
    }
}

/// `2026-07-27` → `27`
fn short_day(day: &str) -> String {
    day.get(8..10).unwrap_or(day).to_string()
}

/// `2026-07` → `Jul`
fn short_month(ym: &str) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let m: usize = ym.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(1);
    MONTHS
        .get(m.saturating_sub(1))
        .copied()
        .unwrap_or("")
        .to_string()
}
