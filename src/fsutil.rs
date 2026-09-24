//! Small filesystem helpers.

use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};

/// Write `contents` to `path` atomically: write a temp file in the same
/// directory, then rename it over the target. Creates parent directories.
pub fn write_atomic(path: &Path, contents: &[u8]) -> Result<()> {
    let dir = path.parent().context("path has no parent directory")?;
    fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let file_name = path.file_name().context("path has no file name")?.to_string_lossy();
    let tmp = dir.join(format!(".{file_name}.tmp-{}", std::process::id()));
    let result = (|| {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(contents)?;
        f.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result.with_context(|| format!("writing {}", path.display()))
}

/// Remove a file, treating "already gone" as success. Returns whether it existed.
pub fn remove_if_exists(path: &Path) -> Result<bool> {
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e).with_context(|| format!("removing {}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_and_replaces_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b/file.txt");
        write_atomic(&path, b"one").unwrap();
        write_atomic(&path, b"two").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"two");
        let leftovers: Vec<_> = fs::read_dir(path.parent().unwrap()).unwrap().collect();
        assert_eq!(leftovers.len(), 1, "temp file left behind");
    }

    #[test]
    fn remove_if_exists_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f");
        fs::write(&path, b"x").unwrap();
        assert!(remove_if_exists(&path).unwrap());
        assert!(!remove_if_exists(&path).unwrap());
    }
}
