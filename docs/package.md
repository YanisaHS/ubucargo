# `ubucargo package`

## Synopsis

```console
ubucargo package [<INPUT> [<VERSION>]] [--package-dir <DIR>] \
  [--config <PATH>] [--prepare-only] [--force] \
  [--keep-staging] [--keep <PATH>]... [--replace <PATH>]... [--crates-io-api <URL>]
```

`package` creates or updates a Debian source tree and orig tarball. By default, the source tree includes upstream source, generated packaging, configuration, the ownership manifest, and generated-file hints. The orig tarball is placed beside the source directory.

`--keep-staging` retains the printed temporary debcargo staging directory for inspection, including when generation fails.

Debcargo diagnostics appear on stderr after successful generation or release resolution, with routine progress and FIXME advice filtered out. Patch fuzz notices remain visible. Failed debcargo commands include their complete stdout and stderr in the error report.

## Input and version selection

`package` and `deps` share the following input selectors and version rules:

| Input selector | Selected input | Version selection |
| --- | --- | --- |
| `crate:serde` | Crates.io upstream release | Exact Cargo `<VERSION>`; default latest |
| `ubuntu:resolute/rust-serde` | Published Ubuntu source package | Exact Debian `<VERSION>`; default highest published |
| `debian:unstable/rust-serde` | Published Debian source package | Exact Debian `<VERSION>`; default highest published |
| `ppa:<OWNER>/<NAME>/<SERIES>/<SOURCE>` | Published public PPA source package | Exact Debian `<VERSION>`; default highest published |
| `pkg:./rust-serde` | Existing maintained source package | Current package version |
| `local:../serde` | Current local Cargo source | Version from local `Cargo.toml` |

Positional `<VERSION>` is accepted for crates.io, Ubuntu Archive, Debian archive, and PPA inputs. Local paths resolve against the working directory. Explicit selectors require recognized prefixes, nonempty fields, and well-formed inputs.

Shorthand forms have fixed precedence:

1. `.`, `..`, and paths beginning with `./`, `../`, or `/` select an existing source package containing `debian/changelog`. Explicit `pkg:` requires the same file. Local Cargo crates use `local:<PATH>`.
2. `<SUITE>/<SOURCE>` selects an Ubuntu Archive source package. Use `./` for nested filesystem paths to distinguish them from Archive inputs.
3. A bare name selects crates.io, regardless of whether a directory with that name exists.

With no input, both commands select the nearest directory at or above the working directory containing `debian/changelog`, then validate that package's contents. Finding such a package is required for omitted-input invocations. `deps` reads maintained control files and the top changelog entry; `package` additionally requires usable `debian/debcargo.toml` and literal `[package].name` and `[package].version` in the root `Cargo.toml` for regeneration. A `local:` input selects the Cargo source in its input tree.

Ubuntu Archive, Debian archive, and PPA selectors fully specify the input location. `<OWNER>` and `<NAME>` identify the PPA, `<SERIES>` is its Ubuntu series (for example, `resolute`), and `<SOURCE>` is the Debian source-package name (for example, `rust-serde`). PPA inputs require all four fields.

For Ubuntu Archive inputs, `<SUITE>` is either a base series or a series with a pocket suffix. A bare `resolute` considers release, updates, and security in `main` and `universe`. `resolute-updates`, `resolute-security`, `resolute-proposed`, and `resolute-backports` each select only that pocket. An explicit Debian version must exist in the selected source indexes; otherwise the highest Debian version wins, with deterministic location ordering for ties.

For Debian inputs, `<SUITE>` selects exactly that suite from `https://deb.debian.org/debian` in `main`, for example `unstable`, `sid`, `testing`, or `trixie`. Version selection uses the same Debian ordering and deterministic ties as Ubuntu Archive inputs. Regeneration prepares the selected Debian packaging for Ubuntu, including the Ubuntu maintainer adjustment and changelog conventions described below.

