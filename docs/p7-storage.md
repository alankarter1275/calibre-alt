# Storage, and keeping highlights when a fic updates

Notes for the P7 discussion. Nothing decided.

Short version: **we already have the folder layout you described**, and the
highlight problem is solvable without changing how anything is stored. But
there is a separate, real question hiding underneath, and it is worth doing.

---

## 1. We already store books the Calibre way

Today, every book is a folder:

```
~/.local/share/kalam/
  catalog.db                  the database
  library/
    <uuid>/
      book.epub               the book
      cover.jpg               the cover
```

That is the Calibre shape already — one folder per book, holding the file and
the cover. So "should we adopt a folder per book?" is answered: we did, at P1.

What is *not* in that folder is the metadata. Title, author, tags, reading
position, highlights, saved words — all of that lives in `catalog.db`.

Calibre does the same thing, incidentally. Its `metadata.opf` in each folder is
a *copy* for export; the real data is in `metadata.db`. So even the system you
are comparing us to keeps the database as the source of truth.

---

## 2. Why moving metadata into per-book files would make things worse

This is where I disagree with the proposal, and I want to be concrete about
why rather than just saying no.

If highlights lived in `library/<uuid>/annotations.json` instead of the
database:

- **Every screen that reads across books gets slow.** "Show my saved words",
  the statistics page, the reading history, "which books are in this shelf" —
  each becomes "open 500 files and parse them" instead of one query. We just
  spent this whole session making the app faster; this would undo a chunk of
  it.
- **We lose the query-count budgets** built for A0 step 7. Those catch the
  regression class that has bitten this repo three times. They cannot watch
  file reads.
- **Crash safety gets harder.** SQLite gives us atomic writes for free. Five
  JSON files updated in sequence do not.

And crucially: **it would not fix the highlight problem anyway.** Storing a
highlight in a file next to the book does not tell you where that highlight
belongs in a *different* version of the book. The hard part is not where the
data lives; it is matching old positions to new text.

So I would keep metadata in the database. If the goal is "the folder should be
self-describing so a library survives without the app", that is a real and
separate wish — see §5.

---

## 3. The highlight problem, and why it is already nearly solved

Here is what a highlight stores today:

| Field | What it is |
|---|---|
| `chapter_index` | which chapter, as a number |
| `start_path`, `start_offset` | exact position inside the chapter's markup |
| `end_path`, `end_offset` | where it ends |
| **`text_excerpt`** | **the actual highlighted words** |

The first three break easily. A DOM path is a route through the markup, and any
change to the markup invalidates it.

`text_excerpt` does not break. **The words are still the words.**

So when we replace a fic with a newer download, we can re-find each highlight
by searching the new chapter for its saved text. That is how every ebook app
that survives a file change does it, and **we already store everything needed**.
No new file format, no migration.

### What it looks like in practice

For each highlight on the book:

1. Look in the chapter it used to be in. Search for `text_excerpt`.
2. Exactly one match → re-anchor, silently. Done.
3. No match → look in nearby chapters (a chapter may have shifted).
4. Several identical matches → use the old offset to pick the nearest.
5. Still nothing → mark the highlight *orphaned*, keep the text, tell the user.

Step 5 matters. **An orphan should never be deleted.** The user wrote that note.
Showing "3 highlights could not be placed in the new version" with the text
still readable is a far better outcome than silently losing them.

### Expected results for AO3 specifically

AO3 appends new chapters at the end, so chapters 1..N keep their numbers and
their text is usually byte-identical. Most highlights should re-anchor at step
2 without even needing the search. The interesting cases are authors editing
earlier chapters, which does happen.

---

## 4. The part that genuinely needs new work

Not storage — **identity**.

Right now the app decides "is this book already in my library?" by hashing the
file. That is correct for importing files off disk, and wrong for a fic that
gains chapters: new bytes, new hash, so it imports as a **second copy**.

So we need to record where a book came from:

```
source      "ao3"
remote_id   "17400464"
```

Two columns on `books`. Then "the fic I already have" is a lookup, not a guess,
and the updater can replace in place while keeping the book's identity —
reading position, highlights, shelves, everything.

This is a small change and it is the one that actually unblocks updates.
`source-seam.md` §6 already calls `remote_id` load-bearing; it just has not
landed yet.

---

## 5. The wish underneath — "a library that survives without the app"

I think this is what the folder idea is really reaching for, and it is a good
instinct. If `catalog.db` is lost or corrupted, the books are still there but
everything *about* them is gone.

There is a way to get that without giving up the database: **write a copy of
each book's metadata into its folder, and treat it as a backup, not the truth.**

```
library/<uuid>/
  book.epub
  cover.jpg
  kalam.json      <- copy: title, author, tags, highlights, progress, source
```

- The database stays the source of truth, so nothing gets slower.
- The file is written when things change, best-effort. If it fails, nothing
  breaks.
- If the database is ever lost, a rebuild command can walk `library/` and
  restore everything.
- It also makes a library portable, and inspectable by a human.

The honest costs: extra writes, and a file that can drift from the database if
a write is missed. The mitigation for drift is that it is explicitly a backup —
on conflict, the database wins, always. Never read it during normal operation.

**This is worth doing, but I do not think it should be part of P7.** It is
independent of AO3, and mixing "add a new source" with "change how everything
is stored" makes both harder to judge if something breaks. I would rather do
P7, then do this deliberately.

---

## 6. What I would actually build for P7

1. `source` + `remote_id` columns on `books` — makes "I already have this fic"
   answerable. Small.
2. A **replace** path: swap the file, keep the book id.
3. **Re-anchoring** by `text_excerpt`, with orphans kept and reported.
4. Everything else as planned.

Notably: no storage change, no new file format, no migration risk. The
highlight problem is solved by using data we already collect.

---

## 7. Where I might be wrong

- If you want the metadata files because you want to *hand-edit* them, that is
  a different requirement and it changes the answer. Worth saying if so.
- If the real worry is losing the database, §5 is the answer and we could do it
  sooner than I suggest.
- Re-anchoring is heuristic. It will get some cases wrong. I think "wrong but
  reported" beats "silently lost", but it will never be perfect.
