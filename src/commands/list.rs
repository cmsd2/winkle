use anyhow::{Context, Result};

use crate::cli::ListArgs;
use crate::entry::{InstalledApp, read_entry};
use crate::paths::Paths;

pub fn run(args: ListArgs) -> Result<()> {
    let paths = Paths::from_env()?;
    let apps = installed_apps(&paths)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&apps)?);
        return Ok(());
    }
    if apps.is_empty() {
        println!("No apps installed. Install one with `hermit install <url>`.");
        return Ok(());
    }

    let rows: Vec<[String; 4]> = apps
        .iter()
        .map(|a| {
            [
                a.id.clone(),
                a.name.clone(),
                a.profile.to_string(),
                a.url.to_string(),
            ]
        })
        .collect();
    let headers = ["ID", "NAME", "PROFILE", "URL"];
    let mut widths = headers.map(|h| h.chars().count());
    for row in &rows {
        for (w, cell) in widths.iter_mut().zip(row) {
            *w = (*w).max(cell.chars().count());
        }
    }
    let print_row = |cells: [&str; 4]| {
        let line = format!(
            "{:<w0$}  {:<w1$}  {:<w2$}  {}",
            cells[0],
            cells[1],
            cells[2],
            cells[3],
            w0 = widths[0],
            w1 = widths[1],
            w2 = widths[2],
        );
        println!("{}", line.trim_end());
    };
    print_row(headers);
    for row in &rows {
        print_row([&row[0], &row[1], &row[2], &row[3]]);
    }
    Ok(())
}

/// Every hermit app on disk, sorted by id. Entries hermit can't read are
/// skipped with a warning rather than hiding the rest.
pub fn installed_apps(paths: &Paths) -> Result<Vec<InstalledApp>> {
    let dir = paths.applications_dir();
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e).with_context(|| format!("reading {}", dir.display())),
    };

    let mut apps = Vec::new();
    for entry in entries {
        let path = entry?.path();
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !(file_name.starts_with("hermit-") && file_name.ends_with(".desktop")) {
            continue;
        }
        match read_entry(&path) {
            Ok(Some(app)) if Paths::desktop_file_id(&app.id) == file_name => apps.push(app),
            Ok(_) => {}
            Err(err) => eprintln!("warning: skipping {}: {err:#}", path.display()),
        }
    }
    apps.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(apps)
}
