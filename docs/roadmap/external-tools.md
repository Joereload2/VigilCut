# External Tools Roadmap

These tools are intentionally not implemented and add no dependencies.
Re-evaluate only when the trigger and exit criteria are both real.

| Tool | Consider when | Required evidence before adoption | Why deferred now |
|---|---|---|---|
| Braintrust | Prompt/model evaluations become frequent and reproducibility is poor | Redacted evaluation dataset, privacy review, measurable manual-evaluation cost, offline fallback | Current image flow is mostly deterministic infrastructure and mocks |
| Langfuse | Multiple production LLM calls need tracing across services | Opt-in/data map, self-hosted vs cloud decision, prompt/transcript redaction tests, retention policy | Would expose sensitive content and add another observability system |
| Comet Opik | A dedicated experiment/evaluation platform is needed after comparing alternatives | Side-by-side trial against Braintrust/Langfuse with one chosen owner and success metric | Avoid three overlapping evaluation stacks |
| Cloudinary | Remote transformations/CDN materially reduce export or delivery time | Measured delivery bottleneck, cost model, license/privacy review, offline behavior | Local managed files and optional Supabase Storage cover current needs |
| Neon | A server Postgres workload exists independently of Supabase | Workload/latency measurements, migration plan, auth/RLS equivalent, operational owner | Supabase is already the optional catalog candidate |
| Turso | Multi-region edge SQLite reads are required | Real multi-device read pattern, conflict design, encryption/auth review | Desktop SQLite is local and no edge consumer exists |
| Upstash | Managed Redis/queues are needed by a deployed service | Queue/cache workload, durability semantics, cost cap, data classification | Resident Rust worker and SQLite queue meet desktop needs |
| Portkey | Several paid AI gateways require centralized policy/routing | At least two verified providers, measurable routing problem, privacy/cost audit | OmniRoute plus explicit providers is sufficient and mostly local |
| Auth0 | Supabase Auth cannot satisfy a documented enterprise identity requirement | Customer requirement, protocol/tenant matrix, migration and recurring-cost analysis | No current external identity requirement |
| Clerk | A hosted web product needs turnkey user-management UX | Web product scope, privacy/DPA review, Supabase Auth comparison | VigilCut is desktop/local-first |
| Render | A backend service needs continuous hosting | Defined service, capacity/SLA/cost model, secrets and rollback plan | No backend deployment exists |
| Vercel | A web frontend or edge API is approved | Deployable web surface, privacy/cost review, desktop separation | Current application is Tauri desktop |
| Netlify | A static/web deployment has requirements Vercel/Pages do not meet | Hosting comparison with one selected platform and exit criteria | Avoid parallel hosting platforms |

## Selection rule

Adopt at most one tool per unmet capability. Each adoption requires an ADR,
disabled-by-default configuration where applicable, mock/isolated tests, cost
and data-flow measurements, rollback, the full VigilCut validation gate, and a
and a separate commit. "Free tier" alone is not a product requirement.
