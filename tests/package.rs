//! Offline CLI tests for source-package creation and update.

use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use indoc::{formatdoc, indoc};
use tempfile::TempDir;

/// Creates a dependency-free local crate and a private Cargo cache.
fn create_fixture() -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    fs::create_dir_all(root.join("crate/src")).unwrap();
    fs::create_dir(root.join("cargo-home")).unwrap();
    fs::write(
        root.join("crate/src/lib.rs"),
        "/// Example value.\npub const VALUE: u8 = 1;\n",
    )
    .unwrap();
    write_crate_manifest(root, "1.0.0");
    directory
}

/// Selects the upstream release without using a registry.
fn write_crate_manifest(root: &Path, version: &str) {
    fs::write(
        root.join("crate/Cargo.toml"),
        formatdoc! {r#"
            [package]
            name = "example"
            version = "{version}"
            edition = "2021"
            license = "MIT"
            description = "Example library"
        "#},
    )
    .unwrap();
}

/// Runs the package CLI with offline Cargo and an isolated cache.
fn run_package(root: &Path, arguments: &[&str], expected_status: i32) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ubucargo"));
    command
        .arg("package")
        .args(arguments)
        .current_dir(root)
        .env("CARGO_HOME", root.join("cargo-home"))
        .env("CARGO_NET_OFFLINE", "true")
        .env("DEBFULLNAME", "Example Maintainer")
        .env("DEBEMAIL", "example@example.com")
        .env("LC_ALL", "C.UTF-8")
        .env("TZ", "UTC");
    run_command(&mut command, expected_status)
}

/// Checks an external command's exit status, retaining output for failure diagnostics.
fn run_command(command: &mut Command, expected_status: i32) -> Output {
    let output = command.output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(expected_status),
        "{command:?}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    output
}

/// Creates, regenerates, and upgrades a package while preserving its released changelog.
#[test]
fn create_and_upgrade_package() {
    let fixture = create_fixture();
    let root = fixture.path();
    let packages = root.join("packages");
    let package = packages.join("rust-example");
    run_package(
        root,
        &["local:crate", "--package-dir", "packages/rust-example"],
        0,
    );
    assert!(packages.join("rust-example_1.0.0.orig.tar.gz").is_file());
    assert!(package.join("debian/debcargo.toml").is_file());
    assert!(package.join("debian/ubucargo-state.json").is_file());
    assert_eq!(
        fs::read_to_string(package.join("debian/source/format")).unwrap(),
        "3.0 (quilt)\n"
    );
    let control_path = package.join("debian/control");
    let control = fs::read_to_string(&control_path).unwrap();
    assert!(control.contains("Source: rust-example\n"));
    assert!(control.contains("Package: librust-example-dev\n"));
    assert!(control.contains("Maintainer: Ubuntu Developers"));
    let changelog_path = package.join("debian/changelog");
    let changelog = fs::read_to_string(&changelog_path).unwrap();
    assert!(changelog.starts_with("rust-example (1.0.0-0ubuntu1) UNRELEASED;"));

    run_package(root, &["pkg:packages/rust-example"], 0);
    assert_eq!(fs::read_to_string(&control_path).unwrap(), control);
    assert_eq!(fs::read_to_string(&changelog_path).unwrap(), changelog);

    let released = changelog.replacen("UNRELEASED", "noble", 1);
    fs::write(&changelog_path, &released).unwrap();
    write_crate_manifest(root, "1.0.1");
    run_package(root, &["pkg:packages/rust-example"], 0);
    assert!(packages.join("rust-example_1.0.1.orig.tar.gz").is_file());
    assert!(packages.join("rust-example_1.0.0.orig.tar.gz").is_file());
    assert!(
        fs::read_to_string(package.join("Cargo.toml"))
            .unwrap()
            .contains("version = \"1.0.1\"")
    );
    let upgraded = fs::read_to_string(&changelog_path).unwrap();
    assert!(upgraded.starts_with("rust-example (1.0.1-0ubuntu1) UNRELEASED;"));
    assert!(upgraded.ends_with(&released));

    run_package(root, &["pkg:packages/rust-example"], 0);
    assert_eq!(fs::read_to_string(&changelog_path).unwrap(), upgraded);
}

