//! Talking to the GNOME desktop: menu and icon refreshes, dock favourites,
//! dialogs and notifications. Each helper shells out to a standard tool
//! found on `PATH` (tests put stubs first).

use std::fs::File;
use std::io::ErrorKind;
use std::process::{Command, Stdio};
use std::time::SystemTime;

use anyhow::{Context, Result, bail};

use crate::paths::{APP_NAME, Paths};

/// Make GNOME notice new or removed entries and icons without a re-login.
pub fn refresh(paths: &Paths) {
    let apps = paths.applications_dir();
    if apps.is_dir() {
        // Only strictly needed for MimeType entries; harmless otherwise.
        let _ = Command::new("update-desktop-database")
            .arg("-q")
            .arg(&apps)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    // GTK rescans an icon theme when its directory's mtime changes.
    if let Ok(dir) = File::open(paths.hicolor_dir()) {
        let _ = dir.set_modified(SystemTime::now());
    }
}

/// Unpin `desktop_id` from the dock. `Ok(true)` if it was pinned.
pub fn remove_favourite(desktop_id: &str) -> Result<bool> {
    let output = Command::new("gsettings")
        .args(["get", "org.gnome.shell", "favorite-apps"])
        .output()
        .context("running gsettings")?;
    if !output.status.success() {
        bail!(
            "gsettings get failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let favourites = parse_string_array(&String::from_utf8_lossy(&output.stdout))
        .context("unexpected output from gsettings")?;
    let kept: Vec<&String> = favourites.iter().filter(|f| *f != desktop_id).collect();
    if kept.len() == favourites.len() {
        return Ok(false);
    }
    let value = format!(
        "[{}]",
        kept.iter()
            .map(|f| gvariant_string(f))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let status = Command::new("gsettings")
        .args(["set", "org.gnome.shell", "favorite-apps", &value])
        .status()
        .context("running gsettings")?;
    if !status.success() {
        bail!("gsettings set failed");
    }
    Ok(true)
}

/// Parse a GVariant text array of strings: `['a', "b"]` or `@as []`.
fn parse_string_array(text: &str) -> Option<Vec<String>> {
    let text = text.trim();
    let text = text.strip_prefix("@as").unwrap_or(text).trim();
    let inner = text.strip_prefix('[')?.strip_suffix(']')?;
    let mut items = Vec::new();
    let mut chars = inner.chars().peekable();
    loop {
        while chars.peek().is_some_and(|c| c.is_whitespace() || *c == ',') {
            chars.next();
        }
        let Some(quote) = chars.next() else { break };
        if quote != '\'' && quote != '"' {
            return None;
        }
        let mut item = String::new();
        loop {
            match chars.next()? {
                '\\' => item.push(chars.next()?),
                c if c == quote => break,
                c => item.push(c),
            }
        }
        items.push(item);
    }
    Some(items)
}

fn gvariant_string(s: &str) -> String {
    format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UninstallChoice {
    Cancel,
    Uninstall,
    UninstallAndDeleteData,
}

const DELETE_DATA_LABEL: &str = "Uninstall and Delete Data";

/// Ask whether to uninstall `name`, offering to delete data for isolated apps.
pub fn confirm_uninstall(name: &str, isolated: bool) -> Result<UninstallChoice> {
    let text = if isolated {
        format!(
            "{name} will be removed from your apps.\n\n\
             Its logins and site data can be kept, in case you reinstall it, or deleted."
        )
    } else {
        format!("{name} will be removed from your apps. Your logins in Chromium are not affected.")
    };
    let mut cmd = Command::new("zenity");
    cmd.args([
        "--question",
        "--no-markup",
        &format!("--title=Uninstall {name}?"),
        &format!("--text={text}"),
        "--ok-label=Uninstall",
        "--cancel-label=Cancel",
    ]);
    if isolated {
        cmd.arg(format!("--extra-button={DELETE_DATA_LABEL}"));
    }
    let output = cmd
        .stderr(Stdio::null())
        .output()
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => {
                anyhow::anyhow!("zenity is not installed, so winkle can't ask for confirmation")
            }
            _ => anyhow::Error::new(e).context("running zenity"),
        })?;
    // zenity exits 0 for OK; 1 for Cancel, closing, or an extra button (whose
    // label it prints).
    Ok(match output.status.code() {
        Some(0) => UninstallChoice::Uninstall,
        Some(1) if String::from_utf8_lossy(&output.stdout).trim() == DELETE_DATA_LABEL => {
            UninstallChoice::UninstallAndDeleteData
        }
        _ => UninstallChoice::Cancel,
    })
}

pub fn notify(summary: &str, body: &str) {
    let _ = Command::new("notify-send")
        .args([
            &format!("--app-name={APP_NAME}"),
            "--icon=user-trash-symbolic",
            summary,
            body,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// Show an error dialog, falling back to a notification without zenity.
pub fn show_error(title: &str, message: &str) {
    let shown = Command::new("zenity")
        .args([
            "--error",
            "--no-markup",
            &format!("--title={title}"),
            &format!("--text={message}"),
        ])
        .stderr(Stdio::null())
        .status()
        .is_ok();
    if !shown {
        notify(title, message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gsettings_arrays() {
        assert_eq!(
            parse_string_array("['org.gnome.Nautilus.desktop', 'winkle-github-com.desktop']\n"),
            Some(vec![
                "org.gnome.Nautilus.desktop".into(),
                "winkle-github-com.desktop".into()
            ])
        );
        assert_eq!(parse_string_array("@as []"), Some(vec![]));
        assert_eq!(
            parse_string_array(r#"["it's", 'a\'b']"#),
            Some(vec!["it's".into(), "a'b".into()])
        );
        assert_eq!(parse_string_array("nonsense"), None);
        assert_eq!(parse_string_array("['unterminated"), None);
    }

    #[test]
    fn gvariant_strings_round_trip() {
        let value = format!("[{}]", ["a'b", r"c\d"].map(gvariant_string).join(", "));
        assert_eq!(
            parse_string_array(&value),
            Some(vec!["a'b".into(), r"c\d".into()])
        );
    }
}
