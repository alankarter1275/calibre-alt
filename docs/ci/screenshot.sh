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

sway --config "$SWAY_CONF" > "$OUT/sway.log" 2>&1 &
SWAY_PID=$!

# Wait for the compositor to publish a socket rather than sleeping blindly.
for _ in $(seq 1 30); do
  if swaymsg -t get_version >/dev/null 2>&1; then break; fi
  sleep 1
done
if ! swaymsg -t get_version >/dev/null 2>&1; then
  say "FATAL: sway never came up. Its log:"
  sed 's/^/  /' "$OUT/sway.log" | tee -a "$REPORT"
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

key() { swaymsg exec "wtype -k $1" >/dev/null 2>&1 || true; sleep 1; }

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
"$BIN" > "$OUT/kalam.log" 2>&1 &
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

# Home -> All books. The app has no CLI navigation, so this is keyboard-driven
# and inherently brittle; a wrong page is obvious in the image rather than
# silently passing.
key Tab; key Tab; key Return
for _ in $(seq 1 6); do sample_rss; sleep 1; done
shot "02-after-nav"
# Covers arrive in the background, so the interesting screenshot is the later
# one: anything still grey here is a cover that is never coming.
for _ in $(seq 1 12); do sample_rss; sleep 1; done
shot "03-after-nav-settled"

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
