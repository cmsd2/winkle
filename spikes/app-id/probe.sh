#!/usr/bin/env bash
# Spike: report the Wayland app_id Chromium sets for an --app window.
# Usage: probe.sh <url> [extra chromium args...]
#   PROBE_SHARED=1 probe.sh <url>   use the real default profile (request may be
#                                   forwarded to an already-running Chromium)
set -euo pipefail
url=${1:?usage: probe.sh <url> [chromium args...]}
shift

log=$(mktemp)
profile=""
args=(--no-first-run --no-default-browser-check "--app=$url" "$@")
if [[ -z "${PROBE_SHARED:-}" ]]; then
  base="$HOME/snap/chromium/common/hermit-spike"
  mkdir -p "$base"
  profile=$(mktemp -d "$base/profile.XXXXXX")
  args=("--user-data-dir=$profile" "${args[@]}")
fi

WAYLAND_DEBUG=client /snap/bin/chromium "${args[@]}" 2>"$log" &
pid=$!

for _ in $(seq 1 40); do
  grep -q 'set_app_id' "$log" && break
  sleep 0.5
done

grep -o 'set_app_id("[^"]*")' "$log" | sed 's/set_app_id("\(.*\)")/\1/' | sort -u || echo "(no set_app_id seen)"

# hermit's processes cannot signal snap-confined Chromium (EPERM, even outside
# the Claude sandbox), so the window has to be closed by hand. Wait for that,
# then delete the throwaway profile.
if [[ -n "$profile" ]]; then
  echo "Close the probe window to finish (profile: $profile)" >&2
  while pgrep -f -- "--user-data-dir=$profile" >/dev/null; do sleep 1; done
  rm -rf "$profile"
fi
rm -f "$log"
