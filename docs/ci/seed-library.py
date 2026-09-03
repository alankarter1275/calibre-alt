#!/usr/bin/env python3
"""Build a synthetic Kalam library so CI can screenshot and stress the UI.

Kalam has no fixture library, and CI has no books. Without this the app would
start on an empty Home and every screenshot would show the same "import
something" placeholder -- useless for catching the class of bug that keeps
getting through (grey covers that never fill, a float that is the wrong width).

Writes straight to the catalog schema rather than driving the importer: the
importer needs real EPUB files, and generating 2,000 valid EPUBs to look at a
grid would be slow and beside the point. The rows here are what the *reader*
would produce; anything that actually opens a book is out of scope for a
screenshot run, which is stated in docs/ci/README-screenshots.md.

Usage:
    XDG_DATA_HOME=/tmp/kalam-ci python3 seed-library.py --books 139

Keep SCHEMA_VERSION in step with src/db.rs:53. If the app ever refuses to open
a seeded database, that mismatch is the first thing to check -- the app runs
migrations forward, so a *lower* number here is safe; a higher one is not.
"""

import argparse
import os
import pathlib
import sqlite3
import struct
import sys
import uuid
import zlib
from datetime import datetime, timedelta, timezone

# Must not exceed src/db.rs SCHEMA_VERSION. Lower is fine (the app migrates up).
SCHEMA_VERSION = 14

# Deliberately varied: the point of a synthetic library is to hit the layout
# edges a tidy fixture would miss. Long titles, one-word titles, punctuation,
# and non-ASCII all changed real layout bugs in this project.
TITLES = [
    "The Silent Cartographer",
    "Dune",
    "A Very Long Title That Should Wrap Onto A Second Line And Must Not Widen The Card",
    "Ubik",
    "Le Petit Prince",
    "The Left Hand of Darkness",
    "Piranesi",
    "\u0627\u0644\u0643\u064a\u0645\u064a\u0627\u0626\u064a",  # Arabic: tests RTL + font fallback
    "I",
    "Gödel, Escher, Bach: An Eternal Golden Braid",
    "Kafka on the Shore",
    "The Wind-Up Bird Chronicle",
    "Snow Crash",
    "Neuromancer",
    "The Dispossessed",
]

AUTHORS = [
    "Ursula K. Le Guin",
    "Frank Herbert",
    "Philip K. Dick",
    "Antoine de Saint-Exup\u00e9ry",
    "Susanna Clarke",
    "Haruki Murakami",
    "Neal Stephenson",
    "William Gibson",
    "Douglas Hofstadter",
    "",  # No author at all: a real state, and it used to misalign the card.
]

SERIES = [None, "The Culture", "Earthsea", "Foundation", None, None]

DESCRIPTIONS = [
    "",  # Empty: the book float has to survive this.
    "A short one.",
    "A much longer description that exists to make the book float scroll, "
    "because the description box is a fixed height and the whole point of "
    "removing the read-more toggle was that this text should simply scroll. "
    * 3,
]

TAGS = [
    "science fiction",
    "fantasy",
    "classic",
    "to-read",
    "favourite",
    "borrowed",
    "signed first edition with a needlessly long tag name",
]


def png(width: int, height: int, rgb: tuple) -> bytes:
    """A solid-colour PNG, written by hand.

    No Pillow: adding a pip install to CI for three rectangles is not worth the
    minute it costs or the supply-chain surface. zlib and struct are stdlib.
    """
    r, g, b = rgb
    raw = b"".join(
        b"\x00" + bytes([r, g, b]) * width for _ in range(height)
    )

    def chunk(tag: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 6))
        + chunk(b"IEND", b"")
    )


