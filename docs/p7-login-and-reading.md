# Reading online, and the login question

**2026-09-04.** Answers to your point 1, and the material for the login debate
you asked to have.

---

# 1. Reading online — your plan is already the architecture

You described: open a fic, it downloads to a temporary folder and is cached,
and that is cleared when you close the app or move it to offline reading.

**That is almost exactly how the reader already works.** Today:

```
EpubBook::open(epub_path, cache_dir)
  → unzips the EPUB into cache/reader/<uuid>/
  → reads chapters from those files
```

The reader never reads the EPUB directly. It always reads an unpacked copy in a
cache folder. So it does not care whether the EPUB came from your disk or from
the network five seconds ago — it only cares that there is a cache folder with
chapters in it.

There is also already a cleaner: `prune_reader_cache(&uuids, 14)` runs at
startup and removes cached extractions older than 14 days.

So "read online" needs:

- a temporary identity for a fic that is not in your library (today, cache
  folders are keyed by a library book's `uuid`)
- a rule for when temporary caches die
- a "keep this" action that turns the temporary copy into a real library book

**Nothing about the reader itself has to change.** That is a good position to
be in.

## ~~The one thing I would do differently~~ — withdrawn, the user's model is better

I suggested keeping every temporary cache for a few days with a size cap,
instead of deleting on close. **The user rejected it and was right.**

Their model is three explicit tiers rather than one automatic one:

| What you did | What is kept |
|---|---|
| Just reading / browsing | nothing lasting |
| **Save** | kept in the temp area, *not* a library book |
| **Download** | a real book in your library |

That is better than mine for a plain reason: **it is predictable.** Time-and-size
pruning means the app decides what to keep, disk usage moves on its own, and
"why is this still here / why did this vanish" has no answer a user can
predict. An explicit Save is a decision you made and can see.

It also dissolves the problem I was solving. My worry was "close the app, come
back, have to re-download" — but that only bites if reading requires
downloading the whole work up front. With per-chapter reading (below), casual
reading never produces a big download to lose in the first place.

**Two decisions that fit together.** Per-chapter reading makes browsing cheap;
selective Save makes keeping deliberate. Either alone would be worse.

### Open question this creates

What exactly is the difference between **Save** and **Download**? Both keep the
fic. Candidates: Save keeps it readable without cluttering the library, while
Download makes it a full book with highlights, progress, shelves and
auto-update. Or: Save is a bookmark that keeps the text, Download owns the
file. Worth pinning down before either is built — the answer decides whether
"saved" fics need their own screen.

---

## Per-chapter reading — decided

Fetch chapters as you read them, rather than downloading the whole work to open
it.

Required, not merely nicer: a 2,000-chapter Royal Road serial cannot be
downloaded to read one chapter. It also makes opening anything feel instant,
and it is what makes the casual-reading tier above cost almost nothing.

Consequence for the design: **the trait needs a fetch-one-chapter verb even
though AO3 never uses it** — AO3 hands over a whole EPUB, so its implementation
of that verb is unusual, not typical. Worth remembering when the trait is
written, because designing it around AO3 alone would produce the wrong shape.

---

# 2. Login — the debate

You said writing actions are out: no kudos, no reviews, no posting. Good, that
removes the riskiest part. What remains is **reading your own lists**:
favourites, follows, history, bookmarks, and works restricted to logged-in
users.

## What AO3 themselves say

This matters more than my opinion. From AO3's official post on mobile apps
(admin post 3390, 2015):

> To close on a security note, if a third-party app or website requests your
> AO3 login information, please proceed cautiously and be aware that you are
> providing this information at your own risk.

And the r/AO3 subreddit auto-replies to every app question with a link to that
post plus "you shouldn't trust 3rd party apps with your login credentials."

Note what this is and is not. It is **not** a prohibition — AO3 does not ban
unofficial apps, and explicitly leaves them alone unless they impersonate AO3.
It is a warning that the user is trusting a third party. **We are that third
party.**

Also from that post, and relevant: AO3 has no public API and says one is
"several major releases away" — that was 2015 and it still does not exist. So
there is no sanctioned way to do this. Every login integration is form-posting
to their web login as if you were a browser.

## The three options

### A. No login at all

Everything public still works: browsing, searching, filters, author pages,
downloading. What you lose is your own favourites and follows, and
registered-users-only works.

Simplest and safest. And there is a real argument it is enough: Kalam has its
own library, its own shelves, its own reading list. **Your follows can live
here instead of there.** Following a fic in Kalam does not need an AO3 account
— we poll the work page and notice new chapters.

### B. Store the password

The app asks for username and password, keeps them, and logs in when needed.

This is what most of the unofficial apps do, and it is what AO3 is warning
about. Storing a reusable password on disk is the thing I would least like to
defend. Even encrypted, the app must be able to decrypt it unattended, so the
key is on the same machine.

### C. Store only a session cookie

You log in **once**, through a real browser window — we already ship WebKit, so
we can show AO3's actual login page. We never see the password. We keep the
resulting session cookie.

Meaningfully better than B:

- **We never handle your password.** You type it into AO3's own page.
- A cookie is limited: it expires, and you can revoke it by logging out on the
  site.
- No password means nothing to leak that works anywhere else.

The costs are honest ones. AO3 sessions expire — roughly two weeks, longer with
"remember me", and users report unpredictable early logouts. So you would be
re-logging-in periodically. And a session cookie is still a credential: anyone
with your machine and the file can act as you until it expires.

## Status: undecided — "we will see" (2026-09-04)

Left open deliberately. Login is stage P7e, the last one, so this does not
block anything. Recorded here so the argument does not have to be rebuilt.

## What I lean towards

**A first, C later if you want it, never B.**

Reasoning:

- Everything you named as the point of the phase — browsing, filters, author
  pages, exploring — works **without any login**. The login only adds your
  site-side lists.
- Kalam already has better versions of favourites and follows locally, and
  those work across all four sources rather than just one.
- Doing A first means P7 ships sooner and we learn whether you actually miss
  the site-side lists.
- If you do, C is the right shape, and by then WebKit-for-browsing will already
  exist for FFN — so the login window is nearly free.

**The one thing that genuinely needs login and has no local substitute** is
registered-users-only works on AO3. Some authors lock their fics. If a lot of
what you read is locked, that changes the calculation and pushes C earlier.
Only you know that.

## If we do C, the rules I would want

- Login through a real AO3 page in a WebKit window. **We never see, ask for, or
  store a password.**
- Store the cookie with the strictest permissions the OS gives us, in the
  config directory, never inside a library folder — so it cannot be copied to
  another machine by accident with the books.
- A visible "logged in as X · log out" and a log-out that actually deletes it.
- Never send the cookie anywhere except that one site.
- Read-only. No kudos, no comments, no posting — which you have already ruled
  out, and which also means a stolen cookie cannot be used through our app to
  damage your account's reputation.

---

# Questions

1. **Is A enough to start?** Or do you read enough locked works that C has to
   be there from the beginning?
2. **Temporary cache lifetime** — happy with "a few days or a size cap" rather
   than "gone when you close the app"?
3. **Long serials.** A 2,000-chapter Royal Road story cannot be read by
   downloading the whole thing. Do you want per-chapter reading for those, or
   is "download the lot, it takes a minute" acceptable?
