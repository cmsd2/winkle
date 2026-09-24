//! winkle's desktop entries: the model, writer and reader. The entry is the
//! registry (design decision 1); winkle only reads entries it wrote.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use url::Url;

use crate::browser::ProfileMode;
use crate::paths::Paths;

/// Bump when the entry format changes, so later versions can find old entries.
pub const FORMAT_VERSION: u32 = 1;

const MAIN_GROUP: &str = "Desktop Entry";

/// Prefix of winkle's own keys in desktop entries (`X-Winkle-Id`, ...).
pub const KEY_PREFIX: &str = "X-Winkle-";

fn key(name: &str) -> String {
    format!("{KEY_PREFIX}{name}")
}
const UNINSTALL_ACTION: &str = "uninstall";

/// Everything needed to write an app's desktop entry.
#[derive(Debug, Clone)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub url: Url,
    pub profile: ProfileMode,
    pub exec: Vec<String>,
    pub startup_wm_class: String,
    pub keywords: Vec<String>,
    pub shortcuts: Vec<ShortcutAction>,
    pub uninstall_exec: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ShortcutAction {
    pub name: String,
    pub url: Url,
    pub exec: Vec<String>,
}

/// What winkle reads back from an installed entry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct InstalledApp {
    pub id: String,
    pub name: String,
    pub url: Url,
    pub profile: ProfileMode,
    pub shortcuts: Vec<InstalledShortcut>,
    #[serde(skip)]
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct InstalledShortcut {
    pub name: String,
    pub url: Url,
}

impl AppEntry {
    pub fn to_desktop_file(&self) -> String {
        let host = self.url.host_str().unwrap_or_default();
        let mut actions: Vec<String> = (1..=self.shortcuts.len())
            .map(|i| format!("shortcut-{i}"))
            .collect();
        actions.push(UNINSTALL_ACTION.into());

        let mut out = Writer::default();
        out.group(MAIN_GROUP);
        out.kv("Type", "Application");
        out.kv("Version", "1.5");
        out.kv("Name", &escape_string(&self.name));
        out.kv("Comment", &escape_string(&format!("Web app for {host}")));
        out.kv("Icon", &Paths::icon_name(&self.id));
        out.kv("Exec", &escape_string(&exec_line(&self.exec)));
        out.kv("StartupWMClass", &escape_string(&self.startup_wm_class));
        // A launch handed to an already-running Chromium never claims GNOME's
        // activation token, so with `true` the dock icon only appears after a
        // ~10 s timeout (spikes/app-id/FINDINGS.md).
        out.kv("StartupNotify", "false");
        out.kv("Terminal", "false");
        out.kv("Categories", "Network;");
        out.kv("Keywords", &escape_list(&self.keywords));
        out.kv("Actions", &escape_list(&actions));
        out.kv(&key("Id"), &self.id);
        out.kv(&key("Url"), &escape_string(self.url.as_str()));
        out.kv(&key("Profile"), &self.profile.to_string());
        out.kv(&key("Version"), &FORMAT_VERSION.to_string());

        for (i, shortcut) in self.shortcuts.iter().enumerate() {
            out.group(&format!("Desktop Action shortcut-{}", i + 1));
            out.kv("Name", &escape_string(&shortcut.name));
            out.kv("Exec", &escape_string(&exec_line(&shortcut.exec)));
            out.kv(&key("Url"), &escape_string(shortcut.url.as_str()));
        }

        out.group(&format!("Desktop Action {UNINSTALL_ACTION}"));
        out.kv("Name", "Uninstall");
        out.kv("Exec", &escape_string(&exec_line(&self.uninstall_exec)));
        out.0
    }
}

#[derive(Default)]
struct Writer(String);

impl Writer {
    fn group(&mut self, name: &str) {
        if !self.0.is_empty() {
            self.0.push('\n');
        }
        self.0.push_str(&format!("[{name}]\n"));
    }

    /// `value` must already be escaped.
    fn kv(&mut self, key: &str, value: &str) {
        self.0.push_str(&format!("{key}={value}\n"));
    }
}

