# Independent Visual Library Domain

## Decision

The Visual Library is an application domain, not a B-roll subsystem. B-roll
and the future Story Builder are consumers of the same `VisualLibrary`
contract. They do not access SQLite tables, managed-storage paths, generation
providers, daily-feed state, or candidate records.

The first compatibility refactor deliberately keeps the existing SQLite
schema and low-level persistence functions in place. New code enters through
`src-tauri/src/visual_library`; the legacy `pipeline::visual` persistence is an
infrastructure adapter until repositories can be extracted without a
destructive migration.

## Boundary

The library owns assets, concepts, provenance, licensing, QA, usage,
generation jobs, candidates, provider capability, and daily-feed settings.
B-roll owns needs, assignments, placements, plans, and time mapping.

The shared boundary consists of:

- `AssetQuery`
- `AssetMatch`
- `AssetSelection`
- `AssetIngestionRequest`
- `AssetIngestionResult`
- `LibraryGenerationRequest`
- `AssetUsage`
- the `VisualLibrary` trait

`SceneRequirement` uses the same search contract and does not inherit from
`VisualNeed`.

## Single ingestion path

All newly usable images enter through `LibraryService::ingest_asset`.
Supported sources are manual import, folder import, daily generation, B-roll
generation, Story Builder generation, and remote synchronization.

The compatibility implementation performs file validation, SHA-256
deduplication, perceptual-hash calculation, thumbnail creation, managed copy,
metadata/provenance persistence, license persistence, concept links, QA
metadata, and returns the stable `asset_id`. Approval is idempotent: repeated
approval resolves to the existing asset.

Generated candidates cannot be placed directly. Worker or human approval
first ingests a `MediaAsset`; B-roll assignment accepts only a
`media_asset_id`.

## UI

`Biblioteca` is a main navigation mode alongside Silencios, Clips, and
Visual/B-roll. It can open without video, EDL, transcript, project key, or
visual plan. The initial independent workspace reuses the library-owned asset,
review, daily-feed, and generation views in library-only mode. B-roll keeps a
consumer-focused workspace and may open the library rather than duplicating
the full explorer.

## Compatibility and rollback

No dependency or destructive migration is introduced. Existing assets remain
in the current SQLite database and managed storage. Rollback is the single
phase commit; the additive module and UI route can be reverted without
rewriting stored data.

## Validation evidence

- Manual and folder import route through the ingestion service.
- B-roll generation approval routes through ingestion before assignment.
- Daily generation approval creates a library asset and no placement.
- A later video can assign that daily asset by `media_asset_id`.
- SHA-256 repeated ingestion returns the same asset.
- Story Builder contracts compile against the `VisualLibrary` interface.
- Cost observed: zero; all generation tests use the mock provider.