For crates.io inputs, both commands query the crates.io metadata API to resolve the canonical published name, treating hyphens, underscores, and case as equivalent. For example, `serde-json` resolves to `serde_json`. `--crates-io-api <URL>` overrides the metadata API base URL, which defaults to `https://crates.io/api/v1`.

Omitting `<VERSION>` selects the highest non-yanked stable release from the same response. An exact request may select a prerelease or yanked release. Crates published only as prereleases require an exact version.

For dependency-checking suites, pockets, and additional PPAs, see [`ubucargo deps`](deps.md).

## Destination selection

`--package-dir` selects the destination for every input kind. Select the input separately using the rules above; for example, use `ubucargo package pkg:<PATH>` to select an existing package from outside its directory.

An explicit or implicit package input is updated in place by default. With `--package-dir`, the same directory is updated in place; a different, new directory receives a regenerated copy while the input remains untouched. A different existing destination is rejected. Paths referring to the same directory through symlinks are treated alike.

Published source inputs (Ubuntu Archive, Debian archive, and PPA) require a new destination, defaulting to `./<SOURCE>` in the current directory. Use `--package-dir` to select another new directory.

Without `--package-dir`, a crates.io input uses the nearest parent package containing `debian/changelog` as its destination; if none is found, the destination defaults to the unsuffixed Debian source name in the current directory.

For crates.io and local crate inputs, an existing destination supplies configuration, patches, maintainer overrides, and generated-file ownership state. A nonexistent destination creates a package with the supplied `--config` or default configuration, even when the current directory is inside another package. An existing destination must be a valid source package for the same Cargo crate. To switch from local source to a registry release, remove `crate_src_path` from the destination's configuration.

`local:<PATH>` requires an explicit `--package-dir`. The resolved crate and package directories must be separate, non-nested directory trees. For an existing package, the explicit local path must resolve to the same directory as the saved `crate_src_path`. To change the source location or switch to local source, edit `debian/debcargo.toml` first. Subsequent runs resolve that setting automatically. Local Cargo inputs use Cargo metadata to resolve workspace-inherited versions.

Orig tarballs remain beside the destination directory. Registry-source packages require matching contents for an existing orig filename. Local-source packages replace their generated `+local` orig as the source changes.

Output identifies the operation as `Create new package: <DIR>`, `Update existing package: <DIR>`, or `Create package from existing packaging: <DIR>`. The last form describes a package copy or a published input that carries maintained packaging into a new destination.

## Initial configuration

`--config <PATH>` supplies the initial debcargo configuration for a new package from crates.io or a local Cargo input. The file is used during generation and saved as `debian/debcargo.toml`. See debcargo's [`debcargo.toml.example`](https://salsa.debian.org/rust-team/debcargo/-/blob/master/debcargo.toml.example) for the available settings. For example, save the following configuration as `bevy.toml` to enable prerelease dependency conversion:

```toml
allow_prerelease_deps = true
```

Then create the package using that file:

```console
ubucargo package bevy --config ./bevy.toml
```

The configuration path resolves against the working directory. Relative `crate_src_path` values resolve against the supplied configuration's directory and are rebased for the destination's `debian` directory. An explicit `local:<PATH>` input supplies the local crate path in preference to the configuration's value.

Existing packages and published source inputs use their own `debian/debcargo.toml`; edit that file to change their configuration. Ubucargo writes the file only when initializing a new destination. When initializing a copy or imported package, relative `crate_src_path` values are rebased once to retain the same source location; the original package configuration stays untouched.

## Regeneration requirements

For an existing package input, ubucargo regenerates the crate and version identified by the root `Cargo.toml`, using any configured local source. The top `debian/changelog` entry must describe the upstream source currently present in the working tree.

Published inputs are downloaded and regenerated in staging, preserving the upstream release contained in the selected source package. They require usable `debian/debcargo.toml`, root Cargo identity, and changelog, and use the same update rules and `--keep`/`--replace` decisions as an existing package. Regeneration uses the main orig tarball as its source-merge baseline. Acquisition and validation complete before writing to the destination.

