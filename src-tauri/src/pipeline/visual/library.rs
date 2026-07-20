//! Local visual library: SQLite metadata + managed asset copies.

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};
use crate::models::visual::{AssetStatus, LicenseStatus, MediaAsset};
use crate::state::AppState;

pub fn library_root() -> AppResult<PathBuf> {
    let root = AppState::app_data_dir()?.join("library");
    std::fs::create_dir_all(root.join("assets"))?;
    std::fs::create_dir_all(root.join("thumbs"))?;
    Ok(root)
}

pub fn open_db() -> AppResult<Connection> {
    let db_path = library_root()?.join("library.db");
    let conn = Connection::open(db_path).map_err(|e| AppError::Message(e.to_string()))?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS media_assets (
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            managed_path TEXT NOT NULL,
            thumbnail_path TEXT,
            sha256 TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            description TEXT,
            tags TEXT NOT NULL DEFAULT '[]',
            concepts TEXT NOT NULL DEFAULT '[]',
            category TEXT,
            width INTEGER NOT NULL,
            height INTEGER NOT NULL,
            orientation TEXT NOT NULL,
            mime_type TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            license_status TEXT NOT NULL,
            source TEXT,
            attribution TEXT,
            times_used INTEGER NOT NULL DEFAULT 0,
            last_used_at TEXT,
            allow_same_video_repeat INTEGER NOT NULL DEFAULT 0,
            minimum_videos_before_reuse INTEGER NOT NULL DEFAULT 0,
            quality_score REAL,
            status TEXT NOT NULL,
            original_path TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS asset_usage (
            id TEXT PRIMARY KEY,
            asset_id TEXT NOT NULL,
            media_path TEXT NOT NULL,
            run_id TEXT,
            used_at TEXT NOT NULL,
            output_start REAL,
            output_end REAL
        );
        CREATE INDEX IF NOT EXISTS idx_assets_status ON media_assets(status);
        CREATE INDEX IF NOT EXISTS idx_usage_asset ON asset_usage(asset_id);
        "#,
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(conn)
}

fn sha256_file(path: &Path) -> AppResult<String> {
    let data = std::fs::read(path)?;
    let mut h = Sha256::new();
    h.update(&data);
    Ok(hex::encode(h.finalize()))
}

fn mime_of(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "image/jpeg",
    }
}

/// Import image: validate, hash, dedupe, copy managed, thumb, insert row.
pub fn import_image(
    source: &Path,
    title: Option<String>,
    tags: Vec<String>,
    concepts: Vec<String>,
    license: LicenseStatus,
) -> AppResult<MediaAsset> {
    if !source.is_file() {
        return Err(AppError::NotFound(source.display().to_string()));
    }
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    if !["jpg", "jpeg", "png", "webp"].contains(&ext.as_str()) {
        return Err(AppError::Invalid(format!(
            "Formato no soportado: {ext}. Usa jpg/png/webp."
        )));
    }

    let sha = sha256_file(source)?;
    let conn = open_db()?;
    if let Ok(existing) = conn.query_row(
        "SELECT id FROM media_assets WHERE sha256 = ?1",
        params![sha],
        |r| r.get::<_, String>(0),
    ) {
        return get_asset(&conn, &existing);
    }

    let img = image::open(source).map_err(|e| AppError::Invalid(format!("Imagen inválida: {e}")))?;
    let (w, h) = (img.width(), img.height());
    let orientation = if w >= h { "landscape" } else { "portrait" };
    let file_size = std::fs::metadata(source)?.len();
    let id = uuid::Uuid::new_v4().to_string();
    let root = library_root()?;
    let managed = root.join("assets").join(format!("{id}.{ext}"));
    // Atomic-ish: copy to temp then rename
    let tmp = root.join("assets").join(format!(".{id}.part"));
    std::fs::copy(source, &tmp)?;
    std::fs::rename(&tmp, &managed)?;

    let thumb_path = root.join("thumbs").join(format!("{id}.jpg"));
    let thumb = img.thumbnail(320, 320);
    thumb
        .to_rgb8()
        .save(&thumb_path)
        .map_err(|e| AppError::Message(format!("thumb: {e}")))?;

    let now = chrono::Utc::now().to_rfc3339();
    let title = title.unwrap_or_else(|| {
        source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("image")
            .to_string()
    });
    let asset = MediaAsset {
        id: id.clone(),
        kind: "image".into(),
        managed_path: managed.to_string_lossy().into_owned(),
        thumbnail_path: Some(thumb_path.to_string_lossy().into_owned()),
        sha256: sha,
        title,
        description: None,
        tags,
        concepts,
        category: None,
        width: w,
        height: h,
        orientation: orientation.into(),
        mime_type: mime_of(source).into(),
        file_size,
        license_status: license,
        source: Some(source.to_string_lossy().into_owned()),
        attribution: None,
        times_used: 0,
        last_used_at: None,
        allow_same_video_repeat: false,
        minimum_videos_before_reuse: 0,
        quality_score: Some(((w * h) as f64 / 1_000_000.0).min(1.0)),
        status: AssetStatus::Active,
        original_path: Some(source.to_string_lossy().into_owned()),
        created_at: now.clone(),
        updated_at: now,
    };

    insert_asset(&conn, &asset)?;
    Ok(asset)
}

