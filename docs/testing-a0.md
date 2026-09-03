# Testing the A0 changes on Arch

Everything here is optional except **Test 1** — that is the one that catches a
real bug I cannot catch from CI. The rest is measurement.

## Short answer to "do you need parameters?"

**No parameters are required.** Just `cargo run --release` and use the app
normally. The app behaves the same as before; the changes are internal.

Environment variables only *add* output or *turn things off*. There are three,
all optional:

| Variable | What it does | When to use it |
|---|---|---|
| `KALAM_TIMING=1` | Prints `[timing]` lines to the terminal | To see the speed numbers |
| `KALAM_NO_WEBVIEW_POOL=1` | Turns the new WebView reuse **off** | To compare before/after |
| `KALAM_NO_CSS=1` | Starts with no custom stylesheet | Only for debugging GTK warnings |

They are read from the environment, so put them **before** the command:

```bash
KALAM_TIMING=1 cargo run --release
```

Nothing is written to a file — the lines go to the terminal you launched from,
so launch from a terminal, not from a desktop icon.

---

## Test 1 — the one that matters (WebView reuse)

**Why:** the reader now reuses one WebKit view across books instead of building
a new one each time. If I got the signal-handler cleanup wrong, the *first*
book will look perfect and the *second* will be subtly broken. That is exactly
the kind of thing CI cannot see.

**Do this, in this order:**

1. Open book **A**, read a bit, scroll down.
2. Go **back** to the library.
3. Open a **different** book **B**.
4. Go back, then open book **A** again.

**On the second and third opens, check all five:**

- [ ] Text renders (not a blank white page)
- [ ] Existing **highlights** still show up in colour
- [ ] Selecting text shows the **chip**, and Dictionary / highlight both work
- [ ] **Tapping a word** opens the dictionary popup
- [ ] It **resumes at the right place**, not back at chapter 1

**If something breaks**, confirm it is my change by turning the pooling off:

```bash
KALAM_NO_WEBVIEW_POOL=1 cargo run --release
```

If the problem disappears with that set, it is the WebView pool — tell me which
of the five broke and on which open (2nd or 3rd), and that is enough for me to
fix it. If the problem happens *with the variable set too*, it was already
there and is not from this change.

---

## Test 2 — the new All books button

1. **Home** → the header now has **All books** next to **+ Add books**.
2. Click it → the full grid, with search and sort.
3. Type nonsense into the search (e.g. `zzzz`).
   - It should say **No books match "zzzz"** — *not* "Your library is empty".
     That wrong message was the bug I fixed.
4. Clear the search → all books come back.

---

## Test 2b — the My Library quick links (new)

Under the **My Library** title there is now a row: **All books · Reading list ·
Tags · Analytics**. Click each one; each should open its page and **Back**
should return.

Reading list, Tags and Analytics were unreachable before this — the pages were
built and wired into the router, but nothing in the UI opened them. If any of
them looks broken or unfinished, that is why, and it is worth telling me: they
have had no real use yet.

## Test 3 — the pages I rewired (Home, Analytics, Tags)

These now get their data through the new service layer. They should look
**exactly as before** — this is a refactor, so "nothing changed" is a pass.

- [ ] **Home** — counts, Continue row, Up next, Recently added all populated
- [ ] **Analytics** — numbers, charts, streaks; change the reading goal and it sticks
- [ ] **Tags** — the tag cloud, and clicking a tag lists that tag's books, and sorting works

---

## Getting the timings

```bash
KALAM_TIMING=1 cargo run --release
```

Use `--release`. A debug build is several times slower and the numbers are
meaningless.

You will see lines like this in the terminal:

```text
[timing] window_shown          912.4 ms
[timing] book_open              47.2 ms
[timing] chapter_load           52.8 ms
[timing] dict_lookup             6.1 ms
```

What each one means:

| Label | Measures | Baseline (before this work) |
|---|---|---|
| `window_shown` | Process start → first window drawn (cold start) | ~0.9 s warm |
| `book_open` | Parsing the EPUB when a book opens | 3.5 ms on revisit |
| `chapter_load` | Chapter handed to WebKit → finished rendering | ~50 ms warm, **~400 ms after a WebView re-spawn** |
| `dict_lookup` | A dictionary search returning | — |

Each line prints once, when that operation finishes.

### The measurement that shows whether my change worked

The 400 ms was paid on **every book open**, because the old code built a new
WebView each time. So:

1. Run with timing on:
   ```bash
   KALAM_TIMING=1 cargo run --release
   ```
2. Open a book → note `chapter_load`. Go back. Open a **different** book →
   note `chapter_load` again.
3. Now do the identical thing with pooling disabled:
   ```bash
   KALAM_NO_WEBVIEW_POOL=1 KALAM_TIMING=1 cargo run --release
   ```

**Expected:** in run 2 the *second* book's `chapter_load` is roughly 400 ms
slower than in run 1. That difference is the fix. (The very first book of a
session pays the WebKit startup either way — there is nothing parked yet, so
run 1 and run 2 should look similar for the first book. The gain is on every
open after that.)

Paste both sets of numbers to me and I will record them in the roadmap.

### If cold start looks slow — run it more than once

**Measured 2026-09-02, six consecutive runs on the user's Arch machine:**

| run | db_open | dicts | first_page | window_shown |
|---:|---:|---:|---:|---:|
| 1 | 6.3 | 0.2 | 577.9 | **6290.1** |
| 2 | 6.8 | 0.2 | 474.8 | **1596.5** |
| 3 | 5.7 | 0.1 | 137.5 | 964.8 |
| 4 | 5.9 | 0.1 | 135.8 | 852.0 |
| 5 | 5.5 | 0.1 | 144.2 | 945.1 |
| 6 | 5.6 | 0.1 | 135.2 | 830.6 |

**The first run after a build is meaningless.** It is the OS reading a
freshly-linked binary and its GTK/WebKit libraries off disk for the first time.
It settles at ~900 ms by run 3, which is the real number. Always discard run 1.

Of that steady ~900 ms, only ~138 ms is Kalam's own startup work — the other
~750 ms is GTK/WebKit/CSS initialisation that happens before and around our
code. So a slow cold start is mostly not something the app controls.

### Optional: the headless data-layer probes

These need no display and seed 2,000 fake books:

```bash
cargo test --release perf -- --ignored --nocapture
```

They are `#[ignore]`d so normal test runs stay fast. This is the check that the
database layer has not regressed; it prints a table of query timings.

---

## What I cannot test and you can

I have no display and no Rust toolchain in the sandbox — CI compiles, clippys
and runs the 173 unit tests on every push, but nothing renders a window. So
anything visual, and anything about WebKit's real runtime behaviour, has to
come from you. Error text pasted as text is ideal; I cannot read screenshots.
