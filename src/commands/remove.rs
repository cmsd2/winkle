use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};

use crate::browser::ProfileMode;
use crate::cli::RemoveArgs;
use crate::commands::install::validate_id;
use crate::desktop::{self, UninstallChoice};
use crate::entry::{InstalledApp, read_entry};
use crate::fsutil::remove_if_exists;
use crate::icons;
use crate::paths::Paths;

pub fn run(args: RemoveArgs) -> Result<()> {
    let paths = Paths::from_env()?;
    if args.interactive {
        return interactive(&paths, &args.id);
    }
    let removal = remove(&paths, &args.id, args.purge)?;
    for warning in &removal.warnings {
        eprintln!("warning: {warning}");
    }
    println!("Removed {} ({})", removal.name, args.id);
    for note in &removal.notes {
        println!("{note}");
    }
    Ok(())
}

pub struct Removal {
    pub name: String,
    pub notes: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn lookup(paths: &Paths, id: &str) -> Result<InstalledApp> {
    let not_installed = || anyhow!("no hermit app `{id}` is installed (see `hermit list`)");
    validate_id(id).map_err(|_| not_installed())?;
    let path = paths.desktop_file(id);
    if !path.exists() {
        return Err(not_installed());
    }
    read_entry(&path)?
        .filter(|app| app.id == id)
        .ok_or_else(not_installed)
}

pub fn remove(paths: &Paths, id: &str, purge: bool) -> Result<Removal> {
    let app = lookup(paths, id)?;
    let profile_dir = paths.profile_dir(id);

    // Check before changing anything, so a refusal leaves the app intact.
    if purge && app.profile == ProfileMode::Isolated && profile_in_use(&profile_dir) {
        bail!("{} is still open. Close it, then try again.", app.name);
    }

    let mut notes = Vec::new();
    let mut warnings = Vec::new();

    remove_if_exists(&app.path)?;
    icons::uninstall(paths, id)?;
    match desktop::remove_favourite(&Paths::desktop_file_id(id)) {
        Ok(true) => notes.push("Unpinned from the dock.".to_string()),
        Ok(false) => {}
        Err(err) => warnings.push(format!("couldn't check dock favourites: {err:#}")),
    }

    match (app.profile, purge) {
        (ProfileMode::Isolated, true) => {
            if profile_dir.exists() {
                // The id is validated, so this is always a child of the root.
                assert!(Paths::is_within(&profile_dir, &paths.profiles_root()));
                std::fs::remove_dir_all(&profile_dir)
                    .with_context(|| format!("deleting {}", profile_dir.display()))?;
                notes.push(format!("Deleted its logins and site data ({}).", profile_dir.display()));
            }
        }
        (ProfileMode::Isolated, false) => {
            if profile_dir.exists() {
                notes.push(format!(
                    "Kept its logins and site data in {} (use --purge to delete them).",
                    profile_dir.display()
                ));
            }
        }
        (ProfileMode::Shared, true) => notes.push(format!(
            "No app-specific data to delete: {} used your main Chromium profile, which is untouched.",
            app.name
        )),
        (ProfileMode::Shared, false) => {}
    }

    desktop::refresh(paths);
    Ok(Removal {
        name: app.name,
        notes,
        warnings,
    })
}

/// The app's Uninstall action: confirm with a dialog, report with a notification.
fn interactive(paths: &Paths, id: &str) -> Result<()> {
    let app = match lookup(paths, id) {
        Ok(app) => app,
        Err(err) => {
            desktop::show_error("Can't uninstall", &format!("{err:#}"));
            return Err(err);
        }
    };
    let choice = match desktop::confirm_uninstall(&app.name, app.profile == ProfileMode::Isolated) {
        Ok(choice) => choice,
        Err(err) => {
            desktop::notify(
                &format!("Can't uninstall {}", app.name),
                &format!("{err:#}"),
            );
            return Err(err);
        }
    };
    let purge = match choice {
        UninstallChoice::Cancel => return Ok(()),
        UninstallChoice::Uninstall => false,
        UninstallChoice::UninstallAndDeleteData => true,
    };
    match remove(paths, id, purge) {
        Ok(removal) => {
            desktop::notify(
                &format!("Uninstalled {}", removal.name),
                &removal.notes.join("\n"),
            );
            Ok(())
        }
        Err(err) => {
            desktop::show_error(
                &format!("Couldn't uninstall {}", app.name),
                &format!("{err:#}"),
            );
            Err(err)
        }
    }
}

/// Chromium holds `SingletonLock`, a symlink to `<hostname>-<pid>`, while a
/// profile is open. hermit can't signal snap Chromium, so it only checks.
fn profile_in_use(profile_dir: &Path) -> bool {
    let Ok(target) = std::fs::read_link(profile_dir.join("SingletonLock")) else {
        return false;
    };
    let target = target.to_string_lossy();
    let Some((host, pid)) = target.rsplit_once('-') else {
        return false;
    };
    let Ok(pid) = pid.parse::<u32>() else {
        return false;
    };
    let this_host = std::fs::read_to_string("/proc/sys/kernel/hostname").unwrap_or_default();
    host == this_host.trim() && Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn hostname() -> String {
        std::fs::read_to_string("/proc/sys/kernel/hostname")
            .unwrap()
            .trim()
            .to_string()
    }

    #[test]
    fn lock_detection() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!profile_in_use(dir.path()), "no lock");

        let lock = dir.path().join("SingletonLock");
        symlink(format!("{}-{}", hostname(), std::process::id()), &lock).unwrap();
        assert!(profile_in_use(dir.path()), "live pid on this host");

        std::fs::remove_file(&lock).unwrap();
        symlink(format!("{}-{}", hostname(), u32::MAX), &lock).unwrap();
        assert!(!profile_in_use(dir.path()), "dead pid");

        std::fs::remove_file(&lock).unwrap();
        symlink(format!("elsewhere-{}", std::process::id()), &lock).unwrap();
        assert!(!profile_in_use(dir.path()), "other host");
    }
}