/// Preserves a maintainer override and lets the maintainer adopt the generated hint.
#[test]
fn preserve_maintainer_overrides() {
    let fixture = create_fixture();
    let root = fixture.path();
    run_package(
        root,
        &["local:crate", "--package-dir", "packages/rust-example"],
        0,
    );
    let debian = root.join("packages/rust-example/debian");
    let control = fs::read_to_string(debian.join("control")).unwrap();
    let edited = format!("{control}\n# Maintainer override.\n");
    fs::write(debian.join("control"), &edited).unwrap();
    fs::write(debian.join("local-notes"), "Maintainer notes.\n").unwrap();

    run_package(root, &["pkg:packages/rust-example"], 0);
    assert_eq!(fs::read_to_string(debian.join("control")).unwrap(), edited);
    assert_eq!(
        fs::read_to_string(debian.join("control.debcargo.hint")).unwrap(),
        control
    );
    assert_eq!(
        fs::read_to_string(debian.join("local-notes")).unwrap(),
        "Maintainer notes.\n"
    );

    fs::copy(debian.join("control.debcargo.hint"), debian.join("control")).unwrap();
    run_package(root, &["pkg:packages/rust-example"], 0);
    assert_eq!(fs::read_to_string(debian.join("control")).unwrap(), control);
    assert!(!debian.join("control.debcargo.hint").exists());
}

/// Resolves an edited package's unknown ownership with explicit keep and replace choices.
#[test]
fn resolve_ambiguous_ownership() {
    let fixture = create_fixture();
    let root = fixture.path();
    run_package(
        root,
        &["local:crate", "--package-dir", "packages/rust-example"],
        0,
    );
    let debian = root.join("packages/rust-example/debian");
    let control = fs::read_to_string(debian.join("control")).unwrap();
    let edited = format!("{control}\n# Maintainer override.\n");
    let changelog = fs::read_to_string(debian.join("changelog")).unwrap();
    // Packages imported from other tools have no ubucargo ownership manifest.
    fs::remove_file(debian.join("ubucargo-state.json")).unwrap();
    fs::write(debian.join("control"), &edited).unwrap();

    let output = run_package(root, &["pkg:packages/rust-example"], 2);
    assert!(String::from_utf8_lossy(&output.stdout).contains("Ambiguous debian/control"));
    assert_eq!(fs::read_to_string(debian.join("control")).unwrap(), edited);
    assert_eq!(
        fs::read_to_string(debian.join("changelog")).unwrap(),
        changelog
    );
    run_package(
        root,
        &["pkg:packages/rust-example", "--keep", "debian/control"],
        0,
    );
    assert_eq!(fs::read_to_string(debian.join("control")).unwrap(), edited);
    assert_eq!(
        fs::read_to_string(debian.join("control.debcargo.hint")).unwrap(),
        control
    );

    run_package(
        root,
        &["pkg:packages/rust-example", "--replace", "debian/control"],
        0,
    );
    assert_eq!(fs::read_to_string(debian.join("control")).unwrap(), control);
    assert!(!debian.join("control.debcargo.hint").exists());
}

/// Rejects local edits to the source tree and preserves local additions when forced.
#[test]
fn reject_local_source_edits() {
    let fixture = create_fixture();
    let root = fixture.path();
    run_package(
        root,
        &["local:crate", "--package-dir", "packages/rust-example"],
        0,
    );
    let package = root.join("packages/rust-example");
    let upstream = fs::read_to_string(root.join("crate/src/lib.rs")).unwrap();
    let edited = "// Locally edited upstream.\n";
    fs::write(package.join("src/lib.rs"), edited).unwrap();
    fs::write(package.join("notes.txt"), "Local notes.\n").unwrap();
    let changelog = fs::read_to_string(package.join("debian/changelog")).unwrap();

    let output = run_package(root, &["pkg:packages/rust-example"], 2);
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("source conflicts:"), "{error}");
    assert!(error.contains("src/lib.rs"), "{error}");
    assert_eq!(
        fs::read_to_string(package.join("src/lib.rs")).unwrap(),
        edited
    );
    assert_eq!(
        fs::read_to_string(package.join("debian/changelog")).unwrap(),
        changelog
    );

    run_package(root, &["pkg:packages/rust-example", "--force"], 0);
    assert_eq!(
        fs::read_to_string(package.join("src/lib.rs")).unwrap(),
        upstream
    );
    assert_eq!(
        fs::read_to_string(package.join("notes.txt")).unwrap(),
        "Local notes.\n"
    );
}