Package copies use the same staging and writing flow as in-place updates and published inputs. Maintainer files, local additions, and generated-file ownership state travel with the copy. Relative `crate_src_path` settings are rebased to keep referring to the same local crate. Input and destination trees must be separate and non-nested, and the destination must remain separate from any configured local crate. Full generation pops applied quilt patches and restores the maintained patch position in staging; `--prepare-only` leaves the staged stack unapplied.

The selected release must belong to the same Cargo crate as the existing root `Cargo.toml` (allowing normalized case and underscore/hyphen spelling). The Debian source name may change: adding or removing `semver_suffix`, or upgrading a suffixed package to another semver line, regenerates the package under its new identity in the same working directory. During full generation, maintainer overrides follow the normal update rules; if the planned `debian/control` has a `Source` field inconsistent with the new identity, ubucargo issues a warning, since the maintainer would need to correct this before the package can build.

Cargo's release selection considers version constraints and publication eligibility. If the selected crate's `[package]` table in `Cargo.toml` declares `rust-version`, debcargo includes that minimum version in the generated Debian `rustc` dependencies.

Local-source packaging requires dependencies to be resolvable from crates.io during generation. The generated Debian package builds against dependencies declared from the Ubuntu Archive. Local outputs are disposable development builds. Prepare Archive uploads from published crates.io releases, with `crate_src_path` removed from the configuration.

### Local builds

`package` gives local source selected through `local:<PATH>` or `crate_src_path` a fixed `+local` suffix. For example, Cargo version `0.1.7` produces Debian upstream version `0.1.7+local` and orig filename `rust-ubucargo_0.1.7+local.orig.tar.gz`.

Each run generates the package once. Matching orig bytes are reused; changed contents replace the local orig under the same filename. The old orig is captured in staging as the source-merge baseline before replacement. This also applies to `--prepare-only` and package copies. Published release origs retain their separate filenames and normal content checks.

An explicit or default repack suffix precedes the local suffix: `repack_suffix = "ds"` produces `0.1.7+ds+local`. Ubucargo passes the combined suffix through its temporary debcargo configuration; the maintained configuration retains the original setting.

Local versions sort after their base version: `0.1.7 < 0.1.7+local < 0.1.8`. Keeping the last released Cargo version during development lets the next release supersede its local builds. Local builds allow version changes in either direction as the working tree or repack configuration changes.

Use separate parent directories for independent development package trees, since the orig beside a local package is mutable. Rebuild `.dsc` and `.changes` files after changing local orig contents so their checksums match the current archive.

To package the next published release, remove `crate_src_path` from `debian/debcargo.toml` and select that release, for example `ubucargo package ubucargo 0.1.8`.

## Examples

These invocations create or update packages:

```console
# Create the latest serde package.
ubucargo package serde

# Create a particular release in a chosen directory.
ubucargo package serde 1.0.220 --package-dir ./rust-serde

# Create a package from an unpublished local crate.
ubucargo package local:../example --package-dir ./rust-example

# Regenerate an explicit package.
ubucargo package ./rust-serde

# Regenerate a copy in a new directory.
ubucargo package ./rust-serde --package-dir ./copies/rust-serde

# Regenerate packaging from Ubuntu, Debian, or a PPA.
ubucargo package ubuntu:resolute-proposed/rust-serde
ubucargo package debian:unstable/rust-serde
ubucargo package ppa:owner/staging/resolute/rust-serde

# Regenerate the current package at its existing version.
cd rust-serde
ubucargo package

# Move the current package to another release.
ubucargo package serde 1.0.229
```

## Preparing source for patch porting

`--prepare-only` uses the same input, version, destination, source-merge, and configuration rules as full generation. Use it to move to a new release whose maintainer patches need porting, or to initialize a package before adding patches:

```console
ubucargo package serde 1.0.229 --package-dir ./rust-serde --prepare-only
cd rust-serde
# Port the maintained patches with quilt, then generate the full packaging.
ubucargo package
```

