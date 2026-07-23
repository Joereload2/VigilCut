# Resident Worker and Scheduler

## Decision

Generation execution belongs to one resident Rust supervisor started from the
Tauri application setup. Svelte may enqueue, cancel, and poll persisted state;
it cannot invoke a worker tick or become the executor.

The supervisor:

- recovers `running` jobs whose persisted lease is absent or expired;
- atomically claims one queued job with a 120-second lease;
- orders `video_need` ahead of background daily work;
- processes generation independently of the active UI view;
- cooperatively cancels the in-flight provider future;
- runs the daily scheduler only while the application is open and video work
  is idle;
- applies the persisted interval, daily cap, cooldown, and provider
  `free_verified` gate.

## Enqueue semantics

Library and B-roll generation requests persist a queued job, request a
supervisor wake, and return the job identifier. `visual_cover_needs` is now
enqueue-only. The old Tauri `visual_worker_tick` surface and TypeScript wrapper
were removed. The CLI and Rust tests may still call the worker directly as an
explicit diagnostic mechanism.

Enabling the daily feed only persists the setting and wakes the supervisor. It
does not run a cycle inside the Svelte command. The settings response exposes
`executor=resident_rust` and the persisted `nextCheckAt`; the UI reports that
next check when one has been established.

## Cancellation

Queued cancellation is immediate and persisted. Running cancellation sets
`cancel_requested`, signals the in-memory cancellation registry, and races the
provider future against a 100 ms cancellation poll. Dropping the future aborts
the local request future; providers must remain cancellation-safe. A result
arriving across the boundary is discarded and its temporary file removed.

## Safety, cost, and rollback

Daily generation accepts only the local mock or a provider capability recorded
as `free_verified`. `free_configured` is insufficient. Paid providers remain
disabled by default. Tests use only the mock provider; measured external cost
is zero.

No dependency or schema migration was added. Rollback is the phase commit.
Queued jobs remain compatible with the previous SQLite representation.

## Validation

- enqueue persists `queued` and creates no candidate inline;
- video work is claimed before an older daily job;
- two successive persisted timestamps produce two successive scheduler checks;
- disabling the feed produces no next check;
- stale `running` jobs recover and process after restart simulation;
- queued cancellation and double approval are idempotent;
- daily limit, cooldown, rejection/regeneration, and unverified-free blocking
  remain covered by the visual test suite.
