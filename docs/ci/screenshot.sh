#!/usr/bin/env bash
# Run Kalam on a headless display and photograph it.
#
# Why this exists: CI has been green through two bugs a human spotted in ten
# seconds -- Home's covers never loaded, and only two rows of the grid ever
# filled. Compiling proves the code builds, not that anything appeared on
# screen. This step produces PNGs a person (or the agent) can look at.
#
# It also writes report.txt, a plain-text summary. That matters because the
# Arena sandbox CANNOT download Actions artifacts (the blob host is
# unreachable from it -- the same reason clippy logs are committed to
# ci-logs/). Pictures are for humans; report.txt is what the agent can read.
#
# Read the caveats in docs/ci/README-screenshots.md before trusting a green
# run here. In particular the software renderer is not the renderer you use,
# so this is evidence, not proof.
set -uo pipefail

OUT="${OUT:-ci-shots}"
BOOKS="${BOOKS:-139}"
# Generous: a cold binary on a shared runner is slow to first paint, and a
# screenshot taken too early shows an empty window and looks exactly like a bug.
SETTLE="${SETTLE:-25}"

mkdir -p "$OUT"
REPORT="$OUT/report.txt"
: > "$REPORT"

# Everything interesting goes to BOTH the console and report.txt.
say() { echo "$*" | tee -a "$REPORT"; }

say "=== kalam screenshot run ==="
say "books=$BOOKS out=$OUT settle=$SETTLE"

export XDG_DATA_HOME="${XDG_DATA_HOME:-/tmp/kalam-ci-data}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp/kalam-ci-run}"
mkdir -p "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"

# No GPU on a runner. GSK's Cairo renderer is the documented software
# fallback; without this GTK4 tries GL, fails, and may render nothing.
export GSK_RENDERER="${GSK_RENDERER:-cairo}"
export LIBGL_ALWAYS_SOFTWARE=1
export GDK_BACKEND=wayland
export KALAM_TIMING=1
# GTK will happily run for ever waiting for a display that is not coming;
# these make failures loud instead of silent.
export G_MESSAGES_DEBUG="${G_MESSAGES_DEBUG:-}"

BIN="$PWD/target/release/kalam"
if [ ! -x "$BIN" ]; then
  say "FATAL: no binary at $BIN"
  exit 0   # never fail the build from here; the report says what happened
fi
say "binary: $(ls -la "$BIN" | awk '{print $5" bytes"}')"

say ""
say "=== seeding $BOOKS books ==="
if ! python3 docs/ci/seed-library.py --books "$BOOKS" 2>&1 | tee -a "$REPORT"; then
  say "FATAL: seeding failed"
  exit 0
fi

say ""
say "=== starting headless sway ==="
export WLR_BACKENDS=headless
export WLR_LIBINPUT_NO_DEVICES=1
export WLR_RENDERER=pixman          # software renderer for wlroots
export WLR_HEADLESS_OUTPUTS=1

SWAY_CONF="$(mktemp)"
cat > "$SWAY_CONF" <<'EOF'
# `xwayland disable` is load-bearing, not tidiness. Without it sway tries to
# start Xwayland, cannot find the binary on a runner, and treats that as fatal
# -- which is exactly how the first two screenshot runs died. Kalam is a native
# Wayland (GTK4) app and never needs X11, so there is nothing to lose here.
xwayland disable
output HEADLESS-1 resolution 1600x1000
default_border none
focus_follows_mouse no
EOF

say "sway binary: $(command -v sway || echo MISSING) $(sway --version 2>&1 | head -1)"

sway --config "$SWAY_CONF" > "$OUT/sway.log" 2>&1 &
SWAY_PID=$!

# Find sway's IPC socket and export SWAYSOCK ourselves.
#
# This is not belt-and-braces, it is the fix for the second failure: sway
# names its socket after its own pid and exports SWAYSOCK to processes *it*
# launches. This script is sway's parent, not its child, so it inherits
# nothing -- swaymsg then has no idea where to connect and reports the same
# "cannot connect" whether sway is healthy or dead. The first run showed
# exactly that: an empty sway.log (no errors at all) next to "sway never came
# up", which is the signature of a running compositor we simply could not
# talk to.
for _ in $(seq 1 30); do
  if [ -z "${SWAYSOCK:-}" ]; then
    CANDIDATE="$(ls -t "$XDG_RUNTIME_DIR"/sway-ipc.*.sock 2>/dev/null | head -1)"
    [ -n "$CANDIDATE" ] && export SWAYSOCK="$CANDIDATE"
  fi
  if [ -n "${SWAYSOCK:-}" ] && swaymsg -t get_version >/dev/null 2>&1; then
    break
  fi
  # A dead compositor will never produce a socket; stop waiting 30s for it.
  if ! kill -0 "$SWAY_PID" 2>/dev/null; then
    break
  fi
  sleep 1
done