For a fresh crate input, preparation writes upstream source, an orig tarball beside the destination, `debian/debcargo.toml`, the prepared changelog, `debian/source/format`, and any automatic patches and their series. Full generation in this minimal tree creates the remaining packaging and initializes `debian/ubucargo-state.json`. Package discovery uses `debian/changelog`, so omitted-input commands recognize prepared trees.

For maintained inputs, including copies and published packages, preparation updates upstream source, the orig tarball, the changelog, and the automatic patches and their series entries. Existing packaging, generated-file hints, and ownership state are retained. Maintainer patches and the other series lines retain their contents and order. The fresh automatic patch set replaces the previous set, including removal when generation produces none.

Preparation rejects unrefreshed applied patch edits, then pops the staged quilt stack and clears its old backup state. The result has all patches unapplied. Debcargo generates against the new source using the configuration and prepared changelog, with only fresh automatic patches in its overlay. Successful debcargo generation and output validation are required before writing the result. If generation needs configuration changes, edit `debian/debcargo.toml`, or supply `--config` for a fresh package, and retry.

`--force` resolves source conflicts as in full generation, and `--keep-staging` retains staging directories for inspection. Use `--keep` and `--replace` with full generation to resolve generated-file ownership decisions. Combining either with `--prepare-only` is a command-line error.

`deps` reads maintained control files. In a fresh prepared tree it asks you to run `ubucargo package` to generate them; for an existing package it reports the retained metadata until full generation updates it.

## Updating packages

Ubucargo invokes debcargo's `package` command for the selected exact release. Debcargo and Cargo obtain the crate from crates.io or the configured local source, apply `debian/debcargo.toml`, derive Debian names and versions, copy or repack the orig tarball, extract the upstream source, apply the retained patch stack temporarily for full generation, and generate `debian/`. Preparation uses only fresh automatic patches during this generation.

When creating a package with full generation, ubucargo writes the generated source tree and Debian packaging to the destination directory, adds `debian/debcargo.toml`, and records generated-file fingerprints in `debian/ubucargo-state.json`. Primary files that match the generated output have no corresponding `.debcargo.hint` files.

By default, the initial configuration for both crates.io and local crates includes:

```toml
maintainer = "Ubuntu Developers <ubuntu-devel-discuss@lists.ubuntu.com>"
```

This setting gives debcargo the same maintainer for the first generation and subsequent runs, including the generated `debian/*` copyright attribution. A supplied `--config` provides the initial maintainer settings. Existing configurations are retained; initializing a copy rebases any relative local-crate path to preserve its meaning.

When updating an existing package, ubucargo preserves durable maintainer-owned state:

- `debian/changelog`;
- `debian/debcargo.toml`;
- maintainer patches and non-automatic entries in `debian/patches/series`;
- maintainer scripts, install files, service units, and other unknown `debian/` paths; and
- maintainer changes to generated files such as `debian/control`.

The selected crate source, orig tarball, automatic patches, and generated packaging come from the fresh debcargo result.

### Changelog

The changelog remains primarily maintainer-owned, but each regeneration of a released package starts a new `UNRELEASED` entry, even when the rest of the package is unchanged. Subsequent runs update that entry. Ubucargo updates the version number using `dch --vendor Ubuntu`:

- a new package starts at `<upstream>-0ubuntu1`;
- a released top entry for the same upstream version is advanced with `dch --increment`, creating a new top entry;
- a released top entry for a different upstream version gets a new `<upstream>-0ubuntu1` entry;
- an `UNRELEASED` top entry for the same upstream version is retained, with ubucargo's generated changelog item added or updated in that entry;
- an `UNRELEASED` top entry for a different upstream version is retained and changed to `<upstream>-0ubuntu1`; and
- `dch` handles existing Ubuntu revision forms such as `ubuntuN`, stable-update suffixes, rebuilds, and other derivative revisions.

A change to the Debian source name follows the same rules: a released top entry is preserved and a new entry is created under the new name; an `UNRELEASED` top entry is renamed in place, retaining the maintainer's change notes and its Debian version when the upstream version is unchanged. A different upstream version starts at `<upstream>-0ubuntu1`. A rename item records both source names, and earlier entries retain their original names and contents.

