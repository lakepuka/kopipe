# Security Policy

## Supported Versions

kopipe is a rolling-release desktop app. Only the latest published release on
the [Releases page](https://github.com/lakepuka/kopipe/releases) is
supported; users are encouraged to always run the newest version.

## Reporting a Vulnerability

Please **do not** open a public GitHub issue for security vulnerabilities.
Instead, report them privately via
[GitHub Security Advisories](https://github.com/lakepuka/kopipe/security/advisories/new)
for this repository.

We aim to acknowledge reports within a few days and to publish a fix (and,
where relevant, a coordinated advisory) once a patch is available.

## Scope

kopipe runs entirely on the local machine and stores clipboard history in a
local SQLite database (`%APPDATA%\io.github.lakepuka.kopipe\kopipe.db`). It
makes no network connections except an optional, opt-out update check against
GitHub (see the [README's Privacy section](README.md#privacy)). Reports
involving that update check, the local database, the global-shortcut
listener, or the Windows clipboard integration are all in scope.

## Code Signing & Release Integrity

See [`docs/code-signing-policy.md`](docs/code-signing-policy.md) for how
release binaries are built, signed, and published.
