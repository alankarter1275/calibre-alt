//! P4 — create / edit a shelf.
//!
//! Manual shelves need only a name and description. Smart shelves add the rule
//! builder: a flat list of `field / operator / value` rows joined by one
//! All-or-Any switch, with a live "N books match" readout so you can see the
//! result before saving.
//!
//! Built as a plain `gtk::Window` (not a Relm4 component) because the rule rows
//! are added and removed dynamically, which is far simpler with direct widget
//! handling than with a static `view!` tree.

use crate::db::{Catalog, ShelfKind};
use crate::shelf_rules::{MatchMode, Rule, RuleField, RuleOp, RuleSet};
use gtk::prelude::*;
use relm4::RelmWidgetExt;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// Self-referential redraw hook: rule rows need to trigger a full rebuild of
/// the list they live in, so the closure is shared back into itself.
type RebuildHook = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

/// What the dialog is editing.
pub enum ShelfEditorMode {
    Create(ShelfKind),
    Edit { shelf_id: i64 },
}

/// Open the editor. `on_saved` fires after a successful write so the caller can
/// refresh its list.
pub fn open_shelf_editor(
    parent: Option<&gtk::Window>,
    catalog: Arc<Catalog>,
    mode: ShelfEditorMode,
    on_saved: impl Fn() + 'static,
) {
    let (shelf_id, kind, name0, desc0, rules0) = match mode {
        ShelfEditorMode::Create(kind) => {
            (None, kind, String::new(), String::new(), RuleSet::default())
        }
        ShelfEditorMode::Edit { shelf_id } => match catalog.get_shelf(shelf_id) {
            Ok(Some(shelf)) => (
                Some(shelf.id),
                shelf.kind,
                shelf.name.clone(),
                shelf.description.clone(),
                shelf.rule_set(),
            ),
            _ => return,
        },
    };

    let window = gtk::Window::builder()
        .title(match (&shelf_id, kind) {
            (None, ShelfKind::Smart) => "New smart shelf",
            (None, ShelfKind::Manual) => "New shelf",
            (Some(_), _) => "Edit shelf",
        })
        .modal(true)
        .default_width(if kind == ShelfKind::Smart { 620 } else { 460 })
        .default_height(if kind == ShelfKind::Smart { 560 } else { 260 })
        .build();
    window.add_css_class("kalam-window");
    if let Some(parent) = parent {
        window.set_transient_for(Some(parent));
    }

    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    root.set_margin_all(18);
    root.add_css_class("kalam-shelf-editor");

    // ── name ────────────────────────────────────────────────────────────
    let name_label = gtk::Label::new(Some("NAME"));
    name_label.add_css_class("kalam-detail-section-title");
    name_label.set_halign(gtk::Align::Start);
    root.append(&name_label);

    let name_entry = gtk::Entry::new();
    name_entry.set_placeholder_text(Some(match kind {
        ShelfKind::Smart => "Unread fantasy",
        ShelfKind::Manual => "Favourites",
    }));
    name_entry.set_text(&name0);
    root.append(&name_entry);

    // ── description ─────────────────────────────────────────────────────
    let desc_label = gtk::Label::new(Some("DESCRIPTION (OPTIONAL)"));
    desc_label.add_css_class("kalam-detail-section-title");
    desc_label.set_halign(gtk::Align::Start);
    root.append(&desc_label);

    let desc_entry = gtk::Entry::new();
    desc_entry.set_placeholder_text(Some("What lives on this shelf"));
    desc_entry.set_text(&desc0);
    root.append(&desc_entry);

    // Shared rule state for smart shelves.
    let rules = Rc::new(RefCell::new(rules0));
    let count_label = gtk::Label::new(None);
    count_label.add_css_class("kalam-rule-count");
    count_label.set_halign(gtk::Align::End);

    let rules_host = gtk::Box::new(gtk::Orientation::Vertical, 8);

    if kind == ShelfKind::Smart {
        let rules_label = gtk::Label::new(Some("RULES"));
        rules_label.add_css_class("kalam-detail-section-title");
        rules_label.set_halign(gtk::Align::Start);
        root.append(&rules_label);

        // Match All / Any switch.
        let match_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        match_row.append(&gtk::Label::new(Some("Match")));

        let match_combo = gtk::DropDown::from_strings(&["All", "Any"]);
        match_combo.set_selected(match rules.borrow().mode() {
            MatchMode::All => 0,
            MatchMode::Any => 1,
        });
        match_row.append(&match_combo);
        match_row.append(&gtk::Label::new(Some("of the following:")));
        root.append(&match_row);

        let scroll = gtk::ScrolledWindow::builder()
            .min_content_height(200)
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&rules_host)
            .build();
        root.append(&scroll);

        let add_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let add_btn = gtk::Button::with_label("+ Add rule");
        add_btn.add_css_class("kalam-secondary-btn");
        add_row.append(&add_btn);
        count_label.set_hexpand(true);
        add_row.append(&count_label);
        root.append(&add_row);

        // Rebuild is a closure stored in an Rc so rule rows can call it after
        // removing themselves.
        let rebuild: RebuildHook = Rc::new(RefCell::new(None));
        {
            let rules = rules.clone();
            let catalog = catalog.clone();
            let rules_host = rules_host.clone();
            let count_label = count_label.clone();
            let rebuild_ref = rebuild.clone();
            let f: Rc<dyn Fn()> = Rc::new(move || {
                rebuild_rule_rows(
                    &rules_host,
                    &rules,
                    &catalog,
                    &count_label,
                    rebuild_ref.borrow().clone(),
                );
            });
            *rebuild.borrow_mut() = Some(f);
        }
        let do_rebuild = rebuild.borrow().clone().expect("rebuild closure");
        do_rebuild();

        {
            let rules = rules.clone();
            let do_rebuild = do_rebuild.clone();
            match_combo.connect_selected_notify(move |combo| {
                let mode = if combo.selected() == 1 {
                    MatchMode::Any
                } else {
                    MatchMode::All
                };
                rules.borrow_mut().set_mode(mode);
                do_rebuild();
            });
        }

        {
            let rules = rules.clone();
            let do_rebuild = do_rebuild.clone();
            add_btn.connect_clicked(move |_| {
                rules
                    .borrow_mut()
                    .rules
                    .push(Rule::new(RuleField::Tag, RuleOp::Is, ""));
                do_rebuild();
            });
        }
    }

    // ── error line + actions ────────────────────────────────────────────
    let error_label = gtk::Label::new(None);
    error_label.add_css_class("kalam-error-text");
    error_label.set_halign(gtk::Align::Start);
    error_label.set_visible(false);
    error_label.set_wrap(true);
    root.append(&error_label);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    actions.set_halign(gtk::Align::End);
    actions.set_margin_top(6);

    let cancel = gtk::Button::with_label("Cancel");
    cancel.add_css_class("kalam-secondary-btn");
    let save = gtk::Button::with_label(if shelf_id.is_some() { "Save" } else { "Create" });
    save.add_css_class("kalam-primary-btn");
    actions.append(&cancel);
    actions.append(&save);
    root.append(&actions);

    window.set_child(Some(&root));

    {
        let window = window.clone();
        cancel.connect_clicked(move |_| window.close());
    }

    {
        let window = window.clone();
        let catalog = catalog.clone();
        let name_entry = name_entry.clone();
        let desc_entry = desc_entry.clone();
        let rules = rules.clone();
        let error_label = error_label.clone();
        let on_saved = Rc::new(on_saved);

        save.connect_clicked(move |_| {
            let name = name_entry.text().trim().to_string();
            if name.is_empty() {
                show_error(&error_label, "Give the shelf a name.");
                return;
            }
            if catalog.shelf_name_taken(&name, shelf_id).unwrap_or(false) {
                show_error(&error_label, "A shelf with that name already exists.");
                return;
            }

            let desc = desc_entry.text().to_string();
            let rules_json = if kind == ShelfKind::Smart {
                rules.borrow().to_json()
            } else {
                String::new()
            };

            let result = match shelf_id {
                Some(id) => catalog.update_shelf(id, &name, &desc, &rules_json),
                None => catalog
                    .create_shelf(&name, kind, &desc, &rules_json)
                    .map(|_| ()),
            };

            match result {
                Ok(()) => {
                    on_saved();
                    window.close();
                }
                Err(err) => show_error(&error_label, &format!("Could not save: {err}")),
            }
        });
    }

    // Esc closes.
    let key = gtk::EventControllerKey::new();
    {
        let window = window.clone();
        key.connect_key_pressed(move |_, keyval, _, _| {
            if keyval == gtk::gdk::Key::Escape {
                window.close();
                return gtk::glib::Propagation::Stop;
            }
            gtk::glib::Propagation::Proceed
        });
    }
    window.add_controller(key);

    window.present();
}

