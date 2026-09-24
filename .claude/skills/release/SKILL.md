---
name: release
description: Release this Rust crate (winkle) to crates.io, end to end. Picks the next semver version, updates CHANGELOG.md, runs the zero-warning preflight checks, commits and tags vX.Y.Z, pushes, runs cargo publish, and creates the matching GitHub release. Use this whenever the user wants to release, publish, ship, cut, or tag a version of the crate, bump the version for a release, publish to crates.io, or do a release dry run, even if they don't say "skill" or name every step.
---

# Release to crates.io

Publishing is permanent: a version on crates.io can be yanked but never deleted or
replaced. So this workflow is built around two ideas:

- **Check everything before anything leaves the machine.** The preflight script
  enforces the project rule that fmt, clippy and tests pass with zero errors *and*
  zero warnings, and that the package contains only what it should.
- **The user approves the irreversible part explicitly.** Local steps (version
  bump, changelog, commit, tag) are cheap to undo. Pushing, `cargo publish` and the
  GitHub release are not. Stop and get a clear "yes" right before them, even if the
  user asked for a release earlier in the conversation. Approval for one release
  doesn't carry over to another.

## Modes

- **Real release** (default): all steps.
- **Dry run**: the user says "dry run", "rehearse", "test the release" or similar.
  Do every local step, finish with `cargo publish --dry-run`, and never push,
  publish or create a GitHub release. Say clearly at the end that nothing left the
  machine.

## Steps

### 1. Work out the version

First run `git status --porcelain`. If anything is uncommitted, stop here and ask the
user (see "Uncommitted changes" under step 3).

```bash
git describe --tags --abbrev=0 2>/dev/null    # last release tag, if any
cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])'
```

- **No tags yet (first release):** release the version already in Cargo.toml.
  Don't bump.
- **Otherwise:** read the commits since the last tag
  (`git log --no-merges --format='%h %s%n%b' <last-tag>..HEAD`) and recommend a bump
  using Cargo's semver rules. While the major version is 0, a breaking change bumps
  the **minor** version (0.1.x → 0.2.0), and new features or fixes bump the patch
  version. If the user named a version or bump level, use that. Otherwise tell the
  user your recommendation and why, and ask.
- If there are no commits since the last tag, there is nothing to release. Say so
  and stop.

Bump with `cargo set-version <X.Y.Z>` (from cargo-edit), which updates Cargo.toml and
Cargo.lock together.

### 2. Update CHANGELOG.md

Use the [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format. If the file
doesn't exist, create it with the standard header, and use the full history for the
first entry.

Write for people who *use* the tool, not for the people who built it. Commit
messages are raw material, not text to paste:
- group entries under Added / Changed / Deprecated / Removed / Fixed / Security
- mark breaking changes clearly
- leave out internal-only commits (spikes, planning docs, refactors with no visible
  effect, test-only changes)

The new section is `## [X.Y.Z] - YYYY-MM-DD`, using today's date. Add the matching
link references at the bottom:
- `[X.Y.Z]: https://github.com/cmsd2/winkle/releases/tag/vX.Y.Z` for the first release
- a compare link (`.../compare/vPREV...vX.Y.Z`) for later releases

Show the user the new section and let them adjust it before continuing. It becomes
the public release notes.

### 3. Preflight

Run `scripts/preflight.sh` from this skill's directory, from inside the repository:

```bash
<skill-dir>/scripts/preflight.sh --version X.Y.Z [--dry-run]
```

It expects a clean working tree, so commit first (step 4), then run it. If any check
FAILs, stop and fix the cause, then run it again. Don't work around a failing check.
WARN lines are for the user to judge; an active OpenSpec change, for example, usually
means work isn't finished.

**Uncommitted changes that aren't part of the release belong to the user.** They may
be work in progress, or something that must not ship. Don't stash, commit, discard or
`--allow-dirty` past them. Stop, show the user what's uncommitted (`git status`,
`git diff --stat`), and let them decide. Check this before bumping the version, so the
release commit can only ever contain Cargo.toml, Cargo.lock and CHANGELOG.md.

### 4. Commit and tag (local)

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "Release vX.Y.Z"      # plus the repository's usual commit trailers
```

Then run preflight (step 3). When it passes:

```bash
git tag -a vX.Y.Z -m "winkle vX.Y.Z"
```

A dry run stops here. Report what was done and what a real release would do next,
and point out that the local commit and tag can be dropped with
`git tag -d vX.Y.Z && git reset --hard HEAD~1` if the rehearsal ran in the real repo.

### 5. Confirm, then publish

Summarise for the user:
- the version
- the changelog section
- the preflight result
- exactly what happens next: push `main` and the tag to `origin`, `cargo publish`,
  and a GitHub release

Then ask for explicit approval. Only after a clear yes:

```bash
git push origin main
git push origin vX.Y.Z
cargo publish
gh release create vX.Y.Z --title "winkle vX.Y.Z" --notes-file <file with the changelog section>
```

Check the result on crates.io
(`curl -s -A winkle-release https://crates.io/api/v1/crates/winkle/X.Y.Z`; indexing can
take a minute) and give the user the crates.io and GitHub release URLs.

## When something goes wrong

- **No crates.io credentials:** ask the user to run `cargo login` in their own
  terminal. The token is a secret. Never ask for it, paste it, or read credential
  files.
- **`cargo publish` fails after the tag was pushed:** the tag stays. Fix the problem
  and run `cargo publish` again for the same version, rather than moving or deleting a
  published tag. Only if the fix needs a code change, release it as the next patch
  version.
- **A bad version is already on crates.io:** it can't be replaced. Offer
  `cargo yank --version X.Y.Z` plus a fixed patch release, but only yank if the user
  asks. Yanking stops new projects depending on that version; it doesn't delete it.
