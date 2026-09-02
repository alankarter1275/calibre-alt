# Testing before A1 (in-app dialogs)

Short version: **you do not have to test anything, and there are no new
parameters.** A0 step 2 is a refactor plus error reporting. CI already proves
it compiles, passes clippy with `-D warnings`, and passes 199 tests.

If you want to spend ten minutes on it, the section "The one test worth doing"
below is the only one that catches something CI cannot.

---

## Are there new parameters?

**No.** Still just:

```bash
cargo run --release
```

The three environment variables are unchanged and all still optional:

| Variable | What it does |
|---|---|
| `KALAM_TIMING=1` | Prints `[timing]` lines to the terminal |
| `KALAM_NO_WEBVIEW_POOL=1` | Turns WebView reuse off, to compare |
| `KALAM_NO_CSS=1` | Starts with no custom stylesheet |

Step 2 added **no** new variables and **no** command-line flags.

---

## What changed that you could actually notice

Almost nothing, on a healthy database. That is the point — the app should look
and behave exactly as before. Three things are genuinely different:

1. **One real bug is fixed.** `book_first_opened` used to fail on any book you
   had never opened (`SELECT MIN(at)` over zero rows returns a row containing
   NULL). Every caller hid it with `.ok().flatten()`, so it produced the right
   screen by accident. The book page's "First opened" timeline entry is the
   place this lived.

2. **Some pages are faster**, because five N+1 query storms are gone. The most
   noticeable is **Saved quotes**: it used to run about `2N + 1` queries every
   time you typed a character in the search box (~1,001 queries with 500
   quotes). If typing there ever felt sticky, it should not any more.

3. **Failures now say so.** Previously a failed read drew an empty page. Now it
   shows a toast. On a healthy database you will never see one.

---

## The one test worth doing

This is the only test that checks the actual point of step 2, and CI cannot do
it because CI has no display.

**It is completely safe** — it never touches your real library. `XDG_DATA_HOME`
redirects the whole data directory, so the app builds a fresh empty library in
`/tmp` and your real books in `~/.local/share/kalam` are not opened at all.

```bash
# 1. Start with a throwaway library, import one or two books,
#    highlight something, save a word.
XDG_DATA_HOME=/tmp/kalam-test cargo run --release

# 2. Quit the app, then corrupt that throwaway database:
printf 'garbage garbage garbage' > /tmp/kalam-test/kalam/catalog.db

# 3. Start it again against the corrupt database:
XDG_DATA_HOME=/tmp/kalam-test cargo run --release
```

**What should happen:** error toasts naming what failed ("Could not read your
library", "Could not read this book", and so on).

**What used to happen, and is the bug step 2 removes:** a friendly, fully
laid-out, completely empty app. My Library showed zero books and a reading goal
of zero. History said "Opened and finished books show up here as you read", the
same message a brand-new install shows. The reader opened your book and quietly
displayed none of your highlights. Nothing anywhere said a word was wrong.

If step 3 of that recipe just shows a hard error at startup instead of toasts,
that is also fine and arguably better — it means SQLite rejected the file at
open time, before any page ran. The failure mode being tested is the *silent*
one.

Clean up when done:

```bash
rm -rf /tmp/kalam-test
```

---

## Quicker version, if that feels like too much

Open each page once on your real library and confirm nothing looks wrong and no
toast appears: My Library, All books, Book page, Reader, Saved quotes,
Vocabulary, History, Lookup history, Shelves, Shelf detail, Tags, Reading list,
Analytics, Settings.

That is a "did I break the happy path" check. It takes about two minutes and
CI genuinely cannot do it, because there is no display in CI.

---

## Do I need timings again?

**No.** Step 2 changed no rendering and no startup work, so `window_shown` and
`chapter_load` should be unchanged. If you want to confirm nothing regressed:

```bash
KALAM_TIMING=1 cargo run --release
```

Discard the first run after a build (it is page-cache warming — the earlier
measurements showed run 1 at 6290 ms versus a steady state of about 898 ms).
Compare the second run against the numbers already recorded in `ROADMAP.md`.

---

## What to look at before A1 starts

Not a test, but worth two minutes, because it decides how A1 should feel.

A1 converts the five dialogs that are still separate windows. Five other
dialogs are **already** in-app, so open one and decide whether you like the
pattern before I copy it five more times:

- Click a book to open the **book float**, or the **shelves** or **tags**
  panel from a book.

Things to judge:

- Is the ✕ where you expect it?
- Does `Esc` closing it feel right, or too easy to trigger by accident?
- Does the dimmed background behind it read as "this is modal"?

Your rule that **every in-app dialog needs a visible ✕** is already recorded
and will be enforced for all five conversions. What I would like your opinion
on is whether the existing five are the standard to match, or whether you want
something different before the count grows to ten.

---

## Summary

| Question | Answer |
|---|---|
| New parameters? | No |
| Must I test before A1? | No — CI is green, 199 tests |
| Anything CI cannot check? | Yes: the corrupt-database test above, and that pages still look right |
| Will A1 conflict with this? | No. A1 touches dialog *hosting*, step 2 touched data *reading* |
