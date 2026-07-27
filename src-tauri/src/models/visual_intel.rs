//! Intelligent visual library domain — concepts, needs, generation, QA.
//! Separated from placement/composition (`visual.rs`) so assets stay global.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Enums ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QaStatus {
    #[default]
    None,
    Pending,
    AutomatedReview,
    NeedsHumanReview,
    Approved,
    Rejected,
}

impl QaStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Pending => "pending",
            Self::AutomatedReview => "automated_review",
            Self::NeedsHumanReview => "needs_human_review",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "pending" => Self::Pending,
            "automated_review" => Self::AutomatedReview,
            "needs_human_review" => Self::NeedsHumanReview,
            "approved" => Self::Approved,
            "rejected" => Self::Rejected,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConceptStatus {
    #[default]
    Draft,
    Active,
    Archived,
    Priority,
}

impl ConceptStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Archived => "archived",
            Self::Priority => "priority",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "archived" => Self::Archived,
            "priority" => Self::Priority,
            _ => Self::Draft,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NeedCoverage {
    #[default]
    Uncovered,
    Matched,
    Generating,
    NeedsReview,
    Covered,
    Skipped,
    Failed,
}

impl NeedCoverage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Uncovered => "uncovered",
            Self::Matched => "matched",
            Self::Generating => "generating",
            Self::NeedsReview => "needs_review",
            Self::Covered => "covered",
            Self::Skipped => "skipped",
            Self::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "matched" => Self::Matched,
            "generating" => Self::Generating,
            "needs_review" => Self::NeedsReview,
            "covered" => Self::Covered,
            "skipped" => Self::Skipped,
            "failed" => Self::Failed,
            _ => Self::Uncovered,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    #[default]
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    BlockedPolicy,
}

impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::BlockedPolicy => "blocked_policy",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "running" => Self::Running,
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            "blocked_policy" => Self::BlockedPolicy,
            _ => Self::Queued,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CandidateStatus {
    #[default]
    Generated,
    AutomatedReview,
    NeedsHumanReview,
    Approved,
    Rejected,
    Discarded,
}

impl CandidateStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Generated => "generated",
            Self::AutomatedReview => "automated_review",
            Self::NeedsHumanReview => "needs_human_review",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Discarded => "discarded",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "automated_review" => Self::AutomatedReview,
            "needs_human_review" => Self::NeedsHumanReview,
            "approved" => Self::Approved,
            "rejected" => Self::Rejected,
            "discarded" => Self::Discarded,
            _ => Self::Generated,
        }
    }
}

// ── Provenance / rights (embedded on assets) ───────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AssetProvenance {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub negative_prompt: Option<String>,
    #[serde(default)]
    pub seed: Option<i64>,
    #[serde(default)]
    pub generated_at: Option<String>,
}

// ── Theme / Concept ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub id: String,
    pub slug: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualConcept {
    pub id: String,
    /// Stable key for dedupe: slugified title + theme
    pub canonical_key: String,
    pub theme_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub literal_description: Vec<String>,
    #[serde(default)]
    pub meanings: Vec<String>,
    #[serde(default)]
    pub positive_contexts: Vec<String>,
    #[serde(default)]
    pub negative_contexts: Vec<String>,
    #[serde(default)]
    pub hard_exclusions: Vec<String>,
    #[serde(default)]
    pub desired_formats: Vec<String>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub request_count: u32,
    #[serde(default)]
    pub coverage_count: u32,
    pub status: ConceptStatus,
    pub created_at: String,
    pub updated_at: String,
}

