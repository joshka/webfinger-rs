docs-rs:
    cargo +nightly docs-rs -p webfinger-rs

docs-rs-open:
    cargo +nightly docs-rs -p webfinger-rs --open

publish:
    cargo publish -p webfinger-rs
    cargo publish -p webfinger-cli
