//! Visual concepts & themes — reusable meanings, not tied to a single video.

use rusqlite::{params, Connection};

use crate::error::{AppError, AppResult};
use crate::models::visual_intel::{canonical_key, ConceptStatus, Theme, VisualConcept};
use crate::pipeline::visual::library::open_db;

fn json_vec(v: &[String]) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "[]".into())
}
fn parse_vec(s: &str) -> Vec<String> {
    serde_json::from_str(s).unwrap_or_default()
}

pub fn upsert_theme(slug: &str, title: &str, description: Option<&str>) -> AppResult<Theme> {
    let conn = open_db()?;
    if let Ok(t) = conn.query_row(
        "SELECT id, slug, title, description, created_at, updated_at FROM themes WHERE slug = ?1",
        params![slug],
        |r| {
            Ok(Theme {
                id: r.get(0)?,
                slug: r.get(1)?,
                title: r.get(2)?,
                description: r.get(3)?,
                created_at: r.get(4)?,
                updated_at: r.get(5)?,
            })
        },
    ) {
        return Ok(t);
    }
    let now = chrono::Utc::now().to_rfc3339();
    let t = Theme {
        id: uuid::Uuid::new_v4().to_string(),
        slug: slug.into(),
        title: title.into(),
        description: description.map(|s| s.into()),
        created_at: now.clone(),
        updated_at: now,
    };
    conn.execute(
        "INSERT INTO themes(id,slug,title,description,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6)",
        params![t.id, t.slug, t.title, t.description, t.created_at, t.updated_at],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(t)
}

pub fn insert_concept(mut c: VisualConcept) -> AppResult<VisualConcept> {
    let conn = open_db()?;
    c.canonical_key = canonical_key(&c.title, c.theme_id.as_deref());
    // Dedupe by canonical key
    if let Ok(existing) = get_concept_by_key(&conn, &c.canonical_key) {
        return Ok(existing);
    }
    c.updated_at = chrono::Utc::now().to_rfc3339();
    conn.execute(
        r#"INSERT INTO visual_concepts (
            id, canonical_key, theme_id, title, literal_description, meanings,
            positive_contexts, negative_contexts, hard_exclusions, desired_formats,
            priority, request_count, coverage_count, status, created_at, updated_at
        ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)"#,
        params![
            c.id,
            c.canonical_key,
            c.theme_id,
            c.title,
            json_vec(&c.literal_description),
            json_vec(&c.meanings),
            json_vec(&c.positive_contexts),
            json_vec(&c.negative_contexts),
            json_vec(&c.hard_exclusions),
            json_vec(&c.desired_formats),
            c.priority,
            c.request_count as i64,
            c.coverage_count as i64,
            c.status.as_str(),
            c.created_at,
            c.updated_at,
        ],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(c)
}

fn get_concept_by_key(conn: &Connection, key: &str) -> AppResult<VisualConcept> {
    conn.query_row(
        "SELECT id, canonical_key, theme_id, title, literal_description, meanings,
         positive_contexts, negative_contexts, hard_exclusions, desired_formats,
         priority, request_count, coverage_count, status, created_at, updated_at
         FROM visual_concepts WHERE canonical_key = ?1",
        params![key],
        row_concept,
    )
    .map_err(|e| AppError::NotFound(e.to_string()))
}

pub fn get_concept(id: &str) -> AppResult<VisualConcept> {
    let conn = open_db()?;
    conn.query_row(
        "SELECT id, canonical_key, theme_id, title, literal_description, meanings,
         positive_contexts, negative_contexts, hard_exclusions, desired_formats,
         priority, request_count, coverage_count, status, created_at, updated_at
         FROM visual_concepts WHERE id = ?1",
        params![id],
        row_concept,
    )
    .map_err(|e| AppError::NotFound(e.to_string()))
}

fn row_concept(r: &rusqlite::Row<'_>) -> rusqlite::Result<VisualConcept> {
    let st: String = r.get(13)?;
    Ok(VisualConcept {
        id: r.get(0)?,
        canonical_key: r.get(1)?,
        theme_id: r.get(2)?,
        title: r.get(3)?,
        literal_description: parse_vec(&r.get::<_, String>(4)?),
        meanings: parse_vec(&r.get::<_, String>(5)?),
        positive_contexts: parse_vec(&r.get::<_, String>(6)?),
        negative_contexts: parse_vec(&r.get::<_, String>(7)?),
        hard_exclusions: parse_vec(&r.get::<_, String>(8)?),
        desired_formats: parse_vec(&r.get::<_, String>(9)?),
        priority: r.get(10)?,
        request_count: r.get::<_, i64>(11)? as u32,
        coverage_count: r.get::<_, i64>(12)? as u32,
        status: ConceptStatus::parse(&st),
        created_at: r.get(14)?,
        updated_at: r.get(15)?,
    })
}

pub fn list_concepts(theme_id: Option<&str>, limit: usize) -> AppResult<Vec<VisualConcept>> {
    let conn = open_db()?;
    let limit = limit.clamp(1, 500) as i64;
    let mut out = Vec::new();
    if let Some(tid) = theme_id {
        let mut stmt = conn
            .prepare(
                "SELECT id, canonical_key, theme_id, title, literal_description, meanings,
                 positive_contexts, negative_contexts, hard_exclusions, desired_formats,
                 priority, request_count, coverage_count, status, created_at, updated_at
                 FROM visual_concepts WHERE theme_id = ?1 ORDER BY priority DESC, title LIMIT ?2",
            )
            .map_err(|e| AppError::Message(e.to_string()))?;
        for row in stmt
            .query_map(params![tid, limit], row_concept)
            .map_err(|e| AppError::Message(e.to_string()))?
            .flatten()
        {
            out.push(row);
        }
    } else {
        let mut stmt = conn
            .prepare(
                "SELECT id, canonical_key, theme_id, title, literal_description, meanings,
                 positive_contexts, negative_contexts, hard_exclusions, desired_formats,
                 priority, request_count, coverage_count, status, created_at, updated_at
                 FROM visual_concepts ORDER BY priority DESC, title LIMIT ?1",
            )
            .map_err(|e| AppError::Message(e.to_string()))?;
        for row in stmt
            .query_map(params![limit], row_concept)
            .map_err(|e| AppError::Message(e.to_string()))?
            .flatten()
        {
            out.push(row);
        }
    }
    Ok(out)
}

pub fn link_asset_concept(asset_id: &str, concept_id: &str, weight: f64) -> AppResult<()> {
    let conn = open_db()?;
    conn.execute(
        "INSERT OR REPLACE INTO asset_concepts(asset_id, concept_id, weight) VALUES(?1,?2,?3)",
        params![asset_id, concept_id, weight],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    conn.execute(
        "UPDATE visual_concepts SET coverage_count = (
            SELECT COUNT(*) FROM asset_concepts WHERE concept_id = ?1
         ), updated_at = ?2 WHERE id = ?1",
        params![concept_id, chrono::Utc::now().to_rfc3339()],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(())
}

/// Seed a curated economy theme (no image generation).
pub fn seed_economy_theme() -> AppResult<Vec<VisualConcept>> {
    let theme = upsert_theme(
        "economia-dinero-negocios",
        "Economía, dinero y negocios",
        Some("Conceptos reutilizables para noticias y finanzas personales"),
    )?;
    // (title, literal, meanings, positive_contexts, hard_exclusions)
    #[allow(clippy::type_complexity)]
    let seeds: &[(&str, &[&str], &[&str], &[&str], &[&str])] = &[
        (
            "Persona comparando precios en supermercado",
            &["persona", "supermercado", "etiquetas de precios"],
            &["inflación", "costo de vida"],
            &["economía doméstica", "presupuesto familiar"],
            &["criptomonedas", "lujo", "marcas comerciales"],
        ),
        (
            "Familia preparando presupuesto mensual",
            &["familia", "mesa", "calculadora", "papel"],
            &["planificación", "ahorro"],
            &["economía doméstica"],
            &["lujo", "criptomonedas"],
        ),
        (
            "Comerciante revisando ventas",
            &["comercio", "caja", "tablet"],
            &["ventas", "PYME"],
            &["negocios", "retail"],
            &["bolsa de valores", "futurista"],
        ),
        (
            "Persona organizando sus ahorros",
            &["persona", "monedas", "alcancía"],
            &["ahorro", "disciplina financiera"],
            &["finanzas personales"],
            &["criptomonedas", "billetes flotando"],
        ),
        (
            "Cliente pagando con teléfono",
            &["pago móvil", "smartphone", "TPV"],
            &["pagos digitales"],
            &["comercio", "tecnología cotidiana"],
            &["hacking", "criptomonedas"],
        ),
    ];
    let mut out = Vec::new();
    for (title, lit, mean, pos, hard) in seeds {
        let mut c = VisualConcept::new(*title, Some(theme.id.clone()));
        c.literal_description = lit.iter().map(|s| (*s).into()).collect();
        c.meanings = mean.iter().map(|s| (*s).into()).collect();
        c.positive_contexts = pos.iter().map(|s| (*s).into()).collect();
        c.hard_exclusions = hard.iter().map(|s| (*s).into()).collect();
        c.negative_contexts = vec!["lujo ostentoso".into(), "especulación cripto".into()];
        c.status = ConceptStatus::Priority;
        c.priority = 80;
        out.push(insert_concept(c)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::visual::library::set_library_root_override;

    #[test]
    fn seed_and_dedupe() {
        let _lock = crate::pipeline::visual::library::lock_library_for_test();
        let dir = std::env::temp_dir().join(format!("vc-c-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        set_library_root_override(Some(dir.clone()));
        let a = seed_economy_theme().unwrap();
        let b = seed_economy_theme().unwrap();
        assert_eq!(a.len(), b.len());
        assert_eq!(a[0].id, b[0].id);
        set_library_root_override(None);
        let _ = std::fs::remove_dir_all(dir);
    }
}
