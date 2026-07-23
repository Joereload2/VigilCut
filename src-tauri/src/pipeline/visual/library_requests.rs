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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLibraryRequest {
    pub title: String,
    #[serde(default = "default_target")]
    pub target_count: u32,
    #[serde(default = "default_format")]
    pub desired_format: String,
    #[serde(default)]
    pub positive_contexts: Vec<String>,
    #[serde(default)]
    pub negative_contexts: Vec<String>,
    #[serde(default)]
    pub hard_exclusions: Vec<String>,
    #[serde(default = "default_priority")]
    pub priority: i32,
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
    pub title: String,
    pub target_count: u32,
    pub desired_format: String,
    pub positive_contexts: Vec<String>,
    pub negative_contexts: Vec<String>,
    pub hard_exclusions: Vec<String>,
    pub priority: i32,
    pub status: String,
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
pub struct LibraryRequestPreview {
    pub request: LibraryRequest,
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
    input.title = input.title.trim().to_string();
    if input.title.len() < 3 || input.title.len() > 180 {
        return Err(AppError::Invalid(
            "La instrucción debe tener entre 3 y 180 caracteres.".into(),
        ));
    }
    input.target_count = input.target_count.clamp(1, 10);
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
    Ok(())
}

fn hydrate(conn: &rusqlite::Connection, id: &str) -> AppResult<LibraryRequest> {
    let mut request = conn
        .query_row(
            "SELECT id, concept_id, title, target_count, desired_format,
                    positive_contexts, negative_contexts, hard_exclusions,
                    priority, status, created_at, updated_at
             FROM library_requests WHERE id=?1",
            params![id],
            |r| {
                Ok(LibraryRequest {
                    id: r.get(0)?,
                    concept_id: r.get(1)?,
                    title: r.get(2)?,
                    target_count: r.get::<_, i64>(3)?.max(0) as u32,
                    desired_format: r.get(4)?,
                    positive_contexts: parse_vec(r.get(5)?),
                    negative_contexts: parse_vec(r.get(6)?),
                    hard_exclusions: parse_vec(r.get(7)?),
                    priority: r.get(8)?,
                    status: r.get(9)?,
                    useful_assets: 0,
                    queued: 0,
                    running: 0,
                    awaiting_review: 0,
                    failed: 0,
                    deficit: 0,
                    created_at: r.get(10)?,
                    updated_at: r.get(11)?,
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
    request.deficit = request.target_count.saturating_sub(request.useful_assets);
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
            id, concept_id, title, target_count, desired_format,
            positive_contexts, negative_contexts, hard_exclusions,
            priority, status, created_at, updated_at
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,'draft',?10,?10)",
        params![
            id,
            concept.id,
            input.title,
            input.target_count as i64,
            input.desired_format,
            json_vec(&input.positive_contexts),
            json_vec(&input.negative_contexts),
            json_vec(&input.hard_exclusions),
            input.priority,
            now,
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
        .deficit
        .min(snapshot.limits.local_remaining_today)
        .min(CostPolicy::from_env().max_generations_per_project);
    Ok(LibraryRequestPreview {
        can_confirm: request.deficit == 0 || (snapshot.can_work && max_enqueueable > 0),
        max_enqueueable,
        blocked_reason: if request.deficit == 0 {
            Some("El concepto ya tiene la cobertura solicitada.".into())
        } else {
            snapshot.blocked_reason
        },
        provider: snapshot.provider.name,
        request,
    })
}

pub fn confirm(id: &str) -> AppResult<LibraryRequestPreview> {
    let current = preview(id)?;
    if current.request.deficit == 0 {
        return Ok(current);
    }
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
        need.required_contexts = current.request.positive_contexts.clone();
        need.forbidden_contexts = current.request.negative_contexts.clone();
        need.hard_exclusions = current.request.hard_exclusions.clone();
        need.desired_aspect = current.request.desired_format.clone();
        need.priority = current.request.priority;
        save_needs(std::slice::from_ref(&need))?;
        let key = format!("library:{id}:slot:{slot}:v1");
        queue_generation_with_key(&mut need, false, &key, "library_request")?;
    }

    let conn = open_db()?;
    conn.execute(
        "UPDATE library_requests SET status='active', updated_at=?2 WHERE id=?1",
        params![id, chrono::Utc::now().to_rfc3339()],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
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
    use crate::pipeline::visual::library::{lock_library_for_test, set_library_root_override};

    fn isolated() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("vc-request-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        set_library_root_override(Some(dir.clone()));
        std::env::set_var("VIGILCUT_IMAGE_PROVIDER", "mock");
        dir
    }

    #[test]
    fn request_is_persistent_and_confirm_is_idempotent() {
        let _lock = lock_library_for_test();
        let dir = isolated();
        let created = create_request(CreateLibraryRequest {
            title: "Personas usando transporte público".into(),
            target_count: 2,
            desired_format: "16:9".into(),
            positive_contexts: vec!["ciudad".into()],
            negative_contexts: vec![],
            hard_exclusions: vec!["logos".into()],
            priority: 70,
        })
        .unwrap();
        assert_eq!(created.request.deficit, 2);
        assert_eq!(confirm(&created.request.id).unwrap().request.queued, 2);
        assert_eq!(confirm(&created.request.id).unwrap().request.queued, 2);
        assert_eq!(list_requests(10).unwrap().len(), 1);
        set_library_root_override(None);
        std::env::remove_var("VIGILCUT_IMAGE_PROVIDER");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn invalid_format_is_rejected_before_persistence() {
        let _lock = lock_library_for_test();
        let dir = isolated();
        let result = create_request(CreateLibraryRequest {
            title: "Una imagen válida".into(),
            target_count: 1,
            desired_format: "panorámico".into(),
            positive_contexts: vec![],
            negative_contexts: vec![],
            hard_exclusions: vec![],
            priority: 50,
        });
        assert!(result.is_err());
        assert!(list_requests(10).unwrap().is_empty());
        set_library_root_override(None);
        std::env::remove_var("VIGILCUT_IMAGE_PROVIDER");
        let _ = std::fs::remove_dir_all(dir);
    }
}
