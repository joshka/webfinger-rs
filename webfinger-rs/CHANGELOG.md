# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.31](https://github.com/joshka/webfinger-rs/compare/webfinger-rs-v0.0.30...webfinger-rs-v0.0.31) - 2026-07-02

### Other

- Add WebFinger link templates ([#181](https://github.com/joshka/webfinger-rs/pull/181))

## [0.0.30](https://github.com/joshka/webfinger-rs/compare/webfinger-rs-v0.0.29...webfinger-rs-v0.0.30) - 2026-07-02

### Other

- align examples and published docs ([#179](https://github.com/joshka/webfinger-rs/pull/179))

## [0.0.29](https://github.com/joshka/webfinger-rs/compare/webfinger-rs-v0.0.28...webfinger-rs-v0.0.29) - 2026-07-02

### Fixed

- encode WebFinger query values ([#177](https://github.com/joshka/webfinger-rs/pull/177))
- reject invalid WebFinger resource URIs
- tighten WebFinger URI validation
- align JRD response types ([#174](https://github.com/joshka/webfinger-rs/pull/174))
- validate WebFinger relation types ([#172](https://github.com/joshka/webfinger-rs/pull/172))
- validate WebFinger resources ([#171](https://github.com/joshka/webfinger-rs/pull/171))
- validate WebFinger server request routing ([#170](https://github.com/joshka/webfinger-rs/pull/170))
- set Actix JRD media type ([#169](https://github.com/joshka/webfinger-rs/pull/169))
- add WebFinger CORS header ([#168](https://github.com/joshka/webfinger-rs/pull/168))
- enforce HTTPS redirects

### Other

- align runnable examples ([#178](https://github.com/joshka/webfinger-rs/pull/178))

## [0.0.28](https://github.com/joshka/webfinger-rs/compare/webfinger-rs-v0.0.27...webfinger-rs-v0.0.28) - 2026-07-01

### Other

- Reduce dependency feature surface ([#164](https://github.com/joshka/webfinger-rs/pull/164))

## [0.0.27](https://github.com/joshka/webfinger-rs/compare/webfinger-rs-v0.0.26...webfinger-rs-v0.0.27) - 2026-07-01

### Other

- Preserve percent escapes in URLs ([#161](https://github.com/joshka/webfinger-rs/pull/161))
- Use query parser in server extractors ([#160](https://github.com/joshka/webfinger-rs/pull/160))
- Add WebFinger query parser ([#159](https://github.com/joshka/webfinger-rs/pull/159))

## [0.0.26](https://github.com/joshka/webfinger-rs/compare/webfinger-rs-v0.0.25...webfinger-rs-v0.0.26) - 2026-06-27

### Other

- harden maintenance automation ([#155](https://github.com/joshka/webfinger-rs/pull/155))
- *(deps)* bump the cargo-dependencies group with 9 updates ([#151](https://github.com/joshka/webfinger-rs/pull/151))

## [0.0.25](https://github.com/joshka/webfinger-rs/compare/webfinger-rs-v0.0.24...webfinger-rs-v0.0.25) - 2026-06-12

### Other

- relax dependency lower bounds ([#147](https://github.com/joshka/webfinger-rs/pull/147))
- *(deps)* bump the cargo-dependencies group across 1 directory with 5 updates ([#142](https://github.com/joshka/webfinger-rs/pull/142))
- refresh README front page
- clarify actix integration ([#138](https://github.com/joshka/webfinger-rs/pull/138))
- clarify axum integration
- clarify reqwest execution ([#136](https://github.com/joshka/webfinger-rs/pull/136))
- document request URL semantics
- rewrite docs.rs landing page and README

## [0.0.24](https://github.com/joshka/webfinger-rs/compare/webfinger-rs-v0.0.23...webfinger-rs-v0.0.24) - 2026-03-30

### Other

- align dependency floors and CI ([#116](https://github.com/joshka/webfinger-rs/pull/116))

## [0.0.23](https://github.com/joshka/webfinger-rs/compare/webfinger-rs-v0.0.22...webfinger-rs-v0.0.23) - 2026-03-27

### Other

- update Cargo.toml dependencies

## [0.0.22](https://github.com/joshka/webfinger-rs/compare/webfinger-rs-v0.0.21...webfinger-rs-v0.0.22) - 2025-12-16

### Other

- update Cargo.toml dependencies

## [0.0.21](https://github.com/joshka/webfinger-rs/compare/webfinger-rs-v0.0.20...webfinger-rs-v0.0.21) - 2025-11-29

### Other

- *(deps)* bump the cargo-dependencies group across 1 directory with 15 updates ([#91](https://github.com/joshka/webfinger-rs/pull/91))

## [0.0.18](https://github.com/joshka/webfinger-rs/compare/webfinger-rs-v0.0.17...webfinger-rs-v0.0.18) - 2025-07-06

### Other

- update Cargo.toml dependencies

## [0.0.17](https://github.com/joshka/webfinger-rs/compare/webfinger-rs-v0.0.16...webfinger-rs-v0.0.17) - 2025-06-20

### Other

- move deps to workspace manifest and fix versions

## [0.0.15](https://github.com/joshka/webfinger-rs/compare/webfinger-rs-v0.0.14...webfinger-rs-v0.0.15) - 2025-06-09

### Added

- implement Clone, PartialEq, Eq on types

### Other

- cleanup dependencies ([#62](https://github.com/joshka/webfinger-rs/pull/62))

## [0.0.14](https://github.com/joshka/webfinger-rs/compare/webfinger-rs-v0.0.13...webfinger-rs-v0.0.14) - 2025-05-01

### Other

- bump edition to 2024 and MSRV to 1.85 ([#59](https://github.com/joshka/webfinger-rs/pull/59))
- add rustfmt and reformat code
- release v0.0.13 ([#57](https://github.com/joshka/webfinger-rs/pull/57))