say "SWAYSOCK=${SWAYSOCK:-<none found>}"
if ! swaymsg -t get_version >/dev/null 2>&1; then
  # Distinguish the two cases explicitly. Reporting "sway never came up" for
  # a sway that is alive and well cost a whole round-trip.
  if kill -0 "$SWAY_PID" 2>/dev/null; then
    say "FATAL: sway IS RUNNING but its IPC socket was unreachable."
    say "sockets present in $XDG_RUNTIME_DIR:"
    ls -la "$XDG_RUNTIME_DIR" 2>&1 | sed 's/^/  /' | tee -a "$REPORT"
  else
    say "FATAL: the sway process exited."
  fi
  say "sway log (${OUT}/sway.log):"
  if [ -s "$OUT/sway.log" ]; then
    sed 's/^/  /' "$OUT/sway.log" | tee -a "$REPORT"
  else
    say "  (empty -- sway logged nothing, which usually means it started fine)"
  fi
  exit 0
fi
say "sway up: $(swaymsg -t get_version -r | head -c 120)"

# Find the socket sway just created and point clients at it. Without this the
# app inherits no WAYLAND_DISPLAY, finds no compositor, and exits immediately
# -- which would look identical to a rendering bug in the screenshots.
if [ -z "${WAYLAND_DISPLAY:-}" ]; then
  SOCK="$(ls -t "$XDG_RUNTIME_DIR"/wayland-* 2>/dev/null \
          | grep -v '\.lock$' | head -1)"
  if [ -n "$SOCK" ]; then
    export WAYLAND_DISPLAY="$(basename "$SOCK")"
  fi
fi
say "WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-<unset>}"
if [ -z "${WAYLAND_DISPLAY:-}" ]; then
  say "FATAL: sway is running but published no wayland socket in $XDG_RUNTIME_DIR"
  ls -la "$XDG_RUNTIME_DIR" | sed 's/^/  /' | tee -a "$REPORT"
  swaymsg exit >/dev/null 2>&1 || true
  exit 0
fi

shot() { # shot <name>
  local name="$1"
  if grim "$OUT/$name.png" 2>>"$OUT/grim.log"; then
    say "  shot: $name ($(stat -c%s "$OUT/$name.png") bytes)"
  else
    say "  shot FAILED: $name -- $(tail -1 "$OUT/grim.log" 2>/dev/null)"
  fi
}

# (The Tab/Tab/Return `key()` helper was removed on 2026-09-04: navigation
# is requested with KALAM_ROUTE now, so nothing needs synthetic keystrokes.)

# Sample the app's own memory. `/usr/bin/time` cannot help here: sway starts
# kalam, so it is not a child of this script and its RSS is never reported.
# Peak RSS is the one number from a CI runner worth trusting -- wall-clock
# timings on a shared VM are noise, but memory is memory. It is also the
# number that decides whether the unbounded cover cache actually matters.
PEAK_KB=0
sample_rss() {
  local pid rss
  pid="$(pgrep -n -x kalam 2>/dev/null || true)"
  [ -z "$pid" ] && return 0
  rss="$(awk '/^VmRSS:/ {print $2}' "/proc/$pid/status" 2>/dev/null || true)"
  [ -z "$rss" ] && return 0
  [ "$rss" -gt "$PEAK_KB" ] && PEAK_KB="$rss"
  return 0
}

say ""
say "=== launching kalam ==="
# Run it directly rather than via `swaymsg exec`, so we own the process and
# can read its stderr. KALAM_TIMING output lands in the log, which is how the
# agent sees grid_build / covers_queued without downloading an artifact.
#
# ROUTE names a page for the app to open on its own (KALAM_ROUTE, added
# 2026-09-04). Previously this script sent Tab/Tab/Return and hoped the focus
# order was what it guessed; when it was not, the run photographed Home three
# times and still reported success.
ROUTE="${ROUTE:-all-books}"
say "requested route: $ROUTE"
KALAM_ROUTE="$ROUTE" "$BIN" > "$OUT/kalam.log" 2>&1 &
APP_PID=$!

for _ in $(seq 1 "$SETTLE"); do
  sample_rss
  sleep 1
done

if ! kill -0 "$APP_PID" 2>/dev/null; then
  say "FATAL: kalam exited during startup. Its output:"
  sed 's/^/  /' "$OUT/kalam.log" | tail -40 | tee -a "$REPORT"
  swaymsg exit >/dev/null 2>&1 || true
  exit 0
fi

WINDOWS="$(swaymsg -t get_tree | grep -c '"app_id"' || true)"
say "toplevel windows seen: $WINDOWS"
swaymsg -t get_tree > "$OUT/tree.json" 2>/dev/null || true

shot "01-home"

# The app navigates itself now, so there is nothing to send. Focus is still
# worth setting: without it some GTK paint paths behave differently under a
# headless compositor, and an unfocused window is not what a user sees.
swaymsg '[app_id=".*"] focus' >/dev/null 2>&1 \
  || swaymsg focus >/dev/null 2>&1 || true
say "focused: $(swaymsg -t get_tree | grep -c '"focused": true' || echo 0)"

