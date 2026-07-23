//! Persistent, video-independent instructions for filling the visual library.

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::models::visual_intel::{ConceptStatus, CostPolicy, VisualConcept, VisualNeed};
use crate::pipeline::visual::concepts::insert_concept;
use crate::pipeline::visual::generation::worker::queue_generation_with_key;
use crate::pipeline::visual::library::open_db;
use crate::pipeline::visual::library_dashboard::dashboard;
use crate::pipeline::visual::needs::save_needs;
use crate::visual_library::{AssetQuery, LibraryService, VisualLibrary};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLibraryRequest {
    #[serde(default = "default_origin")]
    pub origin: String,
    #[serde(default)]
    pub theme: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub negative_prompt: String,
    #[serde(default = "default_target")]
    pub target_count: u32,
    #[serde(default = "default_format")]
    pub desired_format: String,
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
    #[serde(default = "default_style")]
    pub style: String,
    #[serde(default)]
    pub positive_contexts: Vec<String>,
    #[serde(default)]
    pub negative_contexts: Vec<String>,
    #[serde(default)]
    pub hard_exclusions: Vec<String>,
    #[serde(default = "default_priority")]
    pub priority: i32,
}

fn default_origin() -> String {
    "manual".into()
}
fn default_style() -> String {
    "photorealistic".into()
}
fn default_width() -> u32 {
    1280
}
fn default_height() -> u32 {
    720
}
fn default_target() -> u32 {
    3
}
fn default_format() -> String {
    "16:9".into()
}
fn default_priority() -> i32 {
    50
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRequest {
    pub id: String,
    pub concept_id: String,
    pub origin: String,
    pub theme: String,
    pub title: String,
    pub description: String,
    pub prompt: String,
    pub negative_prompt: String,
    pub target_count: u32,
    pub desired_format: String,
    pub width: u32,
    pub height: u32,
    pub style: String,
    pub positive_contexts: Vec<String>,
    pub negative_contexts: Vec<String>,
    pub hard_exclusions: Vec<String>,
    pub priority: i32,
    pub status: String,
    pub searched_at: Option<String>,
    pub useful_assets: u32,
    pub queued: u32,
    pub running: u32,
    pub awaiting_review: u32,
    pub failed: u32,
    pub deficit: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestAssetMatch {
    pub asset_id: String,
    pub title: String,
    pub thumbnail_path: Option<String>,
    pub score: f64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRequestPreview {
    pub request: LibraryRequest,
    pub matches: Vec<RequestAssetMatch>,
    pub can_confirm: bool,
    pub max_enqueueable: u32,
    pub blocked_reason: Option<String>,
    pub provider: String,
}
fn json_vec(v: &[String]) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "[]".into())
}

fn parse_vec(s: String) -> Vec<String> {
    serde_json::from_str(&s).unwrap_or_default()
}

fn validate(input: &mut CreateLibraryRequest) -> AppResult<()> {
    input.origin = input.origin.trim().to_lowercase();
    if !matches!(
        input.origin.as_str(),
        "manual" | "daily_feed" | "broll_missing" | "visual_video_missing" | "import"
    ) {
        return Err(AppError::Invalid("Origen de solicitud no válido.".into()));
    }
    input.theme = input.theme.trim().to_string();
    input.title = input.title.trim().to_string();
    input.description = input.description.trim().to_string();
    input.prompt = input.prompt.trim().to_string();
    input.negative_prompt = input.negative_prompt.trim().to_string();
    input.style = input.style.trim().to_lowercase();
    if input.title.len() < 2 || input.title.len() > 180 {
        return Err(AppError::Invalid(
            "El concepto debe tener entre 2 y 180 caracteres.".into(),
        ));
    }
    if input.description.len() < 3 || input.description.len() > 1200 {
        return Err(AppError::Invalid(
            "La descripción debe tener entre 3 y 1200 caracteres.".into(),
        ));
    }
    if !matches!(
        input.style.as_str(),
        "photorealistic" | "illustration" | "infographic" | "cinematic" | "other"
    ) {
        return Err(AppError::Invalid("Estilo de imagen no válido.".into()));
    }
    input.target_count = 1;
    input.priority = input.priority.clamp(0, 100);
    input.desired_format = input.desired_format.trim().to_string();
    if !matches!(
        input.desired_format.as_str(),
        "16:9" | "9:16" | "1:1" | "4:5"
    ) {
        return Err(AppError::Invalid(
            "El formato debe ser 16:9, 9:16, 1:1 o 4:5.".into(),
        ));
    }
    for values in [
        &mut input.positive_contexts,
        &mut input.negative_contexts,
        &mut input.hard_exclusions,
    ] {
        values.iter_mut().for_each(|v| *v = v.trim().to_string());
        values.retain(|v| !v.is_empty());
        values.truncate(20);
    }
    (input.width, input.height) = dimensions_for(&input.desired_format);
    if input.prompt.is_empty() {
        input.prompt = build_request_prompt(input);
    }
    Ok(())
}

pub fn dimensions_for(format: &str) -> (u32, u32) {
    match format {
        "9:16" => (720, 1280),
        "1:1" => (1024, 1024),
        "4:5" => (1024, 1280),
        _ => (1280, 720),
    }
}

pub fn build_request_prompt(input: &CreateLibraryRequest) -> String {
    let style = match input.style.as_str() {
        "illustration" => "Editorial illustration",
        "infographic" => "Clean infographic without text",
        "cinematic" => "Cinematic still",
        "other" => "High quality image",
        _ => "Photorealistic editorial photograph",
    };
    let mut parts = vec![
        style.to_string(),
        input.description.clone(),
        format!("Concept: {}", input.title),
        format!("Aspect ratio: {}", input.desired_format),
    ];
    if !input.theme.is_empty() {
        parts.push(format!("Theme: {}", input.theme));
    }
    parts.join(". ") + "."
}

fn hydrate(conn: &rusqlite::Connection, id: &str) -> AppResult<LibraryRequest> {
    let mut request = conn
        .query_row(
            "SELECT id, concept_id, origin, theme, title, description, prompt, negative_prompt,
                    target_count, desired_format, width, height, style, positive_contexts,
                    negative_contexts, hard_exclusions, priority, status, searched_at,
                    created_at, updated_at
             FROM library_requests WHERE id=?1",
            params![id],
            |r| {
                Ok(LibraryRequest {
                    id: r.get(0)?,
                    concept_id: r.get(1)?,
                    origin: r.get(2)?,
                    theme: r.get(3)?,
                    title: r.get(4)?,
                    description: r.get(5)?,
                    prompt: r.get(6)?,
                    negative_prompt: r.get(7)?,
                    target_count: r.get::<_, i64>(8)?.max(0) as u32,
                    desired_format: r.get(9)?,
                    width: r.get::<_, i64>(10)?.max(0) as u32,
                    height: r.get::<_, i64>(11)?.max(0) as u32,
                    style: r.get(12)?,
                    positive_contexts: parse_vec(r.get(13)?),
                    negative_contexts: parse_vec(r.get(14)?),
                    hard_exclusions: parse_vec(r.get(15)?),
                    priority: r.get(16)?,
                    status: r.get(17)?,
                    searched_at: r.get(18)?,
                    useful_assets: 0,
                    queued: 0,
                    running: 0,
                    awaiting_review: 0,
                    failed: 0,
                    deficit: 0,
                    created_at: r.get(19)?,
                    updated_at: r.get(20)?,
                })
            },
        )
        .map_err(|_| AppError::NotFound(format!("Solicitud de biblioteca no encontrada: {id}")))?;

    request.useful_assets = conn
        .query_row(
            "SELECT COUNT(DISTINCT a.id)
             FROM asset_concepts ac JOIN media_assets a ON a.id=ac.asset_id
             WHERE ac.concept_id=?1 AND a.status='active'
               AND COALESCE(a.qa_status,'none') != 'rejected'",
            params![request.concept_id],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        .max(0) as u32;

    let prefix = format!("library_request:{}", request.id);
    let jobs = conn
        .query_row(
            "SELECT
                SUM(CASE WHEN j.status='queued' THEN 1 ELSE 0 END),
                SUM(CASE WHEN j.status='running' THEN 1 ELSE 0 END),
                SUM(CASE WHEN j.status='failed' THEN 1 ELSE 0 END)
             FROM generation_jobs j JOIN visual_needs n ON n.id=j.need_id
             WHERE n.project_key=?1",
            params![prefix],
            |r| {
                Ok((
                    r.get::<_, Option<i64>>(0)?.unwrap_or(0),
                    r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                    r.get::<_, Option<i64>>(2)?.unwrap_or(0),
                ))
            },
        )
        .unwrap_or((0, 0, 0));
    request.queued = jobs.0.max(0) as u32;
    request.running = jobs.1.max(0) as u32;
    request.failed = jobs.2.max(0) as u32;
    request.awaiting_review = conn
        .query_row(
            "SELECT COUNT(DISTINCT c.id)
             FROM generated_candidates c
             JOIN generation_jobs j ON j.id=c.job_id
             JOIN visual_needs n ON n.id=j.need_id
             WHERE n.project_key=?1
               AND c.status IN ('generated','automated_review','needs_human_review')",
            params![prefix],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        .max(0) as u32;
    let latest_candidate_status: Option<String> = conn
        .query_row(
            "SELECT c.status FROM generated_candidates c JOIN visual_needs n ON n.id=c.need_id WHERE n.project_key=?1 ORDER BY c.created_at DESC LIMIT 1",
            params![prefix],
            |row| row.get(0),
        )
        .optional()
        .unwrap_or(None);
    request.deficit = request.target_count.saturating_sub(request.useful_assets);
    request.status = if request.awaiting_review > 0 {
        "pending_review".into()
    } else if request.running > 0 {
        "generating".into()
    } else if request.queued > 0 {
        "queued".into()
    } else if request.useful_assets > 0 {
        "approved".into()
    } else if request.failed > 0 {
        "failed".into()
    } else if latest_candidate_status.as_deref() == Some("rejected") {
        "rejected".into()
    } else {
        request.status
    };
    Ok(request)
}

pub fn create_request(mut input: CreateLibraryRequest) -> AppResult<LibraryRequestPreview> {
    validate(&mut input)?;
    let mut concept = VisualConcept::new(&input.title, None);
    concept.literal_description = vec![input.title.clone()];
    concept.positive_contexts = input.positive_contexts.clone();
    concept.negative_contexts = input.negative_contexts.clone();
    concept.hard_exclusions = input.hard_exclusions.clone();
    concept.desired_formats = vec![input.desired_format.clone()];
    concept.priority = input.priority;
    concept.status = ConceptStatus::Active;
    let concept = insert_concept(concept)?;

    let conn = open_db()?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO library_requests(
            id, concept_id, origin, theme, title, description, prompt, negative_prompt,
            target_count, desired_format, width, height, style, positive_contexts,
            negative_contexts, hard_exclusions, priority, status, searched_at, created_at, updated_at
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,'searched',?18,?18,?18)",
        params![
            id, concept.id, input.origin, input.theme, input.title, input.description,
            input.prompt, input.negative_prompt, input.target_count as i64, input.desired_format,
            input.width as i64, input.height as i64, input.style,
            json_vec(&input.positive_contexts), json_vec(&input.negative_contexts),
            json_vec(&input.hard_exclusions), input.priority, now,
        ],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    preview(&id)
}

pub fn preview(id: &str) -> AppResult<LibraryRequestPreview> {
    let conn = open_db()?;
    let request = hydrate(&conn, id)?;
    let snapshot = dashboard()?;
    let max_enqueueable = request
        .target_count
        .min(snapshot.limits.local_remaining_today)
        .min(CostPolicy::from_env().max_generations_per_project);
    let query = AssetQuery {
        terms: vec![
            request.title.clone(),
            request.description.clone(),
            request.theme.clone(),
            request.style.clone(),
        ],
        required_contexts: request.positive_contexts.clone(),
        forbidden_contexts: request.negative_contexts.clone(),
        hard_exclusions: request.hard_exclusions.clone(),
        desired_aspect: Some(request.desired_format.clone()),
        min_score: Some(0.05),
        allow_unknown_license: true,
        ..Default::default()
    };
    let matches = LibraryService::new()
        .search(&query)?
        .into_iter()
        .take(6)
        .map(|item| {
            let thumbnail_path = LibraryService::new()
                .get_asset(&item.asset_id)
                .ok()
                .and_then(|a| a.thumbnail_path);
            RequestAssetMatch {
                asset_id: item.asset_id,
                title: item.title,
                thumbnail_path,
                score: item.score,
                reasons: item.reasons,
            }
        })
        .collect();
    Ok(LibraryRequestPreview {
        matches,
        can_confirm: snapshot.can_work && max_enqueueable > 0,
        max_enqueueable,
        blocked_reason: snapshot.blocked_reason,
        provider: snapshot.provider.name,
        request,
    })
}

pub fn confirm(id: &str) -> AppResult<LibraryRequestPreview> {
    let current = preview(id)?;
    if !current.can_confirm {
        return Err(AppError::Invalid(
            current
                .blocked_reason
                .unwrap_or_else(|| "La solicitud no puede ejecutarse.".into()),
        ));
    }

    let project_key = format!("library_request:{id}");
    let conn = open_db()?;
    let existing_slots: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM visual_needs WHERE project_key=?1",
            params![project_key],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        .max(0) as u32;
    drop(conn);

    let to_create = current
        .max_enqueueable
        .min(current.request.target_count.saturating_sub(existing_slots));
    for slot in existing_slots..existing_slots + to_create {
        let mut need = VisualNeed::from_label(&project_key, &current.request.title);
        need.concept_id = Some(current.request.concept_id.clone());
        need.terms = vec![
            current.request.title.clone(),
            current.request.description.clone(),
            current.request.theme.clone(),
        ];
        need.required_contexts = current.request.positive_contexts.clone();
        need.required_contexts
            .push(current.request.description.clone());
        need.required_contexts
            .push(format!("style:{}", current.request.style));
        need.forbidden_contexts = current.request.negative_contexts.clone();
        need.hard_exclusions = current.request.hard_exclusions.clone();
        need.desired_aspect = current.request.desired_format.clone();
        need.priority = current.request.priority;
        save_needs(std::slice::from_ref(&need))?;
        let key = format!("library:{id}:slot:{slot}:v1");
        if let Some(job_id) = queue_generation_with_key(&mut need, false, &key, "library_request")?
        {
            let conn = open_db()?;
            conn.execute(
                "UPDATE generation_jobs SET prompt=?1, negative_prompt=?2, updated_at=?3 WHERE id=?4",
                params![current.request.prompt, current.request.negative_prompt, chrono::Utc::now().to_rfc3339(), job_id],
            ).map_err(|e| AppError::Message(e.to_string()))?;
        }
    }

    let conn = open_db()?;
    conn.execute(
        "UPDATE library_requests SET status='queued', updated_at=?2 WHERE id=?1",
        params![id, chrono::Utc::now().to_rfc3339()],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    preview(id)
}

pub fn use_existing(id: &str, asset_id: &str) -> AppResult<LibraryRequest> {
    let asset = LibraryService::new().get_asset(asset_id)?;
    if !matches!(asset.status, crate::models::visual::AssetStatus::Active) {
        return Err(AppError::Invalid(
            "La imagen seleccionada no está activa.".into(),
        ));
    }
    let conn = open_db()?;
    let changed = conn.execute(
        "UPDATE library_requests SET selected_asset_id=?1, status='approved', updated_at=?2 WHERE id=?3",
        params![asset_id, chrono::Utc::now().to_rfc3339(), id],
    ).map_err(|e| AppError::Message(e.to_string()))?;
    if changed == 0 {
        return Err(AppError::NotFound(format!(
            "Solicitud de biblioteca no encontrada: {id}"
        )));
    }
    hydrate(&conn, id)
}
pub fn regenerate(
    id: &str,
    prompt: String,
    negative_prompt: String,
) -> AppResult<LibraryRequestPreview> {
    let current = preview(id)?;
    let prompt = prompt.trim().to_string();
    if prompt.len() < 3 || prompt.len() > 4000 {
        return Err(AppError::Invalid(
            "El prompt debe tener entre 3 y 4000 caracteres.".into(),
        ));
    }
    let negative_prompt = negative_prompt
        .trim()
        .chars()
        .take(2000)
        .collect::<String>();
    let project_key = format!("library_request:{id}");
    let conn = open_db()?;
    let version: i64 = conn.query_row(
        "SELECT COUNT(*) + 1 FROM generation_jobs WHERE need_id IN (SELECT id FROM visual_needs WHERE project_key=?1)",
        params![project_key], |r| r.get(0),
    ).unwrap_or(1);
    drop(conn);
    let mut need = VisualNeed::from_label(&project_key, &current.request.title);
    need.concept_id = Some(current.request.concept_id.clone());
    need.terms = vec![
        current.request.title.clone(),
        current.request.description.clone(),
        current.request.theme.clone(),
    ];
    need.required_contexts = vec![
        current.request.description.clone(),
        format!("style:{}", current.request.style),
    ];
    need.hard_exclusions = current.request.hard_exclusions.clone();
    need.desired_aspect = current.request.desired_format.clone();
    save_needs(std::slice::from_ref(&need))?;
    let key = format!("library:{id}:regenerate:v{version}");
    let job_id = queue_generation_with_key(&mut need, false, &key, "library_request")?
        .ok_or_else(|| AppError::Invalid("La política de coste bloqueó la regeneración.".into()))?;
    let conn = open_db()?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE generation_jobs SET prompt=?1, negative_prompt=?2, updated_at=?3 WHERE id=?4",
        params![prompt, negative_prompt, now, job_id],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    conn.execute(
        "UPDATE library_requests SET prompt=?1, negative_prompt=?2, status='queued', updated_at=?3 WHERE id=?4",
        params![prompt, negative_prompt, now, id],
    ).map_err(|e| AppError::Message(e.to_string()))?;
    preview(id)
}
pub fn list_requests(limit: usize) -> AppResult<Vec<LibraryRequest>> {
    let conn = open_db()?;
    let mut stmt = conn
        .prepare("SELECT id FROM library_requests ORDER BY created_at DESC LIMIT ?1")
        .map_err(|e| AppError::Message(e.to_string()))?;
    let ids = stmt
        .query_map(params![limit.clamp(1, 100) as i64], |r| {
            r.get::<_, String>(0)
        })
        .map_err(|e| AppError::Message(e.to_string()))?
        .flatten()
        .collect::<Vec<_>>();
    ids.iter().map(|id| hydrate(&conn, id)).collect()
}

pub fn cancel(id: &str) -> AppResult<LibraryRequest> {
    let conn = open_db()?;
    let changed = conn
        .execute(
            "UPDATE generation_jobs SET status='cancelled', stage='cancelled',
                    cancel_requested=1, updated_at=?2
             WHERE need_id IN (SELECT id FROM visual_needs WHERE project_key=?1)
               AND status='queued'",
            params![
                format!("library_request:{id}"),
                chrono::Utc::now().to_rfc3339()
            ],
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    conn.execute(
        "UPDATE generation_jobs SET cancel_requested=1, stage='cancelling', updated_at=?2
         WHERE need_id IN (SELECT id FROM visual_needs WHERE project_key=?1)
           AND status='running'",
        params![
            format!("library_request:{id}"),
            chrono::Utc::now().to_rfc3339()
        ],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    let exists: Option<String> = conn
        .query_row(
            "SELECT id FROM library_requests WHERE id=?1",
            params![id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| AppError::Message(e.to_string()))?;
    if exists.is_none() {
        return Err(AppError::NotFound(id.into()));
    }
    conn.execute(
        "UPDATE library_requests SET status=?2, updated_at=?3 WHERE id=?1",
        params![
            id,
            if changed > 0 {
                "cancelled"
            } else {
                "cancelling"
            },
            chrono::Utc::now().to_rfc3339()
        ],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    hydrate(&conn, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::visual::generation::supervision::{
        get_candidate, list_pending_candidates,
    };
    use crate::pipeline::visual::generation::worker::{
        human_approve_candidate, human_reject_candidate, process_next_job,
    };
    use crate::pipeline::visual::library::{
        list_assets, lock_library_for_test, set_library_root_override,
    };

    fn isolated() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("vc-request-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        set_library_root_override(Some(dir.clone()));
        std::env::set_var("VIGILCUT_IMAGE_PROVIDER", "mock");
        dir
    }

    fn manual_input(title: &str) -> CreateLibraryRequest {
        CreateLibraryRequest {
            origin: "manual".into(),
            theme: "Economía".into(),
            title: title.into(),
            description: "Familia comparando precios en un supermercado".into(),
            prompt: String::new(),
            negative_prompt: "texto, logos, marcas, billetes estadounidenses, manos deformes"
                .into(),
            target_count: 1,
            desired_format: "16:9".into(),
            width: 0,
            height: 0,
            style: "photorealistic".into(),
            positive_contexts: vec![],
            negative_contexts: vec![],
            hard_exclusions: vec!["logos".into()],
            priority: 70,
        }
    }

    // Intentional: serializes the process-wide test library override.
    #[allow(clippy::await_holding_lock)]
    #[tokio::test(flavor = "current_thread")]
    async fn complete_manual_flow_without_video_persists_and_requires_approval() {
        let _lock = lock_library_for_test();
        let dir = isolated();
        let existing_path = dir.join("existing.png");
        image::RgbImage::from_pixel(320, 180, image::Rgb([30, 80, 120]))
            .save(&existing_path)
            .unwrap();
        crate::pipeline::visual::library::import_image(
            &existing_path,
            Some("Inflación en supermercado".into()),
            vec!["familia".into(), "precios".into()],
            vec!["inflación".into()],
            crate::models::visual::LicenseStatus::Unknown,
        )
        .unwrap();

        let created = create_request(manual_input("Inflación")).unwrap();
        assert!(
            !created.matches.is_empty(),
            "search-before-generate must return the reusable asset"
        );
        let reused = use_existing(&created.request.id, &created.matches[0].asset_id).unwrap();
        assert_eq!(reused.status, "approved");
        let created = create_request(manual_input("Inflación variante")).unwrap();
        assert_eq!(created.request.origin, "manual");
        assert_eq!(created.request.status, "searched");
        assert!(created.request.searched_at.is_some());
        assert_eq!((created.request.width, created.request.height), (1280, 720));
        assert!(created.request.prompt.contains("Inflación"));
        assert!(created.request.prompt.contains("Familia"));
        assert_eq!(list_requests(10).unwrap().len(), 2);

        let confirmed = confirm(&created.request.id).unwrap();
        assert_eq!(confirmed.request.queued, 1);
        let request_after_reopen = list_requests(10).unwrap().remove(0);
        assert_eq!(request_after_reopen.id, created.request.id);
        assert_eq!(
            request_after_reopen.negative_prompt,
            created.request.negative_prompt
        );

        process_next_job()
            .await
            .unwrap()
            .expect("mock job processed");
        let pending = list_pending_candidates(10).unwrap();
        assert_eq!(pending.len(), 1);
        let candidate = &pending[0];
        assert_eq!(candidate.origin, "library_request");
        assert_eq!(candidate.provider.as_deref(), Some("mock"));
        assert_eq!((candidate.width, candidate.height), (Some(1280), Some(720)));
        assert_eq!(
            candidate.negative_prompt.as_deref(),
            Some(created.request.negative_prompt.as_str())
        );
        assert_eq!(
            list_assets(None, 10).unwrap().len(),
            1,
            "candidate must not auto-ingest"
        );

        let first = human_approve_candidate(&candidate.id).unwrap();
        let second = human_approve_candidate(&candidate.id).unwrap();
        assert_eq!(first.id, second.id);
        let assets = list_assets(None, 10).unwrap();
        assert_eq!(assets.len(), 2);
        let generated = assets.iter().find(|asset| asset.id == first.id).unwrap();
        assert_eq!(
            generated.license_status,
            crate::models::visual::LicenseStatus::Unknown
        );
        assert_eq!(generated.commercial_use, None);
        assert_eq!(
            generated
                .provenance
                .as_ref()
                .and_then(|p| p.provider.as_deref()),
            Some("mock")
        );
        assert_eq!(generated.category.as_deref(), Some("Economía"));

        set_library_root_override(None);
        std::env::remove_var("VIGILCUT_IMAGE_PROVIDER");
        let _ = std::fs::remove_dir_all(dir);
    }
    // Intentional: serializes the process-wide test library override.
    #[allow(clippy::await_holding_lock)]
    #[tokio::test(flavor = "current_thread")]
    async fn rejection_creates_no_asset_and_regeneration_keeps_traceability() {
        let _lock = lock_library_for_test();
        let dir = isolated();
        let created = create_request(manual_input("Canasta familiar")).unwrap();
        confirm(&created.request.id).unwrap();
        process_next_job().await.unwrap();
        let first = list_pending_candidates(10).unwrap().remove(0);
        human_reject_candidate(&first.id).unwrap();
        assert!(list_assets(None, 10).unwrap().is_empty());
        regenerate(
            &created.request.id,
            "Nueva variante documental".into(),
            "texto, logos".into(),
        )
        .unwrap();
        process_next_job().await.unwrap();
        let pending = list_pending_candidates(10).unwrap();
        assert_eq!(pending.len(), 1);
        assert_ne!(pending[0].id, first.id);
        assert_eq!(get_candidate(&first.id).unwrap().status, "rejected");
        assert_eq!(
            pending[0].prompt.as_deref(),
            Some("Nueva variante documental")
        );

        set_library_root_override(None);
        std::env::remove_var("VIGILCUT_IMAGE_PROVIDER");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn invalid_format_is_rejected_before_persistence() {
        let _lock = lock_library_for_test();
        let dir = isolated();
        let mut input = manual_input("Una imagen válida");
        input.desired_format = "panorámico".into();
        assert!(create_request(input).is_err());
        assert!(list_requests(10).unwrap().is_empty());
        set_library_root_override(None);
        std::env::remove_var("VIGILCUT_IMAGE_PROVIDER");
        let _ = std::fs::remove_dir_all(dir);
    }
}
