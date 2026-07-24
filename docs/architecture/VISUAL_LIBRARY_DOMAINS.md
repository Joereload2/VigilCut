# Visual Library Domains

## Dependency rule

```text
Image Factory -> Visual Library
B-roll -> Smart Search -> Visual Library
Future visual creator -> Smart Search -> Visual Library
```

The Visual Library owns approved `MediaAsset` records, managed files, thumbnails, SHA-256 deduplication, metadata, provenance, license, QA, usage and read-only catalog operations. It does not know about a video timeline.

The Image Factory owns themes, coverage analysis, scene proposals, approved generation plans, jobs and candidates. It may call Library ingestion only after human approval.

B-roll owns video-scoped `VisualNeed`, assignment, placement, timing and composition. It may search and select approved assets. It must not create generation jobs, requests, candidates, provider calls or library metadata changes.

## Invariants

- A `VisualPlacement` contains only an approved `media_asset_id`.
- A `GeneratedCandidate` is never a placement and is never active automatically.
- Search is read-only; usage recording is explicit.
- Local SQLite is operational source of truth. Supabase remains optional.
- Unknown license is visible for manual review but not eligible for automatic B-roll selection.

## Current compatibility boundary

The existing `VisualNeed` and resident generation supervisor remain under `pipeline/visual` while extraction proceeds incrementally. This is an organizational debt, not a permission for B-roll to generate. The B-roll picker now exposes only search, selection, skip and navigation to Biblioteca.
