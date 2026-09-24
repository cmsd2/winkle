//! End-to-end tests of the `winkle` binary. Every run gets a temp HOME and
//! XDG_DATA_HOME, and stub `gsettings`, `zenity`, `notify-send`,
//! `update-desktop-database` and browser scripts first on PATH, so the real
//! desktop is never touched.

use std::fs;
use std::io::Cursor;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;

use assert_cmd::prelude::*;
use httpmock::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

struct Env {
    _dir: TempDir,
    home: PathBuf,
    data: PathBuf,
    stubs: PathBuf,
    log: PathBuf,
}

const GSETTINGS: &str = r#"#!/bin/sh
echo "gsettings $*" >> "$STUB_LOG"
case "$1" in
  get) if [ -f "$STUB_DIR/favorites" ]; then cat "$STUB_DIR/favorites"; else echo "@as []"; fi ;;
  set) printf '%s\n' "$4" > "$STUB_DIR/favorites" ;;
esac
"#;

const ZENITY: &str = r#"#!/bin/sh
echo "zenity $*" >> "$STUB_LOG"
[ -f "$STUB_DIR/zenity.stdout" ] && cat "$STUB_DIR/zenity.stdout"
exit "$(cat "$STUB_DIR/zenity.exit" 2>/dev/null || echo 0)"
"#;

const LOGGER: &str = r#"#!/bin/sh
echo "$(basename "$0") $*" >> "$STUB_LOG"
"#;

impl Env {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let data = dir.path().join("data");
        let stubs = dir.path().join("stubs");
        let log = dir.path().join("stub.log");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&stubs).unwrap();
        fs::write(&log, "").unwrap();
        for (name, script) in [
            ("gsettings", GSETTINGS),
            ("zenity", ZENITY),
            ("notify-send", LOGGER),
            ("update-desktop-database", LOGGER),
            ("chromium", LOGGER),
        ] {
            let path = stubs.join(name);
            fs::write(&path, script).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        Self {
            _dir: dir,
            home,
            data,
            stubs,
            log,
        }
    }

    fn winkle(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_winkle"));
        cmd.env_clear()
            .env("HOME", &self.home)
            .env("XDG_DATA_HOME", &self.data)
            .env("WINKLE_BROWSER", self.stubs.join("chromium"))
            .env("PATH", format!("{}:/usr/bin:/bin", self.stubs.display()))
            .env("STUB_DIR", &self.stubs)
            .env("STUB_LOG", &self.log);
        cmd
    }

    fn desktop_file(&self, id: &str) -> PathBuf {
        self.data.join(format!("applications/winkle-{id}.desktop"))
    }

    fn icon_png(&self, id: &str) -> PathBuf {
        self.data
            .join(format!("icons/hicolor/256x256/apps/winkle-{id}.png"))
    }

    fn profile_dir(&self, id: &str) -> PathBuf {
        self.home.join(format!("snap/chromium/common/winkle/{id}"))
    }

    fn log(&self) -> String {
        fs::read_to_string(&self.log).unwrap()
    }

    fn favourites(&self) -> String {
        fs::read_to_string(self.stubs.join("favorites")).unwrap_or_default()
    }

    fn zenity_answers(&self, exit: i32, stdout: &str) {
        fs::write(self.stubs.join("zenity.exit"), exit.to_string()).unwrap();
        fs::write(self.stubs.join("zenity.stdout"), stdout).unwrap();
    }

    fn install(&self, url: &str, id: &str, extra: &[&str]) {
        self.winkle()
            .args(["install", url, "--id", id])
            .args(extra)
            .assert()
            .success();
    }
}

fn png(size: u32) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::new_rgba8(size, size)
        .write_to(&mut out, image::ImageFormat::Png)
        .unwrap();
    out.into_inner()
}

/// A site with a manifest (name, start_url, icons, shortcuts).
fn manifest_site() -> MockServer {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/");
        then.status(200).header("content-type", "text/html").body(
            r#"<html><head><title>Example</title><link rel="manifest" href="/manifest.json"></head></html>"#,
        );
    });
    server.mock(|when, then| {
        when.method(GET).path("/manifest.json");
        then.status(200).body(
            r##"{"name": "Example App", "start_url": "/app?source=pwa", "theme_color": "#224466",
                "icons": [{"src": "/icon-512.png", "sizes": "512x512", "type": "image/png"}],
                "shortcuts": [{"name": "Compose", "url": "/compose"}]}"##,
        );
    });
    server.mock(|when, then| {
        when.method(GET).path("/icon-512.png");
        then.status(200)
            .header("content-type", "image/png")
            .body(png(512));
    });
    server
}

