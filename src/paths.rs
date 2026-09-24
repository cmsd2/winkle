//! Where hermit reads and writes. Everything is under the user's home.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

pub const DEFAULT_BROWSER: &str = "/snap/bin/chromium";

#[derive(Debug, Clone)]
pub struct Paths {
    pub home: PathBuf,
    pub data_home: PathBuf,
    pub browser: PathBuf,
}

impl Paths {
    pub fn from_env() -> Result<Self> {
        Self::from_vars(|key| std::env::var_os(key))
    }

    /// Resolve paths from environment variables, looked up with `var`.
    ///
    /// `HERMIT_BROWSER` overrides the browser executable; tests point it at a stub.
    pub fn from_vars(var: impl Fn(&str) -> Option<OsString>) -> Result<Self> {
        let home = var("HOME")
            .filter(|h| !h.is_empty())
            .map(PathBuf::from)
            .context("HOME is not set")?;
        if !home.is_absolute() {
            bail!("HOME is not an absolute path: {}", home.display());
        }
        // Per the XDG spec, a relative XDG_DATA_HOME is invalid and ignored.
        let data_home = var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".local/share"));
        let browser = var("HERMIT_BROWSER")
            .filter(|b| !b.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_BROWSER));
        Ok(Self {
            home,
            data_home,
            browser,
        })
    }

    pub fn applications_dir(&self) -> PathBuf {
        self.data_home.join("applications")
    }

    /// The desktop file ID GNOME knows the app by, e.g. `hermit-github-com.desktop`.
    pub fn desktop_file_id(id: &str) -> String {
        format!("hermit-{id}.desktop")
    }

    pub fn desktop_file(&self, id: &str) -> PathBuf {
        self.applications_dir().join(Self::desktop_file_id(id))
    }

    pub fn icon_name(id: &str) -> String {
        format!("hermit-{id}")
    }

    pub fn hicolor_dir(&self) -> PathBuf {
        self.data_home.join("icons/hicolor")
    }

    pub fn icon_png(&self, id: &str) -> PathBuf {
        self.hicolor_dir()
            .join("256x256/apps")
            .join(format!("{}.png", Self::icon_name(id)))
    }

    pub fn icon_svg(&self, id: &str) -> PathBuf {
        self.hicolor_dir()
            .join("scalable/apps")
            .join(format!("{}.svg", Self::icon_name(id)))
    }

    /// Root of isolated profiles. It has to be inside the Chromium snap's own
    /// per-user area: the snap can't use hidden directories in `$HOME`.
    pub fn profiles_root(&self) -> PathBuf {
        self.home.join("snap/chromium/common/hermit")
    }

    pub fn profile_dir(&self, id: &str) -> PathBuf {
        self.profiles_root().join(id)
    }

    /// True if `path` is inside `root` (lexically; both are built by hermit).
    pub fn is_within(path: &Path, root: &Path) -> bool {
        path.starts_with(root) && path != root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn paths(vars: &[(&str, &str)]) -> Result<Paths> {
        let map: HashMap<String, OsString> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), OsString::from(v)))
            .collect();
        Paths::from_vars(|k| map.get(k).cloned())
    }

    #[test]
    fn defaults() {
        let p = paths(&[("HOME", "/home/u")]).unwrap();
        assert_eq!(p.data_home, PathBuf::from("/home/u/.local/share"));
        assert_eq!(p.browser, PathBuf::from("/snap/bin/chromium"));
        assert_eq!(
            p.desktop_file("github-com"),
            PathBuf::from("/home/u/.local/share/applications/hermit-github-com.desktop")
        );
        assert_eq!(
            p.icon_png("github-com"),
            PathBuf::from("/home/u/.local/share/icons/hicolor/256x256/apps/hermit-github-com.png")
        );
        assert_eq!(
            p.icon_svg("github-com"),
            PathBuf::from("/home/u/.local/share/icons/hicolor/scalable/apps/hermit-github-com.svg")
        );
        assert_eq!(
            p.profile_dir("app-hey-com"),
            PathBuf::from("/home/u/snap/chromium/common/hermit/app-hey-com")
        );
    }

    #[test]
    fn overrides() {
        let p = paths(&[
            ("HOME", "/home/u"),
            ("XDG_DATA_HOME", "/tmp/data"),
            ("HERMIT_BROWSER", "/tmp/stub-browser"),
        ])
        .unwrap();
        assert_eq!(p.data_home, PathBuf::from("/tmp/data"));
        assert_eq!(p.browser, PathBuf::from("/tmp/stub-browser"));
        assert_eq!(
            p.applications_dir(),
            PathBuf::from("/tmp/data/applications")
        );
    }

    #[test]
    fn relative_xdg_data_home_is_ignored() {
        let p = paths(&[("HOME", "/home/u"), ("XDG_DATA_HOME", "relative")]).unwrap();
        assert_eq!(p.data_home, PathBuf::from("/home/u/.local/share"));
    }

    #[test]
    fn missing_home_is_an_error() {
        assert!(paths(&[]).is_err());
        assert!(paths(&[("HOME", "relative")]).is_err());
    }

    #[test]
    fn is_within() {
        let root = Path::new("/a/b");
        assert!(Paths::is_within(Path::new("/a/b/c"), root));
        assert!(!Paths::is_within(Path::new("/a/b"), root));
        assert!(!Paths::is_within(Path::new("/a/bc"), root));
    }
}
