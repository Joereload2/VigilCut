# Optional Supabase Library Synchronization

## Status

The runtime is **blocked/unverified**, not connected. SQLite remains the sole
operational source of truth. No development or production Supabase project was
contacted because no URL, publishable key, user access token, workspace ID, or
Supabase CLI is available in this environment.

Implemented locally:

- opt-in configuration with a second `RLS_VERIFIED` safety gate;
- publishable-key and per-user access-token contract;
- rejection of `sb_secret_` keys;
- fixed hosted/local URL allow-list;
- authenticated health-check client;
- private Storage upload through the Storage API (never `storage.objects`);
- metadata upsert by stable asset UUID/SHA;
- additive SQLite `sync_queue`;
- idempotent enqueue, atomic claim, retries/backoff, and offline preservation;
- library UI status (`local_only`, queue counts);
- mock tests for enqueue, deduplication, resume behavior, and local continuity.

Not implemented/verified:

- Auth UI or secure session refresh;
- remote schema application;
- real health check, upload, metadata push, pull, download, or reconnect;
- conflict resolution beyond remote UUID/SHA upsert;
- signed URL flow;
- automatic queue supervision;
- deletion propagation (intentionally absent).

## Current Supabase guidance reviewed

Reviewed on 2026-07-23:

- breaking-change changelog;
- Data API security and explicit grants;
- Storage access control;
- Storage schema read-only rules;
- private downloads/signed URLs;
- publishable versus secret keys.

Relevant current behavior: starting 2026-05-30, new `public` tables are not
automatically exposed to Data/GraphQL APIs, with enforcement for all projects
scheduled for 2026-10-30. Grants and RLS are separate controls and both must be
verified.

Official references:

- <https://supabase.com/changelog?types=breaking-change>
- <https://supabase.com/docs/guides/api/securing-your-api>
- <https://supabase.com/docs/guides/storage/security/access-control>
- <https://supabase.com/docs/guides/storage/schema/design>
- <https://supabase.com/docs/guides/storage/serving/downloads>
- <https://supabase.com/docs/guides/getting-started/api-keys>

## Security audit of the existing migration

Do **not** apply
`supabase/migrations/20260722000000_visual_library.sql` to a shared project in
its current form.

Blocking findings:

1. Storage SELECT/INSERT/UPDATE/DELETE policies authorize any authenticated
   user for every object in `visual-library`; paths are not constrained by
   workspace membership or owner.
2. `public.is_workspace_member` is `SECURITY DEFINER` in an exposed schema and
   EXECUTE is not revoked from PUBLIC.
3. `workspaces` and `workspace_members` have RLS enabled but no ownership or
   membership policies, so the intended bootstrap/access flow is incomplete.
4. `asset_concepts`, candidates, QA, usage, assignments, and sync state have RLS
   enabled but no policies, so runtime operations cannot work through a user
   session.
5. Workspace members can update broad catalog rows without an explicit
   owner/admin role check.
6. Data API grants are not explicit and will vary with project creation date.
7. The migration inserts the bucket row directly. Supabase now recommends
   treating Storage schema records as read-only and using the Storage API or
   Dashboard for object/bucket operations.

The client requires `VIGILCUT_SUPABASE_RLS_VERIFIED=1` in addition to sync
enablement, preventing accidental use before these findings are resolved.

## Required development-project verification

Install the current Supabase CLI, then discover commands with `--help`. Create
an additive hardening migration using `supabase migration new`; do not edit the
existing applied migration or invent a filename.

Minimum acceptance sequence:

1. Create a development-only project and user/workspace fixtures.
2. Apply migrations.
3. Add explicit least-privilege grants for authenticated roles.
4. Replace the public definer helper or move it to a non-exposed schema, revoke
   PUBLIC execute, set a safe search path, and test it.
5. Add policies for every table used by sync, with UPDATE `USING` and
   `WITH CHECK`.
6. Restrict Storage policies to the user's workspace path and membership.
7. Run `supabase db advisors` and resolve security/performance findings.
8. With user A, push one approved asset and metadata.
9. Verify user B outside the workspace cannot list/read/update/delete it.
10. Download through authenticated API or short signed URL; verify SHA-256.
11. Disconnect network; confirm SQLite search/render/import still work.
12. Reconnect; process the queue twice and assert one remote asset by UUID/SHA.
13. Pull catalog metadata without Base64 and without overwriting newer local
    changes.
14. Record exact conflict policy and never propagate deletes automatically.

Only after all assertions pass may `VIGILCUT_SUPABASE_RLS_VERIFIED=1` be set in
a development configuration. Production remains a separate authorization.

## Configuration

```text
VIGILCUT_SUPABASE_SYNC=0
VIGILCUT_SUPABASE_RLS_VERIFIED=0
SUPABASE_URL=https://<development-ref>.supabase.co
SUPABASE_PUBLISHABLE_KEY=sb_publishable_...
SUPABASE_ACCESS_TOKEN=<user-session-jwt>
SUPABASE_WORKSPACE_ID=<uuid>
```

Never use `service_role`, `sb_secret_`, or an admin token in desktop/Svelte.

## Cost, dependencies, and rollback

No dependency was added; the existing Rust HTTP client is reused. No remote
request or Storage operation ran, so observed external cost is zero. Rollback
removes the Supabase client/commands/UI status and the additive local queue
table code. Existing SQLite assets and queue data must not be deleted.