/// A site with no metadata beyond a title.
fn bare_site(title: &str) -> MockServer {
    let server = MockServer::start();
    let body = format!("<html><head><title>{title}</title></head></html>");
    server.mock(move |when, then| {
        when.method(GET).path("/");
        then.status(200).body(body.clone());
    });
    server
}

fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(files_under(&path));
            } else {
                out.push(path);
            }
        }
    }
    out
}

fn hostname() -> String {
    fs::read_to_string("/proc/sys/kernel/hostname")
        .unwrap()
        .trim()
        .to_string()
}

// ---- install -------------------------------------------------------------

#[test]
fn install_writes_a_valid_entry_and_icon() {
    let env = Env::new();
    let site = manifest_site();
    let port = site.port();
    env.winkle()
        .args(["install", &site.url("/"), "--id", "example"])
        .assert()
        .success()
        .stdout("Installed Example App (example)\n");

    let entry = fs::read_to_string(env.desktop_file("example")).unwrap();
    let chromium = env.stubs.join("chromium");
    for expected in [
        "Name=Example App".to_string(),
        format!(
            "Exec={} --profile-directory=Default \"--app=http://127.0.0.1:{port}/app?source=pwa\"",
            chromium.display()
        ),
        "StartupWMClass=chrome-127.0.0.1__app-Default".into(),
        "Icon=winkle-example".into(),
        "Keywords=127.0.0.1;".into(),
        "Actions=shortcut-1;uninstall;".into(),
        "X-Winkle-Id=example".into(),
        "X-Winkle-Profile=shared".into(),
        "[Desktop Action shortcut-1]\nName=Compose".into(),
        format!(
            "Exec={} --profile-directory=Default --app=http://127.0.0.1:{port}/compose",
            chromium.display()
        ),
    ] {
        assert!(
            entry.contains(&expected),
            "missing {expected:?} in:\n{entry}"
        );
    }

    // Task 6.4: the Uninstall action runs this very binary by absolute path.
    let exe = env!("CARGO_BIN_EXE_winkle");
    assert!(Path::new(exe).is_absolute());
    assert!(
        entry.contains(&format!(
            "[Desktop Action uninstall]\nName=Uninstall\nExec={exe} remove example --interactive\n"
        )),
        "{entry}"
    );

    let icon = image::open(env.icon_png("example")).unwrap();
    assert_eq!((icon.width(), icon.height()), (256, 256));

    let status = Command::new("desktop-file-validate")
        .arg(env.desktop_file("example"))
        .output()
        .unwrap();
    assert!(
        status.status.success() && status.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&status.stdout)
    );

    assert!(
        env.log().contains("update-desktop-database -q"),
        "{}",
        env.log()
    );
    assert!(
        !env.profile_dir("example").exists(),
        "shared apps get no profile dir"
    );
}

#[test]
fn isolated_install_gets_its_own_profile() {
    let env = Env::new();
    let site = bare_site("Isolated");
    env.install(&site.url("/"), "iso", &["--isolated"]);
    let entry = fs::read_to_string(env.desktop_file("iso")).unwrap();
    assert!(
        entry.contains(&format!(
            "--user-data-dir={}",
            env.profile_dir("iso").display()
        )),
        "{entry}"
    );
    assert!(entry.contains("X-Winkle-Profile=isolated"));
    assert!(env.profile_dir("iso").is_dir());
}

#[test]
fn site_without_icons_gets_a_placeholder_and_a_warning() {
    let env = Env::new();
    let site = bare_site("Plain Site");
    env.winkle()
        .args(["install", &site.url("/"), "--id", "plain"])
        .assert()
        .success()
        .stderr(predicate::str::contains("placeholder"));
    assert!(env.icon_png("plain").exists());
    assert!(
        env.data
            .join("icons/hicolor/scalable/apps/winkle-plain.svg")
            .exists()
    );
    let entry = fs::read_to_string(env.desktop_file("plain")).unwrap();
    assert!(entry.contains("Name=Plain Site"));
}

