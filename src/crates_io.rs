//! Resolves canonical crate names and stable releases from crates.io metadata.

use anyhow::{Context, Result, bail};
use reqwest::{StatusCode, blocking::Client};
use semver::Version;
use serde::Deserialize;

use crate::resolve::normalize_crate_name;

/// Published crate identity and highest non-yanked stable release.
pub struct Crate {
    /// Canonical published package name.
    pub name: String,
    /// Highest stable version, when one is published.
    pub latest_version: Option<Version>,
}

/// Identity and version records returned by the crate metadata endpoint.
#[derive(Deserialize)]
struct CrateMetadata {
    /// Published crate identity.
    #[serde(rename = "crate")]
    identity: CrateIdentity,
    /// Published releases in API order.
    versions: Vec<ReleaseMetadata>,
}

/// Canonical identity returned by crates.io.
#[derive(Deserialize)]
struct CrateIdentity {
    /// Published package name.
    name: String,
}

/// Fields needed to select a stable crates.io release.
#[derive(Deserialize)]
struct ReleaseMetadata {
    /// Semantic release version.
    num: String,
    /// Whether new resolution should exclude this release.
    yanked: bool,
}

/// Queries a crate's canonical name and latest stable release, returning `None` for HTTP 404.
pub fn read_crate(crate_name: &str, api: &str) -> Result<Option<Crate>> {
    if crate_name.is_empty()
        || !crate_name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
    {
        bail!("invalid crate name {crate_name:?}");
    }
    let response = Client::builder()
        .user_agent(concat!("ubucargo/", env!("CARGO_PKG_VERSION")))
        .build()?
        .get(format!("{}/crates/{crate_name}", api.trim_end_matches('/')))
        .send()
        .with_context(|| format!("query crates.io for {crate_name}"))?;
    match response.status() {
        StatusCode::NOT_FOUND => return Ok(None),
        StatusCode::OK => {}
        status => bail!("crates.io lookup for {crate_name} failed: HTTP {status}"),
    }
    let metadata: CrateMetadata = response
        .json()
        .with_context(|| format!("parse crates.io metadata for {crate_name}"))?;
    if normalize_crate_name(&metadata.identity.name) != normalize_crate_name(crate_name) {
        bail!(
            "crates.io returned crate {} instead of {crate_name}",
            metadata.identity.name
        );
    }
    let mut latest: Option<Version> = None;
    for release in metadata.versions {
        let version = Version::parse(&release.num)
            .with_context(|| format!("parse crates.io version for {crate_name}"))?;
        if release.yanked || !version.pre.is_empty() {
            continue;
        }
        if latest
            .as_ref()
            .is_none_or(|old| version.cmp_precedence(old).is_gt())
        {
            latest = Some(version);
        }
    }
    Ok(Some(Crate {
        name: metadata.identity.name,
        latest_version: latest,
    }))
}
