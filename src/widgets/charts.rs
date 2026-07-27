//! Shared cairo-drawn charts.
//!
//! Kept out of any one page so the Library dashboard and the Analytics page
//! render identical visuals from a single implementation. Colours are read
//! back from CSS via `widget.color()`, so palette decisions stay in style.rs.

use crate::db::LibraryStats;
use gtk::prelude::*;

/// Books-added series, padded so a sparkline always has two points to draw.
pub fn monthly_series(stats: &LibraryStats) -> Vec<i64> {
    let v: Vec<i64> = stats.added_by_month.iter().map(|(_, n)| *n).collect();
    if v.len() < 2 {
        vec![0, v.first().copied().unwrap_or(0)]
    } else {
        v
    }
}

pub fn sparkline(series: &[i64], css_class: &str) -> gtk::DrawingArea {
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
pub fn line_chart(series: &[i64], labels: &[String]) -> gtk::Box {
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
