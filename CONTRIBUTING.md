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

## Rust style guidelines

- Avoid defining functions inside of other function definitions.
- Prefer not to use helper functions unless they are both reusable and independently meaningful.
- Avoid creating helper functions that are only used in one place or which would be simpler to inline.
- Avoid complex iterator chains that combine operations such as mapping, filtering, and collecting. Prefer loops instead. Simple one-liners like `.into_iter().collect()` are ok.
- Avoid casts or `from`-conversions of `bool` to integer types. Use `if-else` instead.
- Function names should start with a verb.
- Include a doc comment for each function and type definition.
- Where useful, include additional comments, e.g. to clarify the meaning of struct fields and enum variants, and to explain non-obvious logic in functions.
- Use simple `pub` rather than `pub(crate)` or `pub(super)`.

## Security

To report a vulnerability, follow [SECURITY.md](SECURITY.md). Don't open a public issue.
