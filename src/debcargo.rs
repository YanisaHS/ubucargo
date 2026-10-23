//! Runs debcargo while keeping useful diagnostics visible.

use std::process::Command;

use anyhow::Result;

use crate::util;

/// Reports filtered output on success and preserves full diagnostics on failure.
pub fn run_command(command: &mut Command, operation: &str) -> Result<()> {
    let output = util::run_command(command, operation)?;
    for stream in [&output.stdout, &output.stderr] {
        eprint!("{}", filter_output(stream));
    }
    Ok(())
}

/// Removes known progress and FIXME advice, retaining other diagnostic lines.
fn filter_output(output: &[u8]) -> String {
    let mut filtered = String::new();
    let mut in_fixme_report = false;
    for line in String::from_utf8_lossy(output).lines() {
        // Debcargo emits color escapes even when its output is captured.
        let mut plain = String::new();
        let mut chars = line.chars().peekable();
        while let Some(character) = chars.next() {
            if character == '\x1b' && chars.next_if_eq(&'[').is_some() {
                for code in chars.by_ref() {
                    if ('@'..='~').contains(&code) {
                        break;
                    }
                }
            } else {
                plain.push(character);
            }
        }
        let text = plain.trim();
        if text.is_empty() {
            in_fixme_report = false;
            continue;
        }
        if matches!(
            text,
            "FIXME found in the following files." | "To fix, try combinations of the following:"
        ) {
            in_fixme_report = true;
            continue;
        }
        if in_fixme_report {
            if plain.starts_with("\t •  ")
                || plain.starts_with("\t(•) ")
                || plain.starts_with("\t    ")
            {
                continue;
            }
            in_fixme_report = false;
        }
        if matches!(
            text,
            "applying patches.."
                | "reloading Cargo.toml.."
                | "unapplying patches again.."
                | "crate tarball was modified; repacking for debian"
                | "Cargo.toml manually canonicalized!"
                | "No patches applied"
        ) || text.starts_with("Package Source: ")
            || text.starts_with("Original Tarball for package: ")
            || text.starts_with("Filtered out files from .orig.tar.gz: ")
            || text.starts_with("Applying patch ")
            || text.starts_with("patching file ")
            || text.starts_with("Now at patch ")
            || text.starts_with("Removing patch ")
            || text.starts_with("Restoring ")
        {
            continue;
        }
        if text.starts_with("Hunk #") && text.contains(" succeeded at ") && !text.contains("fuzz") {
            continue;
        }
        if plain.starts_with(' ')
            && [
                "Updating crates.io index",
                "Downloading crates ...",
                "Downloaded ",
                "Packaging ",
                "Packaged ",
                "Archiving ",
            ]
            .iter()
            .any(|prefix| text.starts_with(prefix))
        {
            continue;
        }
        filtered.push_str(&plain);
        filtered.push('\n');
    }
    filtered
}