/// Read the winkle entry at `path`. `Ok(None)` if the file isn't a winkle entry.
pub fn read_entry(path: &Path) -> Result<Option<InstalledApp>> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let groups = parse_groups(&text);
    let Some(main) = groups.get(MAIN_GROUP) else {
        return Ok(None);
    };
    let Some(id) = main.get(&key("Id")) else {
        return Ok(None);
    };
    let id = unescape(id);

    let corrupt = |what: &str| anyhow!("{}: invalid {what}", path.display());
    let name = main
        .get("Name")
        .map(|n| unescape(n))
        .ok_or_else(|| corrupt("Name"))?;
    let url = main
        .get(&key("Url"))
        .and_then(|u| Url::parse(&unescape(u)).ok())
        .ok_or_else(|| corrupt(&key("Url")))?;
    let profile = main
        .get(&key("Profile"))
        .and_then(|p| p.parse::<ProfileMode>().ok())
        .ok_or_else(|| corrupt(&key("Profile")))?;

    let actions = main
        .get("Actions")
        .map(|a| split_list(a))
        .unwrap_or_default();
    let shortcuts = actions
        .iter()
        .filter(|a| a.as_str() != UNINSTALL_ACTION)
        .filter_map(|a| groups.get(format!("Desktop Action {a}").as_str()))
        .filter_map(|g| {
            Some(InstalledShortcut {
                name: unescape(g.get("Name")?),
                url: Url::parse(&unescape(g.get(&key("Url"))?)).ok()?,
            })
        })
        .collect();

    Ok(Some(InstalledApp {
        id,
        name,
        url,
        profile,
        shortcuts,
        path: path.to_path_buf(),
    }))
}

/// Groups of raw (still escaped) key/value pairs. Comments and localised keys
/// are kept as-is; winkle only looks up exact keys.
fn parse_groups(text: &str) -> HashMap<String, HashMap<String, String>> {
    let mut groups: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        let line = line.trim_start();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            current = Some(name.to_string());
            groups.entry(name.to_string()).or_default();
        } else if let (Some(group), Some((key, value))) = (&current, line.split_once('=')) {
            groups
                .get_mut(group)
                .expect("group was inserted")
                .entry(key.trim_end().to_string())
                .or_insert_with(|| value.trim_start().to_string());
        }
    }
    groups
}

/// Escape a `string`/`localestring` value (Desktop Entry spec, "Possible value types").
pub fn escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for (i, c) in s.chars().enumerate() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            ' ' if i == 0 => out.push_str("\\s"),
            c => out.push(c),
        }
    }
    out
}

/// Escape a list of strings, each terminated by `;`.
fn escape_list(items: &[String]) -> String {
    items
        .iter()
        .map(|item| format!("{};", escape_string(item).replace(';', "\\;")))
        .collect()
}

