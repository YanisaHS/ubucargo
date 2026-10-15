//! Acquires existing orig tarballs used as source-merge baselines.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};

use crate::{changelog::TopChangelog, util::run_streaming_command};

/// Finds the old orig locally or downloads and saves it beside the package.
pub fn acquire_old_orig(root: &Path, top: &TopChangelog) -> Result<PathBuf> {
    let parent = root.parent().context("package root has no parent")?;
    if let Some(local) = find_orig(parent, top) {
        return Ok(local);
    }

    let download = tempfile::tempdir_in(parent).context("create orig download directory")?;
    eprintln!("Downloading orig {} {} ...", top.source, top.version);
    run_streaming_command(
        Command::new("pull-lp-source")
            .arg("--download-only")
            .arg(&top.source)
            .arg(&top.version)
            .current_dir(download.path()),
        "pull-lp-source",
    )?;

    // pull-lp-source already verifies downloaded source files against their `.dsc`.
    // So here we only check that the downloaded tarball exists.
    let orig = find_orig(download.path(), top).with_context(|| {
        format!(
            "pull-lp-source did not produce an orig tarball for {} {}",
            top.source, top.upstream
        )
    })?;
    let destination = parent.join(orig.file_name().context("orig has no filename")?);
    // The download directory shares the destination filesystem, so this move is atomic.
    fs::rename(&orig, &destination)
        .with_context(|| format!("save orig tarball {}", destination.display()))?;
    Ok(destination)
}

/// Finds the main orig tarball using the compression formats supported by dpkg-source.
fn find_orig(directory: &Path, top: &TopChangelog) -> Option<PathBuf> {
    for extension in ["gz", "xz", "bz2", "lzma"] {
        let path = directory.join(format!(
            "{}_{}.orig.tar.{extension}",
            top.source, top.upstream
        ));
        if path.is_file() {
            return Some(path);
        }
    }
    None
}
