//! The Chromium snap: launch command lines and the window app_id it assigns.
//! This is the seam for supporting other browsers later.

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use url::Url;

/// The profile directory hermit pins for shared apps, and the only one in an
/// isolated user-data-dir. It is the suffix of every window's app_id.
pub const PROFILE_DIRECTORY: &str = "Default";

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProfileMode {
    Shared,
    Isolated,
}

impl fmt::Display for ProfileMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Shared => "shared",
            Self::Isolated => "isolated",
        })
    }
}

impl FromStr for ProfileMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "shared" => Ok(Self::Shared),
            "isolated" => Ok(Self::Isolated),
            other => Err(format!("unknown profile mode `{other}`")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Profile {
    /// The user's main Chromium profile.
    Shared,
    /// A user-data-dir of the app's own.
    Isolated(PathBuf),
}

impl Profile {
    pub fn mode(&self) -> ProfileMode {
        match self {
            Self::Shared => ProfileMode::Shared,
            Self::Isolated(_) => ProfileMode::Isolated,
        }
    }
}

/// The command line that opens `url` as an app window.
pub fn launch_argv(browser: &Path, profile: &Profile, url: &Url) -> Vec<String> {
    let profile_arg = match profile {
        Profile::Shared => format!("--profile-directory={PROFILE_DIRECTORY}"),
        Profile::Isolated(dir) => format!("--user-data-dir={}", dir.display()),
    };
    vec![
        browser.display().to_string(),
        profile_arg,
        format!("--app={url}"),
    ]
}

/// The Wayland app_id Chromium gives an `--app=<url>` window, which GNOME
/// matches against `StartupWMClass`. Measured in `spikes/app-id/FINDINGS.md`:
/// `chrome-<host>_<path, "/" → "_">-<profile directory>`, query dropped.
pub fn app_id(url: &Url) -> String {
    let app_name = format!("{}_{}", url.host_str().unwrap_or_default(), url.path());
    let sanitised: String = app_name
        .chars()
        .map(|c| if is_illegal_in_file_name(c) { '_' } else { c })
        .collect();
    format!("chrome-{sanitised}-{PROFILE_DIRECTORY}")
}

/// Characters Chromium replaces when turning an app name into a file-name-safe
/// class (`base::i18n::ReplaceIllegalCharactersInPath`). Only `/` has been
/// measured; the rest follow Chromium's list and are pinned by the spike's
/// probe if they ever matter.
fn is_illegal_in_file_name(c: char) -> bool {
    c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    /// Every value measured with `spikes/app-id/probe.sh`.
    #[test]
    fn app_id_matches_measurements() {
        for (launch, measured) in [
            ("https://github.com/", "chrome-github.com__-Default"),
            ("https://github.com/notifications", "chrome-github.com__notifications-Default"),
            ("https://github.com/pulls", "chrome-github.com__pulls-Default"),
            ("https://github.com/issues/new?x=1", "chrome-github.com__issues_new-Default"),
        ] {
            assert_eq!(app_id(&url(launch)), measured, "{launch}");
        }
    }

    #[test]
    fn shared_argv() {
        assert_eq!(
            launch_argv(Path::new("/snap/bin/chromium"), &Profile::Shared, &url("https://github.com/")),
            ["/snap/bin/chromium", "--profile-directory=Default", "--app=https://github.com/"]
        );
    }

    #[test]
    fn isolated_argv() {
        let profile = Profile::Isolated(PathBuf::from("/home/u/snap/chromium/common/hermit/app-hey-com"));
        assert_eq!(
            launch_argv(Path::new("/snap/bin/chromium"), &profile, &url("https://app.hey.com/imbox")),
            [
                "/snap/bin/chromium",
                "--user-data-dir=/home/u/snap/chromium/common/hermit/app-hey-com",
                "--app=https://app.hey.com/imbox",
            ]
        );
    }

    #[test]
    fn shortcut_argv_uses_the_app_profile() {
        let profile = Profile::Isolated(PathBuf::from("/p"));
        assert_eq!(
            launch_argv(Path::new("/b"), &profile, &url("https://x.example/compose?to=a&b=c")),
            ["/b", "--user-data-dir=/p", "--app=https://x.example/compose?to=a&b=c"]
        );
    }

    #[test]
    fn profile_mode_round_trips() {
        for mode in [ProfileMode::Shared, ProfileMode::Isolated] {
            assert_eq!(mode.to_string().parse::<ProfileMode>().unwrap(), mode);
        }
        assert!("other".parse::<ProfileMode>().is_err());
    }
}
