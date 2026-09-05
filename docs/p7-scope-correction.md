# P7 is a client, not a downloader — scope correction

**2026-09-04.** Written after the user stopped me and said the scope had drifted.
They were right. This records what I had wrong, and what it changes.

---

## What I had wrong

Everything I planned last turn was **downloading**. Find a fic, get the file,
read it offline, follow it for updates. Good, but only part of it.

What you actually want is **an app you browse the site with**. Surfing,
exploring, going down a rabbit hole. Downloading is one thing you can do while
you are in there, not the point of being there.

The tell is in the design doc itself. `source-seam.md` §1 lists the four verbs a
source answers:

1. search — what do you have matching this?
2. detail — tell me more about this one
3. chapters — what parts does it have?
4. content — give me part 3

**Every one of those starts from "I already know which work I want."** That is
a downloader's shape. Nothing there lets you wander.

## What browsing actually needs, that we did not plan

- **Category / fandom browsing.** Books → Harry Potter → sorted by follows.
  Drilling down a tree, not typing a query.
- **Author pages.** Their works, their favourites, their follows, their
  profile, their communities. You explicitly named this and it is not in the
  trait at all.
- **The site's own sort orders.** Kudos, hits, reviews, follows, recently
  updated, word count.
- **Your own lists on the site.** Your favourites, your follows, your reading
  history — the things that need a login.
- **Series, collections, communities.** Sideways links between works.
- **Reviews and comments.** Part of reading fanfic for most people.

None of these fit "search → detail → chapters → content". They are a different
set of verbs, and the trait needs them from the start — retrofitting a browse
model onto a download-shaped API means changing every source and every screen.

---

## The consequence I got wrong: FicHub does not solve FFN

This is the important one, and I should correct it clearly because I presented
FicHub as *the* answer to FanFiction.net last turn.

FicHub has exactly two endpoints:

```
/api/v0/epub?q=<fic url>
/api/v0/meta?q=<fic url>
```

**Both need a fic URL you already have.** There is no search, no category
browse, no author page. FicHub is a conversion service: you point it at a fic,
it gives you an EPUB.

So:

| | FFN via FicHub | FFN by fetching pages |
|---|---|---|
| Download a fic you found | ✅ | ✅ |
| Search FFN | ❌ | ✅ |
| Browse categories | ❌ | ✅ |
| Author pages | ❌ | ✅ |
| Your favourites/follows | ❌ | ✅ (needs login) |

**FicHub solves downloading from FFN. It does not solve browsing FFN.** And
browsing is most of what you asked for.

Since browsing FFN means fetching FFN pages, and FFN is behind Cloudflare, the
Cloudflare problem is back. **The WebKit approach is un-deferred.** I retired it
last turn on the grounds that FicHub made it unnecessary; that reasoning only
held while the goal was downloading.

Best combination is probably both: WebKit for browsing pages, FicHub for
getting the EPUB once you have chosen something. FicHub is better at the
download anyway — they already handle the multi-chapter assembly.

---

## What this does to the shape of P7

**It stops being "add some sources" and becomes "build a browsing client".**
The download path is a fraction of it. The bulk is:

- a browse/explore UI per source, driven by what that source offers
- author pages
- a much richer `Source` trait
- login, for the site-side lists — which `source-seam.md` §13 explicitly parked
  as "probably out of scope until someone asks". Someone just asked.

That last point matters. Your favourites and follows live behind a login, so
credential storage moves from "avoid it" to "required", and it needs doing
properly.

### The three deep sources

You said AO3, FFN and Literotica need to be very well constructed. That reads
as: those three get full browsing, and the trait is designed so a fourth source
can support less without breaking anything — a source should be able to say
"I do search but not author pages" and the UI adapts.

Royal Road then becomes the useful control case: simpler, proves the trait
degrades gracefully.

---

## What I would revise

1. **Rewrite `source-seam.md` §1's four verbs** into a browsing-shaped set.
   This is the root change; everything else follows.
2. **Un-defer WebKit fetching.** Needed for FFN browsing regardless of FicHub.
3. **Move login in-scope**, with real thought about credential storage.
4. **Re-plan P7 in stages**, because it is now much bigger than one phase's
   worth of work. Possibly: browse+search first, then author pages, then
   accounts, then downloading.
5. **Keep FicHub**, but described accurately: a download helper for FFN, not
   the FFN integration.

---

## What I want to check before rewriting the design

1. **Reading online, or always download-then-read?** A browsing client usually
   lets you read a chapter straight from the site without saving it. That is a
   different reader path from the offline EPUB one, and it changes the
   architecture. My assumption so far has been download-first.
2. **Writing actions?** Leaving kudos, bookmarking on the site, posting
   reviews. The Android apps do this. It needs a login and it changes the app
   from a reader to a client.
3. **Is P7 one phase or several?** Honestly it is now several phases of work.
4. **Do you want P6.5 (libraries) still first?** It is a smaller, self-contained
   piece and finishing it before starting something this large has value — but
   it also delays the part you are clearly most interested in.
