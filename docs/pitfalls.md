# Pitfalls — mistakes already made in this repo, and how they were fixed

**Read this before writing code.** Every entry below is a mistake that was
actually made here, caught, and fixed. They are written down so the next agent
does not spend a CI cycle — or ship a bug — rediscovering them.

Format: **what went wrong** → **why** → **what to do instead**.

Related reading: `ROADMAP.md` ("Read this first"), `docs/conversation.md`
(settled design decisions), `docs/ci/README.md` (how CI is installed).

---

## 1. Error paths that have never once executed

**Both real bugs found during the A0 step-2 pass had the same shape: an error
branch nobody had ever run.**

### 1a. A fallback that was a guaranteed panic

`AppModel::init` "handled" a failed `Catalog::open()` like this:

```rust
Err(err) => {
    notify::error("Could not open the library", …);
    Arc::new(Catalog::open().expect("catalog open"))   // ← retries the call that just failed
}
```

It retried the call that had *just* failed and `.expect()`ed the result. On a
corrupt database that is a certain panic — and because `init()` runs inside a
GTK signal callback, the panic **cannot unwind**, so it escalated to
`panic in a function that cannot unwind` → `Aborted (core dumped)` with a raw
backtrace instead of a message.

**Do instead:** an error branch must do something *different* from the thing
that failed. `main()` now opens the catalog **before** `app.run()` and, on
failure, prints the error, the database path and the exact `mv` command to move
the broken file aside, then exits 1.

### 1b. `SELECT MIN(x)` over zero rows returns NULL, not zero rows

`book_first_opened` read `SELECT MIN(at) …` into a `String`. Over zero rows
that query returns **one row containing NULL**, so `.optional()` does not help
— the *value* must be nullable. "Never opened" was therefore a hard error.

Nobody noticed because every caller wrote `.ok().flatten()`, which turned the
error into `None` and produced the correct screen **by accident**.

**Do instead:** read aggregates into `Option<T>`, or wrap them in `IFNULL`. And
treat a swallowed error as a place a bug can hide, not merely as untidy code.

---

## 2. GTK: things a `gtk::Window` was doing for you

When the A1 track moved dialogs from `gtk::Window` into an in-app overlay,
three things silently stopped happening. All three had to be replaced by hand.

| A window gave you | Why it matters | Replacement |
|---|---|---|
| **Teardown** — `close()` destroys the widget tree | A button's callback holds the dialog; the dialog holds the button. Destroying the tree breaks that loop. Removing an overlay child does **not**. | `teardown()` also empties the host, so a closed dialog is actually freed instead of leaking on every open |
| **A height bound** — a window has a default height | A panel centred in an overlay is sized by its content, so a long description or twenty rule rows push the action bar off a 768px screen | `max_content_height` + `propagate_natural_height` on the scroller |
| **A real top-level** for portal dialogs | `gtk::FileDialog` is portal-backed and needs a genuine window as parent | Resolve it from the anchor's root, not from the dialog |

## 3. `set_size_request` is a FLOOR, not a size

This one caused a user-visible bug that took four separate fixes.

The book float had `set_size_request(720, 420)`, and the panel still changed
size from book to book. GTK grows a widget past its size request whenever the
content inside needs more room. So **any unbounded child can resize the
panel**. In that one float there were four:

1. Tags in a `gtk::FlowBox` (`max_children_per_line: 8`, up to 12 chips) —
   9+ tags wrapped to a second row and made the panel taller.
2. Title and series labels with `set_wrap: true` — a long title took 2–3 lines.
3. Authors, filled by the shared `replace_author_links`, which builds a
   wrapping FlowBox.
4. The description section's "no Read more" branch left the section entirely
   unbounded — `height_request(-1)`, natural height, no max — so a short blurb
   gave a short panel and a nearly-long-enough one gave a tall panel.

**Do instead:** if a panel must be a fixed size, every variable-length child
needs an explicit bound — ellipsise, clip, or scroll. Do not assume the size
request is doing it. And when a shared helper (like `replace_author_links`)
behaves correctly elsewhere, constrain it **at the call site**, not in the
helper.

## 3b. A view property and a fill function are two writers

The book float's "Read more" button made the description **shrink to one
line**. The view declared `set_vexpand: true` on the description scroller;
`fill()` — which runs on every update, not just the first — called
`set_vexpand(false)` on the same widget. The fill function always wins, because
it runs last.

