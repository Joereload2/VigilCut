# External Integrations Implementation Status

This file records verified evidence after each isolated implementation gate.
`verified` is used only after the required local commands pass.

| Etapa | Estado | Commit | Pruebas | Real/Mock | Bloqueos |
|-------|--------|--------|---------|-----------|----------|
| Biblioteca independiente | verified | this phase commit | `diff --check`, check, build, fmt, clippy, 42 visual tests, and 7 smoke tests pass | Real SQLite/filesystem; mock image provider | None |
| Worker/Scheduler | verified | this phase commit | Full gate passes; 44 visual and 7 smoke tests | Real SQLite supervisor; mock provider | None |
| Dependabot | verified | this integration commit | YAML assertions and full gate pass | Real configuration; GitHub run not observed | Must reach default branch; security settings and labels require GitHub verification |
| Codecov | experimental | this integration commit | 100 instrumented tests; 182547-byte report; full gate passes | Real local report; remote upload unobserved | Codecov onboarding/OIDC workflow must be observed |
| CodeRabbit | blocked | this integration commit | Official schema keys and full gate pass | Real configuration; no review observed | GitHub App authorization required |
| Pollinations | experimental | this integration commit | Safe catalogue probe 421 ms; 2 provider tests; full gate passes | Real catalogue; no image generation | API key, Pollen authorization, and per-model license verification required |
| Supabase runtime | blocked | this integration commit | 2 sync/security tests, migration idempotency, 5 domain tests, full gate pass | Real local queue/client code; remote untested | Dev project, CLI, RLS hardening, credentials, advisors required |
| Sentry ADR | pending | - | - | - | No SDK or telemetry authorized |

## Stage record: independent library

- Implemented: independent Rust domain/application boundary, one ingestion
  entry point, B-roll matching and assignment adapters, reusable Story Builder
  contract, independent Biblioteca navigation, and review context.
- Tested so far: targeted Rust tests for SHA idempotency, assignment contract,
  candidate approval and placement, and daily asset reuse.
- Real: local SQLite, managed local files, QA metadata, and UI compilation.
- Simulated: image generation uses the explicit mock provider.
- Dependencies added: none.
- Cost observed: 0.
- Authorization required: none.
- Rollback: revert the phase commit; stored SQLite data is unchanged by schema.
- Final gate: `git diff --check`, `npm run check`, `npm run build`,
  `npm run test:fmt`, `npm run test:clippy`, `npm run test:unit:visual`, and
  `npm run test:smoke` passed on 2026-07-23. `svelte-check` reports one
  pre-existing accessibility warning in `ExportSuccess.svelte`; there are no
  Svelte errors. Visual tests: 42 passed. Smoke tests: 7 passed. The Windows
  linker emits an informational import-library message.
## Stage record: resident worker and scheduler

- Implemented: enqueue-only UI commands, resident Rust execution, persistent
  next-check reporting, video-over-daily priority, and removal of the Tauri
  worker-tick surface.
- Tested so far: queued state without inline candidate, video priority, two
  successive interval calculations, disabled scheduler, stale-job recovery,
  cancellation, daily cap, cooldown, and rejection/regeneration.
- Real: SQLite queue, leases, wake signal, scheduler settings, and cancellation
  state.
- Simulated: provider generation uses only the local mock in tests.
- Dependencies added: none.
- Cost observed: 0.
- Authorization required: none.
- Rollback: revert the phase commit; persisted queued jobs remain compatible.
- Final gate: `git diff --check`, check, build, fmt, clippy, 44 visual tests, and 7 smoke tests passed on 2026-07-23.
## Stage record: Dependabot

- Implemented: weekly npm and Cargo updates, PR limit 3 per ecosystem,
  minor/patch grouping, major-version ignore rules, labels, and no auto-merge.
- Tested: YAML parsing and structural assertions plus the complete local gate.
- Real: repository configuration is valid locally.
- Not yet observed: GitHub has not read the feature-branch file and no
  Dependabot PR has been created.
- Dependencies added: none.
- Cost observed: 0.
- Authorization required: repository settings for alerts/security updates and
  creation or verification of the configured labels.
