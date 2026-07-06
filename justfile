# Entry points for herdr-copy-search development and release.

# List available recipes.
default:
    @just --list

# Debug build.
build:
    cargo build

# Run unit tests.
test:
    cargo test

# Full quality gate; also used as the cargo-release pre-release hook.
gate:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test
    cargo build --release

# Manual run against the fixture (needs a real terminal).
run mode="copy":
    cargo run -- --input fixtures/sample.txt --mode {{mode}}

# Register this checkout as a linked herdr plugin.
link:
    herdr plugin link .
    cargo build --release --locked

# Bump version, sync manifests, tag vX.Y.Z locally (no push).
release level="patch":
    cargo release {{level}} --execute
