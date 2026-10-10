# Changelog maintenance

`release-plz.toml` loads `release-cliff.toml` for changelog generation. The full git-cliff config
exposes `render_always`, which release-plz 0.3.160's inline changelog settings do not support.
This ensures unchanged crates receive a section for their shared version bump. The older
`cliff.toml` remains a separate standalone configuration.

The proposed 0.1.0 milestone preserves old output in [the archive](changelog-archive/README.md) and
starts one clean section per crate. It is a pre-1.0 milestone, not a promise of stable 1.x APIs.
All eight manifests inherit the workspace version, and internal requirements match that version.

## Shared release version

The root `[workspace.package].version` is the only crate version value. Every crate uses
`version.workspace = true`. Release-plz 0.3.160 supports this inheritance directly: it selects the
highest required bump, updates the root version, and prepares that version for all eight crates,
including crates without their own changes. It preserves each crate's version inheritance and
updates internal dependency requirements and the lockfile. No `version_group` is needed.

A patch in just one crate therefore raises all eight from 0.1.0 to 0.1.1. A patch in one crate and a
breaking change in another raise all eight to 0.2.0. Their notes remain crate-specific; an unchanged
crate receives a shared-version-only section rather than fabricated feature notes. Published versions
share a number, but registry publication consists of separate crate uploads and can partially fail.

Run `python3 scripts/check-release-versions.py` after regeneration to verify inheritance, internal
requirements, lockfile versions, and the latest changelog headings. Do not replace inheritance with
literal per-crate versions: that changes release-plz's version-selection behavior.

## Release boundaries

Published crates are compared with their registry packages. The verified latest library and CLI
release is 0.0.36. The six unpublished service/viewer crates receive initial-release notes from their
package history, including features introduced before the latest workspace version bump.

Release-plz uses package contents and dependency traversal to attribute commits. A workspace
`Cargo.toml` edit alone does not imply a bullet in every crate. At this reset the library receives
[the seven-dependency update](https://github.com/joshka/webfinger-rs/pull/225), while the CLI receives
[the thirteen-dependency update](https://github.com/joshka/webfinger-rs/pull/240). These are generated
commit summaries, not claims that every dependency in those groups belongs to that crate.
The latter commit also changes CLI source. Lockfile-only updates may not appear as package changes.

The generator removes routine release commits, workflow dependency updates, CI, tests, style and
other routine chores. It retains dependency updates, documentation, refactors, unconventional
subjects and breaking changes. Identical commit IDs are emitted once per group. No compare links
are invented for unpublished crates.

Initial-release scanning defaults to at most 1,000 commits. This repository's current initial crate
histories fit within that limit. For longer histories, increase `--max-analyze-commits` explicitly;
filtering cannot recover commits outside the scan. Historical archives remain immutable.

## Local regeneration

Use release-plz 0.3.160 for the reset validation and git-cliff 2.13.1 for the regression fixture.
Run from a materialized checkout. Release-plz requires Git history, tags and registry access; for a
jj workspace use an isolated disposable Git tooling copy with the same history and file contents.
Keep normal version-control operations in jj.

```sh
release-plz update --allow-dirty --release-date 2026-10-10
python3 scripts/check-changelog.py
python3 scripts/check-release-versions.py
```

`update` changes local release files; it does not publish. The fixed date makes comparison
repeatable; choose the intended date for a future release. Repeat `update` and compare all manifests,
`Cargo.lock` and crate changelogs byte-for-byte. A second run must not add sections or repeat bullets.

For the one-time version proposal, use `release-plz set-version` with each `package@0.1.0`, then
`cargo update --workspace`. The tool materializes crate versions: restore `version.workspace = true`
in each crate and set `[workspace.package].version` to `0.1.0` to retain shared-version inheritance.
Keep internal workspace dependency requirements at `0.1.0`. Regenerate once more after restoring
inheritance. Do not use `--update-deps` for this maintenance change.

The regression fixture creates a temporary Git repository and checks release/workflow filtering,
breaking-change protection, retained docs/refactors and unconventional subjects, and duplicate
commit suppression. Full release-plz regeneration additionally verifies registry/package boundaries.
