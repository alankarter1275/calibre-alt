# Smoke-testing A0 steps 4 + 5 on your machine

Everything here needs a screen, which is why CI cannot do it. Nothing below
takes longer than a few minutes.

**Total time: about 10 minutes.** Test 1 is the important one. If you only do
one, do that.

---

## Status

Confirmed on the user's machine (Arch, 139 books, release build):

| Test | State |
| --- | --- |
| 1 — the grid got lazy | **passed** |
| 1b — Home and author covers | **passed** |
| 1c(a) — thumbnail backfill goes quiet | **passed** — the `thumbs_backfilled` ladder was gone on a second launch |
| 1c(a) — re-import brings it back once | **passed** — ladder returned once, ran to `144` after five imports |
| 1c(b) — `startup_dicts` off the UI thread | **test was wrong, rewritten** — needs a throwaway data dir, see below |
| 2 — chapter turns | not yet run |
| 3 — background work does not freeze the window | not yet run |
| 4 — imports still behave | not yet run |

---

## Setup (once)

```bash
cd ~/calibre-alt          # or wherever your clone is
git fetch origin
git checkout arena/01a061aa-calibre-alt
git pull
```

If this is a fresh machine, install the dependencies first:

```bash
sudo pacman -S --needed rust gtk4 libadwaita webkitgtk-6.0 base-devel pkgconf
```

Then build once. The first build is slow; later ones are not:

```bash
cargo build --release
```

**Always use `--release` for timing.** A debug build is several times slower
and the numbers mean nothing.

---

## Test 1 — the grid got lazy (the main change)

This is the one that shows whether step 5 did anything.

Before this change, opening **All books** decoded every cover before it could
draw anything. Now cards appear immediately as grey placeholders and the
images fill in behind them.

```bash
KALAM_TIMING=1 cargo run --release
```

Then, in the app: **Home → All books**. Watch the terminal.

```text
[timing] grid_build            12.4 ms
[timing] grid_cards                 312
[timing] covers_queued              312
```

`covers_queued` should match `grid_cards` on a first visit and drop to `0` when
you come back to the page — everything is cached by then. If it stops at some
round number well below `grid_cards`, that is the capping bug from
2026-09-03 come back; see `docs/pitfalls.md` §16.

Now do the same thing with the preloader switched off — this is the old
behaviour, in the same session, for a fair comparison:

```bash
KALAM_NO_PRELOAD=1 KALAM_TIMING=1 cargo run --release
```

**Home → All books** again, and compare `grid_build`.

**What should happen:** `grid_build` is much smaller in the first run. The gap
is the decoding that no longer blocks the window. With the preloader off you
should also see `grid_build` grow with library size; with it on, it should
barely move.

**Every** card must end up with a real cover, not just the top rows. Scroll to
the bottom of All books and check the last row — the covers there arrive later
than the first ones, by design, but they must arrive.

**Also watch the window itself**, not just the numbers:

- Preloader **on**: the grid appears at once, covers arrive over the next
  moment.
- Preloader **off** (`KALAM_NO_PRELOAD=1`): the window hangs briefly, then
  everything appears at once, fully drawn.

Paste both sets of numbers to me and tell me your library size.

> If `grid_build` is already tiny in both runs, that is a real result and worth
> telling me — it means your library is small enough that this never hurt, and
> it is evidence against bothering with grid virtualization (A0 step 6).

---

## Test 1b — Home and author covers

Deferred cards are not only built by the grid. Home builds its own "continue
reading" and "recently added" strips, and an author page builds a strip of that
author's books. Each of those had to be wired to the preloader separately, and
Home's was missed the first time — its covers never loaded at all.

1. Open **Home**. The covers in both strips must fill in.
2. Open any **author** page. Same for the strip of their books.
3. Go to Home, then away, then back. Covers should be there instantly the
   second time (they are cached), with no grey flash.

**What should happen:** no strip is left showing grey boxes. If one is, the
page is building cards without calling `preload::warm_books` — that is the bug
in `docs/pitfalls.md` §16, not a slow disk.

## Test 1c — the startup work that should now be invisible

Two startup fixes only show themselves in the `KALAM_TIMING=1` output, and one
of them **cannot be verified in CI at all** — every CI run starts from a freshly
seeded library, so it is always a first launch.

Run the app twice with timing on and compare.

**a) The thumbnail backfill should go quiet.**

Before this change, every launch printed a ladder of lines like:

```text
[timing] thumbs_backfilled        50
[timing] thumbs_backfilled       100
[timing] thumbs_backfilled       139
```

even though every thumbnail already existed. It was listing your whole library
and checking each file, to do nothing.