#[test]
fn dry_run_prints_the_plan_and_writes_nothing() {
    let env = Env::new();
    let site = manifest_site();
    env.winkle()
        .args([
            "install",
            &site.url("/"),
            "--id",
            "example",
            "--isolated",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Would install Example App (example)")
                .and(predicate::str::contains("profile:      isolated"))
                .and(predicate::str::contains(
                    "window class: chrome-127.0.0.1__app-Default",
                ))
                .and(predicate::str::contains("(512×512)"))
                .and(predicate::str::contains("Compose → "))
                .and(predicate::str::contains(
                    env.desktop_file("example").to_str().unwrap(),
                ))
                .and(predicate::str::contains(
                    env.profile_dir("example").to_str().unwrap(),
                )),
        );
    assert!(
        files_under(&env.data).is_empty(),
        "{:?}",
        files_under(&env.data)
    );
    assert!(!env.home.join("snap").exists());
}

#[test]
fn invalid_id_is_rejected_before_anything_happens() {
    let env = Env::new();
    env.winkle()
        .args(["install", "http://127.0.0.1:1/", "--id", "My App"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid id `My App`"));
    assert!(files_under(&env.data).is_empty());
}

#[test]
fn non_web_scheme_is_rejected() {
    let env = Env::new();
    env.winkle()
        .args(["install", "file:///etc/passwd"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("`file`"));
}

#[test]
fn unreachable_site_needs_name() {
    let env = Env::new();
    env.winkle()
        .args(["install", "http://127.0.0.1:1/", "--id", "down"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--name"));
    assert!(files_under(&env.data).is_empty());

    env.winkle()
        .args([
            "install",
            "http://127.0.0.1:1/",
            "--id",
            "down",
            "--name",
            "Intranet",
        ])
        .assert()
        .success()
        .stdout("Installed Intranet (down)\n");
}

#[test]
fn duplicate_install_needs_force_and_force_keeps_isolated_data() {
    let env = Env::new();
    let site = bare_site("Dup");
    env.install(&site.url("/"), "dup", &["--isolated"]);
    let marker = env.profile_dir("dup").join("Cookies");
    fs::write(&marker, "logged in").unwrap();

    env.winkle()
        .args(["install", &site.url("/"), "--id", "dup", "--isolated"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("already installed").and(predicate::str::contains("--force")),
        );

    env.winkle()
        .args([
            "install",
            &site.url("/"),
            "--id",
            "dup",
            "--isolated",
            "--force",
            "--name",
            "Renamed",
        ])
        .assert()
        .success()
        .stdout("Reinstalled Renamed (dup)\n");
    assert_eq!(fs::read_to_string(&marker).unwrap(), "logged in");
    assert!(
        fs::read_to_string(env.desktop_file("dup"))
            .unwrap()
            .contains("Name=Renamed")
    );
}

#[test]
fn never_overwrites_a_desktop_entry_winkle_did_not_write() {
    let env = Env::new();
    let site = bare_site("Mine");
    let path = env.desktop_file("theirs");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let original = "[Desktop Entry]\nType=Application\nName=Theirs\nExec=theirs\n";
    fs::write(&path, original).unwrap();

    env.winkle()
        .args(["install", &site.url("/"), "--id", "theirs", "--force"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("wasn't created by winkle"));
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
}

// ---- list ----------------------------------------------------------------

#[test]
fn list_when_empty() {
    let env = Env::new();
    env.winkle()
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No apps installed"));
    env.winkle()
        .args(["list", "--json"])
        .assert()
        .success()
        .stdout("[]\n");
}

#[test]
fn list_shows_winkle_apps_sorted_and_ignores_others() {
    let env = Env::new();
    let site = manifest_site();
    let other = bare_site("Zed");
    env.install(&other.url("/"), "zed", &["--isolated"]);
    env.install(&site.url("/"), "alpha", &[]);
    let apps = env.data.join("applications");
    fs::write(
        apps.join("other.desktop"),
        "[Desktop Entry]\nType=Application\nName=Other\nExec=o\n",
    )
    .unwrap();
    fs::write(
        apps.join("winkle-fake.desktop"),
        "[Desktop Entry]\nType=Application\nName=Fake\nExec=f\n",
    )
    .unwrap();

    let out = env.winkle().arg("list").output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3, "{text}");
    assert!(lines[0].starts_with("ID"));
    assert!(
        lines[1].starts_with("alpha")
            && lines[1].contains("Example App")
            && lines[1].contains("shared")
    );
    assert!(lines[2].starts_with("zed") && lines[2].contains("isolated"));
    assert!(!text.contains("Other") && !text.contains("Fake"));

    let out = env.winkle().args(["list", "--json"]).output().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let apps = json.as_array().unwrap();
    assert_eq!(apps.len(), 2);
    assert_eq!(apps[0]["id"], "alpha");
    assert_eq!(apps[0]["name"], "Example App");
    assert_eq!(apps[0]["profile"], "shared");
    assert_eq!(apps[0]["url"], site.url("/app?source=pwa"));
    assert_eq!(apps[0]["shortcuts"][0]["name"], "Compose");
    assert_eq!(apps[0]["shortcuts"][0]["url"], site.url("/compose"));
    assert_eq!(apps[1]["shortcuts"], serde_json::json!([]));
}

#[test]
fn hand_deleted_entries_disappear_from_list() {
    let env = Env::new();
    let site = bare_site("Gone");
    env.install(&site.url("/"), "gone", &[]);
    fs::remove_file(env.desktop_file("gone")).unwrap();
    env.winkle()
        .args(["list", "--json"])
        .assert()
        .success()
        .stdout("[]\n");
}

// ---- remove --------------------------------------------------------------

#[test]
fn remove_deletes_files_and_unpins() {
    let env = Env::new();
    let site = bare_site("Pinned");
    env.install(&site.url("/"), "pinned", &[]);
    fs::write(
        env.stubs.join("favorites"),
        "['org.gnome.Nautilus.desktop', 'winkle-pinned.desktop']\n",
    )
    .unwrap();

    env.winkle()
        .args(["remove", "pinned"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Removed Pinned (pinned)")
                .and(predicate::str::contains("Unpinned")),
        );
    assert!(!env.desktop_file("pinned").exists());
    assert!(!env.icon_png("pinned").exists());
    assert!(
        !env.data
            .join("icons/hicolor/scalable/apps/winkle-pinned.svg")
            .exists()
    );
    assert_eq!(env.favourites(), "['org.gnome.Nautilus.desktop']\n");
    env.winkle()
        .args(["list", "--json"])
        .assert()
        .stdout("[]\n");
}

#[test]
fn remove_unknown_or_foreign_app_fails() {
    let env = Env::new();
    env.winkle()
        .args(["remove", "does-not-exist"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no winkle app `does-not-exist`"));

    let foreign = env.desktop_file("foreign");
    fs::create_dir_all(foreign.parent().unwrap()).unwrap();
    fs::write(
        &foreign,
        "[Desktop Entry]\nType=Application\nName=F\nExec=f\n",
    )
    .unwrap();
    env.winkle().args(["remove", "foreign"]).assert().failure();
    assert!(foreign.exists());

    env.winkle()
        .args(["remove", "../../etc"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no winkle app"));
}

#[test]
fn remove_keeps_isolated_data_unless_purged() {
    let env = Env::new();
    let site = bare_site("Iso");
    env.install(&site.url("/"), "iso", &["--isolated"]);
    fs::write(env.profile_dir("iso").join("Cookies"), "x").unwrap();

    env.winkle()
        .args(["remove", "iso"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Kept its logins"));
    assert!(env.profile_dir("iso").join("Cookies").exists());

    // Reinstalling finds the old data again.
    env.install(&site.url("/"), "iso", &["--isolated"]);
    assert!(env.profile_dir("iso").join("Cookies").exists());

    env.winkle()
        .args(["remove", "iso", "--purge"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted its logins"));
    assert!(!env.profile_dir("iso").exists());
}

#[test]
fn purge_on_shared_app_leaves_chromium_alone() {
    let env = Env::new();
    let site = bare_site("Shared");
    env.install(&site.url("/"), "shared", &[]);
    let main_profile = env.home.join("snap/chromium/common/chromium/Default");
    fs::create_dir_all(&main_profile).unwrap();
    fs::write(main_profile.join("Cookies"), "x").unwrap();

    env.winkle()
        .args(["remove", "shared", "--purge"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No app-specific data to delete"));
    assert!(main_profile.join("Cookies").exists());
}

#[test]
fn purge_refuses_while_the_app_is_open() {
    let env = Env::new();
    let site = bare_site("Busy");
    env.install(&site.url("/"), "busy", &["--isolated"]);
    symlink(
        format!("{}-{}", hostname(), std::process::id()),
        env.profile_dir("busy").join("SingletonLock"),
    )
    .unwrap();

    env.winkle()
        .args(["remove", "busy", "--purge"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("still open"));
    assert!(env.desktop_file("busy").exists(), "nothing removed");
    assert!(env.icon_png("busy").exists());
}

// ---- remove --interactive (the Uninstall action) --------------------------

#[test]
fn interactive_cancel_changes_nothing() {
    let env = Env::new();
    let site = bare_site("Keep");
    env.install(&site.url("/"), "keep", &[]);
    env.zenity_answers(1, "");
    env.winkle()
        .args(["remove", "keep", "--interactive"])
        .assert()
        .success();
    assert!(env.desktop_file("keep").exists());
    let log = env.log();
    assert!(log.contains("zenity --question"), "{log}");
    assert!(
        !log.contains("--extra-button"),
        "shared apps get no delete-data button: {log}"
    );
    assert!(!log.contains("notify-send"), "{log}");
}

#[test]
fn interactive_uninstall_notifies() {
    let env = Env::new();
    let site = bare_site("Bye");
    env.install(&site.url("/"), "bye", &["--isolated"]);
    env.zenity_answers(0, "");
    env.winkle()
        .args(["remove", "bye", "--interactive"])
        .assert()
        .success();
    assert!(!env.desktop_file("bye").exists());
    assert!(
        env.profile_dir("bye").exists(),
        "plain Uninstall keeps data"
    );
    let log = env.log();
    assert!(
        log.contains("--extra-button=Uninstall and Delete Data"),
        "{log}"
    );
    assert!(
        log.contains("notify-send --app-name=winkle --icon=user-trash-symbolic Uninstalled Bye"),
        "{log}"
    );
}

#[test]
fn interactive_delete_data_purges() {
    let env = Env::new();
    let site = bare_site("Purge");
    env.install(&site.url("/"), "purge", &["--isolated"]);
    env.zenity_answers(1, "Uninstall and Delete Data\n");
    env.winkle()
        .args(["remove", "purge", "--interactive"])
        .assert()
        .success();
    assert!(!env.desktop_file("purge").exists());
    assert!(!env.profile_dir("purge").exists());
}

#[test]
fn interactive_failure_is_shown() {
    let env = Env::new();
    let site = bare_site("Open");
    env.install(&site.url("/"), "open", &["--isolated"]);
    symlink(
        format!("{}-{}", hostname(), std::process::id()),
        env.profile_dir("open").join("SingletonLock"),
    )
    .unwrap();
    env.zenity_answers(1, "Uninstall and Delete Data\n");
    env.winkle()
        .args(["remove", "open", "--interactive"])
        .assert()
        .failure();
    assert!(env.desktop_file("open").exists());
    let log = env.log();
    assert!(log.contains("zenity --error"), "{log}");
    assert!(log.contains("still open"), "{log}");
}

// ---- apps installed under the old name (hermit) are not winkle's ----------

/// A desktop entry as the tool wrote it before the rename.
fn write_old_hermit_entry(env: &Env, id: &str) -> PathBuf {
    let path = env.data.join(format!("applications/hermit-{id}.desktop"));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        format!(
            "[Desktop Entry]\nType=Application\nName=Old\nExec=chromium --app=https://github.com/\n\
             X-Hermit-Id={id}\nX-Hermit-Url=https://github.com/\nX-Hermit-Profile=shared\nX-Hermit-Version=1\n"
        ),
    )
    .unwrap();
    path
}

#[test]
fn old_hermit_apps_are_not_listed() {
    let env = Env::new();
    write_old_hermit_entry(&env, "github-com");
    env.winkle()
        .args(["list", "--json"])
        .assert()
        .success()
        .stdout("[]\n");
}

#[test]
fn old_hermit_apps_are_not_removed() {
    let env = Env::new();
    let old = write_old_hermit_entry(&env, "github-com");
    env.winkle()
        .args(["remove", "github-com"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no winkle app `github-com`"));
    assert!(old.exists());
}
