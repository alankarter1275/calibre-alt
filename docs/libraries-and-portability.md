# Libraries, portability, and the site list

Notes for discussion. Nothing decided.

Two separate topics that came up together:

1. Calibre-style **libraries** — pick a folder, several libraries inside it,
   switch between them, copy the folder to another PC and it just works.
2. Which **sites** P7 targets.

---

# Part 1 — Libraries

## What you described, and what we have

Calibre's arrangement:

```
~/Books/                       ← you pick this folder
  Fiction/                     ← a library
    metadata.db
    Author Name/
      Book Title/
        book.epub
        cover.jpg
        metadata.opf
  Research/                    ← a different library
    metadata.db
    ...
```

Key properties: you choose where it lives, there can be several, they are
completely separate, and copying the folder to another machine works.

What Kalam has today:

```
~/.local/share/kalam/          ← fixed, you cannot choose
  catalog.db                   ← exactly one
  library/
    <uuid>/
      book.epub
      cover.jpg                ← no metadata file
```

So we have **one** library, in a **fixed** place, with **no** metadata file in
the folder.

## The good news: we are closer than it looks

Two things I checked in the code, both encouraging.

**1. Nothing machine-specific is stored in the database.** The `books` table
holds `file_name` ("book.epub") and `cover_name` ("cover.jpg") — plain names,
not full paths. Full paths are worked out at read time from the folder name.

That means a Kalam folder is *already* portable. Copy it to another PC and it
would work. We just have no way to tell the app to look somewhere else.

**2. Every path in the app comes from one function**, `data_dir()` in
`src/paths.rs`. Everything else is built on it. So "let the user choose where
the library lives" is genuinely a small change — make that one function read a
setting instead of a fixed location.

That is a much better position than I expected. The hard part is not the
plumbing.

## The hard parts, honestly

**Where does the setting live?** If the library location is stored *in* the
library, we cannot find the library to read it. It has to go somewhere fixed —
a small config file in `~/.config/kalam/`, holding the list of libraries and
which one was last open. That is the one thing that stays machine-specific, and
it is correct that it does, because it is about *this machine*, not about the
books.

**Things that are not per-library.** Right now everything sits in one place.
Some of it clearly belongs to a library (books, covers, highlights, shelves).
Some clearly does not — the dictionaries you installed, your theme, your
preferences. Those should stay global, or you would re-install dictionaries for
every library. Needs a deliberate split, and getting it wrong is annoying to
undo.

**Moving an existing library.** Everyone's current library is in the old fixed
place. The app has to notice that, treat it as "your first library", and offer
to move it if you pick somewhere else. Copy-verify-delete, never move-and-hope.

**Reading position and the reader cache.** The reader unpacks EPUBs into a
cache folder. That is throwaway data and should *not* travel with a library —
it should be rebuilt on the new machine. Easy to get wrong by copying it.

## On `metadata.opf`

Calibre writes one per book. Worth being precise about why: it is a **copy**,
for export and recovery. Calibre reads from `metadata.db`. If you edit the
`.opf` by hand, Calibre generally ignores you.

So this is the same thing I described in `docs/p7-storage.md` §5 — a
self-describing backup file, database stays authoritative. Doing it makes a
library genuinely self-contained: even if `catalog.db` were lost, everything
could be rebuilt by walking the folders.

I would use JSON rather than OPF. OPF is an ebook-packaging format that cannot
express our things (highlights, reading sessions, saved words) without abuse.
Nothing else reads a stray `metadata.opf` anyway.

## Shelves versus libraries — you are right, these are different

Calibre has two features and we have conflated them:

| Calibre | What it is | Kalam today |
|---|---|---|
| **Library** | Separate folder, separate database. Books in one are invisible in the other. You switch. | ✗ nothing |
| **Virtual library** | A saved filter over one library. Just a view. | ✓ this is our shelves |

Our shelves — manual lists and rule-based smart shelves — are Calibre's
*virtual* libraries. Useful, and worth keeping exactly as they are.

What is missing is real libraries: a hard wall. Fanfiction in one, technical
books in another, and the fanfiction genuinely not there when you are in the
other one. A shelf cannot do that, because a shelf is a view over everything.

