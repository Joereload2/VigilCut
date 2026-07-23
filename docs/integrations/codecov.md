# Codecov

## Scope

The `Rust coverage` GitHub Actions workflow generates a real Codecov JSON
report from all Rust library tests using `cargo-llvm-cov`. Frontend coverage is
not included because VigilCut currently has no stable frontend unit-test
runner or instrumentation strategy; reporting a build or Svelte check as
coverage would be misleading.

The patch target starts at 70%, within the requested 70–75% range for new
code. Project and patch statuses are informational and tolerate a two-point
fluctuation. Explicit tests for ingestion, idempotency, worker recovery,
priority, cancellation, licensing, and cost gates remain mandatory regardless
of the percentage.

## Authentication and activation

The GitHub repository is public. The workflow uses GitHub OIDC
(`id-token: write` and `use_oidc: true`) instead of storing a Codecov upload
token. No secret is added to source or required by this configuration.

Codecov still needs to recognize/onboard `Joereload2/VigilCut`. A successful
workflow upload and a report visible in Codecov are the acceptance evidence.
Until that occurs, local report generation is real but remote reporting is not
claimed as active. If OIDC is unavailable for the repository configuration,
create a repository-scoped `CODECOV_TOKEN` secret and change the upload step
explicitly; never commit it.

## Local reproduction

Install `cargo-llvm-cov` as a developer tool, not a project dependency, and
ensure the Rust `llvm-tools-preview` component is available:

```powershell
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
cargo llvm-cov --manifest-path src-tauri/Cargo.toml --lib --codecov --output-path codecov.json
```

Acceptance assertions:

- the command exits successfully;
- `codecov.json` exists and is larger than 100 bytes;
- the JSON parses and contains coverage data for `src-tauri/src`;
- the GitHub workflow uploads the same file;
- Codecov shows a `rust` flag and patch report.

The generated `codecov.json` is a local artifact and must not be committed.

Local measurement on 2026-07-23 with `cargo-llvm-cov 0.8.7`: 100 Rust library tests passed; the report is 182,547 bytes with 91 file entries. A line-entry approximation from the Codecov JSON measured 44.67% repository-wide Rust coverage and 59.44% for paths containing `visual`. These baseline figures are measurements, not quality waivers; the 70% target applies to changed lines.

## Cost, dependencies, and rollback

No application dependency or runtime network call is added. GitHub-hosted
Actions and Codecov account limits remain external operational concerns;
observed local monetary cost is zero. Rollback removes
`.github/workflows/coverage.yml` and `codecov.yml`. Uninstalling the local
developer tool is optional and unrelated to the repository.
