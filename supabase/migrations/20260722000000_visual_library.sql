-- VigilCut Intelligent Visual Library — Supabase catalog (optional remote)
-- Local SQLite remains SoT for offline desktop. This schema is the central catalog.
-- No images as Base64. Files live in Storage bucket visual-library.

-- Enable extensions commonly available on Supabase
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Workspace readiness without fake auth: owner_id nullable until real auth lands.
-- When auth exists, RLS checks auth.uid() = owner_id OR workspace membership.

CREATE TABLE IF NOT EXISTS public.workspaces (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS public.workspace_members (
  workspace_id UUID NOT NULL REFERENCES public.workspaces(id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
  role TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('owner', 'admin', 'member', 'viewer')),
  PRIMARY KEY (workspace_id, user_id)
);

CREATE TABLE IF NOT EXISTS public.themes (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID REFERENCES public.workspaces(id) ON DELETE CASCADE,
  owner_id UUID REFERENCES auth.users(id),
  slug TEXT NOT NULL,
  title TEXT NOT NULL,
  description TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (workspace_id, slug)
);

CREATE TABLE IF NOT EXISTS public.visual_concepts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID REFERENCES public.workspaces(id) ON DELETE CASCADE,
  owner_id UUID REFERENCES auth.users(id),
  theme_id UUID REFERENCES public.themes(id) ON DELETE SET NULL,
  canonical_key TEXT NOT NULL,
  title TEXT NOT NULL,
  literal_description JSONB NOT NULL DEFAULT '[]'::jsonb,
  meanings JSONB NOT NULL DEFAULT '[]'::jsonb,
  positive_contexts JSONB NOT NULL DEFAULT '[]'::jsonb,
  negative_contexts JSONB NOT NULL DEFAULT '[]'::jsonb,
  hard_exclusions JSONB NOT NULL DEFAULT '[]'::jsonb,
  desired_formats JSONB NOT NULL DEFAULT '["16:9"]'::jsonb,
  priority INT NOT NULL DEFAULT 50 CHECK (priority BETWEEN 0 AND 100),
  request_count INT NOT NULL DEFAULT 0 CHECK (request_count >= 0),
  coverage_count INT NOT NULL DEFAULT 0 CHECK (coverage_count >= 0),
  status TEXT NOT NULL DEFAULT 'active'
    CHECK (status IN ('draft', 'active', 'archived', 'priority')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (workspace_id, canonical_key)
);

CREATE TABLE IF NOT EXISTS public.media_assets (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID REFERENCES public.workspaces(id) ON DELETE CASCADE,
  owner_id UUID REFERENCES auth.users(id),
  kind TEXT NOT NULL DEFAULT 'image',
  storage_path TEXT NOT NULL,
  thumbnail_path TEXT,
  preview_path TEXT,
  sha256 TEXT NOT NULL,
  perceptual_hash TEXT,
  title TEXT NOT NULL,
  description TEXT,
  literal_description JSONB NOT NULL DEFAULT '[]'::jsonb,
  meanings JSONB NOT NULL DEFAULT '[]'::jsonb,
  positive_contexts JSONB NOT NULL DEFAULT '[]'::jsonb,
  negative_contexts JSONB NOT NULL DEFAULT '[]'::jsonb,
  hard_exclusions JSONB NOT NULL DEFAULT '[]'::jsonb,
  tags JSONB NOT NULL DEFAULT '[]'::jsonb,
  width INT NOT NULL CHECK (width > 0),
  height INT NOT NULL CHECK (height > 0),
  aspect_ratio TEXT,
  orientation TEXT NOT NULL DEFAULT 'landscape',
  mime_type TEXT NOT NULL,
  file_size BIGINT NOT NULL DEFAULT 0 CHECK (file_size >= 0),
  safe_area TEXT DEFAULT 'center',
  license_status TEXT NOT NULL DEFAULT 'unknown'
    CHECK (license_status IN ('owned','licensed','public_domain','attribution_required','unknown')),
  commercial_use BOOLEAN,
  provenance JSONB,
  technical_score DOUBLE PRECISION,
  semantic_score DOUBLE PRECISION,
  qa_status TEXT NOT NULL DEFAULT 'none'
    CHECK (qa_status IN ('none','pending','automated_review','needs_human_review','approved','rejected')),
  status TEXT NOT NULL DEFAULT 'active'
    CHECK (status IN ('active','archived','blocked','missing','invalid')),
  times_used INT NOT NULL DEFAULT 0,
  last_used_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (workspace_id, sha256)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_media_assets_sha256_global
  ON public.media_assets (sha256) WHERE workspace_id IS NULL;

CREATE TABLE IF NOT EXISTS public.asset_concepts (
  asset_id UUID NOT NULL REFERENCES public.media_assets(id) ON DELETE CASCADE,
  concept_id UUID NOT NULL REFERENCES public.visual_concepts(id) ON DELETE CASCADE,
  weight DOUBLE PRECISION NOT NULL DEFAULT 1.0,
  PRIMARY KEY (asset_id, concept_id)
);

CREATE TABLE IF NOT EXISTS public.visual_needs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID REFERENCES public.workspaces(id) ON DELETE CASCADE,
  owner_id UUID REFERENCES auth.users(id),
  project_key TEXT NOT NULL,
  media_path TEXT,
  semantic_event_id TEXT,
  concept_id UUID REFERENCES public.visual_concepts(id) ON DELETE SET NULL,
  label TEXT NOT NULL,
  terms JSONB NOT NULL DEFAULT '[]'::jsonb,
  required_contexts JSONB NOT NULL DEFAULT '[]'::jsonb,
  forbidden_contexts JSONB NOT NULL DEFAULT '[]'::jsonb,
  hard_exclusions JSONB NOT NULL DEFAULT '[]'::jsonb,
  desired_aspect TEXT NOT NULL DEFAULT '16:9',
  approx_duration_secs DOUBLE PRECISION NOT NULL DEFAULT 5,
  source_start DOUBLE PRECISION,
  source_end DOUBLE PRECISION,
  output_start DOUBLE PRECISION,
  output_end DOUBLE PRECISION,
  priority INT NOT NULL DEFAULT 50,
  coverage TEXT NOT NULL DEFAULT 'uncovered'
    CHECK (coverage IN ('uncovered','matched','generating','needs_review','covered','skipped','failed')),
  matched_asset_id UUID REFERENCES public.media_assets(id) ON DELETE SET NULL,
  match_score DOUBLE PRECISION,
  match_reasons JSONB NOT NULL DEFAULT '[]'::jsonb,
  generation_job_id UUID,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_visual_needs_project ON public.visual_needs(project_key);
CREATE INDEX IF NOT EXISTS idx_visual_needs_coverage ON public.visual_needs(coverage);

CREATE TABLE IF NOT EXISTS public.generation_jobs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID REFERENCES public.workspaces(id) ON DELETE CASCADE,
  owner_id UUID REFERENCES auth.users(id),
  idempotency_key TEXT NOT NULL,
  need_id UUID REFERENCES public.visual_needs(id) ON DELETE SET NULL,
  concept_id UUID REFERENCES public.visual_concepts(id) ON DELETE SET NULL,
  status TEXT NOT NULL DEFAULT 'queued'
    CHECK (status IN ('queued','running','succeeded','failed','cancelled','blocked_policy')),
  provider TEXT,
  model TEXT,
  prompt TEXT NOT NULL DEFAULT '',
  negative_prompt TEXT NOT NULL DEFAULT '',
  attempt INT NOT NULL DEFAULT 0,
  max_attempts INT NOT NULL DEFAULT 2,
  last_error TEXT,
  is_paid BOOLEAN NOT NULL DEFAULT false,
  opportunistic BOOLEAN NOT NULL DEFAULT false,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (workspace_id, idempotency_key)
);

CREATE INDEX IF NOT EXISTS idx_generation_jobs_status ON public.generation_jobs(status);

CREATE TABLE IF NOT EXISTS public.generated_candidates (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  job_id UUID NOT NULL REFERENCES public.generation_jobs(id) ON DELETE CASCADE,
  need_id UUID REFERENCES public.visual_needs(id) ON DELETE SET NULL,
  storage_path TEXT,
  local_path TEXT,
  sha256 TEXT,
  perceptual_hash TEXT,
  status TEXT NOT NULL DEFAULT 'generated'
    CHECK (status IN ('generated','automated_review','needs_human_review','approved','rejected','discarded')),
  technical_score DOUBLE PRECISION,
  semantic_score DOUBLE PRECISION,
  qa_decision TEXT,
  qa_reason TEXT,
  approved_asset_id UUID REFERENCES public.media_assets(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS public.qa_checks (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  candidate_id UUID REFERENCES public.generated_candidates(id) ON DELETE CASCADE,
  asset_id UUID REFERENCES public.media_assets(id) ON DELETE CASCADE,
  technical_quality DOUBLE PRECISION NOT NULL,
  semantic_alignment DOUBLE PRECISION NOT NULL,
  forbidden_detected JSONB NOT NULL DEFAULT '[]'::jsonb,
  text_detected BOOLEAN NOT NULL DEFAULT false,
  watermark_detected BOOLEAN NOT NULL DEFAULT false,
  decision TEXT NOT NULL,
  reason TEXT NOT NULL,
  details JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS public.asset_usage (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  asset_id UUID NOT NULL REFERENCES public.media_assets(id) ON DELETE CASCADE,
  media_path TEXT NOT NULL,
  run_id TEXT,
  used_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  output_start DOUBLE PRECISION,
  output_end DOUBLE PRECISION,
  owner_id UUID REFERENCES auth.users(id)
);

CREATE TABLE IF NOT EXISTS public.provider_capabilities (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  provider TEXT NOT NULL,
  model TEXT NOT NULL,
  supports_image BOOLEAN NOT NULL DEFAULT false,
  free_tier BOOLEAN NOT NULL DEFAULT true,
  last_probe_ok BOOLEAN NOT NULL DEFAULT false,
  last_probe_at TIMESTAMPTZ,
  last_error TEXT,
  latency_ms INT,
  notes TEXT,
  UNIQUE (provider, model)
);

CREATE TABLE IF NOT EXISTS public.scene_asset_assignments (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID REFERENCES public.workspaces(id) ON DELETE CASCADE,
  owner_id UUID REFERENCES auth.users(id),
  project_key TEXT NOT NULL,
  scene_id TEXT,
  media_asset_id UUID NOT NULL REFERENCES public.media_assets(id) ON DELETE CASCADE,
  need_id UUID REFERENCES public.visual_needs(id) ON DELETE SET NULL,
  output_start DOUBLE PRECISION NOT NULL,
  output_end DOUBLE PRECISION NOT NULL,
  mode TEXT NOT NULL DEFAULT 'fullframe',
  match_score DOUBLE PRECISION,
  provenance TEXT NOT NULL DEFAULT 'library_match',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS public.sync_state (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID REFERENCES public.workspaces(id) ON DELETE CASCADE,
  owner_id UUID REFERENCES auth.users(id),
  entity TEXT NOT NULL,
  local_updated_at TIMESTAMPTZ,
  remote_updated_at TIMESTAMPTZ,
  last_sync_at TIMESTAMPTZ,
  status TEXT NOT NULL DEFAULT 'idle',
  UNIQUE (workspace_id, owner_id, entity)
);

-- updated_at trigger helper
CREATE OR REPLACE FUNCTION public.set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- RLS
ALTER TABLE public.workspaces ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.workspace_members ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.themes ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.visual_concepts ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.media_assets ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.asset_concepts ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.visual_needs ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.generation_jobs ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.generated_candidates ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.qa_checks ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.asset_usage ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.provider_capabilities ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.scene_asset_assignments ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.sync_state ENABLE ROW LEVEL SECURITY;

-- Membership helper (no user_metadata)
CREATE OR REPLACE FUNCTION public.is_workspace_member(ws UUID)
RETURNS BOOLEAN
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = public
AS $$
  SELECT EXISTS (
    SELECT 1 FROM public.workspace_members m
    WHERE m.workspace_id = ws AND m.user_id = auth.uid()
  );
$$;

-- Owner or workspace member policies (SELECT)
CREATE POLICY themes_select ON public.themes FOR SELECT
  USING (owner_id = auth.uid() OR (workspace_id IS NOT NULL AND public.is_workspace_member(workspace_id)));
CREATE POLICY themes_insert ON public.themes FOR INSERT
  WITH CHECK (owner_id = auth.uid() OR (workspace_id IS NOT NULL AND public.is_workspace_member(workspace_id)));
CREATE POLICY themes_update ON public.themes FOR UPDATE
  USING (owner_id = auth.uid() OR (workspace_id IS NOT NULL AND public.is_workspace_member(workspace_id)))
  WITH CHECK (owner_id = auth.uid() OR (workspace_id IS NOT NULL AND public.is_workspace_member(workspace_id)));
CREATE POLICY themes_delete ON public.themes FOR DELETE
  USING (owner_id = auth.uid());

CREATE POLICY concepts_select ON public.visual_concepts FOR SELECT
  USING (owner_id = auth.uid() OR (workspace_id IS NOT NULL AND public.is_workspace_member(workspace_id)));
CREATE POLICY concepts_insert ON public.visual_concepts FOR INSERT
  WITH CHECK (owner_id = auth.uid() OR (workspace_id IS NOT NULL AND public.is_workspace_member(workspace_id)));
CREATE POLICY concepts_update ON public.visual_concepts FOR UPDATE
  USING (owner_id = auth.uid() OR (workspace_id IS NOT NULL AND public.is_workspace_member(workspace_id)))
  WITH CHECK (owner_id = auth.uid() OR (workspace_id IS NOT NULL AND public.is_workspace_member(workspace_id)));
CREATE POLICY concepts_delete ON public.visual_concepts FOR DELETE
  USING (owner_id = auth.uid());

CREATE POLICY assets_select ON public.media_assets FOR SELECT
  USING (owner_id = auth.uid() OR (workspace_id IS NOT NULL AND public.is_workspace_member(workspace_id)));
CREATE POLICY assets_insert ON public.media_assets FOR INSERT
  WITH CHECK (owner_id = auth.uid() OR (workspace_id IS NOT NULL AND public.is_workspace_member(workspace_id)));
CREATE POLICY assets_update ON public.media_assets FOR UPDATE
  USING (owner_id = auth.uid() OR (workspace_id IS NOT NULL AND public.is_workspace_member(workspace_id)))
  WITH CHECK (owner_id = auth.uid() OR (workspace_id IS NOT NULL AND public.is_workspace_member(workspace_id)));
CREATE POLICY assets_delete ON public.media_assets FOR DELETE
  USING (owner_id = auth.uid());

CREATE POLICY needs_select ON public.visual_needs FOR SELECT
  USING (owner_id = auth.uid() OR (workspace_id IS NOT NULL AND public.is_workspace_member(workspace_id)));
CREATE POLICY needs_insert ON public.visual_needs FOR INSERT
  WITH CHECK (owner_id = auth.uid() OR (workspace_id IS NOT NULL AND public.is_workspace_member(workspace_id)));
CREATE POLICY needs_update ON public.visual_needs FOR UPDATE
  USING (owner_id = auth.uid() OR (workspace_id IS NOT NULL AND public.is_workspace_member(workspace_id)))
  WITH CHECK (owner_id = auth.uid() OR (workspace_id IS NOT NULL AND public.is_workspace_member(workspace_id)));
CREATE POLICY needs_delete ON public.visual_needs FOR DELETE
  USING (owner_id = auth.uid());

CREATE POLICY jobs_select ON public.generation_jobs FOR SELECT
  USING (owner_id = auth.uid() OR (workspace_id IS NOT NULL AND public.is_workspace_member(workspace_id)));
CREATE POLICY jobs_insert ON public.generation_jobs FOR INSERT
  WITH CHECK (owner_id = auth.uid() OR (workspace_id IS NOT NULL AND public.is_workspace_member(workspace_id)));
CREATE POLICY jobs_update ON public.generation_jobs FOR UPDATE
  USING (owner_id = auth.uid() OR (workspace_id IS NOT NULL AND public.is_workspace_member(workspace_id)))
  WITH CHECK (owner_id = auth.uid() OR (workspace_id IS NOT NULL AND public.is_workspace_member(workspace_id)));

-- Provider capabilities: read for authenticated; write restricted to owner of probe rows (service role for workers)
CREATE POLICY provider_caps_select ON public.provider_capabilities FOR SELECT TO authenticated USING (true);

-- Storage bucket (private). Apply via dashboard or storage API; do not edit storage.objects schema manually beyond policies.
INSERT INTO storage.buckets (id, name, public)
VALUES ('visual-library', 'visual-library', false)
ON CONFLICT (id) DO NOTHING;

-- Storage path convention: visual-library/themes/{slug}/concepts/{concept_id}/assets/{asset_id}/original.webp
CREATE POLICY visual_library_storage_select ON storage.objects FOR SELECT TO authenticated
  USING (bucket_id = 'visual-library' AND auth.uid() IS NOT NULL);
CREATE POLICY visual_library_storage_insert ON storage.objects FOR INSERT TO authenticated
  WITH CHECK (bucket_id = 'visual-library' AND auth.uid() IS NOT NULL);
CREATE POLICY visual_library_storage_update ON storage.objects FOR UPDATE TO authenticated
  USING (bucket_id = 'visual-library' AND auth.uid() IS NOT NULL)
  WITH CHECK (bucket_id = 'visual-library' AND auth.uid() IS NOT NULL);
CREATE POLICY visual_library_storage_delete ON storage.objects FOR DELETE TO authenticated
  USING (bucket_id = 'visual-library' AND auth.uid() IS NOT NULL);

-- NOTE: Tighten storage policies further with folder ownership when auth product lands.
-- Prefer signed URLs from server for production downloads.
