# P7 — AO3 and the fiction platform: planning notes

Written before any code, to be argued with. Nothing here is decided.

The design work is already done in [`docs/source-seam.md`](./source-seam.md) —
the `Source` trait, the types, pagination, rate limits. This document is about
the things that document deliberately left open, plus what I found when I went
looking at the actual code and the actual site.

---

## The big finding: we do not have to scrape AO3 to get the text

This changes the shape of the whole phase, so it goes first.

AO3 generates a real EPUB for every work, at a plain URL:

```
https://download.archiveofourown.org/downloads/<work_id>/fic.epub
```

That is not a hack or a leak. AO3 builds those files with Calibre, and their
own FAQ page lists and links the download tools people use. Every established
tool in this space (FanFicFare, ao3downloader) uses it.

Two consequences:

**1. We never parse fic text.** No chapter scraping, no HTML sanitising, no
worrying about their markup changing under us. We download a file and hand it
to `import_epub`, which already exists and already works.

**2. Scraping is only needed for *finding* things** — search results, tags,
work metadata. That is the part AO3 has no API for.

This splits the risk cleanly. The fragile part (parsing their HTML) only
affects *browsing*, and when it breaks you get a bad search page, not a corrupt
book. The part that must not break (getting the text) is a file download.

I want to flag: this is a big enough difference that the roadmap's phrasing
("chapter content (sanitized)") should probably be revised.

---

## Question 1 — how search filters work

`source-seam.md` §13 leaves this open and leans towards *source-declared
filters*, Tachiyomi-style: the source describes its own filters and the UI
draws whatever it is given.

**I agree with the lean, and here is the concrete reason.** AO3's tag search is
the headline feature of this phase. A user wants "Ongoing, >50k words,
excluding this tag, in this fandom, sorted by kudos". A fixed list of filters
picked to suit both AO3 and MangaDex would express none of that well.

The cost is real, though: it is the most complex part of the Tachiyomi API, and
it means the search screen is generic code that renders widgets from data.

**A middle option worth considering:** ship AO3's filters as a hand-written
native search screen for P7, and only generalise when the second source arrives
and shows what actually needs to vary. That contradicts "design the seam
first", but it matches this repo's own rule from `source-seam.md` §11 — *do not
write the plugin API before two implementations exist, because an API designed
against zero implementations is a guess*. A filter system designed against one
source is the same guess in a smaller costume.

**Open for discussion.** My weak preference: generic filter *types* in the
trait (so the shape is right), but only the widgets AO3 needs actually built.

---

## Question 2 — where downloaded fiction lands

`source-seam.md` §13 leans EPUB. **The finding above settles it**: AO3 hands us
an EPUB, so "convert to EPUB" is not even work. There is nothing to decide for
AO3.

It will come back for a source that has no download endpoint, and then we will
have to build an EPUB from chapters. Worth knowing: `epub_write.rs` *edits*
existing EPUBs, it does not create them, so that is genuinely new code. The zip
library is already compiled with write support. Not a P7 problem.

---

## Question 3 — login

`source-seam.md` says "probably out of scope". I agree. Some works are visible
only to logged-in users; those simply will not appear. Storing someone's AO3
password is a security question worth avoiding until it is actually wanted.

---

## What I found that is not in the design doc

### A. Re-downloading a fic will create a duplicate book

`import_epub` decides "have I seen this?" by hashing the file. A fic that
gained a chapter is a different file, so it hashes differently, so it imports
as a **second, separate book**.

That is exactly wrong for the auto-updater — the whole point is to replace the
existing book while keeping its identity, reading position, highlights and
shelves.

So the updater needs a replace path, not an import path: keep the `book_id`,
swap the file, refresh the chapter list. Not hard, but it is real work and it
is not currently in the roadmap's P7 scope list.

### B. The annotation risk is smaller than the roadmap implies, but real

The roadmap flags this: *"annotations anchor to spine — re-downloaded fics may
lose anchors; best-effort"*.

Looking at the schema, annotations and reading progress anchor to
`chapter_index`, a number. AO3 **appends** new chapters at the end, so chapters
1..N keep their positions when N+1 arrives. **In the common case, anchors
survive.**

They break when an author inserts or reorders a chapter, which is rarer. They
would also break if AO3 changed its EPUB layout, since their files include
front matter in the spine.

Worth deciding: do we detect the breakage (compare chapter count and titles
before replacing, warn if they do not line up) or accept it silently? Detecting
is cheap and turns a confusing bug into a message.

### C. Following needs a "what changed?" signal that is cheap

The updater polls followed fics. Downloading each EPUB to see if it grew would
be rude and slow.

`WorkRef` already carries `chapter_count` and `updated_at` — both visible on a
work's page without downloading anything. So the poll is: fetch the work page,
compare, and only download when something moved. Worth writing into the plan
explicitly, because getting this wrong is how a tool gets an IP banned.

### D. Rate limiting is not optional and AO3 says so publicly

AO3 has stated they rate-limit deliberately to hinder bulk scraping. Their
tooling community treats "pause and resume on 429" as table stakes.

`source-seam.md` §7 already puts rate limiting in the source. What I want to
add: it should be **conservative by default and not user-adjustable upward**.
This is a personal-use tool; there is no reason to offer a "go faster" knob
whose only function is to get the user blocked.

---

## A proposed order of work

Each step ends somewhere usable, so we can stop or change course.

**1. The trait and types, landing with AO3 search.**
`source-seam.md` §12 is firm that the trait cannot land alone — unused code
fails CI here. So the first commit is the trait plus enough AO3 to use it.
Ends with: type a query, see real AO3 results in a list.

**2. Work detail + download.**
Tap a result, see the full description, download it. Reuses `import_epub`
untouched. Ends with: fics from AO3 in your library, readable offline.

**3. Search filters.**
The part we should discuss most. Ends with: the tag/fandom search that is the
actual point of the phase.

**4. Follow + the updater.**
Needs the replace path from finding A. Ends with: followed fics gain chapters
on their own.

**5. MangaDex** (`source-seam.md` §11 step 3) — proves the image flavour and
tests whether the trait is honest. Arguably P9, not P7.

My instinct is that **1 and 2 together are a genuinely useful tool** even if we
stopped there, and that 3 is where the design risk lives.

---

## Things I want your call on

1. **Filters:** full generic system now, or AO3-shaped now and generalise at
   the second source?
2. **Update safety:** when a re-downloaded fic no longer lines up with the old
   one, warn and skip, or replace anyway and accept losing highlights?
3. **Scope:** is MangaDex part of P7, or does P7 end at "AO3 works well"?
4. **Is the phase framed right?** The roadmap describes P7 as parsing chapter
   content. For AO3 that is not needed. Should P7 be "AO3, done properly" with
   other sites explicitly later?