fn insert_asset(conn: &Connection, a: &MediaAsset) -> AppResult<()> {
    conn.execute(
        r#"INSERT INTO media_assets (
            id, kind, managed_path, thumbnail_path, sha256, title, description, tags, concepts,
            category, width, height, orientation, mime_type, file_size, license_status, source,
            attribution, times_used, last_used_at, allow_same_video_repeat, minimum_videos_before_reuse,
            quality_score, status, original_path, created_at, updated_at
        ) VALUES (
            ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27
        )"#,
        params![
            a.id,
            a.kind,
            a.managed_path,
            a.thumbnail_path,
            a.sha256,
            a.title,
            a.description,
            serde_json::to_string(&a.tags).unwrap_or_else(|_| "[]".into()),
            serde_json::to_string(&a.concepts).unwrap_or_else(|_| "[]".into()),
            a.category,
            a.width,
            a.height,
            a.orientation,
            a.mime_type,
            a.file_size as i64,
            license_str(a.license_status),
            a.source,
            a.attribution,
            a.times_used as i64,
            a.last_used_at,
            a.allow_same_video_repeat as i64,
            a.minimum_videos_before_reuse as i64,
            a.quality_score,
            status_str(a.status),
            a.original_path,
            a.created_at,
            a.updated_at,
        ],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(())
}

fn row_to_asset(r: &rusqlite::Row<'_>) -> rusqlite::Result<MediaAsset> {
    let tags: String = r.get(7)?;
    let concepts: String = r.get(8)?;
    let lic: String = r.get(15)?;
    let st: String = r.get(23)?;
    Ok(MediaAsset {
        id: r.get(0)?,
        kind: r.get(1)?,
        managed_path: r.get(2)?,
        thumbnail_path: r.get(3)?,
        sha256: r.get(4)?,
        title: r.get(5)?,
        description: r.get(6)?,
        tags: serde_json::from_str(&tags).unwrap_or_default(),
        concepts: serde_json::from_str(&concepts).unwrap_or_default(),
        category: r.get(9)?,
        width: r.get::<_, i64>(10)? as u32,
        height: r.get::<_, i64>(11)? as u32,
        orientation: r.get(12)?,
        mime_type: r.get(13)?,
        file_size: r.get::<_, i64>(14)? as u64,
        license_status: parse_license(&lic),
        source: r.get(16)?,
        attribution: r.get(17)?,
        times_used: r.get::<_, i64>(18)? as u32,
        last_used_at: r.get(19)?,
        allow_same_video_repeat: r.get::<_, i64>(20)? != 0,
        minimum_videos_before_reuse: r.get::<_, i64>(21)? as u32,
        quality_score: r.get(22)?,
        status: parse_status(&st),
        original_path: r.get(24)?,
        created_at: r.get(25)?,
        updated_at: r.get(26)?,
    })
}

const SELECT_ALL: &str = r#"SELECT id, kind, managed_path, thumbnail_path, sha256, title, description,
    tags, concepts, category, width, height, orientation, mime_type, file_size, license_status,
    source, attribution, times_used, last_used_at, allow_same_video_repeat, minimum_videos_before_reuse,
    quality_score, status, original_path, created_at, updated_at FROM media_assets"#;

fn get_asset(conn: &Connection, id: &str) -> AppResult<MediaAsset> {
    conn.query_row(
        &format!("{SELECT_ALL} WHERE id = ?1"),
        params![id],
        row_to_asset,
    )
    .map_err(|e| AppError::NotFound(format!("asset {id}: {e}")))
}

