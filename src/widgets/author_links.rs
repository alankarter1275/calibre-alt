use gtk::prelude::*;
use std::rc::Rc;

pub fn replace_author_links(
    host: &gtk::Box,
    authors: &str,
    link_class: &str,
    on_click: Rc<dyn Fn(String)>,
) {
    while let Some(child) = host.first_child() {
        host.remove(&child);
    }

    let names = crate::author::split_author_names(authors);
    if names.is_empty() {
        let label = gtk::Label::new(Some("Unknown"));
        label.add_css_class("kalam-muted");
        label.set_halign(gtk::Align::Start);
        host.append(&label);
        return;
    }

    let flow = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .column_spacing(6)
        .row_spacing(4)
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Start)
        .build();
    flow.add_css_class("kalam-author-links");

    for (idx, name) in names.iter().enumerate() {
        let btn = gtk::Button::with_label(name);
        btn.add_css_class("flat");
        btn.add_css_class("kalam-author-link");
        if !link_class.trim().is_empty() {
            btn.add_css_class(link_class);
        }
        btn.set_focus_on_click(false);
        btn.set_halign(gtk::Align::Start);
        btn.set_valign(gtk::Align::Center);
        btn.set_size_request(-1, 22);
        btn.set_tooltip_text(Some("Open author page"));

        let name = name.clone();
        let on_click = on_click.clone();
        btn.connect_clicked(move |_| on_click(name.clone()));
        flow.insert(&btn, -1);

        if idx + 1 != names.len() {
            let sep = gtk::Label::new(Some("·"));
            sep.add_css_class("kalam-author-sep");
            flow.insert(&sep, -1);
        }
    }

    host.append(&flow);
}