**These should both exist.** Libraries for separation, shelves for organisation
within a library. They are not competing.

## My honest opinion on timing

This is a good idea and I want to build it. **I do not think it should be part
of P7**, for one specific reason: P7 will be *adding* books from new places,
and this changes *where books live*. If a fic goes missing we would not know
whether the downloader or the storage change was at fault.

They are also independent — neither needs the other. So: sequence them, in
whichever order you prefer. If libraries matter more to you than AO3, we can do
libraries first; I would just rather not interleave them.

---

# Part 2 — The site list

You asked for Webnovel, FanFiction.net, and Literotica. Taking them one at a
time, because they are not equally sensible and I would rather say so now.

## FanFiction.net — hard, and the tools have given up

This one has a real problem. FFN sits behind Cloudflare's bot protection, and
it has broken the established tooling:

- FanFicFare's own workaround docs tell you to point it at your **browser's
  cache** instead of fetching pages, because direct fetching fails.
- Users report FanFicFare "basically gave up on FFN — it hasn't worked in a
  year, it's not even listed as supported."

This is not a parsing problem we can be clever about. It is a "prove you are a
real browser" challenge, and the honest ways to pass it are to *be* a browser
or to use someone else's service.

**There is an angle**, and it is interesting: Kalam already embeds a full
browser engine — WebKit, for the reader. A page fetched through that is a real
browser request. That could work where a plain HTTP fetch cannot.

But be clear about what that means: it is a heavier, slower path, it needs the
UI thread, and it is a different fetching mechanism from every other source.
Not impossible — genuinely novel, actually — but it is its own project, not
"another source".

**My suggestion:** keep FFN on the list as a *goal*, but do not make it the
second source. If the second source is also the hardest one, we learn nothing
about whether the design is right, because every problem will look like a
Cloudflare problem.

## Webnovel — works, but most of what you want is paywalled

Webnovel (Qidian International) is technically reachable. The issue is
different: their model is **locked chapters**. Popular novels have a small
number of free early chapters and then everything is behind their coin system.
Users describe completed novels with hundreds of locked chapters.

So a downloader gets the free chapters and stops. That is not a bug, and we
should not try to make it not-a-bug — bypassing a paywall is out of scope, and
it is the kind of thing that gets a project into real trouble.

**Worth deciding what you actually want here.** If it is "download the free
chapters and follow for new free releases", that is legitimate and buildable.
If the expectation is a whole novel, Webnovel will disappoint regardless of how
good our code is.

## Literotica — technically the easiest of the three

Plain HTML, simple structure, no Cloudflare challenge, no paywall. From a
"does our parsing work" point of view it is a good test: multi-page stories,
categories, author pages, and a tag system.

The only notes: content is adult, so the app should not surface it without the
user going looking. And it has no series/chapter concept as clean as AO3's,
which is genuinely useful for testing — it stresses the assumption that every
source has neat chapters.

## What I would suggest instead

The point of source two and three is to find out whether the design is honest.
For that we want sources that are **different from AO3 in shape**, not
different in *hostility*.

A sequence that tests something:

1. **AO3** — has a download endpoint. Proves search, filters, following.
2. **Literotica** — no download endpoint, so it proves the "build an EPUB from
   chapters" path, which is the biggest untested piece. Simple to fetch, so
   when something breaks it is our bug and not a bot-blocker.
3. **Webnovel** — proves partial availability: works we can only get part of.
   That is a real category and the data model should handle it.
4. **FanFiction.net** — last, deliberately, because it needs the browser-engine
   fetch and that is a project of its own.

Royal Road would slot in at position 2 or 3 as a gentler alternative if you
would rather leave adult content out of the early testing — it is the closest
thing to "AO3 without a download endpoint".

---

# Questions

1. **Libraries: before or after P7?** Both, eventually. Which first?
2. **Webnovel:** is "free chapters only" acceptable, or does that make it not
   worth doing?
3. **FFN:** is the browser-engine approach something you want to attempt, given
   it is a bigger job than a normal source?
4. **Site order:** does the sequence above make sense, or do you want your
   three in your order regardless?
