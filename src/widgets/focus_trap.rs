//! Keep keyboard focus inside an in-app dialog.
//!
//! # Why this exists
//!
//! A `gtk::Window` is a focus scope: Tab cycles through the widgets in that
//! window and stops at its edge. Kalam's dialogs are not windows — they are
//! panels drawn into a `gtk::Overlay` over the page (see `in_app_dialog.rs`
//! and the float layer in `app.rs`, and `docs/pitfalls.md` §2 for the other
//! things a window used to do for free).
//!
//! An overlay is not a focus scope. The page underneath is still in the same
//! widget tree, still focusable, and Tab walks straight out of the "modal"
//! dialog into the sidebar and the page behind it. The dim scrim blocks the
//! mouse — `can_target` — but nothing was blocking the keyboard. You could Tab
//! to a button you cannot see, press Enter, and act on the page behind a modal
//! dialog.
//!
//! # How it works
//!
//! One capture-phase key controller on the window root. While the panel is
//! visible it owns Tab and Shift+Tab: it advances focus *within the panel* and
//! wraps around at the ends, so focus can never leave. Everything else is
//! passed straight through, including Esc, which the dialog handles itself.
//!
//! Capture phase matters. In the bubble phase the focused widget sees the key
//! first, and GTK's own focus movement would already have run — the same trap
//! that made a dialog's Esc handler lose to a focused `gtk::Entry`.

use gtk::prelude::*;

/// Which way a keypress moves focus, or `None` if it is not a Tab at all.
///
/// Split out from the controller so it can be tested without a display: CI has
/// no X or Wayland session, so anything touching real widgets cannot run there.
fn tab_direction(
    keyval: gtk::gdk::Key,
    state: gtk::gdk::ModifierType,
) -> Option<gtk::DirectionType> {
    use gtk::gdk::Key;

    // `ISO_Left_Tab` is what most toolkits actually receive for Shift+Tab, but
    // check the modifier too: some input methods and remapped keyboards send a
    // plain Tab with Shift held instead.
    let backward = match keyval {
        Key::Tab => state.contains(gtk::gdk::ModifierType::SHIFT_MASK),
        Key::ISO_Left_Tab => true,
        _ => return None,
    };

    Some(if backward {
        gtk::DirectionType::TabBackward
    } else {
        gtk::DirectionType::TabForward
    })
}

/// A key controller that confines Tab-focus to `panel`.
///
/// Add it to the window root. It is inert whenever `panel` is not visible, so
/// it is safe to attach permanently to a float layer that shows and hides the
/// same host — which is exactly how `app.rs` uses it. `in_app_dialog.rs`
/// instead adds it when the dialog opens and removes it on teardown.
///
/// Attach to the **root**, not to the panel: when a dialog first opens, focus
/// is usually still on the page widget that opened it, so a controller on the
/// panel would never see the keypress that walks away from it.
pub fn controller(panel: &impl IsA<gtk::Widget>) -> gtk::EventControllerKey {
    let controller = gtk::EventControllerKey::new();
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);

    let panel: gtk::Widget = panel.as_ref().clone();
    controller.connect_key_pressed(move |_, keyval, _, state| {
        // Not showing: this is the permanent-controller case in `app.rs`.
        // Leave every key alone so normal Tab navigation still works.
        if !panel.is_visible() {
            return gtk::glib::Propagation::Proceed;
        }

        let Some(direction) = tab_direction(keyval, state) else {
            return gtk::glib::Propagation::Proceed;
        };

        // `child_focus` returns false when it runs off the end of the panel —
        // which is precisely the moment focus was about to escape. Clear the
        // window's focus and search again: with no focus inside it, the panel
        // starts from its first (or last) focusable child, which is the wrap.
        if !panel.child_focus(direction) {
            if let Some(root) = panel.root() {
                // Named trait, no downcast. Several traits in scope expose a
                // `set_focus`/`focus`, and a bare call on a widget has been
                // ambiguous here before (E0034) — see `docs/pitfalls.md` §7.
                gtk::prelude::RootExt::set_focus(&root, None::<&gtk::Widget>);
            }
            panel.child_focus(direction);
        }

        // Always stop: while a dialog is open, Tab belongs to the dialog. If
        // this proceeded, GTK's own focus handling would run afterwards and
        // undo the containment.
        gtk::glib::Propagation::Stop
    });

    controller
}

#[cfg(test)]
mod tests {
    use super::*;
    use gtk::gdk::{Key, ModifierType};

    #[test]
    fn plain_tab_moves_forward() {
        assert_eq!(
            tab_direction(Key::Tab, ModifierType::empty()),
            Some(gtk::DirectionType::TabForward)
        );
    }

    #[test]
    fn shift_tab_moves_backward_both_ways_it_arrives() {
        // The usual keysym for Shift+Tab.
        assert_eq!(
            tab_direction(Key::ISO_Left_Tab, ModifierType::empty()),
            Some(gtk::DirectionType::TabBackward)
        );
        // ...and the fallback: a plain Tab with Shift held.
        assert_eq!(
            tab_direction(Key::Tab, ModifierType::SHIFT_MASK),
            Some(gtk::DirectionType::TabBackward)
        );
    }

    #[test]
    fn other_keys_are_left_alone() {
        // Esc in particular: the dialog's own handler must still see it. If
        // the trap swallowed it, a dialog would become impossible to close by
        // keyboard.
        assert_eq!(tab_direction(Key::Escape, ModifierType::empty()), None);
        assert_eq!(tab_direction(Key::q, ModifierType::empty()), None);
        assert_eq!(tab_direction(Key::Return, ModifierType::empty()), None);
    }
}