**What should happen now:** on a settled library those lines are **gone**.
Nothing about thumbnails should be printed at all.

Then import one book and restart. The ladder should come back once (the pass
re-runs because the book count changed), and then go quiet again on the launch
after that.

If the lines never stop appearing, the skip marker is not being written — the
likely cause is a cover that cannot be thumbnailed, which deliberately prevents
the marker being recorded.

> **Confirmed, both halves.** On a second launch of a settled 139-book library
> the ladder was gone. After importing five more books it came back once and ran
> to `144`, which is the marker noticing the library changed. This is the only
> evidence there will be for this fix — CI seeds a fresh library on every run,
> so it always does the real work and the ladder always appears there, correctly.
>
> Note that the returning pass printed no `thumbs_backfill_done` line. That is
> right: the importer already writes a thumbnail for each new book, so the pass
> found all 144 present and generated nothing. It re-checked, confirmed, and
> recorded the new count.

**b) `startup_dicts` — ignore the line's position. My earlier instruction here
was wrong.**

This doc used to say "`startup_dicts` should now print *after* `window_shown`".
That test does not work on your machine, and you should not read anything into
it either way.

Why it was wrong: the line prints when the dictionary work *finishes*. On your
machine the packs installed months ago, so the work early-outs on a pref in
about `0.6 ms` — far sooner than the window appears at ~700 ms. So it prints
before `window_shown`. But it would *also* have printed before `window_shown`
back when it ran on the UI thread, because 0.6 ms of work is invisible either
way. **The check could never tell a fixed build from a broken one on a settled
machine**, which makes it not a test.

The real property is "it does not hold up the window", and the honest way to
see that is to give the app a throwaway data directory so the dictionaries
genuinely have to install:

```bash
XDG_DATA_HOME=/tmp/kalam-dicttest KALAM_TIMING=1 cargo run --release
```

That starts from an empty library and no dictionaries, so the import does its
real ~2–3 s of work. **Now the order means something:** `window_shown` should
come first, with `startup_dicts` landing seconds later. Delete `/tmp/kalam-dicttest`
afterwards. Your real library lives in `~/.local/share/kalam` and is not touched.

CI already shows this on every run, for the same reason — it seeds a fresh
library, so `window_shown 116 ms` is followed by `startup_dicts 3739 ms`.

## Test 2 — chapter turns

```bash
KALAM_TIMING=1 cargo run --release
```

Open a book you have **not** opened recently, and read forward through three
or four chapters. Watch `chapter_load`.

**What should happen:** turning to the *next* chapter should be at least as
fast as before, ideally a little faster, because its file was already read in
the background while you were reading the current one.

**Be honest about this one — the effect may be invisible.** It only helps when
the file is not already in the OS page cache, so on a warm second run of the
same book you will likely see no difference at all. That is expected, not a
failure. What matters is that nothing got *slower* and no chapter loads wrong.

Please do check: **jump around** via the table of contents, and change the
**theme or font size** mid-book. The chapter shown must always match the one
you asked for. (Preloading only warms the file read, never the rendered HTML,
specifically so that changing settings cannot show you a stale chapter — but
it is worth confirming on real hardware.)

---

## Test 3 — background work does not freeze the window (step 4)

Three quick ones.

**a. Import a dictionary.** Settings → + Import dictionary → pick a large pack.

- The window must stay responsive while it parses — you should be able to
  scroll and click.
- One toast naming the file, then a result toast. Not a duplicate.

**b. Errors from background work must be visible.** Try importing a file that
is not a valid dictionary (rename any `.txt` to `.tsv` and pick it).

- You should get an error toast.
- **And** it must be listed under Settings → Notifications. That second half
  is the bug I fixed — errors from worker threads were being silently thrown
  away.

**c. Quitting mid-job.** On a first launch with a big library, thumbnails
generate in the background. Quit while that is happening.

- It should exit promptly, not hang.

---

## Test 4 — imports still behave

Home → + Add books, and import several EPUBs at once.

- The status line counts up per file.
- The button says "Importing…" and is greyed out while it runs.
- At the end: a summary line and a toast.

The second and third points are worth a close look. Home's status line and
button state were **frozen** before this change — a bug I found while doing
step 5, where the page never refreshed its own display. They should now
update as the import runs.

Do the same on My Library → All books, which had a separate copy of this code
that is now shared.

---

## What to send me

- The `grid_build` / `grid_cards` numbers from both halves of Test 1, plus
  your library size.
- Anything from Tests 2–4 that looked wrong, felt slow, or surprised you.
- "Everything looked fine" is a genuinely useful answer — it is the difference
  between me guessing and knowing.
