# Code Signing Policy

This document describes how kopipe's release binaries are built, verified,
and (once approved) signed. It is written primarily for the
[SignPath Foundation](https://signpath.org/) application and for anyone
auditing the release process.

## What gets signed

Only the Windows installer produced for tagged releases:

- `kopipe_<version>_x64-setup.exe` (NSIS installer, built by
  [Tauri](https://v2.tauri.app/)'s bundler)

No other artifacts (dev builds, forks, pull-request builds) are signed.

## Project facts relevant to eligibility

- **License**: [MIT](LICENSE), OSI-approved.
- **Source**: fully public at
  [github.com/lakepuka/kopipe](https://github.com/lakepuka/kopipe).
- **Distribution**: free of charge, no account or payment required, published
  on the repository's [Releases page](https://github.com/lakepuka/kopipe/releases).
- **Purpose**: a local-only Windows clipboard-history utility. It makes no
  network connections other than an optional, opt-out GitHub version check
  (see [README § Privacy](../README.md#privacy)); it is not, and has never
  been, associated with malware or security-circumvention tooling.

## Build process

- **Toolchain**: Node.js + [pnpm](https://pnpm.io/) for the frontend
  (React + TypeScript, bundled with Vite), [Rust](https://www.rust-lang.org/)
  + Cargo for the Tauri shell. Tool versions are pinned in
  [`mise.toml`](../mise.toml); JS dependency versions are pinned in
  [`pnpm-lock.yaml`](../pnpm-lock.yaml).
- **Continuous integration**: every push to `main` and every pull request
  runs on GitHub Actions (`windows-latest`) via
  [`.github/workflows/ci.yml`](../.github/workflows/ci.yml). The workflow
  runs formatting, linting, `tsc`, and the frontend test suite
  (`pnpm verify`), then `cargo fmt --check` and `cargo test` for the Rust
  side. No code reaches `main` without passing these checks.
- **Release build**: `pnpm tauri build` compiles the frontend, then builds
  and bundles the Rust/Tauri binary into the NSIS installer (see
  [README § Build from source](../README.md#build-from-source)). Release
  artifacts are built from a tagged commit on `main` only.
- **Checksums**: every release's notes publish the installer's SHA-256 hash
  so users can verify integrity independently of code signing (see
  [README § Verify your download](../README.md#verify-your-download-optional)).

## Access control

- Only the repository owner ([@lakepuka](https://github.com/lakepuka)) has
  write access to `main` and publishes releases.
- Signing credentials (once granted by SignPath) will be used exclusively
  from the maintainer's release process; they are never committed to the
  repository or exposed in CI logs.

## Roadmap toward fully automated, signed releases

Release builds are currently produced locally by the maintainer rather than
in CI. Moving the release build (and, once approved, the signing step) into
a dedicated GitHub Actions release workflow — triggered on version tags — is
planned, to make the chain from source to signed artifact fully auditable.

## Vulnerability reporting

See [`SECURITY.md`](../SECURITY.md).
