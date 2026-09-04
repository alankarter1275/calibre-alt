# The book grid uses too much memory — what we found, and what we could do

Notes written before any code is changed.

Two earlier attempts to decide this were based on numbers that turned out to be
measured wrong, so this time the numbers come first.

## The problem in one line

When you open the "All books" page with a big library, the app uses about
**twice as much memory** as it should.

## The numbers

We measure memory in CI. Until 4 September the test never actually opened the
All books page — it thought it did, but it stayed on Home. Now it really opens
the page, and the numbers changed a lot:

| Library size | Did the test open the book grid? | Memory used |
|---|---|---|
| 139 books | No — stayed on Home | 231 MB |
| 2,000 books | No — stayed on Home | 226 MB |
| 139 books | **Yes** | 275 MB |
| 2,000 books | **Yes** | **507 MB** |

The bottom row is the problem. 507 MB is a lot on a 4 GB machine.

Working out the cost per book:

- 1,861 extra books cost 232 MB extra
- That is about **128 KB of memory per book**

## What that 128 KB per book is actually made of

This is the part that surprised us.

Each book on the page shows a small cover image, 128 by 204 pixels. Once the
app has loaded a cover and is showing it on screen, that image sits in memory
as raw pixels:

```
128 pixels wide × 204 pixels tall × 4 bytes per pixel = 102 KB
```

So **102 KB of the 128 KB is just the cover image**. The rest — about 26 KB —
is the box, the title text, the author text and the click handler.

In other words: this is mostly a *pictures* problem, not a *boxes* problem.
That is the opposite of what we assumed yesterday.

## Why our existing safety limit did not help

The app already has a limit: it only keeps 300 cover images in its reuse pile.
That limit is real and it works. But it does not do what we thought.

Here is the catch. When the app finishes loading a cover, it hands that image
to the book's card on screen. Now **two** things are holding the same image:
the reuse pile, and the card.

When the reuse pile gets full and throws an image away, it only lets go of its
own hold. The card is still on screen and still holding the same image, so the
memory is not actually freed.

The limit controls **how many covers we can re-use quickly**. It does not
control **how many covers are sitting in memory** — that is decided by how many
cards exist. And right now the app builds one card for every book in your
library, all of them, whether or not they are on screen.

So we had two pieces of code that were each fine on their own, but together
they had no limit. That is why "memory is flat" looked true for a day: there
*was* a real, working, tested limit sitting right next to the problem, and it
made the problem look like it was already handled.

## Three ways forward

### Option A — only build cards for books you can actually see

The proper fix. Instead of making 2,000 cards, make about 30 (enough to fill
the screen) and reuse them as you scroll. Memory would stop growing with
library size. The page would also open faster — right now building the grid
takes about half a second with 2,000 books.

**What it costs:**

- This uses a part of GTK we have never used anywhere in this app before.
- It needs a fair amount of plumbing code to make our book data work with it.
- **There is a real risk of covers landing on the wrong book.** Right now each
  cover slot belongs to one book forever. If slots get reused, a cover that
  loads slowly could arrive after its slot has been given to a different book,
  and paint the wrong picture. We would need to handle that carefully.
- It changes 3 pages plus the shared card code.
- The grid currently always uses 6 columns. This approach works out the columns
  itself, so the layout would change — probably for the better, since it would
  adapt to the window size, but **it is a visible change you would need to
  look at**.

### Option B — let go of covers for books that are scrolled off screen

Keep everything else as it is. Just make the app drop a cover image when that
book scrolls out of view, and load it again when it comes back.

**What it gets:** about three quarters of the memory back — the 102 KB per
book, which is the big part.

**What it costs:**

- Changes one file. No new GTK concepts. Easy to undo.
- Does not make the page open any faster.
- Does not recover the smaller 26 KB per book.
- **Risk: covers might visibly flicker while you scroll fast** — grey box
  first, then the picture. Whether that looks bad is something only you can
  judge by using it.

### Option C — leave it alone for now

Your actual library is 139 books, which uses 275 MB. That is fine. The 507 MB
only shows up at 2,000 books, which is a test library, not yours.

The argument against: we closed this once already because "the numbers do not
justify it", and those numbers were wrong. 507 MB is a real number.

## Recommendation

**Do B first. Then measure again and decide whether A is still needed.**

Why:

- B gets most of the benefit for much less risk.
- B changes one file, so if it goes wrong it is easy to undo.
- A is a bigger change to the screen you use most, and you are the only person
  who can check whether it looks right.
- If we did A first and things improved, we would not know which part of the
  change did the improving. Doing B alone gives a clean before-and-after.

A is probably the right long-term answer. It just does not have to be first.

## We can measure whichever we pick

This is new as of yesterday and worth saying:

- CI now really opens the book grid, and proves it by taking a picture of Home
  as well and checking the two are different.
- It reports memory used at both 139 and 2,000 books, every run.
- It reports how long building the grid takes.

So whatever we choose, we will be able to see whether it worked.

## Two questions for you

1. **A, B, or C?**
2. **If B: is a brief grey flicker acceptable while scrolling fast?** This is
   the one thing that decides whether B is workable, and only you can judge it.
