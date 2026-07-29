import { invoke } from "@tauri-apps/api/core";
import type { ClipCandidate, ClipFraming, ClippingRun } from "$lib/types";
import type {
  ArtifactManifestV1,
  ContentProjectSnapshotV1,
  CreateProjectRequestV1,
  CreateRenderPlanRequestV1,
  JobSnapshotV1,
  SaveReviewDecisionRequestV1,
  VerticalRenderPlanV1,
} from "$lib/types/vnext/v1";
import { isTauri } from "./tauri";

function webStub(): never {
  throw new Error("vNext API requiere la aplicación de escritorio (Tauri)");
}

export async function createContentProject(
  req: CreateProjectRequestV1,
): Promise<ContentProjectSnapshotV1> {
  if (!isTauri()) webStub();
  return invoke("vnext_create_content_project", { req });
}

export async function listContentProjects(limit = 50): Promise<ContentProjectSnapshotV1[]> {
  if (!isTauri()) return [];
  return invoke("vnext_list_content_projects", { limit });
}

export async function getContentProject(id: string): Promise<ContentProjectSnapshotV1> {
  if (!isTauri()) webStub();
  return invoke("vnext_get_content_project", { id });
}

export async function getJob(id: string): Promise<JobSnapshotV1> {
  if (!isTauri()) webStub();
  return invoke("vnext_get_job", { id });
}

export async function listProjectJobs(
  contentProjectId: string,
  limit = 100,
): Promise<JobSnapshotV1[]> {
  if (!isTauri()) return [];
  return invoke("vnext_list_project_jobs", { contentProjectId, limit });
}

export async function listQueueJobs(limit = 100): Promise<JobSnapshotV1[]> {
  if (!isTauri()) return [];
  return invoke("vnext_list_queue_jobs", { limit });
}

export async function retryJob(id: string): Promise<JobSnapshotV1> {
  if (!isTauri()) webStub();
  return invoke("vnext_retry_job", { id });
}

export async function cancelJob(id: string): Promise<JobSnapshotV1> {
  if (!isTauri()) webStub();
  return invoke("vnext_cancel_job", { id });
}

export async function listShortCandidates(
  contentProjectId: string,
): Promise<ClipCandidate[]> {
  if (!isTauri()) return [];
  return invoke("vnext_list_short_candidates", { contentProjectId });
}

export async function listClippingRuns(contentProjectId: string): Promise<string[]> {
  if (!isTauri()) return [];
  return invoke("vnext_list_clipping_runs", { contentProjectId });
}

export async function getClippingRun(runId: string): Promise<ClippingRun> {
  if (!isTauri()) webStub();
  return invoke("vnext_get_clipping_run", { runId });
}

export async function saveReviewDecision(
  req: SaveReviewDecisionRequestV1,
): Promise<ClipCandidate> {
  if (!isTauri()) webStub();
  return invoke("vnext_save_review_decision", { req });
}

export async function createVerticalRenderPlan(
  req: CreateRenderPlanRequestV1,
): Promise<VerticalRenderPlanV1> {
  if (!isTauri()) webStub();
  return invoke("vnext_create_vertical_render_plan", { req });
}

export async function startVerticalRender(planId: string): Promise<JobSnapshotV1> {
  if (!isTauri()) webStub();
  return invoke("vnext_start_vertical_render", { planId });
}

export async function listProjectArtifacts(
  contentProjectId: string,
  limit = 100,
): Promise<ArtifactManifestV1[]> {
  if (!isTauri()) return [];
  return invoke("vnext_list_project_artifacts", { contentProjectId, limit });
}

export async function projectNextAction(contentProjectId: string): Promise<string> {
  if (!isTauri()) return "ingest";
  return invoke("vnext_project_next_action", { contentProjectId });
}

export async function runClippingForProject(
  contentProjectId: string,
): Promise<ClippingRun> {
  if (!isTauri()) webStub();
  return invoke("vnext_run_clipping_for_project", { contentProjectId });
}

export type { ClipFraming };