for _ in $(seq 1 6); do sample_rss; sleep 1; done
shot "02-after-nav"
# Covers arrive in the background, so the interesting screenshot is the later
# one: anything still grey here is a cover that is never coming.
for _ in $(seq 1 12); do sample_rss; sleep 1; done
shot "03-after-nav-settled"

# Three checks, because each catches a different way this can quietly fail.
NAV_OK=1

# 1. Did the app accept the route? It prints on both paths, so silence means
#    a binary built before KALAM_ROUTE existed.
if grep -q "KALAM_ROUTE=$ROUTE — navigating" "$OUT/kalam.log" 2>/dev/null; then
  say "navigation: app accepted route '$ROUTE'"
elif grep -q "unknown route" "$OUT/kalam.log" 2>/dev/null; then
  say "navigation: FAILED -- app rejected '$ROUTE' as unknown"
  grep "known:" "$OUT/kalam.log" | sed 's/^/    /' | tee -a "$REPORT"
  NAV_OK=0
else
  say "navigation: FAILED -- no KALAM_ROUTE line; binary predates the flag?"
  NAV_OK=0
fi

# 2. Did a grid actually build? Only build_book_grid emits this, so it is
#    positive proof the page rendered rather than merely being asked for.
if grep -q "grid_build" "$OUT/kalam.log" 2>/dev/null; then
  say "navigation: grid_build seen -- a grid page rendered"
else
  say "navigation: WARNING -- no grid_build line."
  say "  Expected for routes that are not grids; suspicious for '$ROUTE'."
  NAV_OK=0
fi

# 3. Are the shots actually different? This is the check that would have
#    caught the original defect on its own: three identical files mean the
#    camera worked and nothing else did. Compared explicitly because the
#    evidence was sitting in the directory listing last time and was read
#    past (pitfalls §19).
HOME_SUM="$(md5sum "$OUT/01-home.png" 2>/dev/null | cut -d" " -f1)"
NAV_SUM="$(md5sum "$OUT/03-after-nav-settled.png" 2>/dev/null | cut -d" " -f1)"
if [ -n "$HOME_SUM" ] && [ "$HOME_SUM" = "$NAV_SUM" ]; then
  say "navigation: FAILED -- 01-home and 03-after-nav-settled are byte-identical."
  say "  The app never left Home; every screenshot below shows the same page."
  NAV_OK=0
elif [ -n "$HOME_SUM" ]; then
  say "navigation: screenshots differ, so the page did change"
fi

if [ "$NAV_OK" = "1" ]; then
  say "navigation: OK"
else
  say "navigation: NOT PROVEN -- treat the images below as Home until checked"
fi

say ""
say "=== timing lines from the app ==="
# The whole point of KALAM_TIMING. Reproduced in the report so the numbers are
# readable without downloading anything.
grep -E "^\[timing\]" "$OUT/kalam.log" | sed 's/^/  /' | tee -a "$REPORT" \
  || say "  (none -- KALAM_TIMING produced no output)"

say ""
say "=== did the covers actually load? ==="
# The seeded covers are deliberately colourful; the placeholder is grey. So
# "how many strongly-coloured pixels are there" is a machine-checkable proxy
# for "did the covers appear", and it does not need a human to squint at a PNG.
python3 docs/ci/check-shot.py "$OUT"/0*.png 2>&1 | tee -a "$REPORT" \
  || say "  (cover check failed to run)"

say ""
say "=== screenshot fingerprints ==="
# Identical checksums mean the screen never changed -- the single cheapest
# signal that navigation did nothing, and the one that would have caught this
# on the first working run.
md5sum "$OUT"/0*.png 2>/dev/null | sed 's/^/  /' | tee -a "$REPORT" \
  || say "  (no shots to fingerprint)"

say ""
say "=== app stderr (non-timing lines) ==="
grep -vE "^\[timing\]" "$OUT/kalam.log" 2>/dev/null | tail -20 | sed 's/^/  /' \
  | tee -a "$REPORT" || say "  (none)"

say ""
say "=== peak memory ==="
printf 'books=%s peak_rss_kb=%s peak_rss_mb=%s\n' \
  "$BOOKS" "$PEAK_KB" "$((PEAK_KB / 1024))" | tee -a "$REPORT"
if [ "$PEAK_KB" -eq 0 ]; then
  say "WARNING: never sampled a running kalam process -- treat all of the"
  say "         above as meaningless."
fi

say ""
say "=== shutting down ==="
kill "$APP_PID" 2>/dev/null || true
swaymsg exit >/dev/null 2>&1 || true
sleep 2
kill "$SWAY_PID" 2>/dev/null || true
wait "$SWAY_PID" 2>/dev/null || true

say ""
say "=== artifacts ==="
ls -la "$OUT" | sed 's/^/  /' | tee -a "$REPORT"
# Never fail the build on a screenshot problem: this step is diagnostic, and a
# flaky compositor must not block a correct code change.
exit 0
