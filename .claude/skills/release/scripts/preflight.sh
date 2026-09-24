#!/usr/bin/env bash
# Checks that must all pass before a crate release. Prints one line per check
# (PASS / FAIL / WARN) and exits non-zero if any check FAILs.
#
# Usage: preflight.sh --version X.Y.Z [--dry-run]
#   --version   the version about to be released (after any bump)
#   --dry-run   skip checks that only matter for a real publish
#               (crates.io credentials, GitHub auth, being in sync with origin)
set -uo pipefail

version=""
dry_run=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --version) version="$2"; shift 2 ;;
    --dry-run) dry_run=1; shift ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
[[ -n "$version" ]] || { echo "usage: preflight.sh --version X.Y.Z [--dry-run]" >&2; exit 2; }

cd "$(git rev-parse --show-toplevel)" || exit 2
failures=0
pass() { echo "PASS  $1"; }
fail() { echo "FAIL  $1"; failures=$((failures + 1)); }
warn() { echo "WARN  $1"; }

crate=$(cargo metadata --no-deps --format-version 1 2>/dev/null \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["name"])')
manifest_version=$(cargo metadata --no-deps --format-version 1 2>/dev/null \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])')

# --- repository state ---------------------------------------------------------
branch=$(git rev-parse --abbrev-ref HEAD)
[[ "$branch" == "main" ]] && pass "on branch main" || fail "on branch '$branch', expected main"

if [[ -z "$(git status --porcelain)" ]]; then
  pass "working tree clean"
else
  fail "working tree has uncommitted changes (git status --porcelain is not empty)"
fi

if git rev-parse -q --verify "refs/tags/v$version" >/dev/null; then
  fail "tag v$version already exists"
else
  pass "tag v$version is free"
fi

[[ "$manifest_version" == "$version" ]] \
  && pass "Cargo.toml version is $version" \
  || fail "Cargo.toml version is $manifest_version, expected $version"

if (( ! dry_run )); then
  if git fetch -q origin 2>/dev/null; then
    behind=$(git rev-list --count HEAD..origin/main 2>/dev/null || echo "?")
    [[ "$behind" == "0" ]] && pass "not behind origin/main" || fail "behind origin/main by $behind commit(s)"
  else
    fail "could not fetch origin"
  fi
fi

# --- OpenSpec: unfinished changes usually mean unfinished work ----------------
if [[ -d openspec ]]; then
  # openspec is often installed with Volta, which non-interactive shells don't have on PATH.
  openspec_bin=$(command -v openspec || true)
  [[ -z "$openspec_bin" && -x "$HOME/.volta/bin/openspec" ]] && openspec_bin="$HOME/.volta/bin/openspec"
  if [[ -z "$openspec_bin" ]]; then
    warn "openspec not found (not on PATH or in ~/.volta/bin); active OpenSpec changes not checked"
  else
    active=$("$openspec_bin" list --json 2>/dev/null | python3 -c \
      'import json,sys; print(" ".join(c["name"] for c in json.load(sys.stdin).get("changes", [])))' 2>/dev/null)
    [[ -z "$active" ]] && pass "no active OpenSpec changes" || warn "active OpenSpec changes: $active"
  fi
fi

# --- code quality: zero errors and zero warnings ------------------------------
if cargo fmt --check >/dev/null 2>&1; then pass "cargo fmt --check"; else fail "cargo fmt --check"; fi

if cargo clippy --all-targets --quiet -- -D warnings >/dev/null 2>&1; then
  pass "cargo clippy --all-targets -- -D warnings"
else
  fail "cargo clippy --all-targets -- -D warnings"
fi

build_warnings=$(cargo build --all-targets 2>&1 | grep -cE '^(warning|error)')
[[ "$build_warnings" == "0" ]] && pass "cargo build: no warnings" || fail "cargo build: $build_warnings warning/error line(s)"

test_out=$(cargo test 2>&1)
if [[ $? -eq 0 ]] && ! grep -qE '^warning' <<<"$test_out"; then
  pass "cargo test ($(grep -oE '[0-9]+ passed' <<<"$test_out" | awk '{s+=$1} END {print s}') tests)"
else
  fail "cargo test (failures or warnings)"
fi

# --- packaging ----------------------------------------------------------------
listing=$(cargo package --list --allow-dirty 2>/dev/null)
leaked=$(grep -E '^(openspec|spikes|docs|\.claude|target)/' <<<"$listing" | head -5)
[[ -z "$leaked" ]] && pass "package contents exclude openspec/, spikes/, docs/, .claude/" \
  || fail "package would include: $(tr '\n' ' ' <<<"$leaked")"
grep -qx 'LICENSE' <<<"$listing" && pass "package includes LICENSE" || fail "package is missing LICENSE"

if cargo publish --dry-run --allow-dirty >/dev/null 2>&1; then
  pass "cargo publish --dry-run"
else
  fail "cargo publish --dry-run (run it to see why)"
fi

# --- crates.io ------------------------------------------------------------------
status=$(curl -s -o /dev/null -w '%{http_code}' -A "$crate-release-preflight" \
  "https://crates.io/api/v1/crates/$crate/$version")
case "$status" in
  404) pass "$crate $version is not yet on crates.io" ;;
  200) fail "$crate $version is already published on crates.io" ;;
  *)   warn "could not check crates.io for $crate $version (HTTP $status)" ;;
esac

if (( ! dry_run )); then
  # Only check that credentials exist; never read or print them.
  if [[ -n "${CARGO_REGISTRY_TOKEN:-}" || -e "${CARGO_HOME:-$HOME/.cargo}/credentials.toml" \
        || -e "${CARGO_HOME:-$HOME/.cargo}/credentials" ]]; then
    pass "crates.io credentials present"
  else
    fail "no crates.io credentials: the user must run 'cargo login' in their own terminal"
  fi
  if gh auth status >/dev/null 2>&1; then pass "gh authenticated"; else fail "gh is not authenticated"; fi
fi

echo
if (( failures )); then
  echo "preflight: $failures check(s) failed"
  exit 1
fi
echo "preflight: all checks passed"
