# Repository Guidelines

Adapted from the Hubuum repository for an independent, reusable Rust library.

## Scope and Architecture

- Keep the crate generic. Do not add consuming-application models, database
  access, HTTP clients, web frameworks, global configuration, or runtime ownership.
- Keep observations, inference policy, schema generation, and validation separate.
  A statistical observation must not silently become a domain constraint.
- Preserve validated facts across boundaries using private-fielded types with
  fallible constructors. Use enums for mutually exclusive result states.
- Distinguish an inferred candidate from a schema validated against a supplied
  corpus. Never imply a guarantee about unseen documents.
- Prefer small explicit interfaces, typed requests, and builders over long
  positional argument lists. Use typestate only when it prevents meaningful
  invalid call order or missing required data.
- Own public errors and report types. Expose third-party types only as intentional
  integration surfaces; `serde_json::Value` is a planned input/output boundary.
- Keep validator and inference dependencies behind internal adapters.
- Follow conventional Rust module discovery; do not use `#[path = "..."]`.
- Keep memory, traversal, retained evidence, and output limits explicit. A limit
  must never silently convert an incomplete result into a complete one.
- Do not retain document scalar values by default. Paths and caller identifiers
  can also contain sensitive information; document what reports retain.

## Rust Standards

- Use idiomatic Rust and mechanical rustfmt formatting.
- Keep invariants near their data. Prefer validating newtypes where raw strings
  or numbers would otherwise carry unchecked domain meaning.
- Put behavior on the types that own it. Keep fields private unless there is a
  specific interoperability reason to expose them.
- Prefer `use` imports over inline fully qualified paths except for genuine
  ambiguity or a one-off reference where an import would mislead.
- Avoid unused code, speculative abstractions, and `#[allow(dead_code)]`.
- Keep output deterministic; define ordering for paths, types, and diagnostics.
- Document public behavior and errors with compiling examples once APIs exist.
- Keep occurrence counts separate from distinct-document contributions. Witness
  and diagnostic truncation must not change completed coverage or exact counts.
- Update `docs/validation.md` and its positive/negative tests when changing schema
  support. Keep reference retrieval disabled independently of Cargo features.

## Verification

- Run `cargo test --all-targets` and `cargo test --doc` for the complete suite.
- Run `cargo clippy --all-targets -- -D warnings` before considering code complete.
- Run `cargo fmt --all -- --check` for Rust formatting.
- Test the Rust 1.90.0 MSRV with `cargo +1.90.0 test --lib --tests --locked`
  and `cargo +1.90.0 test --doc --locked`. Keep CI and `rust-version` aligned.
- Run `cargo test --test validation --locked --features jsonschema/resolve-file`
  when modifying compilation, references or dependency configuration.
- Run `npx markdownlint-cli2 --config .markdownlint.json "**/*.md" "!target"`
  after documentation changes. Every fenced block must declare a language;
  use `text` for plain diagrams. Use compact Markdown table columns consistently.
- No database, `.env`, or service process should be needed for library tests.
- If Python tooling is introduced, require Python 3.11 or newer, ensure `python3`
  on PATH selects a supported version, and use only the standard library.
- When adding tests, benchmarks, fixtures, or embedded inputs, ensure the build
  and CI include them. Add a change-classifier check only if a classifier exists.

## Tests and Benchmarks

- Add regression tests for behavior changes and bug fixes.
- Keep each test focused. Parameterize input variants with the established test
  harness; do not combine unrelated behaviors in one test.
- Verify generated schemas through an independent validator. Include negative
  examples so accepting everything cannot satisfy the test contract.
- Test missing versus null, exact number semantics, arrays, escaped keys,
  incomplete analysis, policy conflicts, and deterministic output.
- Use synthetic fixtures; do not commit production documents or credentials.
- Prefer deterministic library benchmarks with explicit options and limits.
- Put benchmark entrypoints in `benches/`, one target per file, with matching
  `[[bench]]` entries using `harness = false` when required by the harness.

## Changes and Releases

- Keep changes scoped and independently reviewable.
- Sign every Git commit and verify the signature before pushing. Never create
  unsigned commits, including amendments and commits rewritten by rebases.
- Review CHANGELOG.md for every pull request. Record user-facing changes under
  `[Unreleased]`, or explicitly state that the PR has no changelog-worthy impact.
- Call out breaking changes and migration actions in both the PR and changelog.
- Treat the library API as an external consumer contract. Pre-1.0 status permits
  evolution but does not excuse undocumented behavior or accidental leakage.
- Before the first release, settle the MSRV, dependency licenses, crate-name
  availability, repository metadata, and supported JSON Schema vocabulary.
- Update README.md, Cargo.toml, and CHANGELOG.md together for releases. Keep
  `publish = false` until the release is intentionally prepared.
- Use the substantive PR description for squash commit bodies, excluding
  verification-only commands and checklists.