fn split_list(raw: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some(';') => current.push(';'),
                Some(other) => {
                    current.push('\\');
                    current.push(other);
                }
                None => current.push('\\'),
            },
            ';' => items.push(unescape(&std::mem::take(&mut current))),
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        items.push(unescape(&current));
    }
    items
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('s') => out.push(' '),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// Build an `Exec` value: quote arguments with reserved characters and double
/// every `%` so nothing is read as a field code. The result still needs
/// `escape_string` (which doubles backslashes again, as the spec requires).
pub fn exec_line(argv: &[String]) -> String {
    argv.iter()
        .map(|arg| quote_exec_arg(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

fn quote_exec_arg(arg: &str) -> String {
    const RESERVED: &[char] = &[
        ' ', '\t', '\n', '"', '\'', '\\', '>', '<', '~', '|', '&', ';', '$', '*', '?', '#', '(',
        ')', '`',
    ];
    let arg = arg.replace('%', "%%");
    if !arg.is_empty() && !arg.contains(RESERVED) {
        return arg;
    }
    let mut out = String::from("\"");
    for c in arg.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn sample(name: &str) -> AppEntry {
        let url = Url::parse("https://github.com/?q=a%20b&c=d").unwrap();
        AppEntry {
            id: "github-com".into(),
            name: name.into(),
            url: url.clone(),
            profile: ProfileMode::Shared,
            exec: vec![
                "/snap/bin/chromium".into(),
                "--profile-directory=Default".into(),
                format!("--app={url}"),
            ],
            startup_wm_class: "chrome-github.com__-Default".into(),
            keywords: vec!["github.com".into(), "semi;colon".into()],
            shortcuts: vec![ShortcutAction {
                name: "New \"issue\"; 100%".into(),
                url: Url::parse("https://github.com/issues/new").unwrap(),
                exec: vec![
                    "/snap/bin/chromium".into(),
                    "--app=https://github.com/issues/new".into(),
                ],
            }],
            uninstall_exec: vec![
                "/home/u/My Tools/winkle".into(),
                "remove".into(),
                "github-com".into(),
                "--interactive".into(),
            ],
        }
    }

    fn validate(contents: &str) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("winkle-test.desktop");
        std::fs::write(&path, contents).unwrap();
        let out = Command::new("desktop-file-validate")
            .arg(&path)
            .output()
            .expect("desktop-file-validate");
        let report = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        assert!(
            out.status.success() && report.trim().is_empty(),
            "{report}\n---\n{contents}"
        );
    }

    #[test]
    fn generated_entry_passes_desktop_file_validate() {
        validate(&sample("GitHub").to_desktop_file());
    }

    #[test]
    fn hostile_names_are_escaped_and_still_valid() {
        let name = "Evil\nExec=rm -rf ~; 100% \"quoted\" \\ back";
        let contents = sample(name).to_desktop_file();
        validate(&contents);
        assert_eq!(
            contents.matches("\nExec=").count(),
            3,
            "no injected Exec line:\n{contents}"
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("winkle-github-com.desktop");
        std::fs::write(&path, &contents).unwrap();
        assert_eq!(read_entry(&path).unwrap().unwrap().name, name);
    }

    #[test]
    fn exec_quoting() {
        let line = exec_line(&[
            "/snap/bin/chromium".into(),
            "--app=https://x/?a=1&b=%41".into(),
            "has space".into(),
            "q\"uote$`\\".into(),
        ]);
        assert_eq!(
            line,
            r#"/snap/bin/chromium "--app=https://x/?a=1&b=%%41" "has space" "q\"uote\$\`\\""#
        );
        // escape_string then doubles the backslashes, per the spec.
        assert_eq!(escape_string(r#""a\\b""#), r#""a\\\\b""#);
    }

    #[test]
    fn round_trip() {
        let entry = sample("GitHub");
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("winkle-github-com.desktop");
        std::fs::write(&path, entry.to_desktop_file()).unwrap();
        let app = read_entry(&path).unwrap().unwrap();
        assert_eq!(
            app,
            InstalledApp {
                id: "github-com".into(),
                name: "GitHub".into(),
                url: entry.url.clone(),
                profile: ProfileMode::Shared,
                shortcuts: vec![InstalledShortcut {
                    name: "New \"issue\"; 100%".into(),
                    url: Url::parse("https://github.com/issues/new").unwrap(),
                }],
                path: path.clone(),
            }
        );
    }

    #[test]
    fn non_winkle_entries_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("other.desktop");
        std::fs::write(
            &path,
            "[Desktop Entry]\nType=Application\nName=Other\nExec=other\n",
        )
        .unwrap();
        assert_eq!(read_entry(&path).unwrap(), None);
    }

    #[test]
    fn lists_split_on_unescaped_semicolons() {
        assert_eq!(split_list(r"a;b\;c;d"), ["a", "b;c", "d"]);
        assert_eq!(split_list("a;b"), ["a", "b"]);
    }

    #[test]
    fn key_prefix_derives_from_app_name() {
        let name = crate::paths::APP_NAME;
        let capitalised = name[..1].to_uppercase() + &name[1..];
        assert_eq!(KEY_PREFIX, format!("X-{capitalised}-"));
    }

    #[test]
    fn leading_space_is_escaped() {
        assert_eq!(escape_string(" x"), "\\sx");
        assert_eq!(unescape("\\sx\\n"), " x\n");
    }
}