def cover_colour(i: int) -> tuple:
    """Distinct per book, so a screenshot shows *which* cover went where.

    All-grey covers would hide exactly the bug we are hunting: a placeholder
    that never fills looks identical to a grey cover that did.
    """
    return (40 + (i * 37) % 200, 60 + (i * 71) % 180, 90 + (i * 113) % 160)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--books", type=int, default=139)
    ap.add_argument(
        "--no-cover-every",
        type=int,
        default=17,
        help="every Nth book gets no cover at all (a real and untidy state)",
    )
    args = ap.parse_args()

    data_home = os.environ.get("XDG_DATA_HOME")
    if not data_home:
        print("seed-library: refusing to run without XDG_DATA_HOME", file=sys.stderr)
        print("  (it would write into your real ~/.local/share/kalam)", file=sys.stderr)
        return 2

    root = pathlib.Path(data_home) / "kalam"
    library = root / "library"
    library.mkdir(parents=True, exist_ok=True)

    db = root / "catalog.db"
    if db.exists():
        db.unlink()
    conn = sqlite3.connect(db)
    conn.executescript(
        """
        CREATE TABLE schema_version (version INTEGER NOT NULL);
        CREATE TABLE books (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid          TEXT    NOT NULL UNIQUE,
            title         TEXT    NOT NULL,
            sort_title    TEXT    NOT NULL,
            authors       TEXT    NOT NULL DEFAULT '',
            series        TEXT,
            description   TEXT    NOT NULL DEFAULT '',
            format        TEXT    NOT NULL,
            file_name     TEXT    NOT NULL,
            file_hash     TEXT    NOT NULL,
            cover_name    TEXT,
            added_at      TEXT    NOT NULL,
            progress      INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE tags (
            id   INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE COLLATE NOCASE
        );
        CREATE TABLE book_tags (
            book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
            tag_id  INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (book_id, tag_id)
        );
        CREATE INDEX idx_books_title ON books(sort_title);
        CREATE INDEX idx_books_added ON books(added_at);
        CREATE INDEX idx_books_hash  ON books(file_hash);
        CREATE TABLE reading_progress (
            book_id       INTEGER PRIMARY KEY REFERENCES books(id) ON DELETE CASCADE,
            chapter_index INTEGER NOT NULL DEFAULT 0,
            fraction      REAL    NOT NULL DEFAULT 0.0,
            updated_at    TEXT    NOT NULL
        );
        """
    )
    # The app migrates forward from whatever it finds, so it will add the
    # columns and tables this script does not create (rating, publisher, the
    # P3/P4 tables). Only the ones the grid and Home read are needed here.
    conn.execute("INSERT INTO schema_version (version) VALUES (?)", (1,))

    for name in TAGS:
        conn.execute("INSERT OR IGNORE INTO tags (name) VALUES (?)", (name,))

    now = datetime.now(timezone.utc)
    covers_written = 0

    for i in range(args.books):
        book_uuid = str(uuid.uuid4())
        title = TITLES[i % len(TITLES)]
        if args.books > len(TITLES):
            title = f"{title} #{i + 1}"
        author = AUTHORS[i % len(AUTHORS)]
        series = SERIES[i % len(SERIES)]
        description = DESCRIPTIONS[i % len(DESCRIPTIONS)]
        # Spread added_at so "recently added" is not an arbitrary tie-break.
        added = (now - timedelta(hours=i)).isoformat()

        bdir = library / book_uuid
        bdir.mkdir(parents=True, exist_ok=True)
        (bdir / "book.epub").write_bytes(b"not a real epub -- screenshots only")

        cover_name = None
        if i % args.no_cover_every != 0:
            # 600x960 is roughly a real cover: big enough that decoding it is
            # honest work, which matters for the timing numbers.
            (bdir / "cover.png").write_bytes(png(600, 960, cover_colour(i)))
            cover_name = "cover.png"
            covers_written += 1

        cur = conn.execute(
            "INSERT INTO books (uuid, title, sort_title, authors, series, "
            "description, format, file_name, file_hash, cover_name, added_at, "
            "progress) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)",
            (
                book_uuid,
                title,
                title.lower(),
                author,
                series,
                description,
                "epub",
                "book.epub",
                f"hash-{i:06d}",
                cover_name,
                added,
                # A spread of progress values, so "continue reading" has
                # something in it and the progress bar has something to draw.
                (i * 7) % 101,
            ),
        )
        book_id = cur.lastrowid

        for t in range(i % 4):
            tag_id = (i + t) % len(TAGS) + 1
            conn.execute(
                "INSERT OR IGNORE INTO book_tags (book_id, tag_id) VALUES (?,?)",
                (book_id, tag_id),
            )

        # Partially-read books drive Home's "continue reading" strip, which was
        # empty in the first screenshot run and hid that its covers never
        # loaded at all.
        if 0 < (i * 7) % 101 < 100:
            conn.execute(
                "INSERT INTO reading_progress (book_id, chapter_index, "
                "fraction, updated_at) VALUES (?,?,?,?)",
                (book_id, i % 12, ((i * 7) % 101) / 100.0, added),
            )

    conn.commit()
    conn.close()

    print(f"seed-library: {args.books} books, {covers_written} covers -> {root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
