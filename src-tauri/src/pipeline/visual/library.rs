//! Local visual library: SQLite metadata + managed asset copies.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};
use crate::models::visual::{AssetStatus, LicenseStatus, MediaAsset};
use crate::models::visual_intel::{AssetProvenance, QaStatus};
use crate::pipeline::visual::schema;
use crate::state::AppState;

/// Process-wide override for tests (preferred over env when set).
static LIBRARY_ROOT_OVERRIDE: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Serializes library tests that mutate the override.
#[cfg(test)]
static LIBRARY_TEST_LOCK: Mutex<()> = Mutex::new(());

/// Hold during tests that touch library root / SQLite (prevents parallel clobber).
#[cfg(test)]
pub fn lock_library_for_test() -> std::sync::MutexGuard<'static, ()> {
    LIBRARY_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Override library root (tests). Pass `None` to clear.
pub fn set_library_root_override(path: Option<PathBuf>) {
    if let Ok(mut g) = LIBRARY_ROOT_OVERRIDE.lock() {
        *g = path;
    }
}

pub fn library_root() -> AppResult<PathBuf> {
    let root = {
        if let Ok(g) = LIBRARY_ROOT_OVERRIDE.lock() {
            if let Some(p) = g.as_ref() {
                p.clone()
            } else if let Ok(p) = std::env::var("VIGILCUT_LIBRARY_ROOT") {
                PathBuf::from(p)
            } else {
                AppState::app_data_dir()?.join("library")
            }
        } else if let Ok(p) = std::env::var("VIGILCUT_LIBRARY_ROOT") {
            PathBuf::from(p)
        } else {
            AppState::app_data_dir()?.join("library")
        }
    };
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
    schema::migrate(&conn)?;
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
    let aspect = aspect_label(w, h);
    let phash = perceptual_hash_simple(&img);
    let asset = MediaAsset {
        id: id.clone(),
        kind: "image".into(),
        managed_path: managed.to_string_lossy().into_owned(),
        thumbnail_path: Some(thumb_path.to_string_lossy().into_owned()),
        sha256: sha,
        title,
        description: None,
        tags: tags.clone(),
        concepts: concepts.clone(),
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
        literal_description: tags,
        meanings: concepts.clone(),
        positive_contexts: concepts,
        negative_contexts: Vec::new(),
        hard_exclusions: Vec::new(),
        aspect_ratio: Some(aspect),
        safe_area: Some("center".into()),
        perceptual_hash: Some(phash),
        qa_status: QaStatus::Approved,
        technical_score: Some(((w * h) as f64 / 1_000_000.0).min(1.0)),
        semantic_score: None,
        provenance: Some(AssetProvenance {
            source: "import".into(),
            provider: None,
            model: None,
            prompt: None,
            negative_prompt: None,
            seed: None,
            generated_at: None,
        }),
        commercial_use: Some(matches!(
            license,
            LicenseStatus::Owned | LicenseStatus::Licensed | LicenseStatus::PublicDomain
        )),
    };

    insert_asset(&conn, &asset)?;
    Ok(asset)
}

pub fn aspect_label(w: u32, h: u32) -> String {
    if w == 0 || h == 0 {
        return "unknown".into();
    }
    let r = w as f64 / h as f64;
    if (r - 16.0 / 9.0).abs() < 0.08 {
        "16:9".into()
    } else if (r - 9.0 / 16.0).abs() < 0.08 {
        "9:16".into()
    } else if (r - 1.0).abs() < 0.08 {
        "1:1".into()
    } else if (r - 4.0 / 3.0).abs() < 0.08 {
        "4:3".into()
    } else if w >= h {
        "landscape".into()
    } else {
        "portrait".into()
    }
}

/// Simple average-hash (8x8) for near-duplicate detection — not cryptographic.
pub fn perceptual_hash_simple(img: &image::DynamicImage) -> String {
    let small = img.resize_exact(8, 8, image::imageops::FilterType::Triangle);
    let gray = small.to_luma8();
    let mut sum: u32 = 0;
    let mut vals = [0u8; 64];
    for (i, p) in gray.pixels().enumerate() {
        vals[i] = p[0];
        sum += p[0] as u32;
    }
    let avg = (sum / 64) as u8;
    let mut bits: u64 = 0;
    for (i, v) in vals.iter().enumerate() {
        if *v >= avg {
            bits |= 1u64 << i;
        }
    }
    format!("{bits:016x}")
}

