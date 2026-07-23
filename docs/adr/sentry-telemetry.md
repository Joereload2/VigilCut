# ADR: Optional Sentry Diagnostics

- Status: deferred
- Date: 2026-07-23
- Decision owner: VigilCut

## Context

VigilCut is a local-first desktop editor. Normal operation handles local paths,
video/audio, transcripts, prompts, generated images, project identifiers,
provider errors, and possibly authentication tokens. A generic crash-reporting
SDK can capture these values through stack locals, messages, breadcrumbs,
request headers, URLs, panic text, or attachments.

Silent collection would violate the product's local-first expectation. A DSN
being technically non-secret does not make transmission consent-free.

## Decision

Do not install or initialize a Sentry SDK now.

The immutable default is:

```text
telemetry_enabled=false
```

No event may leave the device before a separate implementation satisfies every
precondition below and the user gives explicit, revocable opt-in. Declining
must not reduce product functionality.

## Preconditions for a future implementation

### Consent and controls

- Present a plain-language opt-in before SDK initialization.
- Show the exact allow-listed fields and retention period.
- Separate crash diagnostics from product analytics; no bundled consent.
- Provide an always-visible setting to disable future sends.
- Disabling must stop the client immediately and delete unsent local envelopes.
- Never infer consent from accepting terms, opening the app, or enabling sync.

### Event allow-list

Allowed:

- VigilCut version, OS family/version, CPU architecture;
- anonymous installation-scoped random ID that can be reset;
- error type/code and scrubbed stack frame module/function names;
- operation category such as import, render, local sync queue, or worker;
- persisted state enum and retry count;
- duration and coarse file-size/dimension buckets.

Forbidden:

- local paths or filenames;
- prompts, negative prompts, transcripts, subtitles, scene text, titles, tags,
  concepts, or user-entered instructions;
- video/audio/image bytes, thumbnails, attachments, screenshots, or minidumps
  containing process memory;
- Supabase/Pollinations/OmniRoute keys, access tokens, authorization headers,
  signed URLs, DSNs in event payloads, or environment dumps;
- project keys, asset UUIDs, job UUIDs, workspace/user/email identifiers;
- IP-derived location, device name, username, or home directory;
- provider request/response bodies and query strings.

### Redaction

- Construct events from typed allow-listed fields; do not serialize arbitrary
  errors, command arguments, state objects, HTTP objects, or environment.
- Apply a second `before_send` deny-list for Windows/Unix paths, bearer/JWT/API
  key patterns, URLs/query strings, UUIDs, emails, and known content fields.
- Disable default PII and automatic HTTP breadcrumbs.
- Hashing a path is not acceptable: path structure and stable correlation still
  leak information.
- Unit-test every forbidden category and fail closed by dropping the event.

### Operations and privacy

- Select an appropriate processing region and document subprocessors.
- Initial event retention must be at most 30 days.
- Disable session replay, profiling, screenshots, attachments, user feedback
  attachments, and performance spans until separately approved.
- Document deletion/export handling and the installation-ID reset flow.
- Set a strict event-rate and monthly budget cap; failure to send is silent and
  never blocks editing/export.
- Complete privacy-policy and DPA review before release.

## Acceptance tests

1. Fresh install sends zero network requests to Sentry.
2. Upgrade from an older version remains opted out.
3. Opt-in initializes only after confirmation.
4. Each forbidden sample is dropped or redacted with no recoverable substring.
5. Panic containing a path, prompt, JWT, UUID, email, and signed URL sends none
   of them.
6. Disable stops sends immediately and removes queued envelopes.
7. Offline use remains fully functional.
8. Event volume and retention controls are visible and enforced.
9. A packet capture matches the documented allow-list.
10. Removing the SDK/config leaves no telemetry background process.

## Alternatives

Continue structured local logs with a user-initiated "Export diagnostic
bundle" flow. The export must preview files, apply the same redaction tests,
exclude media/content by default, and require the user to choose how to share
it. This remains the preferred near-term diagnostic path.

## Consequences

Crash aggregation is deferred, but there is no new dependency, data transfer,
cost, or privacy exposure. Revisit only after the consent/redaction product
work is prioritized. Rollback is unnecessary because no SDK is installed.

