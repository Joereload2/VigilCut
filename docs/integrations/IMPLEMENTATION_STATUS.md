# External Integrations Implementation Status

This file records verified evidence after each isolated implementation gate.
`verified` is used only after the required local commands pass.

| Etapa | Estado | Commit | Pruebas | Real/Mock | Bloqueos |
|-------|--------|--------|---------|-----------|----------|
| Biblioteca independiente | verified | this phase commit | `diff --check`, check, build, fmt, clippy, 42 visual tests, and 7 smoke tests pass | Real SQLite/filesystem; mock image provider | None |
| Worker/Scheduler | pending | - | - | - | - |
| Dependabot | pending | - | - | - | GitHub activation to verify |
| Codecov | pending | - | - | - | External upload/authorization may be required |
| CodeRabbit | pending | - | - | - | GitHub App authorization required |
| Pollinations | pending | - | - | - | Terms, license, free route, and network probe required |
| Supabase runtime | pending | - | - | - | Development credentials required for real verification |
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