pub fn hamming_hex(a: &str, b: &str) -> Option<u32> {
    let a = u64::from_str_radix(a, 16).ok()?;
    let b = u64::from_str_radix(b, 16).ok()?;
    Some((a ^ b).count_ones())
}

fn json_vec(v: &[String]) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "[]".into())
}

fn parse_json_vec(s: &str) -> Vec<String> {
    serde_json::from_str(s).unwrap_or_default()
}

fn insert_asset(conn: &Connection, a: &MediaAsset) -> AppResult<()> {
    let prov = a
        .provenance
        .as_ref()
        .and_then(|p| serde_json::to_string(p).ok());
    conn.execute(
        r#"INSERT INTO media_assets (
            id, kind, managed_path, thumbnail_path, sha256, title, description, tags, concepts,
            category, width, height, orientation, mime_type, file_size, license_status, source,
            attribution, times_used, last_used_at, allow_same_video_repeat, minimum_videos_before_reuse,
            quality_score, status, original_path, created_at, updated_at,
            meanings, positive_contexts, negative_contexts, hard_exclusions, aspect_ratio, safe_area,
            perceptual_hash, qa_status, technical_score, semantic_score, provenance_json, commercial_use,
            literal_description
        ) VALUES (
            ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,
            ?28,?29,?30,?31,?32,?33,?34,?35,?36,?37,?38,?39,?40
        )"#,
        params![
            a.id,
            a.kind,
            a.managed_path,
            a.thumbnail_path,
            a.sha256,
            a.title,
            a.description,
            json_vec(&a.tags),
            json_vec(&a.concepts),
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
            json_vec(&a.meanings),
            json_vec(&a.positive_contexts),
            json_vec(&a.negative_contexts),
            json_vec(&a.hard_exclusions),
            a.aspect_ratio,
            a.safe_area,
            a.perceptual_hash,
            a.qa_status.as_str(),
            a.technical_score,
            a.semantic_score,
            prov,
            a.commercial_use.map(|b| b as i64),
            json_vec(&a.literal_description),
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
    // Extended columns may be missing on very old DBs before migrate; use try_get with defaults.
    let meanings: String = r.get::<_, String>(27).unwrap_or_else(|_| "[]".into());
    let pos_ctx: String = r.get::<_, String>(28).unwrap_or_else(|_| "[]".into());
    let neg_ctx: String = r.get::<_, String>(29).unwrap_or_else(|_| "[]".into());
    let hard_ex: String = r.get::<_, String>(30).unwrap_or_else(|_| "[]".into());
    let aspect: Option<String> = r.get(31)?;
    let safe: Option<String> = r.get(32)?;
    let phash: Option<String> = r.get(33)?;
    let qa: String = r.get::<_, String>(34).unwrap_or_else(|_| "none".into());
    let tech: Option<f64> = r.get(35)?;
    let sem: Option<f64> = r.get(36)?;
    let prov_s: Option<String> = r.get(37)?;
    let commercial: Option<i64> = r.get(38)?;
    let literal: String = r.get::<_, String>(39).unwrap_or_else(|_| "[]".into());
    Ok(MediaAsset {
        id: r.get(0)?,
        kind: r.get(1)?,
        managed_path: r.get(2)?,
        thumbnail_path: r.get(3)?,
        sha256: r.get(4)?,
        title: r.get(5)?,
        description: r.get(6)?,
        tags: parse_json_vec(&tags),
        concepts: parse_json_vec(&concepts),
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
        literal_description: parse_json_vec(&literal),
        meanings: parse_json_vec(&meanings),
        positive_contexts: parse_json_vec(&pos_ctx),
        negative_contexts: parse_json_vec(&neg_ctx),
        hard_exclusions: parse_json_vec(&hard_ex),
        aspect_ratio: aspect,
        safe_area: safe,
        perceptual_hash: phash,
        qa_status: QaStatus::parse(&qa),
        technical_score: tech,
        semantic_score: sem,
        provenance: prov_s.and_then(|s| serde_json::from_str(&s).ok()),
        commercial_use: commercial.map(|c| c != 0),
    })
}

const SELECT_ALL: &str = r#"SELECT id, kind, managed_path, thumbnail_path, sha256, title, description,
    tags, concepts, category, width, height, orientation, mime_type, file_size, license_status,
    source, attribution, times_used, last_used_at, allow_same_video_repeat, minimum_videos_before_reuse,
    quality_score, status, original_path, created_at, updated_at,
    meanings, positive_contexts, negative_contexts, hard_exclusions, aspect_ratio, safe_area,
    perceptual_hash, qa_status, technical_score, semantic_score, provenance_json, commercial_use,
    literal_description
    FROM media_assets"#;

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

/// Import every jpg/png/webp under a folder (non-recursive by default).
pub fn import_folder(
    dir: &Path,
    tags: Vec<String>,
    concepts: Vec<String>,
    recursive: bool,
) -> AppResult<ImportFolderResult> {
    import_folder_tracked(dir, tags, concepts, recursive)
}

/// Import folder with explicit duplicate detection.
pub fn import_folder_tracked(
    dir: &Path,
    tags: Vec<String>,
    concepts: Vec<String>,
    recursive: bool,
) -> AppResult<ImportFolderResult> {
    if !dir.is_dir() {
        return Err(AppError::NotFound(dir.display().to_string()));
    }
    let mut result = ImportFolderResult::default();
    visit_images(dir, recursive, &mut |path| {
        result.scanned += 1;
        match import_image_detailed(path, None, tags.clone(), concepts.clone(), LicenseStatus::Owned)
        {
            Ok(ImportOutcome::New(a)) => {
                result.imported += 1;
                result.asset_ids.push(a.id);
            }
            Ok(ImportOutcome::Duplicate(a)) => {
                result.duplicates += 1;
                result.asset_ids.push(a.id);
            }
            Err(e) => {
                result.failed += 1;
                result.errors.push(format!("{}: {e}", path.display()));
            }
        }
    })?;
    Ok(result)
}

#[derive(Debug, Default, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFolderResult {
    pub scanned: u32,
    pub imported: u32,
    pub duplicates: u32,
    pub failed: u32,
    pub asset_ids: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug)]
