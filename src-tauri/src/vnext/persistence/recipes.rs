use rusqlite::{params, OptionalExtension};

use crate::error::{AppError, AppResult};
use crate::vnext::domain::ProductionRecipeRecord;

use super::db::open_vnext_db;

/// Insert-only: recipes are immutable after creation.
pub fn insert_recipe(r: &ProductionRecipeRecord) -> AppResult<()> {
    let conn = open_vnext_db()?;
    conn.execute(
        r#"INSERT INTO production_recipes(
            id, content_project_id, label, recipe_json, supersedes_recipe_id, created_at
        ) VALUES (?1,?2,?3,?4,?5,?6)"#,
        params![
            r.id,
            r.content_project_id,
            r.label,
            r.recipe_json,
            r.supersedes_recipe_id,
            r.created_at,
        ],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(())
}

pub fn get_recipe(id: &str) -> AppResult<Option<ProductionRecipeRecord>> {
    let conn = open_vnext_db()?;
    conn.query_row(
        r#"SELECT id, content_project_id, label, recipe_json, supersedes_recipe_id, created_at
           FROM production_recipes WHERE id = ?1"#,
        params![id],
        |r| {
            Ok(ProductionRecipeRecord {
                id: r.get(0)?,
                content_project_id: r.get(1)?,
                label: r.get(2)?,
                recipe_json: r.get(3)?,
                supersedes_recipe_id: r.get(4)?,
                created_at: r.get(5)?,
            })
        },
    )
    .optional()
    .map_err(|e| AppError::Message(e.to_string()))
}
