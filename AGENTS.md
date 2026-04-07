# Repository Guidelines

## Project Structure & Module Organization

This repository is a Cargo workspace with two crates:

- `webfinger-rs/`: the library crate. Core types live in `src/types/`, protocol and transport
  integration code lives in `src/reqwest.rs`, `src/axum.rs`, and `src/actix.rs`, and runnable
  examples live in `webfinger-rs/examples/`.
- `webfinger-cli/`: the CLI wrapper around the library in `webfinger-cli/src/main.rs`.

Top-level files such as `README.md`, `CHANGELOG.md`, `SECURITY.md`, and `CONTRIBUTING.md` describe
published behavior and project policy.

## Build, Test, and Development Commands

- `cargo build --workspace`: build both crates.
- `cargo test --workspace`: run the full test suite.
- `cargo test -p webfinger-rs --all-features`: verify the library, doctests, and feature-gated
  integrations.
- `cargo run -p webfinger-cli -- acct:carol@example.com`: run the CLI locally.
- `cargo run -p webfinger-rs --example axum --features axum`: run the Axum example server.
- `cargo run -p webfinger-rs --example actix --features actix`: run the Actix example server.
- `cargo fmt --all` and `cargo clippy --workspace --all-features --all-targets`: format and lint
  before opening a PR.
- `markdownlint-cli2`: lint Markdown files across the repository using the repo config.

## Coding Style & Naming Conventions

Use Rust 2024 idioms, four-space indentation, and `rustfmt`. The repo prefers grouped imports,
module-granularity imports, and wrapped comments at about 100 columns; see `rustfmt.toml`.
Public API docs should be concise and docs.rs-friendly. Use snake_case for modules, functions, and
files; use UpperCamelCase for types.

## Testing Guidelines

Prefer focused unit tests near the code they cover. Keep doctests compiling when editing public
docs. When changing feature-gated integrations, run `cargo test -p webfinger-rs --all-features`.
For Rustdoc changes in feature-gated modules, also run `cargo test --locked --all-features --doc`
so local verification matches CI's doctest coverage. When updating documentation, always run
`cargo test --workspace` and `just docs-rs` as part of verification. Run `markdownlint-cli2`
after changing Markdown files. Name tests after observable behavior, for example
`valid_request_with_host_header`.

## Commit & Pull Request Guidelines

Recent history uses imperative Conventional Commit-style subjects such as `chore: release v0.0.24`
and `chore(deps): bump actions/checkout from 4 to 6`. Keep the first line short and scoped. PRs
should include a clear summary, linked issue when applicable, and note any doc or example changes.
When preparing a squash summary for an issue-linked PR, preserve the closing reference with a
trailer such as `Closes #127`.

## Documentation Notes

- Treat `webfinger-rs/src/lib.rs` as the primary docs.rs landing page.
- Keep the top-level README shorter and adoption-focused.
- If you use `cargo-rdme`, make sure generated README content stays in sync with crate-level
  Rustdoc.
- In Rustdoc comments, start the first section heading at `#`, not `##`; reserve deeper heading
  levels for real subsections.
- When updating docs, regenerate them locally with the `just docs-rs` workflow, and always run it
  as part of verification.
- When adding RFC references in Rustdoc, match the citation style used in
  `webfinger-rs/src/lib.rs`.
- Treat `webfinger-rs/src/lib.rs` as the source of truth for generated README content and
  regenerate `README.md` after crate-doc changes.
- Rustdoc examples must compile under the relevant feature set, so gate feature-specific snippets
  explicitly.
- Public examples should be copy-paste friendly and should not rely on undeclared transitive
  dependencies.
