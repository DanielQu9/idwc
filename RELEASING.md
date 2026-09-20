# Releasing IdwC

This checklist keeps the Git repository, Git tag, and crates.io package in the
same state. Publishing a crates.io version is permanent, so do not skip the
dry run or publish from an uncommitted tree.

## Prepare and verify

1. Update the version in `Cargo.toml` and `Cargo.lock`.
2. Update `CHANGELOG.md`, both READMEs, both strict semantic specifications,
   crate documentation, CLI version tests, and `AGENTS.md`.
3. Run:

   ```bash
   cargo fmt -- --check
   cargo test --locked
   cargo clippy --locked --all-targets --all-features -- -D warnings
   RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
   cargo publish --locked --dry-run
   cargo package --locked --list
   ```

4. Inspect the package list and generated archive under `target/package/`.

## Publish

1. Commit the release as `release: vX.Y.Z`.
2. Create an annotated `vX.Y.Z` tag pointing to that commit.
3. Push the branch and tag, then wait for CI to pass.
4. Run `cargo publish --locked` from the tagged, clean working tree.
5. Confirm the version on crates.io and its docs.rs build. Never print or
   commit the crates.io token; authenticate locally with `cargo login`.

If upload succeeds but the Cargo command times out while waiting for the index,
check crates.io before retrying. A published version cannot be overwritten.