The bug hid for a while because the collapsed state used
`PolicyType::Never`, and `Never` forces a scroller to show its content at full
height (see 4b). That masked the missing `vexpand`. Expanding switched the
policy to `Automatic`, which released the scroller back down to its own
minimum — one line.

**Do instead:** decide where each property lives and keep it there. If a value
is static, set it in the view and never touch it in the update path. If it
varies, set it *only* in the update path so there is one writer. When a widget
misbehaves, grep its name across the whole file before assuming the view is the
source of truth.

Corollary: a control that changes a panel's shape does not belong in a panel
whose defining property is a fixed shape. The toggle was deleted rather than
repaired — scrolling already solved the problem it existed for.

## 4. Hiding a widget removes its space

Fixing (3) introduced a new bug: `fill()` hid the tag row when a book had no
tags, and a hidden widget occupies no space — so the action buttons below it
shifted up on untagged books.

**Do instead:** when a widget's job is to reserve space, show it
unconditionally. An empty row is invisible anyway.

## 4b. Reserving space is not the same as anchoring

Fixing (4) by keeping the tag row permanently visible **did not work** — the
user reported the buttons still moving. Two reasons, and the second is the
general lesson.

**`ScrolledWindow` + `PolicyType::Never` ignores your height.** `Never` is a
promise to GTK that the content is fully visible in that direction, so GTK
propagates the child's *whole* minimum height and `min_content_height` /
`max_content_height` cannot shrink it. A chip taller than the requested row
height still grew the row. Use `External` when you want a hard height *and*
scrolling; `Never` is only safe when the content genuinely cannot overflow.

**Reserving space for one child only fixes that child.** Nothing in the float
body had `vexpand: true`, so all leftover height pooled *below* the action row
and its position tracked whatever happened to sit above it. Every variable
child would have needed its own bound, forever.

**Do instead: anchor the thing that must not move.** One `vexpand: true` spacer,
plus `valign: End` on the rows below it, makes the slack collect *above* them.
In a fixed-height panel that pins them absolutely, regardless of what changes
higher up.

**Where you put the spacer decides what gets separated.** Placing it directly
before the action row anchored the buttons but shoved the tag row up with the
slack, leaving a gap between two things that belong together. If several
trailing rows should stay as a group, the spacer goes **above the whole group**,
not between its members.

## 4c. Ellipsising a label does not stop it widening its parent

The book float kept growing sideways for long titles even though the title
label had `EllipsizeMode::End` set. Ellipsize only decides how overflow is
*drawn*; the label still reports its **entire** string as its natural width,
and since `set_size_request` is a floor (see 3), the panel grew to grant it.

**Do instead:** set `set_max_width_chars(n)`, which is what actually caps the
natural width, and pair it with `hexpand: true` so the label is still allocated
the real column width rather than being squeezed to `n` characters. Ellipsize
or wrap then applies within that cap. The same applies to *wrapping* labels: a
wrapping label with no cap asks for its whole text on one line.

Backstop for a whole region: `set_overflow(gtk::Overflow::Hidden)` on the
container, so a label added later cannot silently widen it again.

## 4d. Slack is a resource — give it to something useful

The first anchoring fix parked all the leftover height in a blank `vexpand`
spacer. That worked, but it meant a panel with a short title showed a band of
empty space while the description sat in a cramped scroller right next to it.

**Do instead:** make the element that *benefits* from extra room the expanding
one. Here `desc_section` carries the only `vexpand`, so it does both jobs at
once — it pins everything below it to the bottom, and it donates the spare
height to the description. A dedicated spacer is only right when nothing in the
layout actually wants the space.

## 2b. An overlay is not a focus scope

Related to 2, and missed when the dialogs were first converted. A
`gtk::Window` confines Tab: focus cycles within the window and stops at its
edge. A panel in a `gtk::Overlay` gets no such thing — the page underneath is
still in the same widget tree and still focusable, so Tab walked out of a
"modal" dialog and into the sidebar behind it. You could focus a button you
could not see and activate it with Enter.

The scrim hides this in testing because `can_target` blocks the **mouse**.
Nothing was blocking the **keyboard**.

