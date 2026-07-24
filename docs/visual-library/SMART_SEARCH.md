# Smart Search contract

Smart Search is a read-only service between consumers and the Visual Library.

## Contract

`search_assets(query, filters)` returns asset, normalized score, reasons, warnings, automatic eligibility and exclusion reasons. `get_asset(asset_id)` returns metadata. `explain_match(query, asset_id)` explains the ranking. `record_usage(asset_id, consumer_context)` is a separate write operation.

Ranking evolves incrementally: text, concepts, tags, primary/secondary themes, positive/negative contexts, orientation, aspect ratio, asset status, license and repetition penalty. Embeddings remain a future provider, not an MVP dependency.

Manual search can show archived, unknown-license or otherwise warned assets. Automatic B-roll selection may use only active, present and license-eligible assets. Search itself never creates requests, jobs, candidates, placements or metadata edits.

## Consumer behavior

B-roll submits a video-scoped intent and receives matches. If no eligible match exists, it reports `No se encontró una imagen adecuada` and may persist an explicit passive uncovered need. That record cannot enqueue generation or call OmniRoute.

The Image Factory uses the same search to measure thematic coverage before proposing scenes.
