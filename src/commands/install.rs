use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use url::Url;

use crate::browser::{self, Profile};
use crate::cli::InstallArgs;
use crate::desktop;
use crate::entry::{AppEntry, ShortcutAction, read_entry};
use crate::fsutil::write_atomic;
use crate::icons::{self, IconFiles};
use crate::metadata::fetch::Fetcher;
use crate::metadata::input::parse_site_url;
use crate::metadata::resolve::{AppMetadata, Overrides, resolve};
use crate::paths::Paths;

pub const MAX_ID_LEN: usize = 64;

pub fn run(args: InstallArgs) -> Result<()> {
    let paths = Paths::from_env()?;
    let entered = parse_site_url(&args.url)?;
    if let Some(id) = &args.id {
        validate_id(id)?;
    }

    let fetcher = Fetcher::new()?;
    let overrides = Overrides {
        name: args.name.clone(),
        icon: args.icon.clone(),
    };
    let meta = resolve(&entered, &overrides, &fetcher)?;
    for warning in &meta.warnings {
        eprintln!("warning: {warning}");
    }

    let id = match &args.id {
        Some(id) => id.clone(),
        None => derive_id(&meta.launch_url)?,
    };
    let desktop_path = paths.desktop_file(&id);
    let replacing = check_existing(&desktop_path, &id, args.force)?;

    let profile = if args.isolated {
        Profile::Isolated(paths.profile_dir(&id))
    } else {
        Profile::Shared
    };
    let entry = build_entry(&paths, &id, &meta, &profile)?;
    let icon_files = icons::render(
        meta.icon.as_ref().map(|i| &i.image),
        &meta.name,
        meta.theme_color.as_deref(),
    )?;

    if args.dry_run {
        print_plan(&paths, &entry, &meta, &profile, &icon_files, replacing);
        return Ok(());
    }

    if let Profile::Isolated(dir) = &profile {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    icons::install(&paths, &id, &icon_files)?;
    write_atomic(&desktop_path, entry.to_desktop_file().as_bytes())?;
    desktop::refresh(&paths);

    let verb = if replacing {
        "Reinstalled"
    } else {
        "Installed"
    };
    println!("{verb} {} ({id})", meta.name);
    Ok(())
}

/// Ids appear in file names and on command lines, so they are restricted.
/// A leading hyphen would read as an option.
pub fn validate_id(id: &str) -> Result<()> {
    let ok = !id.is_empty()
        && id.len() <= MAX_ID_LEN
        && !id.starts_with('-')
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !ok {
        bail!(
            "invalid id `{id}`: use up to {MAX_ID_LEN} lowercase letters, digits and hyphens, \
             not starting with a hyphen"
        );
    }
    Ok(())
}

/// `www.github.com` → `github-com`; `app.hey.com` → `app-hey-com`.
pub fn derive_id(launch_url: &Url) -> Result<String> {
    let host = launch_url
        .host_str()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    let mut id = String::new();
    for c in host.chars() {
        let c = if c.is_ascii_lowercase() || c.is_ascii_digit() {
            c
        } else {
            '-'
        };
        if !(c == '-' && (id.is_empty() || id.ends_with('-'))) {
            id.push(c);
        }
    }
    let id = id.trim_end_matches('-');
    let id = &id[..id.len().min(MAX_ID_LEN)];
    validate_id(id).map_err(|_| anyhow!("can't derive an id from `{host}`; pass --id"))?;
    Ok(id.to_string())
}

/// Returns whether an existing winkle app will be replaced. Never touches
/// files winkle didn't create, even with `--force`.
fn check_existing(path: &Path, id: &str, force: bool) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    match read_entry(path) {
        Ok(Some(app)) if app.id == id => {
            if force {
                Ok(true)
            } else {
                bail!(
                    "`{id}` is already installed ({}). Use --force to replace it.",
                    app.name
                )
            }
        }
        _ => bail!(
            "{} exists but wasn't created by winkle; not touching it (choose another --id)",
            path.display()
        ),
    }
}

fn build_entry(paths: &Paths, id: &str, meta: &AppMetadata, profile: &Profile) -> Result<AppEntry> {
    let exe = std::env::current_exe().context("finding the winkle executable")?;
    let host = meta.launch_url.host_str().unwrap_or_default().to_string();
    let mut keywords = vec![host.clone()];
    if let Some(bare) = host.strip_prefix("www.") {
        keywords.push(bare.to_string());
    }
    Ok(AppEntry {
        id: id.to_string(),
        name: meta.name.clone(),
        url: meta.launch_url.clone(),
        profile: profile.mode(),
        exec: browser::launch_argv(&paths.browser, profile, &meta.launch_url),
        startup_wm_class: browser::app_id(&meta.launch_url),
        keywords,
        shortcuts: meta
            .shortcuts
            .iter()
            .map(|s| ShortcutAction {
                name: s.name.clone(),
                url: s.url.clone(),
                exec: browser::launch_argv(&paths.browser, profile, &s.url),
            })
            .collect(),
        uninstall_exec: vec![
            exe.display().to_string(),
            "remove".into(),
            id.to_string(),
            "--interactive".into(),
        ],
    })
}

fn print_plan(
    paths: &Paths,
    entry: &AppEntry,
    meta: &AppMetadata,
    profile: &Profile,
    icon_files: &IconFiles,
    replacing: bool,
) {
    let action = if replacing { "replace" } else { "install" };
    println!("Would {action} {} ({})", entry.name, entry.id);
    println!("  launch URL:   {}", entry.url);
    println!("  profile:      {}", profile.mode());
    println!("  window class: {}", entry.startup_wm_class);
    match &meta.icon {
        Some(icon) => println!(
            "  icon:         {} {} ({})",
            icon.source,
            icon.origin,
            icon.image.describe()
        ),
        None => println!("  icon:         generated placeholder"),
    }
    if meta.shortcuts.is_empty() {
        println!("  shortcuts:    none");
    }
    for (i, s) in meta.shortcuts.iter().enumerate() {
        let label = if i == 0 { "shortcuts:" } else { "" };
        println!("  {label:<13} {} → {}", s.name, s.url);
    }
    println!("Files:");
    let mut files: Vec<PathBuf> = vec![paths.desktop_file(&entry.id), paths.icon_png(&entry.id)];
    if icon_files.svg.is_some() {
        files.push(paths.icon_svg(&entry.id));
    }
    if let Profile::Isolated(dir) = profile {
        files.push(dir.clone());
    }
    for f in files {
        println!("  {}", f.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id_for(url: &str) -> String {
        derive_id(&Url::parse(url).unwrap()).unwrap()
    }

    #[test]
    fn derived_ids() {
        assert_eq!(id_for("https://www.github.com/"), "github-com");
        assert_eq!(id_for("https://app.hey.com/imbox"), "app-hey-com");
        assert_eq!(id_for("http://127.0.0.1:8080/"), "127-0-0-1");
        assert_eq!(id_for("https://MAIL.Google.com/"), "mail-google-com");
        assert_eq!(
            id_for("https://xn--bcher-kva.example/"),
            "xn-bcher-kva-example"
        );
    }

    #[test]
    fn id_validation() {
        assert!(validate_id("github-com").is_ok());
        for bad in ["", "My App", "UPPER", "-flag", "dots.no", &"a".repeat(65)] {
            assert!(validate_id(bad).is_err(), "{bad:?} should be invalid");
        }
    }
}