pub enum ImportOutcome {
    New(MediaAsset),
    Duplicate(MediaAsset),
}

/// Same as import_image but reports whether the SHA already existed.
pub fn import_image_detailed(
    source: &Path,
    title: Option<String>,
    tags: Vec<String>,
    concepts: Vec<String>,
    license: LicenseStatus,
) -> AppResult<ImportOutcome> {
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
        return Ok(ImportOutcome::Duplicate(get_asset(&conn, &existing)?));
    }
    // Reuse main path (will not re-hash miss since we checked)
    drop(conn);
    let asset = import_image(source, title, tags, concepts, license)?;
    Ok(ImportOutcome::New(asset))
}

fn visit_images(dir: &Path, recursive: bool, f: &mut dyn FnMut(&Path)) -> AppResult<()> {
    let entries = std::fs::read_dir(dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if recursive {
                visit_images(&path, true, f)?;
            }
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        if ["jpg", "jpeg", "png", "webp"].contains(&ext.as_str()) {
            f(&path);
        }
    }
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetUsageRow {
    pub id: String,
    pub asset_id: String,
    pub media_path: String,
    pub run_id: Option<String>,
    pub used_at: String,
    pub output_start: Option<f64>,
    pub output_end: Option<f64>,
}

pub fn list_usage(asset_id: Option<&str>, limit: usize) -> AppResult<Vec<AssetUsageRow>> {
    let conn = open_db()?;
    let limit = limit.clamp(1, 500) as i64;
    let mut out = Vec::new();
    if let Some(id) = asset_id {
        let mut stmt = conn
            .prepare(
                "SELECT id, asset_id, media_path, run_id, used_at, output_start, output_end
                 FROM asset_usage WHERE asset_id = ?1 ORDER BY used_at DESC LIMIT ?2",
            )
            .map_err(|e| AppError::Message(e.to_string()))?;
        let rows = stmt
            .query_map(params![id, limit], |r| {
                Ok(AssetUsageRow {
                    id: r.get(0)?,
                    asset_id: r.get(1)?,
                    media_path: r.get(2)?,
                    run_id: r.get(3)?,
                    used_at: r.get(4)?,
                    output_start: r.get(5)?,
                    output_end: r.get(6)?,
                })
            })
            .map_err(|e| AppError::Message(e.to_string()))?;
        for row in rows.flatten() {
            out.push(row);
        }
    } else {
        let mut stmt = conn
            .prepare(
                "SELECT id, asset_id, media_path, run_id, used_at, output_start, output_end
                 FROM asset_usage ORDER BY used_at DESC LIMIT ?1",
            )
            .map_err(|e| AppError::Message(e.to_string()))?;
        let rows = stmt
            .query_map(params![limit], |r| {
                Ok(AssetUsageRow {
                    id: r.get(0)?,
                    asset_id: r.get(1)?,
                    media_path: r.get(2)?,
                    run_id: r.get(3)?,
                    used_at: r.get(4)?,
                    output_start: r.get(5)?,
                    output_end: r.get(6)?,
                })
            })
            .map_err(|e| AppError::Message(e.to_string()))?;
        for row in rows.flatten() {
            out.push(row);
        }
    }
    Ok(out)
}

/// Mark assets whose managed file is gone as `missing`. Does not delete rows.
pub fn scan_missing_assets() -> AppResult<u32> {
    let conn = open_db()?;
    let mut stmt = conn
        .prepare("SELECT id, managed_path, status FROM media_assets")
        .map_err(|e| AppError::Message(e.to_string()))?;
    let rows: Vec<(String, String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .map_err(|e| AppError::Message(e.to_string()))?
        .flatten()
        .collect();
    let now = chrono::Utc::now().to_rfc3339();
    let mut n = 0u32;
    for (id, path, st) in rows {
        if st == "archived" || st == "blocked" {
            continue;
        }
        if !Path::new(&path).is_file() {
            conn.execute(
                "UPDATE media_assets SET status = 'missing', updated_at = ?1 WHERE id = ?2",
                params![now, id],
            )
            .map_err(|e| AppError::Message(e.to_string()))?;
            n += 1;
        }
    }
    Ok(n)
}

pub fn get_asset_by_id(id: &str) -> AppResult<MediaAsset> {
    let conn = open_db()?;
    get_asset(&conn, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    fn with_temp_library<F: FnOnce()>(f: F) {
        let _serial = LIBRARY_TEST_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!("vc-lib-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        set_library_root_override(Some(dir.clone()));
        f();
        set_library_root_override(None);
        let _ = std::fs::remove_dir_all(dir);
    }

    fn write_png(path: &Path, color: [u8; 3]) {
        let img = RgbImage::from_fn(64, 48, |_, _| Rgb(color));
        img.save(path).unwrap();
    }

    #[test]
    fn import_dedupes_by_sha256() {
        with_temp_library(|| {
            let src_dir = library_root().unwrap().join("_src");
            std::fs::create_dir_all(&src_dir).unwrap();
            let p = src_dir.join("a.png");
            write_png(&p, [10, 20, 30]);
            let a1 = import_image_detailed(
                &p,
                Some("one".into()),
                vec!["tag1".into()],
                vec!["inflacion".into()],
                LicenseStatus::Owned,
            )
            .unwrap();
            assert!(matches!(a1, ImportOutcome::New(_)));
            let a2 = import_image_detailed(
                &p,
                Some("two".into()),
                vec![],
                vec![],
                LicenseStatus::Owned,
            )
            .unwrap();
            assert!(matches!(a2, ImportOutcome::Duplicate(_)));
            let list = list_assets(None, 50).unwrap();
            assert_eq!(list.len(), 1);
            // Original file still present
            assert!(p.is_file());
        });
    }

    #[test]
    fn folder_import_and_usage() {
        with_temp_library(|| {
            let src = library_root().unwrap().join("_batch");
            std::fs::create_dir_all(&src).unwrap();
            write_png(&src.join("x.png"), [1, 2, 3]);
            write_png(&src.join("y.png"), [4, 5, 6]);
            let r = import_folder_tracked(
                &src,
                vec!["eco".into()],
                vec!["economia".into()],
                false,
            )
            .unwrap();
            assert_eq!(r.scanned, 2);
            assert_eq!(r.imported, 2);
            assert_eq!(r.failed, 0);
            let id = &r.asset_ids[0];
            record_usage(id, "video.mp4", Some("run1"), 1.0, 5.0).unwrap();
            let usage = list_usage(Some(id), 10).unwrap();
            assert_eq!(usage.len(), 1);
            let a = get_asset_by_id(id).unwrap();
            assert_eq!(a.times_used, 1);
        });
    }

    #[test]
    fn scan_marks_missing() {
        with_temp_library(|| {
            let src = library_root().unwrap().join("z.png");
            write_png(&src, [9, 9, 9]);
            let a = import_image(&src, None, vec![], vec!["t".into()], LicenseStatus::Owned).unwrap();
            std::fs::remove_file(&a.managed_path).unwrap();
            let n = scan_missing_assets().unwrap();
            assert_eq!(n, 1);
            let a2 = get_asset_by_id(&a.id).unwrap();
            assert!(matches!(a2.status, AssetStatus::Missing));
        });
    }
}