- Rollback: remove `.github/dependabot.yml`.
- Final gate: check, build, fmt, clippy, 44 visual tests, and 7 smoke tests
  passed on 2026-07-23; YAML-specific assertions passed.
## Stage record: Codecov

- Implemented: isolated Rust coverage workflow, OIDC upload, informational 70%
  patch target, 2% threshold, rust flag, and ignored local report artifact.
- Tested: workflow/config YAML assertions, 100 instrumented Rust library tests,
  parsed 182,547-byte report with 91 files, and the complete local gate.
- Measurement: 44.67% approximate Rust line entries globally; 59.44% for
  paths containing `visual`.
- Real: local coverage generation with cargo-llvm-cov 0.8.7.
- Not yet observed: GitHub Actions upload and Codecov patch display.
- Dependencies added: none to VigilCut; cargo-llvm-cov is a developer tool.
- Cost observed: 0.
- Authorization required: Codecov repository onboarding if OIDC is not
  accepted automatically.
- Rollback: remove the coverage workflow and `codecov.yml`.
- Final gate: check, build, fmt, clippy, 44 visual tests, and 7 smoke tests
  passed on 2026-07-23 in addition to the instrumented run.
## Stage record: CodeRabbit

- Implemented: low-noise review profile, exclusions, incremental limits, and
  focused instructions for Rust, library boundaries, Svelte, SQL, tests, CI,
  and documentation.
- Tested: YAML parsed and all configured keys checked against the live official
  schema.v2.json; complete local gate passed.
- Real: configuration file only.
- Not operational: no GitHub App installation and no PR review observed.
- Dependencies added: none.
- Cost observed: 0.
- Authorization required: repository owner must authorize the CodeRabbit
  GitHub App and review its permissions/plan.
- Rollback: remove `.coderabbit.yaml` and revoke the App separately if installed.
- Final gate: check, build, fmt, clippy, 44 visual tests, and 7 smoke tests
  passed on 2026-07-23.
## Stage record: Pollinations

- Implemented: library-owned experimental adapter over the existing hardened
  OpenAI-compatible transport, fixed official host, explicit provider identity,
  configuration, metrics probe, and two independent cost gates.
- Tested: public catalogue probe found `flux` in 421 ms; disabled-route and
  paid-before-network tests pass; complete local gate passed.
- Real: public catalogue call only.
- Not run: image generation, because no credential or Pollen spend was
  authorized and commercial rights vary by model.
- Truth: `free_verified=false`, `cost_kind=paid`, license unknown,
  commercial_use unverified, daily feed ineligible.
- Dependencies added: none.
- Cost observed: 0 images and 0 observed monetary spend.
- Authorization required: Pollinations key, explicit permission for up to three
  metered requests, and model-license approval.
- Rollback: remove provider variant/module and environment entries; retain all
  independent assets.
- Final gate: check, build, fmt, clippy, 44 visual tests, 2 Pollinations tests,
  and 7 smoke tests passed on 2026-07-23.
## Stage record: Supabase runtime

- Implemented: opt-in Rust client, publishable/user-token contract, fixed URL
  allow-list, secret-key rejection, health/push code, additive SQLite queue,
  idempotent claim/retry, commands, and local-only UI status.
- Tested: queue deduplication/resume mock, local continuity, disabled default,
  secret rejection, SQLite migration idempotency, 5 independent-domain tests,
  and complete local gate.
- Real: SQLite queue and compiled HTTP client paths.
- Not run: any Supabase request, migration, Storage upload/download, pull, or
  advisor; no credentials or CLI are available.
- Security blocker: existing migration has broad Storage policies and a public
  SECURITY DEFINER helper. Runtime additionally requires RLS_VERIFIED.
- Dependencies added: none.
- Cost observed: 0.
- Authorization required: development project/credentials and later separate
  production approval.
- Rollback: remove optional client/commands/UI and additive queue code without
  deleting local assets.
- Final gate: check, build, fmt, clippy, 5 domain tests, 44 visual tests, and 7
  smoke tests passed on 2026-07-23.