**Do instead:** `crate::widgets::focus_trap` — a capture-phase key controller
on the window root that owns Tab/Shift+Tab while the panel is visible, moves
focus with `child_focus`, and wraps at the ends by clearing the root focus and
searching again. Attach it to the **root**, not the panel: when a dialog opens,
focus is usually still on the page widget that opened it, so a controller on
the panel would never see the keypress that walks away from it.

Remove it when the dialog closes, for the same reason as the Esc controller —
an orphan keeps swallowing Tab for a panel that no longer exists.

## 5. Never use `opacity` on a scrollbar

`src/style.rs` opens with a warning block explaining that `opacity` below 1
makes GTK render through an offscreen surface, and a collapsed overlay
scrollbar's surface is zero-sized:

```
*** BUG *** In pixman_region32_init_rect: Invalid rectangle passed
```

**I wrote `opacity: 0` anyway** while hiding the tag scrollbar, and only caught
it by re-reading that block. Hide a scrollbar by making its **background
transparent**.

More generally: `src/style.rs` says every line in its scrollbar block exists
because of a specific bug. Believe it.

## 6. To hide a scrollbar, use `PolicyType::External`

`Automatic` reserves space for a bar that appears only sometimes — which is its
own version of the "size varies" complaint. `External` keeps wheel, touchpad
and drag scrolling while GTK draws and allocates nothing.

## 7. `.focus()` is ambiguous on a `gtk::Window`

With `gtk::prelude::*` in scope, both `WidgetExt::focus` and
`GtkWindowExt::focus` apply:

```
error[E0034]: multiple applicable items in scope: multiple `focus` found
```

Name the trait: `gtk::prelude::GtkWindowExt::focus(&window)`. Note also that
`gtk::Text` is the inner widget of a `gtk::Entry` and is what actually holds
focus — test for it first.

## 8. A keyboard shortcut on the window root fires while you are typing

The float close handler in `app.rs` fired on `q`/`Q`/Esc whenever a float was
visible, without checking focus. The tags panel has a text entry, so **typing
the letter `q` into it dismissed the panel**.

**Do instead:** a letter shortcut must check whether a text widget has focus.
Esc is safe; letters are not.

---

## 9. Dead code fails the build

This is a **binary crate** and clippy runs `-- -D warnings`. An enum variant
that is only *matched* and never *constructed*, a helper that lost its last
caller, or an import left behind by an edit will all fail CI.

Real examples from this session:

- Removing the ✕ from the series float made `SeriesFloatMsg::Close` and
  `SeriesFloatOut::Close` unreachable.
- Converting the pickers left three copies of a `window_of()` helper unused.
- `shelf_editor.rs` kept `use relm4::RelmWidgetExt;` after its last
  `set_margin_all` went away.

**After deleting a call site, grep for what it used**, including enum variants,
helpers and imports. Do not add an import to `src/db.rs` for a symbol used only
in one submodule.

## 10. Do not write "helpful" defaults that hide failures

The whole A0 step-2 pass exists because pages did
`list_x().unwrap_or_default()`. The result was **the empty-state lie**: a
broken database rendered as "your library is empty", "no dictionaries
installed", "no shelves yet" — each indistinguishable from the real empty case.

Specific traps recorded during that pass:

- `self.x = ….unwrap_or_default()` in a *reload* **overwrites live on-screen
  data**. On error, report and keep what is displayed.
- A silent `return` on a failed read is the same defect with different syntax.
- `unwrap_or(false)` on a uniqueness check silently assumes "the name is free".
- Distinguish "row absent" from "read failed". They are different messages.
- Raw grep counts of `unwrap_or_default()` **over-count** — many are string,
  path or date defaults, or `get_pref(key, default)` calls that default *by
  design*. In `reader.rs`, 35 matches were 4 real ones.

---

## 11. Working with CI in this repo (no local toolchain)

There is **no `cargo` in the sandbox** — no `~/.cargo`, no `pkg-config`, no
gtk4, and no network to `static.rust-lang.org`. **CI is the only gate.** Runs
take roughly 5.5–9 minutes.

Practical consequences:

- **Check the failing *step* before assuming your code is broken.** Twice a
  "failed" run was the rustfmt step alone, whose formatting commit had in fact
  landed. Use `gh run view <id> --json jobs`; `gh run view --log` returns
  nothing here.
- Read the published logs: `ci-logs/clippy-latest.txt` for clippy,
  `ci-logs/test-latest.txt` for tests. **`clippy-latest.txt` can be stale** —
  it keeps a `--- run <id> ---` footer from an older run.
