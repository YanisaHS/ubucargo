# Contributing to Ubucargo

## Development setup

Install the [requirements](README.md#requirements) listed in the README, and add the `rustfmt` and `clippy` components to your Rust toolchain. CI uses the Rust version pinned in [`.github/workflows/ci.yml`](.github/workflows/ci.yml).

## Checks

CI runs these checks, which must pass:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
```

CI runs them against debcargo 2.8.2 and 2.8.4.

To run checks automatically before each commit, install the pre-commit hook:

```sh
git-hooks/setup-hooks.sh
```

The hook also runs `cargo doc` and `cargo audit`. Install `cargo audit` with `cargo install cargo-audit`.

## Security

To report a vulnerability, follow [SECURITY.md](SECURITY.md). Don't open a public issue.