fn show_error(label: &gtk::Label, text: &str) {
    label.set_label(text);
    label.set_visible(true);
}

/// Redraw every rule row from the current rule set and refresh the live count.
fn rebuild_rule_rows(
    host: &gtk::Box,
    rules: &Rc<RefCell<RuleSet>>,
    catalog: &Arc<Catalog>,
    count_label: &gtk::Label,
    rebuild: Option<Rc<dyn Fn()>>,
) {
    while let Some(child) = host.first_child() {
        host.remove(&child);
    }

    let len = rules.borrow().rules.len();
    if len == 0 {
        let empty = gtk::Label::new(Some(
            "No rules yet — add one to describe what belongs on this shelf.",
        ));
        empty.add_css_class("kalam-muted");
        empty.set_halign(gtk::Align::Start);
        empty.set_wrap(true);
        host.append(&empty);
    }

    for idx in 0..len {
        let row = build_rule_row(idx, rules, catalog, count_label, rebuild.clone());
        host.append(&row);
    }

    update_count(count_label, rules, catalog);
}

fn build_rule_row(
    idx: usize,
    rules: &Rc<RefCell<RuleSet>>,
    catalog: &Arc<Catalog>,
    count_label: &gtk::Label,
    rebuild: Option<Rc<dyn Fn()>>,
) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    row.add_css_class("kalam-rule-row");

    let (field, op, value) = {
        let borrowed = rules.borrow();
        let rule = &borrowed.rules[idx];
        (
            rule.field_enum().unwrap_or(RuleField::Tag),
            rule.op_enum().unwrap_or(RuleOp::Is),
            rule.value.clone(),
        )
    };

    // ── field ───────────────────────────────────────────────────────────
    let field_labels: Vec<&str> = RuleField::ALL.iter().map(|f| f.label()).collect();
    let field_combo = gtk::DropDown::from_strings(&field_labels);
    let field_index = RuleField::ALL.iter().position(|f| *f == field).unwrap_or(0) as u32;
    field_combo.set_selected(field_index);
    row.append(&field_combo);

    // ── operator (choices depend on the field) ──────────────────────────
    let ops = field.operators();
    let op_labels: Vec<&str> = ops.iter().map(|o| o.label()).collect();
    let op_combo = gtk::DropDown::from_strings(&op_labels);
    // Fall back to the field's first operator when the stored one doesn't apply.
    let op_index = ops.iter().position(|o| *o == op).unwrap_or(0);
    op_combo.set_selected(op_index as u32);
    row.append(&op_combo);

    // ── value: fixed choices or free text ───────────────────────────────
    let value_widget: gtk::Widget = if let Some(choices) = field.value_choices() {
        let labels: Vec<&str> = choices.iter().map(|(_, label)| *label).collect();
        let combo = gtk::DropDown::from_strings(&labels);
        let active = choices
            .iter()
            .position(|(id, _)| id.eq_ignore_ascii_case(value.trim()))
            .unwrap_or(0);
        combo.set_selected(active as u32);
        // Persist the default immediately so a freshly added rule is valid.
        rules.borrow_mut().rules[idx].value = choices[active].0.to_string();

        {
            let rules = rules.clone();
            let catalog = catalog.clone();
            let count_label = count_label.clone();
            combo.connect_selected_notify(move |c| {
                let Some((id, _)) = choices.get(c.selected() as usize) else {
                    return;
                };
                rules.borrow_mut().rules[idx].value = id.to_string();
                update_count(&count_label, &rules, &catalog);
            });
        }
        combo.upcast()
    } else {
        let entry = gtk::Entry::new();
        entry.set_hexpand(true);
        entry.set_placeholder_text(Some(field.value_hint()));
        entry.set_text(&value);
        if matches!(field, RuleField::Added) {
            entry.set_input_purpose(gtk::InputPurpose::Digits);
        }
        {
            let rules = rules.clone();
            let catalog = catalog.clone();
            let count_label = count_label.clone();
            entry.connect_changed(move |e| {
                rules.borrow_mut().rules[idx].value = e.text().to_string();
                update_count(&count_label, &rules, &catalog);
            });
        }
        entry.upcast()
    };
    row.append(&value_widget);

    // ── remove ──────────────────────────────────────────────────────────
    let remove = gtk::Button::with_label("✕");
    remove.add_css_class("kalam-rule-remove");
    remove.set_tooltip_text(Some("Remove this rule"));
    {
        let rules = rules.clone();
        let rebuild = rebuild.clone();
        remove.connect_clicked(move |_| {
            rules.borrow_mut().rules.remove(idx);
            if let Some(f) = &rebuild {
                f();
            }
        });
    }
    row.append(&remove);

    // Changing the field changes which operators and value widget apply, so the
    // whole list is rebuilt.
    {
        let rules = rules.clone();
        let rebuild = rebuild.clone();
        field_combo.connect_selected_notify(move |c| {
            let Some(new_field) = RuleField::ALL.get(c.selected() as usize).copied() else {
                return;
            };
            {
                let mut borrowed = rules.borrow_mut();
                let rule = &mut borrowed.rules[idx];
                if rule.field == new_field.as_str() {
                    return;
                }
                rule.field = new_field.as_str().to_string();
                rule.op = new_field.operators()[0].as_str().to_string();
                rule.value = String::new();
            }
            if let Some(f) = &rebuild {
                f();
            }
        });
    }

    {
        let rules = rules.clone();
        let catalog = catalog.clone();
        let count_label = count_label.clone();
        op_combo.connect_selected_notify(move |c| {
            let Some(new_op) = ops.get(c.selected() as usize) else {
                return;
            };
            rules.borrow_mut().rules[idx].op = new_op.as_str().to_string();
            update_count(&count_label, &rules, &catalog);
        });
    }

    row
}

fn update_count(label: &gtk::Label, rules: &Rc<RefCell<RuleSet>>, catalog: &Arc<Catalog>) {
    let set = rules.borrow();
    if set.is_empty() {
        label.set_label("No usable rules — matches 0 books");
        return;
    }
    match catalog.count_matching_rules(&set) {
        Ok(n) => label.set_label(&format!("{n} book{} match", if n == 1 { "" } else { "s" })),
        Err(err) => label.set_label(&format!("Rule error: {err}")),
    }
}