- **CI's rustfmt step auto-commits and pushes**, so your next push is often
  rejected. Recover with `git fetch origin <branch>` then
  `git rebase FETCH_HEAD`. Use `FETCH_HEAD`; `origin/<branch>` may not exist
  locally in a shallow clone.
- **Local HEAD silently drifts between turns.** Always
  `git fetch origin <branch> && git reset --hard FETCH_HEAD` before editing.
  The sandbox can also be replaced wholesale — it came back once as a fresh
  clone at old `main`, and the branch had to be re-fetched. Your pushed work is
  safe; your local checkout is not.
- **Two pushes in quick succession race CI's rustfmt.**
- **Never `git commit --amend` after pushing.** Use `git reset --soft <sha>`.
- **`echo "push=$?"` after `git push … | tail` reports `tail`'s status**, not
  git's. A rejected push can print `push=0`. Read the hint text or check
  `$PIPESTATUS`.
- On a rebase conflict in `ROADMAP.md`, it is almost always duelling appended
  changelog rows. Keep both.

## 12. Editing files you cannot compile

Because nothing can be built locally, scripted edits need their own safety net:

- Python heredocs must `assert old in s` **and** assert the occurrence count,
  then verify brace/paren balance afterwards. Compare against
  `git show HEAD:<file>` — `reader.rs` has a pre-existing `+1` brace delta from
  a `{` inside a string literal, and so do `dictionaries.rs` and
  `shelf_rules.rs`.
- **Do not chain `python3 <<'PY' … PY && git commit && git push`.** A failed
  `assert` exits Python but the `&&` chain still proceeds.
- **After a CI rustfmt commit, re-read the file before scripted edits.** An
  anchor matching a one-line expression will fail once rustfmt has reflowed it
  into a 4-line chain.
- **A flat `grep -n "self\.catalog"` misses multi-line method chains.** Sweep
  with `\b(self|model)\s*\n?\s*\.catalog\b`. A single-line grep cost one CI
  failure.
- `awk 'length>100'` counts **bytes, not characters** — curly quotes and em
  dashes trip it. Check with Python before "fixing" a line.
- **Writing new code from memory is unsafe.** Four API mistakes came from it:
  there is no `insert_book_for_test` (use the 10-arg `insert_book`), `Book` has
  no `Default`, `Book.progress`/`rating` are `u8`, and `Book` lives in
  `crate::models`. Also: `Cargo.toml` sets no `rust-version`, so avoid recent
  std APIs (`repeat_n` needs 1.82).
- `catalog()` already returns `&Catalog`, so `&self.service.catalog()` is a
  type error and a needless borrow. And `let cat = self.service.catalog();`
  followed by assigning to `self.<field>` is a borrow conflict — finish reads
  into locals first.

## 13. A route existing in `app.rs` does not mean the user can reach it

`ReadingList`, `Tags` and `Analytics` had complete pages, `PageSlot` variants
and `Route` arms — and **nothing in the UI ever emitted those routes**. Three
finished pages that could not be opened at all. `AllBooks` was linked from
exactly one place: inside the `if total_books == 0` placeholder, so importing
your first book removed the only link to the full grid.

**Reachability means grepping for who *emits* the route**, not who handles it.

## 14. You cannot see the screen

The user is the QA loop for anything visual. Do not ask for screenshots — you
cannot view them. Ask for error text, and say precisely which two states to
compare ("a book with no tags versus one with many — do the buttons sit at the
same height?").

Corollary: **verify a claim before repeating it.** I told the user the series
float "skips the book float you came from"; it opens from the **book page**
(`book.rs:695`), and the book float's series line is a plain label. The
correction is in the ROADMAP rather than quietly dropped.

---

## 15. Do not skip a file on a shallow check

Rejected reasoning, in the user's words: *"it already reports its main error,
so it's fine."* A proper check also covers secondary/enriching reads, N+1 query
patterns, and whether the page could later move off the UI thread.
`saved_quotes.rs` and `saved_words.rs` were skipped on exactly that shallow
basis and had to be revisited.

If a file genuinely needs nothing, say **why** precisely. `author.rs` is the
model: it makes no database reads at all, it only passes the `Arc` to its
children, and its one `unwrap_or_default()` is on a local helper.