pub fn list_assets(query: Option<&str>, limit: usize) -> AppResult<Vec<MediaAsset>> {
    let conn = open_db()?;
    let limit = limit.clamp(1, 500) as i64;
    let mut out = Vec::new();
    if let Some(q) = query.filter(|s| !s.is_empty()) {
        let like = format!("%{}%", q.to_lowercase());
        let mut stmt = conn
            .prepare(&format!(
                "{SELECT_ALL} WHERE lower(title) LIKE ?1 OR lower(tags) LIKE ?1 OR lower(concepts) LIKE ?1
                 ORDER BY updated_at DESC LIMIT ?2"
            ))
            .map_err(|e| AppError::Message(e.to_string()))?;
        let rows = stmt
            .query_map(params![like, limit], row_to_asset)
            .map_err(|e| AppError::Message(e.to_string()))?;
        for r in rows.flatten() {
            out.push(r);
        }
    } else {
        let mut stmt = conn
            .prepare(&format!(
                "{SELECT_ALL} ORDER BY updated_at DESC LIMIT ?1"
            ))
            .map_err(|e| AppError::Message(e.to_string()))?;
        let rows = stmt
            .query_map(params![limit], row_to_asset)
            .map_err(|e| AppError::Message(e.to_string()))?;
        for r in rows.flatten() {
            out.push(r);
        }
    }
    Ok(out)
}

pub fn update_asset_meta(
    id: &str,
    title: Option<String>,
    tags: Option<Vec<String>>,
    concepts: Option<Vec<String>>,
    license: Option<LicenseStatus>,
    status: Option<AssetStatus>,
) -> AppResult<MediaAsset> {
    let conn = open_db()?;
    let mut a = get_asset(&conn, id)?;
    if let Some(t) = title {
        a.title = t;
    }
    if let Some(t) = tags {
        a.tags = t;
    }
    if let Some(c) = concepts {
        a.concepts = c;
    }
    if let Some(l) = license {
        a.license_status = l;
    }
    if let Some(s) = status {
        a.status = s;
    }
    a.updated_at = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE media_assets SET title=?1, tags=?2, concepts=?3, license_status=?4, status=?5, updated_at=?6 WHERE id=?7",
        params![
            a.title,
            serde_json::to_string(&a.tags).unwrap(),
            serde_json::to_string(&a.concepts).unwrap(),
            license_str(a.license_status),
            status_str(a.status),
            a.updated_at,
            a.id,
        ],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(a)
}

fn license_str(l: LicenseStatus) -> &'static str {
    match l {
        LicenseStatus::Owned => "owned",
        LicenseStatus::Licensed => "licensed",
        LicenseStatus::PublicDomain => "public_domain",
        LicenseStatus::AttributionRequired => "attribution_required",
        LicenseStatus::Unknown => "unknown",
    }
}

fn status_str(s: AssetStatus) -> &'static str {
    match s {
        AssetStatus::Active => "active",
        AssetStatus::Archived => "archived",
        AssetStatus::Blocked => "blocked",
        AssetStatus::Missing => "missing",
        AssetStatus::Invalid => "invalid",
    }
}

fn parse_license(s: &str) -> LicenseStatus {
    match s {
        "owned" => LicenseStatus::Owned,
        "licensed" => LicenseStatus::Licensed,
        "public_domain" => LicenseStatus::PublicDomain,
        "attribution_required" => LicenseStatus::AttributionRequired,
        _ => LicenseStatus::Unknown,
    }
}

fn parse_status(s: &str) -> AssetStatus {
    match s {
        "active" => AssetStatus::Active,
        "archived" => AssetStatus::Archived,
        "blocked" => AssetStatus::Blocked,
        "missing" => AssetStatus::Missing,
        "invalid" => AssetStatus::Invalid,
        _ => AssetStatus::Active,
    }
}

pub fn record_usage(
    asset_id: &str,
    media_path: &str,
    run_id: Option<&str>,
    output_start: f64,
    output_end: f64,
) -> AppResult<()> {
    let conn = open_db()?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO asset_usage (id, asset_id, media_path, run_id, used_at, output_start, output_end) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            uuid::Uuid::new_v4().to_string(),
            asset_id,
            media_path,
            run_id,
            now,
            output_start,
            output_end,
        ],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    conn.execute(
        "UPDATE media_assets SET times_used = times_used + 1, last_used_at = ?1, updated_at = ?1 WHERE id = ?2",
        params![now, asset_id],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(())
}

pub fn list_active_assets() -> AppResult<Vec<MediaAsset>> {
    let all = list_assets(None, 500)?;
    Ok(all
        .into_iter()
        .filter(|a| matches!(a.status, AssetStatus::Active))
        .collect())
}
