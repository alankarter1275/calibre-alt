# FicHub, FicLab, and how to get FanFiction.net

You asked me to look into FicHub and FicLab as ways round the FFN problem.
**FicHub is the answer, and it is much better than the browser idea.**

---

## FicHub has a real public API

I read their API page. It is documented, public, and does exactly what we need.

Ask it about any fic URL:

```
GET https://fichub.net/api/v0/epub?q=<the fic url>
```

It answers with metadata *and* a link to a ready-made EPUB:

```json
{
  "err": 0,
  "meta": {
    "title": "Nemesis",
    "author": "BeaconHill",
    "chapters": 14,
    "words": 65754,
    "status": "ongoing",
    "updated": "2023-01-28T23:13:19",
    "description": "..."
  },
  "urls": { "epub": "/cache/epub/NtePoQrV/Nemesis...epub?h=47bd..." }
}
```

Then you download that EPUB. There is also `/api/v0/meta` if you only want the
metadata — which is exactly the cheap "has this fic changed?" call the
auto-updater needs.

They support FanFiction.net, FictionPress, AO3, SpaceBattles, SufficientVelocity,
QuestionableQuesting, AdultFanfiction, and others.

### What this means for us

**We would not scrape FFN at all.** No Cloudflare problem, because FicHub deals
with Cloudflare on their side. We ask a JSON API a question and download a file
— the same shape as the AO3 download path we were already building.

It also handles the exact thing that makes FFN hard: they run the scraping
infrastructure, they cache, they throttle, and they keep it working when FFN
changes.

### The rules they ask you to follow

Their API page is explicit, and all of it is reasonable:

- Set a real user-agent that identifies the project and gives contact info.
- **No concurrent requests.** One at a time.
- Handle `429 Too Many Requests` and honour `Retry-After`.
- Do not try to export a big chunk of a site without talking to them first.
- The API is v0 and they make no stability promise.

We should follow every one of those. Our `Source` design already puts rate
limiting inside the source (`source-seam.md` §7), so there is a natural home
for it.

### The honest downsides

1. **It is someone else's server.** If FicHub is down or drops the API, FFN
   support stops working. That is a real dependency and it should be visible to
   the user — "FFN downloads go through fichub.net" belongs in the UI, not
   hidden.
2. **"Unstable API", their words.** v0 could change. Since it is one JSON call
   and one download, the blast radius is small.
3. **Content can be slightly stale.** They cache and refresh in the background,
   so a brand-new chapter might take a while to appear. Fine for following a
   fic; worth knowing.
4. **It is a courtesy, not a service we pay for.** All the more reason to be
   polite by default and never offer a "go faster" knob.

---

## FicLab — useful to learn from, not to use

FicLab is a browser extension. Its own store listing says it needs
"access all sites" permission so it can add a Save button and fetch story
content from the page you are on.

That means it works the way the browser idea would: it *is* a browser, so it
inherits the browser's session and passes Cloudflare naturally.

We cannot call it — there is no API, it is an extension. But it confirms two
useful things:

- **The browser-session approach genuinely works** for FFN. So our WebKit idea
  was sound, just expensive.
- **Its supported-site list is interesting:** fanfiction.net, fictionpress.com,
  fimfiction.net, and **literotica.com**. Someone else concluded Literotica is
  worth supporting and straightforward enough to do in an extension.

Also worth noting from its reviews: it lost Wattpad and Inkitt support, and
Chrome's extension rule changes threaten it. A reminder that the extension
route is fragile in ways an API is not.

---

## So what about the browser approach?

**I would not build it now**, and FicHub is why.

It was the right idea when the alternative was "FFN is impossible". Now there
is a plain HTTP path that gets us FFN, and building an entire second fetching
mechanism — heavier, slower, tied to the UI thread, unlike every other source —
to solve a problem that a JSON call already solves would be hard to justify.

**Keep it as the fallback.** If FicHub disappears, we still have WebKit in the
app and the option stays open. Worth writing down so the reasoning is not lost:
we are not rejecting it, we are deferring it because something cheaper works.

---

## Royal Road

Since it is now on the list, I checked. No official API, but the page structure
is well understood and stable enough that several tools parse it with simple
rules (`div.chapter-content` for the body). No Cloudflare challenge, no
paywall.

Notably: **FicHub does not support Royal Road.** So Royal Road is where we
actually write a parser and build an EPUB ourselves. That is good — it is the
piece of P7 that most needs proving, and Royal Road is a gentle place to prove
it.

---

## What the four sources now test, each differently

| Source | How we get text | What it proves |
|---|---|---|
| **AO3** | their own EPUB endpoint | search, filters, following |
| **Royal Road** | we parse and build the EPUB | the assembler, real parsing |
| **Literotica** | we parse and build the EPUB | messy structure, no clean chapters |
| **FanFiction.net** | FicHub API | using a third-party bridge |

That is a genuinely good spread. Three different ways of getting content, so
the `Source` design gets tested rather than assumed — which is exactly what you
asked for when you said you wanted real parsing tested.

**Suggested order:** AO3 → Royal Road → Literotica → FFN. Royal Road second
because it is the first one where we build an EPUB ourselves, and it is the
easiest place to get that wrong quietly.