Ubucargo adds a provenance item recording the crate release and both tool versions:

```text
  * Package serde 1.0.229 from crates.io.
    Generated with debcargo 2.8.4 and ubucargo 0.1.0.
```

Local packages use `from local source` in place of `from crates.io`. Preparation uses `Prepare` in place of `Package`; a later full generation replaces that provenance item within the same `UNRELEASED` entry.

An existing ubucargo provenance item in the current `UNRELEASED` entry is updated in place when the crate or either tool version changes. An unchanged item is not duplicated on later runs.

Ubucargo prepares the staged changelog before final generation and always passes `--changelog-ready` to debcargo. Debcargo therefore reads the prepared changelog for generation but does not modify it.

### Control generation

After each full package generation, ubucargo adjusts the generated `debian/control` for Ubuntu. It runs `update-maintainer` on the staged package, preserving current Ubuntu maintainer addresses or setting Ubuntu Developers and recording the previous maintainer in `XSBC-Original-Maintainer`. Debian-derived packages retain their existing configuration while receiving this control adjustment. Packages created with the default configuration already use Ubuntu Developers.

Ubucargo also removes debcargo's Debian-specific `Vcs-Git` and `Vcs-Browser` fields from generated control files. These adjustments follow the normal generated-file update rules, preserving maintainer overrides. Ownership inference for existing raw debcargo control files is described under [migration and ambiguous baselines](#migration-and-ambiguous-baselines).

## Orig tarball and source tree

The orig tarball is placed beside the source directory using Debian naming:

```text
<parent>/
  rust-serde_1.0.220.orig.tar.gz
  rust-serde/
```

When repacking is not required, debcargo copies the verified `.crate` archive unchanged. Matching `excludes` entries cause debcargo to rebuild the orig tarball without those paths. `repack_suffix` supplies the suffix added to the Debian upstream version.

