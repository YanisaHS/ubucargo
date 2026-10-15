//! Offline end-to-end tests of dependency interpretation and reporting.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
};

use indoc::{formatdoc, indoc};
use tempfile::TempDir;

/// Creates maintained packaging, an APT index, and isolated command fixtures.
fn create_fixture(control: &str, tests: Option<&str>, packages: &str) -> TempDir {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path();
    fs::create_dir_all(root.join("package/debian")).unwrap();
    fs::create_dir(root.join("bin")).unwrap();
    fs::create_dir(root.join("cargo-home")).unwrap();
    fs::write(root.join("package/debian/control"), control).unwrap();
    fs::write(
        root.join("package/debian/changelog"),
        indoc! {"
            rust-example (1.0.0-1) unstable; urgency=medium

              * Fixture release.

             -- Example Maintainer <example@example.com>  Thu, 01 Jan 2026 00:00:00 +0000
        "},
    )
    .unwrap();
    if let Some(tests) = tests {
        fs::create_dir(root.join("package/debian/tests")).unwrap();
        fs::write(root.join("package/debian/tests/control"), tests).unwrap();
    }
    fs::write(root.join("Packages"), packages).unwrap();
    fs::write(
        root.join("indextargets"),
        format!(
            "{}|archive.ubuntu.com|https://archive.ubuntu.com/ubuntu/|resolute|universe|amd64|Packages\n",
            root.join("Packages").display(),
        ),
    )
    .unwrap();
    for name in ["apt-get", "curl"] {
        let path = root.join("bin").join(name);
        fs::write(&path, include_str!("fixtures/deps/command.sh")).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fixture
}

/// Formats a binary publication with explicit source identity and virtual packages.
fn format_package(name: &str, version: &str, source: &str, provides: &str) -> String {
    let mut paragraph = formatdoc! {"
        Package: {name}
        Version: {version}
        Architecture: all
        Source: {source}
        Description: Fixture library
    "};
    if !provides.is_empty() {
        paragraph.push_str(&format!("Provides: {provides}\n"));
    }
    paragraph.push('\n');
    paragraph
}

/// Runs the real binary and checks its status and fixture calls.
fn run_deps(root: &Path, expected_status: i32) -> Output {
    fs::write(root.join("calls"), "").unwrap();
    let mut paths = vec![root.join("bin")];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let output = Command::new(env!("CARGO_BIN_EXE_ubucargo"))
        .args([
            "deps",
            "pkg:package",
            "--suite",
            "ubuntu:resolute",
            "--architecture",
            "amd64",
        ])
        .current_dir(root)
        .env("PATH", std::env::join_paths(paths).unwrap())
        .env("UBUCARGO_TEST_FIXTURE", root)
        .env("XDG_CACHE_HOME", root.join("cache"))
        .env("CARGO_HOME", root.join("cargo-home"))
        .env("CARGO_NET_OFFLINE", "true")
        .env("LC_ALL", "C.UTF-8")
        .env("TZ", "UTC")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !root.join("unexpected-calls").exists(),
        "unexpected calls: {}\nstdout:\n{stdout}\nstderr:\n{stderr}",
        fs::read_to_string(root.join("unexpected-calls")).unwrap_or_default(),
    );
    assert_eq!(
        output.status.code(),
        Some(expected_status),
        "stdout:\n{stdout}\nstderr:\n{stderr}",
    );
    if expected_status != 2 {
        assert!(stderr.is_empty(), "{stderr}");
        assert_eq!(
            fs::read_to_string(root.join("calls")).unwrap(),
            "apt-get update\napt-get indextargets\ncurl https://crates.io/api/v1/crates/example\n",
        );
    }
    output
}

/// Reads the plain-text report after checking the CLI result.
fn read_report(root: &Path, expected_status: i32) -> String {
    String::from_utf8(run_deps(root, expected_status).stdout).unwrap()
}

/// Checks a complete report row while allowing column padding to vary.
fn assert_row(report: &str, expected: &[&str]) {
    assert!(
        report
            .lines()
            .any(|line| line.split_whitespace().eq(expected.iter().copied())),
        "missing row: {}\nreport:\n{report}",
        expected.join(" "),
    );
}

/// Checks default and additional features, including an unavailable required feature.
#[test]
fn check_required_features() {
    let control = indoc! {"
        Source: rust-example
        Build-Depends: librust-foo-1-dev (>= 1.0), librust-foo-1+default-dev (>= 1.0),
         librust-foo-1+derive-dev (>= 1.0)
    "};
    for has_derive in [true, false] {
        let mut provides =
            String::from("librust-foo-1-dev (= 1.2.0-1), librust-foo-1+default-dev (= 1.2.0-1)");
        if has_derive {
            provides.push_str(", librust-foo-1+derive-dev (= 1.2.0-1)");
        }
        let packages = format_package("librust-foo-dev", "1.2.0-1", "rust-foo", &provides);
        let fixture = create_fixture(control, None, &packages);
        let status = if has_derive { 0 } else { 1 };
        let report = read_report(fixture.path(), status);
        assert_row(
            &report,
            &[
                "foo",
                if has_derive {
                    "selected"
                } else {
                    "incompatible"
                },
                "resolute/universe",
                "1.2.0-1",
                "1",
                "(>=1.0)",
                "+derive",
            ],
        );
    }
}

/// Filters build and test dependencies for amd64 and the default build profiles.
#[test]
fn filter_architecture_and_profile_requirements() {
    let control = indoc! {"
        Source: rust-example
        Build-Depends: librust-native-dev [amd64], librust-arm-dev [arm64],
         librust-normal-dev <!nocheck>, librust-stage-dev <stage1>
    "};
    let tests = indoc! {"
        Tests: native
        Architecture: amd64
        Depends: librust-test-native-dev

        Tests: arm
        Architecture: arm64
        Depends: librust-test-arm-dev
    "};
    let mut packages = String::new();
    for name in ["native", "normal", "test-native"] {
        packages.push_str(&format_package(
            &format!("librust-{name}-dev"),
            "1.0.0-1",
            &format!("rust-{name}"),
            "",
        ));
    }
    let fixture = create_fixture(control, Some(tests), &packages);
    let report = read_report(fixture.path(), 0);
    for name in ["native", "normal", "test-native"] {
        assert_row(
            &report,
            &[
                name,
                "selected",
                "resolute/universe",
                "1.0.0-1",
                "*",
                "-default",
            ],
        );
    }
    for name in ["arm", "stage", "test-arm"] {
        assert!(!report.contains(name), "{report}");
    }
}

/// Reports every dependency when compatible, incompatible, and missing packages coexist.
#[test]
fn report_unsatisfied_dependencies() {
    let control = indoc! {"
        Source: rust-example
        Build-Depends: librust-compatiblecrate-dev, librust-incompatiblecrate-dev (>= 2.0),
         librust-missingcrate-dev
    "};
    let packages = format_package(
        "librust-compatiblecrate-dev",
        "1.0.0-1",
        "rust-compatiblecrate",
        "",
    ) + &format_package(
        "librust-incompatiblecrate-dev",
        "1.0.0-1",
        "rust-incompatiblecrate",
        "",
    );
    let fixture = create_fixture(control, None, &packages);
    let report = read_report(fixture.path(), 1);
    assert_row(
        &report,
        &[
            "compatiblecrate",
            "selected",
            "resolute/universe",
            "1.0.0-1",
            "*",
            "-default",
        ],
    );
    assert_row(
        &report,
        &[
            "incompatiblecrate",
            "incompatible",
            "resolute/universe",
            "1.0.0-1",
            "*",
            "(>=2.0)",
            "-default",
        ],
    );
    assert_row(
        &report,
        &["missingcrate", "missing", "-", "-", "*", "-default"],
    );
}

/// Propagates APT failures before issuing queries or advisory requests.
#[test]
fn reject_apt_update_failure() {
    let fixture = create_fixture("Source: rust-example\n", None, "");
    fs::write(
        fixture.path().join("update-error"),
        "fixture repository unavailable\n",
    )
    .unwrap();
    let output = run_deps(fixture.path(), 2);
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("apt-get update failed:\nfixture repository unavailable"),
        "{output:?}"
    );
    assert_eq!(
        fs::read_to_string(fixture.path().join("calls")).unwrap(),
        "apt-get update\n"
    );
}

/// Rejects maintained packaging without a source paragraph after loading fixture indexes.
#[test]
fn reject_invalid_control() {
    let fixture = create_fixture(
        "Package: librust-example-dev\nDepends: librust-foo-dev\n",
        None,
        "",
    );
    let output = run_deps(fixture.path(), 2);
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("control file has no source paragraph"),
        "{output:?}"
    );
    assert_eq!(
        fs::read_to_string(fixture.path().join("calls")).unwrap(),
        "apt-get update\napt-get indextargets\n"
    );
}

/// Checks a successful report with binary, build, and autopkgtest dependencies.
#[test]
fn compare_representative_report() {
    let fixture = create_fixture(
        include_str!("fixtures/deps/report.control"),
        Some(include_str!("fixtures/deps/report.tests-control")),
        include_str!("fixtures/deps/report.Packages"),
    );
    similar_asserts::assert_eq!(
        expected: include_str!("fixtures/deps/report.expected"),
        actual: read_report(fixture.path(), 0),
    );
}

/// Exercises valid negated dependency architectures currently rejected by the lossy parser.
// Upstream fix: https://github.com/jelmer/debian-parsers/pull/483
// Re-enable after upgrading debian-control to a release containing the fix.
#[test]
fn accept_negated_dependency_architectures() {
    let control = indoc! {"
        Source: rust-example
        Build-Depends: librust-included-dev [!arm64], librust-excluded-dev [!amd64]
    "};
    let fixture = create_fixture(
        control,
        None,
        &format_package("librust-included-dev", "1.0.0-1", "rust-included", ""),
    );
    let report = read_report(fixture.path(), 0);
    assert_row(
        &report,
        &[
            "included",
            "selected",
            "resolute/universe",
            "1.0.0-1",
            "*",
            "-default",
        ],
    );
    assert!(!report.contains("excluded"), "{report}");
}

/// Exercises profile conjunctions currently parsed as alternatives by the lossy parser.
// Upstream fix: https://github.com/jelmer/debian-parsers/pull/485
// Parsing follow-up: https://github.com/jelmer/debian-parsers/pull/486
// Re-enable after upgrading debian-control to a release containing the fix.
#[test]
fn accept_multiple_profiles_in_one_group() {
    let control = indoc! {"
        Source: rust-example
        Build-Depends: librust-included-dev <!nocheck !stage1>,
         librust-excluded-dev <!nocheck stage1>
    "};
    let fixture = create_fixture(
        control,
        None,
        &format_package("librust-included-dev", "1.0.0-1", "rust-included", ""),
    );
    let report = read_report(fixture.path(), 0);
    assert_row(
        &report,
        &[
            "included",
            "selected",
            "resolute/universe",
            "1.0.0-1",
            "*",
            "-default",
        ],
    );
    assert!(!report.contains("excluded"), "{report}");
}
