//! Immutable render plans — insert only.

use rusqlite::{params, OptionalExtension};

use crate::error::{AppError, AppResult};
use crate::vnext::domain::VerticalRenderPlanV1;

use super::db::open_vnext_db;

pub fn insert_render_plan(plan: &VerticalRenderPlanV1) -> AppResult<()> {
    plan.validate()
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    let conn = open_vnext_db()?;
    let json = serde_json::to_string(plan).map_err(|e| AppError::Message(e.to_string()))?;
    conn.execute(
        r#"INSERT INTO render_plans(
            id, content_project_id, recipe_id, candidate_id, plan_json, created_at, created_by
        ) VALUES (?1,?2,?3,?4,?5,?6,?7)"#,
        params![
            plan.id,
            plan.content_project_id,
            plan.recipe_id,
            plan.candidate_id,
            json,
            plan.created_at,
            plan.created_by,
        ],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(())
}

pub fn get_render_plan(id: &str) -> AppResult<Option<VerticalRenderPlanV1>> {
    let conn = open_vnext_db()?;
    let json: Option<String> = conn
        .query_row(
            "SELECT plan_json FROM render_plans WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| AppError::Message(e.to_string()))?;
    match json {
        None => Ok(None),
        Some(j) => {
            let plan: VerticalRenderPlanV1 =
                serde_json::from_str(&j).map_err(|e| AppError::Message(e.to_string()))?;
            Ok(Some(plan))
        }
    }
}
