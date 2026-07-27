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
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let stats = catalog.library_stats().unwrap_or_default();
        let model = AnalyticsModel { catalog, stats };
        let widgets = view_output!();
        rebuild(&widgets.body, &model.stats);
        let _ = sender;
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

    // ── hero row: three headline cards, each with its own sparkline ─────
    let daily: Vec<i64> = stats.minutes_by_day.iter().map(|(_, s)| *s / 60).collect();

    let hero = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    hero.set_homogeneous(true);

    hero.append(&hero_card(
        "Avg Time /Day",
        &stats.avg_minutes_per_active_day.to_string(),
        "Min/Day",
        &daily,
        "kalam-spark-green",
        &avg_blurb(stats.avg_minutes_per_active_day),
    ));

    hero.append(&hero_card(
        "Books Completed",
        &stats.finished.to_string(),
        "Books",
        &monthly_series(stats),
        "kalam-spark-red",
        &finished_blurb(stats.finished, stats.finished_last_30),
    ));

    hero.append(&hero_card(
        "Current Streak",
        &stats.current_streak_days.to_string(),
        if stats.current_streak_days == 1 {
            "Day"
        } else {
            "Days"
        },
        &daily,
        "kalam-spark-blue",
        &streak_blurb(stats.current_streak_days, stats.longest_streak_days),
    ));
    body.append(&hero);

    // ── reading activity line chart ─────────────────────────────────────
    if daily.iter().any(|v| *v > 0) {
        let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
        card.add_css_class("kalam-chart-card");

        let head = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let title = gtk::Label::new(Some(&format!(
            "{} min",
            daily.iter().sum::<i64>()
        )));
        title.add_css_class("kalam-chart-title");
        title.set_halign(gtk::Align::Start);
        head.append(&title);

        let sub = gtk::Label::new(Some("Read in the last 14 days"));
        sub.add_css_class("kalam-chart-sub");
        sub.set_halign(gtk::Align::Start);
        sub.set_hexpand(true);
        sub.set_valign(gtk::Align::End);
        head.append(&sub);
        card.append(&head);

        let labels: Vec<String> = stats
            .minutes_by_day
            .iter()
            .map(|(d, _)| short_day(d))
            .collect();
        card.append(&line_chart(&daily, &labels));
        body.append(&card);
    }

    // ── library breakdown ───────────────────────────────────────────────
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

    body.append(&section_label("MARKUP"));
    body.append(&tile_row(&[
        ("Highlights", stats.highlights.to_string()),
        ("Quotes", stats.quotes.to_string()),
        ("Saved words", stats.saved_words.to_string()),
        ("Shelves", stats.shelves.to_string()),
    ]));

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

/// Books-added series, padded so the sparkline always has something to draw.
fn monthly_series(stats: &LibraryStats) -> Vec<i64> {
    let v: Vec<i64> = stats.added_by_month.iter().map(|(_, n)| *n).collect();
    if v.len() < 2 {
        vec![0, v.first().copied().unwrap_or(0)]
    } else {
        v
    }
}

fn avg_blurb(minutes: i64) -> String {
    match minutes {
        0 => "No sessions logged yet — open a book to start tracking.".into(),
        1..=14 => "Short and steady. Every page counts.".into(),
        15..=44 => "A solid daily habit. Keep the momentum!".into(),
        _ => "Deep focus sessions — impressive stamina.".into(),
    }
}

fn finished_blurb(total: i64, last_30: i64) -> String {
    if total == 0 {
        "Finish your first book to start the count.".into()
    } else if last_30 > 0 {
        format!("{last_30} finished in the last 30 days. Great progress!")
    } else {
        "Nothing finished this month — pick one up again?".into()
    }
}

fn streak_blurb(current: i64, longest: i64) -> String {
    if current == 0 {
        "Read today to start a new streak.".into()
    } else if current >= longest && current > 1 {
        "This is your longest streak yet!".into()
    } else {
        format!("Longest so far: {longest} days. Keep going!")
    }
}

/// Headline card: label, big number + unit, a sparkline, and a line of copy.
fn hero_card(
    label: &str,
    value: &str,
    unit: &str,
    series: &[i64],
    spark_class: &str,
    blurb: &str,
) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    card.add_css_class("kalam-hero-card");

    let l = gtk::Label::new(Some(label));
    l.add_css_class("kalam-hero-label");
    l.set_halign(gtk::Align::Start);
    card.append(&l);

    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row.set_valign(gtk::Align::Center);

    let v = gtk::Label::new(Some(value));
    v.add_css_class("kalam-hero-value");
    v.set_valign(gtk::Align::Baseline);
    row.append(&v);

    let u = gtk::Label::new(Some(unit));
    u.add_css_class("kalam-hero-unit");
    u.set_valign(gtk::Align::Baseline);
    u.set_hexpand(true);
    u.set_halign(gtk::Align::Start);
    row.append(&u);

    row.append(&sparkline(series, spark_class));
    card.append(&row);

    let b = gtk::Label::new(Some(blurb));
    b.add_css_class("kalam-hero-blurb");
    b.set_halign(gtk::Align::Start);
    b.set_xalign(0.0);
    b.set_wrap(true);
    b.set_lines(2);
    b.set_ellipsize(gtk::pango::EllipsizeMode::End);
    card.append(&b);

    card
}