impl VisualConcept {
    pub fn new(title: impl Into<String>, theme_id: Option<String>) -> Self {
        let title = title.into();
        let key = canonical_key(&title, theme_id.as_deref());
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            canonical_key: key,
            theme_id,
            title,
            literal_description: Vec::new(),
            meanings: Vec::new(),
            positive_contexts: Vec::new(),
            negative_contexts: Vec::new(),
            hard_exclusions: Vec::new(),
            desired_formats: vec!["16:9".into()],
            priority: 50,
            request_count: 0,
            coverage_count: 0,
            status: ConceptStatus::Active,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

pub fn canonical_key(title: &str, theme_id: Option<&str>) -> String {
    let t = title
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>();
    let t = t.trim_matches('_').to_string();
    match theme_id {
        Some(th) => format!("{th}::{t}"),
        None => t,
    }
}

// ── Visual need (project/scene scoped) ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualNeed {
    pub id: String,
    /// Analysis / visual run fingerprint
    pub project_key: String,
    #[serde(default)]
    pub media_path: Option<String>,
    #[serde(default)]
    pub semantic_event_id: Option<String>,
    #[serde(default)]
    pub concept_id: Option<String>,
    pub label: String,
    #[serde(default)]
    pub terms: Vec<String>,
    #[serde(default)]
    pub required_contexts: Vec<String>,
    #[serde(default)]
    pub forbidden_contexts: Vec<String>,
    #[serde(default)]
    pub hard_exclusions: Vec<String>,
    #[serde(default = "default_aspect")]
    pub desired_aspect: String,
    #[serde(default)]
    pub approx_duration_secs: f64,
    #[serde(default)]
    pub source_start: Option<f64>,
    #[serde(default)]
    pub source_end: Option<f64>,
    #[serde(default)]
    pub output_start: Option<f64>,
    #[serde(default)]
    pub output_end: Option<f64>,
    #[serde(default)]
    pub priority: i32,
    pub coverage: NeedCoverage,
    #[serde(default)]
    pub matched_asset_id: Option<String>,
    #[serde(default)]
    pub match_score: Option<f64>,
    #[serde(default)]
    pub match_reasons: Vec<String>,
    #[serde(default)]
    pub generation_job_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn default_aspect() -> String {
    "16:9".into()
}

impl VisualNeed {
    pub fn from_label(project_key: &str, label: impl Into<String>) -> Self {
        let label = label.into();
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            project_key: project_key.into(),
            media_path: None,
            semantic_event_id: None,
            concept_id: None,
            label: label.clone(),
            terms: vec![label],
            required_contexts: Vec::new(),
            forbidden_contexts: Vec::new(),
            hard_exclusions: Vec::new(),
            desired_aspect: "16:9".into(),
            approx_duration_secs: 5.0,
            source_start: None,
            source_end: None,
            output_start: None,
            output_end: None,
            priority: 50,
            coverage: NeedCoverage::Uncovered,
            matched_asset_id: None,
            match_score: None,
            match_reasons: Vec::new(),
            generation_job_id: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

// ── Generation job / candidate ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationJob {
    pub id: String,
    /// Idempotency: same key must not create duplicate work
    pub idempotency_key: String,
    #[serde(default)]
    pub need_id: Option<String>,
    #[serde(default)]
    pub concept_id: Option<String>,
    pub status: JobStatus,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub negative_prompt: String,
    #[serde(default)]
    pub attempt: u32,
    #[serde(default)]
    pub max_attempts: u32,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub is_paid: bool,
    #[serde(default)]
    pub opportunistic: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedCandidate {
    pub id: String,
    pub job_id: String,
    #[serde(default)]
    pub need_id: Option<String>,
    #[serde(default)]
    pub local_path: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub perceptual_hash: Option<String>,
    pub status: CandidateStatus,
    #[serde(default)]
    pub technical_score: Option<f64>,
    #[serde(default)]
    pub semantic_score: Option<f64>,
    #[serde(default)]
    pub qa_decision: Option<String>,
    #[serde(default)]
    pub qa_reason: Option<String>,
    #[serde(default)]
    pub approved_asset_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QaCheckResult {
    pub id: String,
    #[serde(default)]
    pub candidate_id: Option<String>,
    #[serde(default)]
    pub asset_id: Option<String>,
    pub technical_quality: f64,
    pub semantic_alignment: f64,
    #[serde(default)]
    pub forbidden_detected: Vec<String>,
    #[serde(default)]
    pub text_detected: bool,
    #[serde(default)]
    pub watermark_detected: bool,
    /// approve | needs_human | reject
    pub decision: String,
    pub reason: String,
    #[serde(default)]
    pub details: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCapability {
    pub id: String,
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub supports_image: bool,
    #[serde(default)]
    pub free_tier: bool,
    #[serde(default)]
    pub last_probe_ok: bool,
    #[serde(default)]
    pub last_probe_at: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub latency_ms: Option<u64>,
    #[serde(default)]
    pub notes: Option<String>,
}

// ── Cost policy ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostPolicy {
    pub paid_providers_enabled: bool,
    pub daily_paid_budget: f64,
    pub max_daily_generations: u32,
    pub max_generations_per_project: u32,
    pub max_attempts_per_need: u32,
    /// When false, opportunistic free-tier fill is off
    pub opportunistic_enabled: bool,
}

impl Default for CostPolicy {
    fn default() -> Self {
        Self {
            paid_providers_enabled: false,
            daily_paid_budget: 0.0,
            max_daily_generations: std::env::var("VIGILCUT_MAX_DAILY_GENERATIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(20),
            max_generations_per_project: 10,
            max_attempts_per_need: 2,
            opportunistic_enabled: std::env::var("VIGILCUT_OPPORTUNISTIC")
                .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
        }
    }
}

impl CostPolicy {
    pub fn from_env() -> Self {
        let mut p = Self::default();
        if let Ok(v) = std::env::var("VIGILCUT_PAID_PROVIDERS") {
            p.paid_providers_enabled = v == "1" || v.eq_ignore_ascii_case("true");
        }
        if let Ok(v) = std::env::var("VIGILCUT_DAILY_PAID_BUDGET") {
            if let Ok(n) = v.parse() {
                p.daily_paid_budget = n;
            }
        }
        p
    }
}

// ── Coverage summary (UX) ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CoverageSummary {
    pub total: u32,
    pub reused: u32,
    pub generated: u32,
    pub waiting: u32,
    pub needs_review: u32,
    pub uncovered: u32,
    pub failed: u32,
    pub skipped: u32,
}

impl CoverageSummary {
    pub fn from_needs(needs: &[VisualNeed]) -> Self {
        let mut s = Self::default();
        s.total = needs.len() as u32;
        for n in needs {
            match n.coverage {
                NeedCoverage::Covered | NeedCoverage::Matched => {
                    // Matched without job = reused; Covered after gen still counted reused-or-gen
                    if n.generation_job_id.is_some() {
                        s.generated += 1;
                    } else {
                        s.reused += 1;
                    }
                }
                NeedCoverage::Generating => s.waiting += 1,
                NeedCoverage::NeedsReview => s.needs_review += 1,
                NeedCoverage::Uncovered => s.uncovered += 1,
                NeedCoverage::Failed => s.failed += 1,
                NeedCoverage::Skipped => s.skipped += 1,
            }
        }
        s
    }
}

// ── Match explanation ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchCandidate {
    pub asset_id: String,
    pub asset_title: String,
    pub score: f64,
    pub reasons: Vec<String>,
    pub exclusions_checked: Vec<String>,
    pub format_ok: bool,
    pub will_crop: bool,
    pub times_used: u32,
    pub thumbnail_path: Option<String>,
}

// ── Story-builder readiness (not full product) ──────────────────────────

/// Future text-to-video scene requirement shares the same need shape.
pub type SceneRequirement = VisualNeed;
