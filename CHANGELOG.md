<!-- markdownlint-disable no-duplicate-heading -->
# Changelog

All notable changes to this project will be documented in this file.

## [0.0.11] - 2024-10-17

### 🚀 Features

- Impl Display for response as json
- Add colored help text, output, and allow logging verbosity to be set
- Add actix support
- Allow custom reqwest client

### 🚜 Refactor

- Simplify axum implementations

### 📚 Documentation

- Fix license heading level
- Add symbolic links to contributing and license files
- Update lib and readme docs

### ⚙️ Miscellaneous Tasks

- Add workflows and dependabot settings
- Cleanup tests and docs (#3)
- Cargo update

## [0.0.10] - 2024-09-30

### 📚 Documentation

- Update readme using cargo-rdme

## [0.0.9] - 2024-09-30

### 📚 Documentation

- Document all the things

## [0.0.8] - 2024-09-30

### 🚀 Features

- Add builders for response types

## [0.0.7] - 2024-09-30

### 🚀 Features

- Add axum FromRequestParts extractor and example

### 🐛 Bug Fixes

- Add back default features for reqwest
- Remove axum default-features

### 🚜 Refactor

- Move types to modules
- Move types under types module

### ⚙️ Miscellaneous Tasks

- Make axum extraction more robust, add tests, rename link_relation_types to rels
- *(release)* Release 0.0.7

## [0.0.6] - 2024-09-29

### 📚 Documentation

- Fix typo

## [0.0.5] - 2024-09-29

### 🚀 Features

- Add axum integration

## [0.0.4] - 2024-09-29

### 🐛 Bug Fixes

- Justfile typo

### 🚜 Refactor

- Use nutype instead of manual implementations

### ⚙️ Miscellaneous Tasks

- Release 0.0.4

## [0.0.3] - 2024-09-29

### 🚜 Refactor

- Move conversion / integration from lib.rs to modules, add constructors

## [0.0.2] - 2024-09-29

### 🐛 Bug Fixes

- Fix readme location

## [0.0.1] - 2024-09-29

### 🚀 Features

- Initial implementation

<!-- generated by git-cliff -->
