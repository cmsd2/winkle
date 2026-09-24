# Tasks

## 1. Name constant and code rename

- [ ] 1.1 Add `paths::APP_NAME = "winkle"` and derive every name from it: desktop file prefix, icon name, profile root, notification app name, browser override `WINKLE_BROWSER`, and entry key prefix `X-Winkle-`. Verify: `grep -rni hermit src/` returns nothing, and unit tests assert the derived names (`winkle-github-com.desktop`, `~/snap/chromium/common/winkle/<id>`).
- [ ] 1.2 Rename the package and binary to `winkle` in Cargo.toml and clap's command name; update help text and user-facing messages. Verify: `cargo run -- --help` shows `Usage: winkle <COMMAND>`, and `cargo build` produces `target/debug/winkle`.
- [ ] 1.3 Update the integration tests: `CARGO_BIN_EXE_winkle`, `WINKLE_BROWSER`, expected paths, `X-Winkle-*` keys, `--app-name=winkle`. Add tests for the new scenarios: a `hermit-*.desktop` entry is not listed, and `winkle remove <id>` refuses when only `hermit-<id>.desktop` exists. Verify: `cargo test` passes, and `grep -ni hermit tests/cli.rs` hits only those two new tests.

## 2. Crate metadata

- [ ] 2.1 Add `readme`, `keywords`, `categories` and an `include` list (src, tests, README.md, LICENSE, Cargo.lock) to Cargo.toml. Verify: `cargo package --list` shows no `openspec/`, `spikes/`, `.claude/` or `docs/` files, and `cargo publish --dry-run` succeeds.

## 3. Docs

- [ ] 3.1 Rename throughout README.md, and add a "Switching from hermit" section with the four switch-over steps from design decision 2. Verify: every `winkle` command block in the README runs as written in a temp `HOME`/`XDG_DATA_HOME` (same check as mvp task 9.1), and the switch-over section's commands are syntactically valid.
- [ ] 3.2 Rename in CLAUDE.md, docs/roadmap.md, spikes/app-id (probe.sh scratch dir → `winkle-spike`, FINDINGS) and spikes/eme/FINDINGS.md. Leave `openspec/changes/archive/` and docs/acceptance/mvp.md as history. Verify: `grep -rli hermit` outside `target/`, `openspec/changes/archive/`, docs/acceptance/ and this change finds only intentional history mentions (the README switch-over section and the specs' old-name scenarios).
- [ ] 3.3 Update the Purpose text of the `app-list` and `app-removal` main specs, which mention hermit; deltas can't change Purpose. Verify: `grep -i hermit openspec/specs/*/spec.md` after archive/sync hits only the two old-name scenarios.

## 4. Checks, commit and push

- [ ] 4.1 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass with zero warnings; commit and push to `origin` (still `cmsd2/hermit`). Verify: all three exit 0 and `git status -sb` shows `main...origin/main` with nothing ahead.

## 5. Switch over on this machine (with the user)

- [ ] 5.1 With the old binary, `hermit remove` GitHub, HEY and Spotify, keeping HEY's data; then `mv ~/snap/chromium/common/hermit/app-hey-com ~/snap/chromium/common/winkle/app-hey-com`. Verify: `hermit list` shows no apps, and the HEY profile exists under `winkle/`.
- [ ] 5.2 `cargo install --path .`, reinstall the three apps with `winkle` (HEY with `--isolated`), then `cargo uninstall hermit`. Verify: `winkle list` shows all three, `command -v hermit` finds nothing, and the user confirms HEY is still logged in.

## 6. Hosting renames

- [ ] 6.1 Rename the GitHub repo with `gh repo rename winkle`, update `origin`, and set the repository URL in Cargo.toml. Verify: `gh repo view cmsd2/winkle` works, `git remote get-url origin` is `https://github.com/cmsd2/winkle.git`, and the old URL redirects. Commit and push the Cargo.toml change after the checks in 4.1.
- [ ] 6.2 Last: move `~/Development/hermit` to `~/Development/winkle` and move the session there. Verify: `git -C ~/Development/winkle status -sb` is clean and tracks `origin/main`, and `cargo test` passes from the new path.
