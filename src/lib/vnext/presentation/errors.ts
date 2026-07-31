/** Human recovery copy from job error payloads — never raw JSON in primary UI. */

export function humanErrorFromJobs(
  jobs: { status: string; errorJson?: string | null; kind?: string }[],
  fallback: string,
): string {
  const failed = jobs.find((j) => j.status === "failed" || j.status === "interrupted");
  if (!failed?.errorJson) return fallback;
  try {
    const parsed = JSON.parse(failed.errorJson) as {
      message?: string;
      code?: string;
    };
    if (parsed.message && parsed.message.trim()) {
      // strip technical codes from operator-facing sentence when too long
      return parsed.message.length > 220
        ? `${parsed.message.slice(0, 200)}…`
        : parsed.message;
    }
  } catch {
    if (failed.errorJson.length < 180 && !failed.errorJson.startsWith("{")) {
      return failed.errorJson;
    }
  }
  return fallback;
}
