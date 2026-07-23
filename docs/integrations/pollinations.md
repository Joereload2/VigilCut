# Pollinations (experimental)

## Decision

Pollinations is not a verified free provider. As of 2026-07-23, its unified API
requires an API key and consumes metered Pollen. Promotional/quest Pollen is
not a stable zero-cost entitlement, pricing can change, and Pollinations'
current terms explicitly require checking each model license before commercial
use.

Therefore:

- `free_verified` is always false;
- the route is never eligible for daily feed;
- generated assets keep `LicenseStatus::Unknown`;
- `commercial_use` remains false/unverified;
- no real generation was made during implementation.

Official references reviewed:

- API: <https://github.com/pollinations/pollinations/blob/main/APIDOCS.md>
- Terms updated 2026-06-24: <https://enter.pollinations.ai/terms>
- Privacy/retention: <https://enter.pollinations.ai/privacy>

## Architecture

Pollinations exposes `POST /v1/images/generations`, the same OpenAI-compatible
shape already supported by VigilCut's hardened OmniRoute transport. A thin
library-owned provider adapter reuses that transport for timeout, manual
redirect validation, DNS/IP SSRF checks, MIME/decode checks, bounded downloads,
and candidate storage. It does not duplicate HTTP/download logic.

The adapter preserves `provider=pollinations`, uses only the fixed official
`https://gen.pollinations.ai/v1` host so a secret cannot be forwarded to a
configured attacker URL, and defaults to model `flux`.

Configuration is disabled by default:

```text
VIGILCUT_IMAGE_PROVIDER=pollinations
POLLINATIONS_API_KEY=<server-side key>
POLLINATIONS_IMAGE_MODEL=flux
POLLINATIONS_TIMEOUT_SECS=90
VIGILCUT_POLLINATIONS_EXPERIMENTAL=1
VIGILCUT_PAID_PROVIDERS=1
```

Both experimental and paid-provider gates are required because any request may
consume Pollen. The key is read only in Rust and sent as a bearer header to the
fixed official host; it must never be exposed in Svelte or committed.

## Probe and acceptance

The safe probe calls the public `/image/models` catalogue. A successful probe
proves only reachability and that the configured model is listed. It records:

- provider and model;
- latency;
- image catalogue visibility;
- `cost_kind=paid`;
- `free_verified=false`;
- an explicit note that generation, price, and license remain unverified.

It does not mark capability free or enable automatic generation.

Observed safe probe on 2026-07-23: `provider=pollinations`, `model=flux`, catalogue HTTP success, model visible, `free=false`, latency 421 ms. No API key was present and no image endpoint was called.

A future controlled generation requires user-provided credentials and explicit
authorization to consume up to three images. For each image, acceptance must
record model, request duration, dimensions, MIME, bytes, prompt, exclusions,
HTTP/rate-limit outcome, Pollen usage before/after, model-license evidence, and
technical/semantic QA. Results remain candidates for human review and are not
activated automatically.

## Rate limits, cancellation, and fair use

The transport handles 429 with bounded exponential backoff and caps attempts at
three. The provider future participates in the resident worker's cooperative
cancellation race. Pollinations documents `Retry-After`; honoring its exact
duration is a future improvement before production status. Daily feed remains
blocked, which is the effective fair-use cooldown while cost and quotas are
unverified.

## Cost, dependencies, and rollback

No dependency was added. The catalogue probe has no generation charge;
observed image-generation cost is zero because no image request was made.
Rollback removes the provider module, enum variant, and environment template
entries. Existing candidates/assets are independent and must not be deleted.
