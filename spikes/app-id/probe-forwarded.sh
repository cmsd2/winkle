#!/usr/bin/env bash
# Spike: which Wayland app_ids does Chromium set, and when, for an --app window
# opened by a launch that is forwarded to an already-running browser (the normal
# case for shared-profile apps)?
# Usage: probe-forwarded.sh [url] [seconds-to-watch]
set -euo pipefail
url=${1:-https://github.com/}
watch=${2:-20}
base="$HOME/snap/chromium/common/winkle-spike"
mkdir -p "$base"
profile=$(mktemp -d "$base/profile.XXXXXX")
log="$base/wayland.log"

# 1. The "main browser": a normal window, with protocol logging.
WAYLAND_DEBUG=client /snap/bin/chromium --user-data-dir="$profile" --no-first-run \
  --no-default-browser-check about:blank 2>"$log" &
for _ in $(seq 1 40); do grep -q set_app_id "$log" && break; sleep 0.5; done
sleep 2

# 2. The launch a desktop entry performs; it is forwarded to process 1.
t0=$(date +%s.%N)
/snap/bin/chromium --user-data-dir="$profile" "--app=$url" 2>/dev/null
echo "forwarded launch returned after $(echo "$(date +%s.%N) - $t0" | bc) s; watching ${watch}s"
start_ms=$(awk -F'[][]' '/set_app_id/ {print $2}' "$log" | tail -1)
sleep "$watch"

echo "--- toplevels, app_ids and titles (ms since the browser's own window got its app_id)"
grep -E 'get_toplevel|set_app_id|set_title' "$log" \
  | awk -F'[][]' -v s="$start_ms" '{printf "%+9.0f ms  %s\n", $2 - s, $3}' \
  | sed -E 's/set_title\("([^"]{0,40})[^"]*"\)/set_title("\1…")/'

echo "Close both Chromium windows now; waiting for them to go..."
while pgrep -f -- "--user-data-dir=$profile" >/dev/null; do sleep 1; done
rm -rf "$profile" "$log"
echo "cleaned up"