/// Keeps an applied maintainer patch usable after an upgrade and rejects unrefreshed changes.
#[test]
fn preserve_applied_patch() {
    let fixture = create_fixture();
    let root = fixture.path();
    run_package(
        root,
        &["local:crate", "--package-dir", "packages/rust-example"],
        0,
    );
    let package = root.join("packages/rust-example");
    let debian = package.join("debian");
    let patch = indoc! {"
        Description: Change the example value.
        --- a/src/lib.rs
        +++ b/src/lib.rs
        @@ -1,2 +1,2 @@
         /// Example value.
        -pub const VALUE: u8 = 1;
        +pub const VALUE: u8 = 2;
    "};
    fs::create_dir_all(debian.join("patches")).unwrap();
    fs::write(debian.join("patches/example.patch"), patch).unwrap();
    fs::write(debian.join("patches/series"), "example.patch\n").unwrap();
    run_command(
        Command::new("quilt")
            .args(["push", "--quiltrc=-", "-a"])
            .env("QUILT_PATCHES", "debian/patches")
            .current_dir(&package),
        0,
    );
    write_crate_manifest(root, "1.0.1");
    let upstream = "/// Example value.\npub const VALUE: u8 = 1;\n// New upstream comment.\n";
    fs::write(root.join("crate/src/lib.rs"), upstream).unwrap();
    run_package(root, &["pkg:packages/rust-example"], 0);
    assert_eq!(
        fs::read_to_string(package.join(".pc/applied-patches")).unwrap(),
        "example.patch\n"
    );
    assert_eq!(
        fs::read_to_string(debian.join("patches/example.patch")).unwrap(),
        patch
    );
    let patched = upstream.replace("= 1;", "= 2;");
    assert_eq!(
        fs::read_to_string(package.join("src/lib.rs")).unwrap(),
        patched
    );

    run_command(
        Command::new("quilt")
            .args(["pop", "--quiltrc=-", "-a"])
            .env("QUILT_PATCHES", "debian/patches")
            .current_dir(&package),
        0,
    );
    assert_eq!(
        fs::read_to_string(package.join("src/lib.rs")).unwrap(),
        upstream
    );
    run_command(
        Command::new("quilt")
            .args(["push", "--quiltrc=-", "-a"])
            .env("QUILT_PATCHES", "debian/patches")
            .current_dir(&package),
        0,
    );

    let unrefreshed = patched.replace("= 2;", "= 3;");
    fs::write(package.join("src/lib.rs"), &unrefreshed).unwrap();
    let changelog = fs::read_to_string(debian.join("changelog")).unwrap();
    let output = run_package(root, &["pkg:packages/rust-example"], 2);
    assert!(String::from_utf8_lossy(&output.stderr).contains("unrefreshed changes"));
    assert_eq!(
        fs::read_to_string(package.join("src/lib.rs")).unwrap(),
        unrefreshed
    );
    assert_eq!(
        fs::read_to_string(debian.join("changelog")).unwrap(),
        changelog
    );
}

/// Repacks an orig tarball using the maintainer's configured exclusion and suffix.
#[test]
fn repack_source() {
    let fixture = create_fixture();
    let root = fixture.path();
    fs::write(root.join("crate/bundled.txt"), "Excluded upstream data.\n").unwrap();
    run_package(
        root,
        &["local:crate", "--package-dir", "packages/rust-example"],
        0,
    );
    let packages = root.join("packages");
    let package = packages.join("rust-example");
    let original_orig = packages.join("rust-example_1.0.0.orig.tar.gz");
    let original_bytes = fs::read(&original_orig).unwrap();
    let config_path = package.join("debian/debcargo.toml");
    let config = fs::read_to_string(&config_path).unwrap();
    let repack_config = format!("{config}excludes = [\"bundled.txt\"]\nrepack_suffix = \"dfsg\"\n");
    fs::write(&config_path, &repack_config).unwrap();

    run_package(root, &["pkg:packages/rust-example"], 0);
    let repacked_orig = packages.join("rust-example_1.0.0+dfsg.orig.tar.gz");
    assert!(repacked_orig.is_file());
    assert_eq!(fs::read(&original_orig).unwrap(), original_bytes);
    assert!(!package.join("bundled.txt").exists());
    let output = run_command(
        Command::new("tar")
            .arg("--list")
            .arg("--file")
            .arg(&repacked_orig),
        0,
    );
    let listing = String::from_utf8(output.stdout).unwrap();
    assert!(listing.contains("/src/lib.rs"));
    assert!(!listing.contains("bundled.txt"));
    assert!(
        fs::read_to_string(package.join("debian/changelog"))
            .unwrap()
            .starts_with("rust-example (1.0.0+dfsg-0ubuntu1) UNRELEASED;")
    );
    assert_eq!(fs::read_to_string(&config_path).unwrap(), repack_config);
}
