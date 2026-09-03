#!/usr/bin/env bash
# Run Kalam on a headless display and photograph it.
#
# Why this exists: CI has been green through two bugs a human spotted in ten
# seconds -- Home's covers never loaded, and only two rows of the grid ever
# filled. Compiling proves the code builds, not that anything appeared on
# screen. This step produces PNGs a person (or the agent) can look at.
#
# Read the caveats in docs/ci/README-screenshots.md before trusting a green
# run here. In particular the software renderer is not the renderer you use,
# so this is evidence, not proof.
set -uo pipefail

OUT="${OUT:-ci-shots}"
BOOKS="${BOOKS:-139}"
# Generous: a cold cargo-built binary on a shared runner is slow to first
# paint, and a screenshot taken too early shows an empty window and looks
# exactly like a bug.
SETTLE="${SETTLE:-25}"

mkdir -p "$OUT"

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
# Deterministic text: the runner's font set is not yours, and font choice
# changes line wrapping, which changes what a layout screenshot proves.
export FONTCONFIG_FILE="${FONTCONFIG_FILE:-}"

echo "=== seeding $BOOKS books ==="
python3 docs/ci/seed-library.py --books "$BOOKS" || exit 1

echo "=== starting headless sway ==="
export WLR_BACKENDS=headless
export WLR_LIBINPUT_NO_DEVICES=1
export WLR_RENDERER=pixman          # software renderer for wlroots
export WLR_HEADLESS_OUTPUTS=1

SWAY_CONF="$(mktemp)"
cat > "$SWAY_CONF" <<'EOF'
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
  echo "sway did not come up; log follows" >&2
  cat "$OUT/sway.log" >&2
  exit 1
fi
echo "sway is up: $(swaymsg -t get_version | head -c 200)"

shot() { # shot <name>
  local name="$1"
  if grim "$OUT/$name.png" 2>>"$OUT/grim.log"; then
    echo "  shot: $name"
  else
    echo "  shot FAILED: $name" >&2
  fi
}

key() { swaymsg exec "wtype -k $1" >/dev/null 2>&1 || true; sleep 1; }

echo "=== launching kalam ==="
swaymsg exec -- "$PWD/target/release/kalam" > /dev/null 2>&1

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

for _ in $(seq 1 "$SETTLE"); do
  sample_rss
  sleep 1
done

# If no window ever appeared, say so loudly -- an empty screenshot is the most
# misleading artifact this script can produce.
WINDOWS="$(swaymsg -t get_tree | grep -c '"app_id"' || true)"
echo "toplevels seen: $WINDOWS"
swaymsg -t get_tree > "$OUT/tree.json" 2>/dev/null || true

shot "01-home"

# Home -> All books. The app has no CLI navigation, so this is keyboard-driven
# and inherently brittle; the screenshots are still worth having, and a wrong
# page is obvious in the image rather than silently passing.
key Tab; key Tab; key Return
for _ in $(seq 1 6); do sample_rss; sleep 1; done
shot "02-after-nav"
# Covers arrive in the background, so the interesting screenshot is the later
# one: anything still grey here is a cover that is never coming.
for _ in $(seq 1 12); do sample_rss; sleep 1; done
shot "03-after-nav-settled"

echo "=== shutting down ==="
swaymsg exit >/dev/null 2>&1 || true
sleep 2
kill "$SWAY_PID" 2>/dev/null || true
wait "$SWAY_PID" 2>/dev/null || true

echo "=== peak memory ==="
# Written to a file as well as stdout so it survives as an artifact.
printf 'books=%s peak_rss_kb=%s peak_rss_mb=%s\n' \
  "$BOOKS" "$PEAK_KB" "$((PEAK_KB / 1024))" | tee "$OUT/peak-rss.txt"
if [ "$PEAK_KB" -eq 0 ]; then
  echo "WARNING: never sampled a running kalam process -- the app probably" >&2
  echo "         never started. Treat the screenshots as meaningless." >&2
fi

echo "=== artifacts ==="
ls -la "$OUT"
# Never fail the build on a screenshot problem: this step is diagnostic, and a
# flaky compositor must not block a correct code change.
exit 0
