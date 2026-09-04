# A0 step 6 — grid virtualization: where the memory actually goes

Working notes for the decision. Written before any code, because the last two
times this step was estimated the estimate was wrong, and both times the cause
was reasoning from a number without checking what produced it.

## The measurement

From CI, same build, same job, the only difference being whether the harness
actually opened the All-books page (it never did until `KALAM_ROUTE` landed on
2026-09-04):

| Books | Grid reached? | Peak RSS |
|---|---|---|
| 139 | no (Home only) | 231 MB |
| 2,000 | no (Home only) | 226 MB |
| 139 | **yes** | 275 MB |
| 2,000 | **yes** | **507 MB** |

So: **+232 MB for +1,861 cards, ≈128 KB per card.**

## What 128 KB/card is made of

This is the part that changes the plan. A `GtkBox` with two labels is not
128 KB — it is a few kB. The number is suspicious, and it resolves immediately:

```
cover slot          = COVER_W × COVER_H = 128 × 204
one RGBA texture    = 128 × 204 × 4     = 102 KB
```

102 of the 128 KB per card is **one cover texture**. Across the delta:

```
1,752 additional covers × 102 KB ≈ 175 MB   of the measured 232 MB
```

The remaining ~57 MB (≈31 KB/card) is the widget tree itself — the `GtkBox`,
the cell wrapper, two `GtkLabel`s with Pango layouts, a `GestureClick`, and a
tooltip string.

## Why the 300-entry cover cache did not bound this

`COVER_CACHE_MAX = 300` is real, correct, and does what its comment says. It
was still not a bound on live memory, and the reason is worth writing down
because it is the actual bug:

`gdk::Texture` is reference-counted. `swap_in_cover` does

```rust
frame.append(&build_picture(texture, key.1, key.2));
```

which puts a **strong** reference into a `GtkPicture` that lives inside a card
widget. When the LRU later evicts that key, it drops *the cache's* reference —
but the card is still alive and still holding one, so nothing is freed.

The cache bounds how many textures are **re-usable**. It cannot bound how many
are **resident**, because that is decided by how many cards exist. And
`build_book_grid` builds one card per book, unconditionally, for the whole
library.

So the LRU and the grid were each locally correct while jointly unbounded —
which is exactly why "peak memory is flat" was believed for a day: there *was*
a real, working, well-tested bound in the picture, and it made the unbounded
thing next to it look accounted for.

## What this means for the fix

The naive framing — "2,000 widgets is too many widgets, so recycle widgets" —
is only ~25% of the problem. Ranked by payoff:

1. **Stop holding textures for off-screen cards** (~175 MB). This is the win.
2. **Stop holding widgets for off-screen cards** (~57 MB). Nice, and it also
   fixes `grid_build` (421–484 ms at 2,000 books, measured).

Both fall out of virtualization, which is the reason to do it. But they are
separable, and (1) is available much more cheaply than (2) — see the options.

## Options

### A. `GtkGridView` + `ListStore` (true virtualization)

The GTK-native answer. `GridView` recycles a pool of ~visible-count widgets
through a `SignalListItemFactory`; both costs above become O(visible).

Cost, honestly:

- **First use of GTK4 list views in this codebase.** No `GridView`, `ListView`,
  `SelectionModel` or `SignalListItemFactory` appears anywhere in `src/` today.
- `Book` must become a `GObject` subclass (`glib::Object` + properties) to live
  in a `ListStore`, or be wrapped in one. That is boilerplate the repo has so
  far avoided entirely.
- **`PENDING_FRAMES` breaks.** It maps a cover key to a weak ref of the frame
  waiting for it, which assumes a frame belongs to one book for its lifetime.
  Under recycling a frame is reused for a different book mid-flight, so a
  late-arriving decode can paint the wrong cover into a recycled slot. The
  bind/unbind protocol has to cancel pending work per item.
- Touches 3 call sites (`all_books`, `tags`, `shelf_detail`) plus the shared
  widget module.
- **`GRID_COLS = 6` is currently hardcoded**; `GridView` does its own column
  math, which is a behaviour change (probably an improvement — it would become
  responsive — but it is a visual change the user has to look at).

### B. Bound the textures only (cheap, ~75% of the win)

Leave the widget tree alone. Make off-screen cards give up their texture:

- Keep the LRU, but have eviction actively clear the `GtkPicture` of any card
  holding the evicted texture (back to placeholder), so eviction really frees.
- Or: only swap a texture into a frame that is currently within the viewport,
  and drop back to placeholder on scroll-out.

This is a change to `book_row.rs` alone — no new GTK concepts, no `GObject`
subclassing, no changes to the three pages. It does **not** fix `grid_build`
time or the ~57 MB of widgets.

Risk: the placeholder↔cover swap becomes visible during scrolling if the
re-decode is not fast. The thumbnail path makes it cheap, but this is exactly
the kind of thing only the user can judge on real hardware.

### C. Do nothing yet

Defensible. 507 MB at 2,000 books is bad on a 4 GB machine, but the user's real
library is 139 books → 275 MB, which is survivable. Nobody has complained.

The counter-argument: the entire reason this was closed before was "the numbers
do not justify it", the numbers turned out to be measured wrong, and 507 MB is
a genuine number that would have justified it.

## Recommendation

**B first, then reassess; A only if B is not enough.**

Reasoning: B captures ~75% of the regression for a fraction of the risk, in one
file, with no new architectural concepts, and it is reversible. A is the
"right" long-term answer and will probably happen eventually, but it introduces
GTK list views, `GObject` boilerplate and a recycling-correctness hazard
(`PENDING_FRAMES`) all at once, on the most-used screen in the app, in a
codebase whose only visual QA is one person on one machine.

Doing A first would also make it hard to attribute the improvement: if memory
drops and scrolling feels different, we would not know which change did what.
B is measurable in isolation against the budgets that now exist.

## What is already in place to judge either

- Query-count budgets (A0 step 7) — regression-proof, machine-independent.
- CI reaches the grid and reports `grid_build`, `grid_cards`, peak RSS at both
  139 and 2,000 books.
- `04-home-for-comparison.png` proves the page under test is the real one.

Which means: whatever we pick, the before/after is already instrumented. That
was not true a day ago.
