# CodeRabbit

## Scope

`.coderabbit.yaml` prepares focused pull-request review for Rust, Svelte, SQL,
tests, CI, and documentation. It emphasizes Visual Library boundaries,
persistence, concurrency, security, licensing, cost gates, truthful UI state,
accessibility, and meaningful assertions.

Noise controls:

- `chill` profile;
- no poem or progress fortune;
- drafts, WIP titles, dependency bots, generated output, binaries, media,
  targets, and lockfiles are excluded;
- incremental review pauses after five reviewed commits;
- instructions explicitly avoid repeating rustfmt, Clippy, TypeScript, and
  stylistic YAML diagnostics.

CodeRabbit supplements Codex and human QA. It does not replace local gates,
security review, product acceptance, or a real integration test.

## External installation

The GitHub App requires repository-owner authorization. This implementation
does not install or authorize it.

To activate:

1. Open the official CodeRabbit GitHub App installation page.
2. Authorize only the `Joereload2/VigilCut` repository initially.
3. Review requested permissions before accepting.
4. Open or update a non-draft PR targeting `main`.
5. Confirm the review comment states that configuration came from
   `.coderabbit.yaml`.
6. Verify at least one inline observation is relevant and that excluded files
   are absent.

Acceptance requires an actual CodeRabbit review on a PR. Until then the status
is `blocked`, not operational.

## Validation

Local checks parse the YAML and assert:

- the schema declaration uses v2;
- automatic review skips drafts;
- auto-pause is five commits;
- lockfiles, binaries, media, generated files, and build outputs are excluded;
- path instructions exist for Rust, the independent library, Svelte, SQL,
  tests, workflows, and documentation.

The official schema URL is referenced for editor and service validation.
Service-side schema acceptance remains part of the first real PR review.

## Cost, dependencies, and rollback

No application dependency, secret, telemetry, or runtime service is added.
Observed cost is zero. Product limits or pricing must be reviewed by the
repository owner during App authorization. Rollback is removing
`.coderabbit.yaml` and uninstalling/revoking the GitHub App separately.