/// Tiny inline trend, drawn with GtkDrawingArea (smooth, unlike stacked boxes).
fn sparkline(series: &[i64], css_class: &str) -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.add_css_class("kalam-sparkline");
    area.add_css_class(css_class);
    area.set_content_width(84);
    area.set_content_height(30);
    area.set_valign(gtk::Align::Center);

    let data: Vec<f64> = series.iter().map(|v| *v as f64).collect();
    area.set_draw_func(move |area, cr, w, h| {
        if data.len() < 2 {
            return;
        }
        // Colour comes from CSS so themes stay in one place.
        let colour = area.color();
        let max = data.iter().cloned().fold(0.0_f64, f64::max).max(1.0);
        let step = (w as f64) / ((data.len() - 1) as f64);
        let pad = 3.0;
        let usable = (h as f64) - pad * 2.0;

        let point = |i: usize, v: f64| -> (f64, f64) {
            let x = (i as f64) * step;
            let y = pad + usable - (v / max) * usable;
            (x, y)
        };

        cr.set_line_width(1.8);
        cr.set_source_rgba(
            colour.red() as f64,
            colour.green() as f64,
            colour.blue() as f64,
            1.0,
        );
        let (x0, y0) = point(0, data[0]);
        cr.move_to(x0, y0);
        for (i, v) in data.iter().enumerate().skip(1) {
            let (x, y) = point(i, *v);
            cr.line_to(x, y);
        }
        let _ = cr.stroke();
    });
    area
}

/// Full-width line chart with a soft fill under the curve.
fn line_chart(series: &[i64], labels: &[String]) -> gtk::Box {
    let wrap = gtk::Box::new(gtk::Orientation::Vertical, 4);

    let area = gtk::DrawingArea::new();
    area.add_css_class("kalam-linechart");
    area.set_content_height(150);
    area.set_hexpand(true);

    let data: Vec<f64> = series.iter().map(|v| *v as f64).collect();
    area.set_draw_func(move |area, cr, w, h| {
        if data.len() < 2 {
            return;
        }
        let colour = area.color();
        let (r, g, b) = (
            colour.red() as f64,
            colour.green() as f64,
            colour.blue() as f64,
        );
        let max = data.iter().cloned().fold(0.0_f64, f64::max).max(1.0);
        let pad_x = 6.0;
        let pad_y = 10.0;
        let usable_w = (w as f64) - pad_x * 2.0;
        let usable_h = (h as f64) - pad_y * 2.0;
        let step = usable_w / ((data.len() - 1) as f64);

        let point = |i: usize, v: f64| -> (f64, f64) {
            (
                pad_x + (i as f64) * step,
                pad_y + usable_h - (v / max) * usable_h,
            )
        };

        // Faint horizontal guides.
        cr.set_line_width(1.0);
        cr.set_source_rgba(r, g, b, 0.10);
        for k in 0..=3 {
            let y = pad_y + usable_h * (k as f64 / 3.0);
            cr.move_to(pad_x, y);
            cr.line_to(pad_x + usable_w, y);
        }
        let _ = cr.stroke();

        // Fill under the curve.
        let (x0, y0) = point(0, data[0]);
        cr.move_to(x0, pad_y + usable_h);
        cr.line_to(x0, y0);
        for (i, v) in data.iter().enumerate().skip(1) {
            let (x, y) = point(i, *v);
            cr.line_to(x, y);
        }
        cr.line_to(pad_x + usable_w, pad_y + usable_h);
        cr.close_path();
        cr.set_source_rgba(r, g, b, 0.16);
        let _ = cr.fill();

        // The curve itself.
        cr.set_line_width(2.2);
        cr.set_source_rgba(r, g, b, 1.0);
        cr.move_to(x0, y0);
        for (i, v) in data.iter().enumerate().skip(1) {
            let (x, y) = point(i, *v);
            cr.line_to(x, y);
        }
        let _ = cr.stroke();

        // Emphasise the most recent point.
        if let Some(last) = data.last() {
            let (x, y) = point(data.len() - 1, *last);
            cr.arc(x, y, 3.5, 0.0, std::f64::consts::PI * 2.0);
            let _ = cr.fill();
        }
    });
    wrap.append(&area);

    // Sparse x-axis labels — every third day keeps them readable.
    let axis = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    axis.set_homogeneous(true);
    for (i, label) in labels.iter().enumerate() {
        let text = if i % 3 == 0 || i == labels.len() - 1 {
            label.as_str()
        } else {
            ""
        };
        let l = gtk::Label::new(Some(text));
        l.add_css_class("kalam-bar-label");
        axis.append(&l);
    }
    wrap.append(&axis);
    wrap
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
