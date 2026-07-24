# Image Factory: thematic planning

The factory is separate from B-roll. Its unit of work is an approved thematic plan, not a video need.

## Required flow

1. Create a theme with name and objective.
2. Analyze current library coverage.
3. Propose broad concepts and drawable scenes.
4. Search existing approved assets for each scene.
5. Classify coverage as `none`, `basic`, `varied` or `deep`.
6. Let the user edit, remove and add proposals.
7. Require explicit plan approval.
8. Build positive/negative prompts for approved uncovered scenes.
9. Enqueue only approved scenes after cost/provider gates.
10. Send all results to `Por revisar`.
11. Ingest only human-approved candidates.

No image generation may start while the plan is `draft`, `analyzing` or `ready_for_review`.

## MVP defaults

The first factory slice should target 16:9, 1280x720, one photorealistic image per scene, no text/logos/watermarks and mandatory human review. Breadth precedes depth: coverage must vary subject, environment, action, emotion and subtopic, not merely count assets.

## Current status

The repository contains daily-feed and generation infrastructure, but no verified thematic-plan UI with an approval gate. It must not be described as complete until plan persistence, approval, restart recovery and zero-cost gating are tested.
