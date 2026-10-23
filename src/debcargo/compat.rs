//! Defines supported debcargo versions and their packaging defaults.

use std::process::Command;

use anyhow::{Context, Result, bail};
use semver::{Version, VersionReq};

use crate::util::run_command;

const VERSION_REQUIREMENT: &str = "^2.8.2";

/// Returns the installed debcargo version when it is compatible.
pub fn check_version() -> Result<Version> {
    let output = run_command(
        Command::new("debcargo").arg("--version"),
        "debcargo --version",
    )?;
    parse_version(String::from_utf8_lossy(&output.stdout).trim())
}

/// Parses and checks one `debcargo --version` response.
fn parse_version(output: &str) -> Result<Version> {
    let version = output
        .strip_prefix("debcargo ")
        .with_context(|| format!("unrecognized debcargo version output {output:?}"))?;
    let version = Version::parse(version)
        .with_context(|| format!("unrecognized debcargo version output {output:?}"))?;
    let requirement = VersionReq::parse(VERSION_REQUIREMENT).unwrap();
    if !requirement.matches(&version) {
        bail!("unsupported debcargo {version}; this release requires {requirement}");
    }
    Ok(version)
}

/// Returns debcargo's default repack suffix when exclusions are configured.
pub fn get_default_repack_suffix(version: &Version) -> &'static str {
    if *version < Version::new(2, 8, 4) {
        "dfsg"
    } else {
        "ds"
    }
}