An explicit `repack_suffix` in `debian/debcargo.toml` supplies the repack portion of the upstream version. When `excludes` is present, the installed debcargo supplies the default: `dfsg` in 2.8.2 and 2.8.3, or `ds` starting with 2.8.4. Registry releases use the Cargo version alone when both settings are omitted. Local-source packages append their [`+local` suffix](#local-builds) to the resulting upstream version. The existing changelog identifies the old package; the configuration and generator determine the next suffix. To retain `+dfsg` from an existing version such as `1.0.0+dfsg-1`, set `repack_suffix = "dfsg"` in the configuration.

For older crate archives containing an unnormalized manifest, debcargo 2.8.2 normalizes `Cargo.toml` inside the orig tarball and requires a repack suffix. Starting with 2.8.3, debcargo preserves the upstream manifest in the orig and supplies an automatic normalization patch. Changing generators can change orig contents; registry-source packaging requires a distinct `repack_suffix` when the resulting bytes differ, and local-source packaging replaces its local orig.

For an existing package, the old orig tarball is the source-merge baseline. Its source name and upstream version come from the top changelog entry. Ubucargo first looks beside the package, then uses `pull-lp-source --download-only <SOURCE> <VERSION>` to retrieve that exact Ubuntu source version independently of the host's configured APT series. The verified orig is saved beside the existing package immediately and remains available for retries if a later step fails. Acquisition happens before the staged changelog is changed.

When the source name changes, the existing changelog still identifies the old orig baseline. The generated orig is written beside it under the new source name; the old orig remains available.

`pull-lp-source` verifies the downloaded source files against their `.dsc`; ubucargo only checks that the expected orig tarball was produced. If no old orig can be found, ubucargo stops; `--force` does not bypass a missing merge baseline.

If the candidate orig path already exists, ubucargo reuses it when its bytes match the fresh debcargo result. For local-source builds, changed contents replace the generated main `+local` orig through an atomic file replacement. Other conflicting orig tarballs cause an error before writing to the destination, even with `--force`. When repacking a registry release changes the orig tarball, use a new `repack_suffix`, such as `ds2`, to give the new orig a distinct upstream version and filename. Other orig tarballs beside the package retain their contents.

### Source merge

Upgrading to a new upstream release or repacking after changing `excludes` in `debian/debcargo.toml` can add, change, or remove upstream files. When updating an existing package, ubucargo compares three trees outside `debian/`:

- `base`: the source extracted from the old orig tarball;
- `old`: a copy of the current working source with quilt patches unapplied; and
- `new`: the fresh source produced by debcargo.

Paths are updated conservatively:

| Condition                              | Behavior                                                |
| -------------------------------------- | ------------------------------------------------------- |
| `old == base`                          | Accept `new`, including upstream additions and removals |
| `old == new`                           | Keep the common result                                  |
| Path absent from both `base` and `new` | Preserve the local-only path                            |
| Any other difference                   | Report a conflict and make no changes                   |

This preserves VCS administration directories, local CI files, and build artifacts when they are absent from both upstream trees, without inspecting a particular VCS. A newly introduced upstream path that conflicts with a local-only path is reported with an error rather than overwritten.

For full generation, ubucargo restores the maintained patch position in the updated staging copy before writing, as described below. Preparation leaves all patches unapplied.

`--force` resolves source conflicts by choosing `new` for paths owned by either upstream tree. Paths absent from both upstream trees remain preserved.

## Patch state

For full generation, ubucargo stages the maintained files and series entries from `debian/patches/` in a temporary debcargo overlay. The installed debcargo supplies its fresh automatic patches, prepends them to the maintained series, applies the resulting patch stack temporarily, and reads the resulting manifest. If patch application fails, the operation is aborted. With `--prepare-only`, ubucargo supplies only the configuration and changelog so debcargo can generate fresh automatic patches independently of maintainer patches.

Every existing-package update uses a complete staging copy, including its `.pc` state. Ubucargo reads the last entry of `.pc/applied-patches` to remember the top applied patch and runs `quilt diff -z` in staging. Unrefreshed changes trigger an error, since otherwise a stale version of the patch would be supplied to debcargo, which would likely be unintended. It then pops the staged stack before comparing upstream source and regenerating packaging.

For full generation, after materializing the final staged packaging with fresh automatic patches, ubucargo restores the maintained patch position. If the original top patch was maintained, it pushes through that patch by name, including the fresh automatic prefix. If the original top patch was automatic, it applies the fresh automatic prefix. Initially unapplied stacks remain unapplied. A missing remembered maintained patch or a failure to reapply the stack aborts the staged operation. Ubucargo writes the resulting source and quilt backup state together, so later `quilt pop` operations use the updated upstream baseline.

Maintainers can regenerate packages with applied quilt patches.

Ubucargo understands ordinary on-disk quilt state but does not inspect Git history or VCS-specific patch queues. A GBP patch queue must be exported to `debian/patches` and the ordinary packaging branch checked out before running `package`.

## Generated paths

Full generation may produce:

- `debian/cargo-checksum.json`
- `debian/control`
- `debian/copyright`
- `debian/rules`
- `debian/watch`
- `debian/upstream/metadata`
- `debian/tests/control`, for library packages
- `debian/<feature-package>.lintian-overrides`, for each generated non-base feature package
- `debian/patches/auto/<patch>`, for debcargo-generated source transformations

During each full `ubucargo package` run, ubucargo records generated state in `debian/ubucargo-state.json` for files that support maintainer overrides. It writes `<file>.debcargo.hint` only when fresh generated output differs from the resulting primary in content or executable status. The hint contains the generated alternative from that run. Automatic patches use the generator-owned rules below and have neither manifest entries nor hints.

`debian/cargo-checksum.json` uses these same ownership rules; maintainer edits are preserved. `debian/patches/series` retains the special merge behavior described below and has neither a manifest entry nor a hint.

During full generation, if debcargo emits an unrecognized path under `debian/`, `package` warns and omits it during both creation and regeneration. The changelog, configuration, and non-automatic patch files remain maintainer-owned. Source-path changes belong in `debian/debcargo.toml`.

For a new package, ubucargo retains debcargo's `debian/source/format`. On subsequent regenerations it leaves that file unchanged and does not create a `.debcargo.hint` for it.

### Generated patches

Debcargo may generate patches for configuration-driven source transformations such as `remove_features`. The entire `debian/patches/auto/` namespace is exclusively generator-owned. On regeneration, ubucargo silently replaces edited patches, restores deleted generated patches, and removes files no longer generated. No ownership baseline or keep-or-replace decision is required. Therefore, maintainers should not edit these automatically generated patches.

`debian/patches/series` has mixed ownership. During full generation, debcargo receives the maintained series entries as overlay input and supplies its fresh `auto/` entries; ubucargo writes that merged output. During preparation, ubucargo prepends the fresh automatic entries to the maintained series, replacing its old automatic entries and preserving the remaining text.

## Override detection and materialization

The following ownership rules apply to full generation.

The manifest travels with the source package. Version 1 is JSON with paths sorted lexically:

```json
{
  "version": 1,
  "files": {
    "debian/control": {
      "sha256": "<64 lowercase hexadecimal characters>",
      "executable": false
    },
    "debian/watch": null
  }
}
```

Keys are package-relative managed paths. Present files have a SHA-256 content hash and an executable boolean matching Git's file-mode semantics. A `null` entry records generated absence; a missing entry means its baseline is unknown. Entries for previously managed paths remain even after the files disappear. Invalid records, unsupported versions, and paths outside the managed namespaces cause errors before any changes are applied.

For each managed path that supports maintainer overrides, materialization has three values:

- `base`: the previous generated fingerprint or recorded absence
- `old`: the working-tree `<file>`
- `new`: the generated staging file

Comparisons include existence, content hash, and executable status. A missing file differs from an empty file, so deleting a generated file counts as a maintainer override.

Copied files and trees retain their source permissions. Atomic writes of generated files and hints also retain the captured source permissions. Newly created directories and content without a source file, such as the ownership manifest, use ordinary creation permissions filtered by umask. Only executable status participates in ownership comparisons; incidental mode changes do not trigger updates. Preserved overrides and otherwise untouched paths retain their existing permissions. Symbolic links are preserved without changing their targets' permissions.

When `base` is known, including recorded absence, an override exists when `old != base`.

| Condition | Meaning | Behavior |
| --- | --- | --- |
| `old == base` | Unmodified generated file | Take `new` as primary; remove any hint |
| `old != base` | Maintainer override | Preserve `old`; write a hint if `new` exists and differs from `old` |

This comparison is deliberately conservative. Any content or executable-status change preserves the primary as an override rather than risking data loss.

After an update the manifest always records fresh generator output, never preserved maintainer contents. Hints are removed when redundant or when generated output no longer exists. Matching the latest generated state, including absence, clears an override. These rules also handle generator removal and later reintroduction.

### Manual edits between runs

`.debcargo.hint` files reflect the last completed `ubucargo package` run. For example, a manual edit to a newly generated `debian/copyright` is detected on the next run by comparing its fingerprint with the manifest. Ubucargo preserves the edit and writes a hint if the generated alternative differs, even when generator output is unchanged.

Copying a hint's contents and permissions to the primary adopts that generated version; the next run resumes automatic updates and removes the redundant hint. Maintainers can build or upload with manual edits and refresh hints on a later run. Keeping the manifest in the source package lets ubucargo detect and preserve later edits independently of hint presence.

## Migration and ambiguous baselines

These baseline rules apply to files that support maintainer overrides; automatic patches are always regenerated as described above.

When a manifest entry is missing, an existing `.debcargo.hint` establishes the baseline, including executable status. Migration happens during ordinary generation: establish the manifest, keep hints for overrides, and remove redundant recognized hints. Unrecognized hints remain untouched.

When a manifest entry and hint both exist, they must agree on contents and executable status. A hint also conflicts with a recorded absence. Conflicts require `--keep` or `--replace`, even when the primary matches one of the records. Missing hints are normal and do not invalidate manifest entries.

Without either baseline, `package` initializes only cases that cannot overwrite existing content:

| `old`   | `new`                | Behavior when `base` is absent                        |
| ------- | -------------------- | ----------------------------------------------------- |
| absent  | absent               | Record generated absence                              |
| absent  | present              | Write `new` and record its fingerprint; no hint       |
| present | equal to `old`       | Record the fingerprint; no hint                       |
| present | different from `old` | Stop without writing; require `--keep` or `--replace` |
| present | absent               | Preserve the primary and record generated absence     |

When neither a manifest entry nor a hint exists, ubucargo also compares the existing `debian/control` with debcargo's freshly generated control output before Ubuntu maintainer adjustments and VCS-field removal. For this comparison, the candidate uses the existing source paragraph's `Standards-Version`; all other contents and executable status must match exactly. A match establishes generator ownership, allowing ubucargo to adopt the generated policy version and apply its Ubuntu adjustments automatically. Other differences follow the baseline rules above.

For an ambiguous path, the user may disambiguate by supplying a `--keep` or `--replace` option using a package-relative path:

```console
ubucargo package --keep debian/control
ubucargo package --replace debian/control
```

`--keep` accepts ambiguous paths, preserves the primary, and writes a hint if the generated alternative exists and differs. `--replace` accepts any managed path and adopts its freshly generated state: it installs the generated contents and permissions when present, or deletes the primary when generation produces absence. It also removes any hint. Both options record fresh generated state in the manifest for files that support overrides. They may be repeated for several paths; each path may be named by only one option.

If any ambiguity remains, `package` reports every affected path and preserves the package tree.

Restoring a primary to its hint contents and permissions relinquishes the override; the next run resumes automatic updates and removes the redundant hint. Generated output converging on the override also removes its hint. Ubucargo does not merge file contents or retain older history.

## Staging and applying changes

The command exits 0 on success and 2 on errors or unresolved ambiguities.

Ubucargo stages the complete candidate before modifying the destination. In-place updates, package copies, Archive inputs, and PPA inputs use the same staging and writing flow. The temporary debcargo overlay contains the prepared changelog and, for full generation, the maintained patch stack. Existing generated packaging and hints do not affect generation.

The staged invocation is equivalent to:

```console
debcargo package \
  --config /<TEMPORARY PATH>/stage/debcargo.toml \
  --directory /<TEMPORARY PATH>/stage/output \
  --no-overlay-write-back \
  --changelog-ready \
  <CRATE> <VERSION>
```

Ubucargo validates the selected crate identity, Debian source identity, source tree, orig filename and contents, the patches supplied to generation, generated packaging, and complete materialization plan before writing.

For full generation, ubucargo materializes primary files, patch series, hints, and generated ownership state in staging, then restores the maintained patch position. Preparation materializes source, changelog, and automatic patch updates with all patches unapplied. Before writing, ubucargo compares this completed tree with the destination and changes only differing paths; unchanged source and packaging files retain their modification times. Regular files are written atomically. When writing a changed package, `.pc` is copied with its quilt backup timestamps. For files that support maintainer overrides, a rerun preserves changed primaries and may require an explicit decision after an interruption while applying changes.

Ubucargo applies changes to files only. It does not create commits, branches, tags, pristine-tar data, `.dsc` files, source `.changes`, or `.buildinfo` files. Standard Debian and VCS tools remain responsible for those artifacts.

## Debcargo compatibility

Ubucargo requires debcargo 2.8.2 or a later compatible 2.x release (`^2.8.2`) and checks the installed version before release resolution or generation. CI exercises the integration suite with debcargo 2.8.2 and 2.8.4.

Ubucargo tries to minimize depending on behavior that varies across debcargo versions. It lets debcargo control the packaging policy and configuration interpretation, and this behavior can differ between versions: for example, `collapse_features` defaults to `true` starting with debcargo 2.8.3, and the default `repack_suffix` has also changed. Maintainers can set `collapse_features` and `repack_suffix` explicitly when consistent behavior across debcargo versions matters.
