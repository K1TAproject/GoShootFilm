use crate::digital;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Semaphore;

pub const THUMBNAIL_MAX_EDGE: u32 = 384;
pub const DISPLAY_MAX_EDGE: u32 = 2400;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDescriptor {
    pub id: String,
    pub entity_type: String,
    pub entity_sync_id: String,
    pub version: String,
    pub asset_type: String,
    pub sha256: Option<String>,
    pub byte_size: Option<i64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub available: bool,
}

pub struct AssetRow {
    pub id: String,
    pub entity_type: String,
    pub entity_sync_id: String,
    pub asset_type: String,
    pub relative_path: String,
    pub source_path: String,
    pub sha256: Option<String>,
    pub byte_size: Option<i64>,
}

pub struct GeneratedAsset {
    pub path: PathBuf,
    pub sha256: String,
    pub byte_size: i64,
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn random_id() -> String {
    let mut bytes = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut bytes);
    hex(&bytes)
}

fn safe_media_path(media_dir: &Path, relative: &str) -> Option<PathBuf> {
    let path = Path::new(relative);
    if path.is_absolute() {
        return None;
    }
    let parts = path
        .components()
        .map(|part| match part {
            Component::Normal(value) => Some(value),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    if parts.is_empty() {
        return None;
    }
    Some(media_dir.join(path))
}

fn source_metadata(source: &Path) -> (Option<i64>, Option<i64>) {
    let metadata = fs::metadata(source).ok();
    let size = metadata.as_ref().map(|value| value.len() as i64);
    let mtime = metadata
        .and_then(|value| value.modified().ok())
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_secs() as i64);
    (size, mtime)
}

#[allow(clippy::too_many_arguments)]
fn descriptor_from_row(
    id: String,
    entity_type: String,
    entity_sync_id: String,
    version: String,
    asset_type: String,
    sha256: Option<String>,
    byte_size: Option<i64>,
    width: Option<i64>,
    height: Option<i64>,
) -> AssetDescriptor {
    let available = sha256.is_some();
    AssetDescriptor {
        id,
        entity_type,
        entity_sync_id,
        version,
        asset_type,
        sha256,
        byte_size,
        width,
        height,
        available,
    }
}

type ExistingAssetRow = (
    String,
    Option<String>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    String,
    Option<i64>,
);

#[allow(clippy::too_many_arguments)]
async fn upsert_asset(
    db: &SqlitePool,
    entity_type: &str,
    entity_sync_id: &str,
    version: &str,
    asset_type: &str,
    source_path: &str,
    source_size: Option<i64>,
    source_mtime: Option<i64>,
) -> Result<AssetDescriptor, String> {
    let existing: Option<ExistingAssetRow> = sqlx::query_as(
        "SELECT id, sha256, byte_size, width, height, source_size, source_path, source_mtime
         FROM mobile_assets
         WHERE entity_type = ? AND entity_sync_id = ? AND version = ? AND asset_type = ?",
    )
    .bind(entity_type)
    .bind(entity_sync_id)
    .bind(version)
    .bind(asset_type)
    .fetch_optional(db)
    .await
    .map_err(|error| error.to_string())?;

    if let Some((id, sha256, byte_size, width, height, old_size, old_source, old_mtime)) = existing
    {
        let source_changed = old_source != source_path
            || old_size != source_size
            || (source_mtime.is_some() && old_mtime != source_mtime);
        if source_changed {
            sqlx::query(
                "UPDATE mobile_assets
                 SET source_path = ?, source_size = ?, source_mtime = ?,
                     sha256 = NULL, byte_size = NULL, width = NULL, height = NULL,
                     generated_at = NULL
                 WHERE id = ?",
            )
            .bind(source_path)
            .bind(source_size)
            .bind(source_mtime)
            .bind(&id)
            .execute(db)
            .await
            .map_err(|error| error.to_string())?;
            return Ok(descriptor_from_row(
                id,
                entity_type.into(),
                entity_sync_id.into(),
                version.into(),
                asset_type.into(),
                None,
                None,
                None,
                None,
            ));
        }
        return Ok(descriptor_from_row(
            id,
            entity_type.into(),
            entity_sync_id.into(),
            version.into(),
            asset_type.into(),
            sha256,
            byte_size,
            width,
            height,
        ));
    }

    let id = random_id();
    let folder = if asset_type == "thumbnail" {
        "thumbs"
    } else {
        "display"
    };
    let relative = format!("mobile/{folder}/{id}.jpg");
    sqlx::query(
        "INSERT INTO mobile_assets
         (id, entity_type, entity_sync_id, version, asset_type, relative_path,
          source_path, source_size, source_mtime, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(entity_type)
    .bind(entity_sync_id)
    .bind(version)
    .bind(asset_type)
    .bind(&relative)
    .bind(source_path)
    .bind(source_size)
    .bind(source_mtime)
    .bind(sqlite_now())
    .execute(db)
    .await
    .map_err(|error| error.to_string())?;
    Ok(descriptor_from_row(
        id,
        entity_type.into(),
        entity_sync_id.into(),
        version.into(),
        asset_type.into(),
        None,
        None,
        None,
        None,
    ))
}

fn sqlite_now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{seconds}")
}

/// Create stable asset rows for the versions that currently exist on a photo.
pub async fn ensure_descriptors(
    db: &SqlitePool,
    media_dir: &Path,
    entity_type: &str,
    entity_sync_id: &str,
    versions: &[(String, String)],
) -> Result<Vec<AssetDescriptor>, String> {
    let mut descriptors = Vec::new();
    for (version, source_path) in versions {
        let absolute = safe_media_path(media_dir, source_path)
            .ok_or_else(|| "invalid mobile source path".to_string())?;
        let (source_size, source_mtime) = source_metadata(&absolute);
        for asset_type in display_asset_types(version) {
            descriptors.push(
                upsert_asset(
                    db,
                    entity_type,
                    entity_sync_id,
                    version,
                    asset_type,
                    source_path,
                    source_size,
                    source_mtime,
                )
                .await?,
            );
        }
    }
    Ok(descriptors)
}

fn display_asset_types(version: &str) -> &'static [&'static str] {
    if version == "edit" {
        &["thumbnail", "edit_display"]
    } else {
        &["thumbnail", "original_preview"]
    }
}

async fn load_asset(db: &SqlitePool, asset_id: &str) -> Result<AssetRow, String> {
    let row = sqlx::query(
        "SELECT id, entity_type, entity_sync_id, asset_type, relative_path,
                source_path, sha256, byte_size
         FROM mobile_assets WHERE id = ?",
    )
    .bind(asset_id)
    .fetch_optional(db)
    .await
    .map_err(|error| error.to_string())?
    .ok_or_else(|| "asset not found".to_string())?;
    Ok(AssetRow {
        id: row.get("id"),
        entity_type: row.get("entity_type"),
        entity_sync_id: row.get("entity_sync_id"),
        asset_type: row.get("asset_type"),
        relative_path: row.get("relative_path"),
        source_path: row.get("source_path"),
        sha256: row.get("sha256"),
        byte_size: row.get("byte_size"),
    })
}

async fn entity_is_live(
    db: &SqlitePool,
    entity_type: &str,
    entity_sync_id: &str,
) -> Result<bool, String> {
    let deleted: Option<i64> = sqlx::query_scalar(
        "SELECT deleted FROM sync_entities WHERE entity_type = ? AND sync_id = ?",
    )
    .bind(entity_type)
    .bind(entity_sync_id)
    .fetch_optional(db)
    .await
    .map_err(|error| error.to_string())?;
    Ok(deleted == Some(0))
}

/// Generate the JPEG for an asset if needed. The caller must hold the per-asset
/// lock; this function also uses the shared semaphore to bound image decoding.
pub async fn generate_file(
    db: &SqlitePool,
    media_dir: &Path,
    preview_dir: &Path,
    asset_id: &str,
    semaphore: &Semaphore,
) -> Result<GeneratedAsset, String> {
    let asset = load_asset(db, asset_id).await?;
    if !entity_is_live(db, &asset.entity_type, &asset.entity_sync_id).await? {
        return Err("entity no longer exists".into());
    }

    let target = preview_dir.join(&asset.relative_path);
    if let (Some(hash), Some(size)) = (&asset.sha256, asset.byte_size) {
        if let Ok(metadata) = fs::metadata(&target) {
            if metadata.len() as i64 == size {
                if let Ok(existing) = fs::read(&target) {
                    if sha256_hex(&existing) == *hash {
                        return Ok(GeneratedAsset {
                            path: target.clone(),
                            sha256: hash.clone(),
                            byte_size: size,
                        });
                    }
                }
            }
        }
    }

    let source = safe_media_path(media_dir, &asset.source_path)
        .ok_or_else(|| "invalid mobile source path".to_string())?;
    if !source.is_file() {
        return Err("source file is missing".into());
    }

    let _permit = semaphore
        .acquire()
        .await
        .map_err(|_| "asset worker unavailable".to_string())?;
    let target_for_task = target.clone();
    let asset_type = asset.asset_type.clone();
    let source_for_task = source.clone();
    let (width, height) = tokio::task::spawn_blocking(move || {
        render_asset(&source_for_task, &target_for_task, &asset_type)
    })
    .await
    .map_err(|error| error.to_string())??;

    let bytes = fs::read(&target).map_err(|error| error.to_string())?;
    let hash = sha256_hex(&bytes);
    let byte_size = bytes.len() as i64;
    sqlx::query(
        "UPDATE mobile_assets
         SET sha256 = ?, byte_size = ?, width = ?, height = ?,
             generated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE id = ?",
    )
    .bind(&hash)
    .bind(byte_size)
    .bind(i64::from(width))
    .bind(i64::from(height))
    .bind(&asset.id)
    .execute(db)
    .await
    .map_err(|error| error.to_string())?;

    Ok(GeneratedAsset {
        path: target,
        sha256: hash,
        byte_size,
    })
}

fn render_asset(source: &Path, target: &Path, asset_type: &str) -> Result<(u32, u32), String> {
    let max_edge = if asset_type == "thumbnail" {
        THUMBNAIL_MAX_EDGE
    } else {
        DISPLAY_MAX_EDGE
    };
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let image = if extension == "cr2" {
        let bytes = fs::read(source).map_err(|_| "source file is missing".to_string())?;
        let jpeg =
            digital::cr2_embedded_jpeg(&bytes).map_err(|_| "CR2 preview failed".to_string())?;
        image::load_from_memory(jpeg).map_err(|_| "CR2 preview decode failed".to_string())?
    } else if matches!(extension.as_str(), "tif" | "tiff") {
        let tiff_preview = target.with_extension("tiff-preview.png");
        digital::build_tiff_preview(source, &tiff_preview)
            .map_err(|_| "TIFF preview failed".to_string())?;
        let image =
            image::open(&tiff_preview).map_err(|_| "TIFF preview decode failed".to_string())?;
        let _ = fs::remove_file(&tiff_preview);
        image
    } else {
        image::open(source).map_err(|_| "image decode failed".to_string())?
    };

    let image = if image.width() > max_edge || image.height() > max_edge {
        image.thumbnail(max_edge, max_edge)
    } else {
        image
    };
    let width = image.width();
    let height = image.height();
    let rgb = image.to_rgb8();

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temporary = target.with_extension(format!("tmp-{}.jpg", std::process::id()));
    {
        let file = fs::File::create(&temporary).map_err(|error| error.to_string())?;
        let mut writer = std::io::BufWriter::new(file);
        let quality = if asset_type == "thumbnail" { 82 } else { 88 };
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, quality);
        encoder
            .encode_image(&rgb)
            .map_err(|error| error.to_string())?;
        writer.flush().map_err(|error| error.to_string())?;
        writer
            .into_inner()
            .map_err(|error| error.to_string())?
            .sync_all()
            .map_err(|error| error.to_string())?;
    }
    let _ = fs::remove_file(target);
    fs::rename(&temporary, target).map_err(|error| error.to_string())?;
    Ok((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::time::{SystemTime, UNIX_EPOCH};

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create asset test database");
        sqlx::raw_sql(
            "CREATE TABLE sync_entities (
                entity_type TEXT NOT NULL,
                local_id INTEGER NOT NULL,
                sync_id TEXT NOT NULL UNIQUE,
                revision INTEGER NOT NULL DEFAULT 1,
                updated_at TEXT NOT NULL,
                deleted INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (entity_type, local_id)
            );
            CREATE TABLE mobile_assets (
                id TEXT PRIMARY KEY,
                entity_type TEXT NOT NULL,
                entity_sync_id TEXT NOT NULL,
                version TEXT NOT NULL,
                asset_type TEXT NOT NULL,
                relative_path TEXT NOT NULL UNIQUE,
                source_path TEXT NOT NULL,
                source_size INTEGER,
                source_mtime INTEGER,
                sha256 TEXT,
                byte_size INTEGER,
                width INTEGER,
                height INTEGER,
                generated_at TEXT,
                created_at TEXT NOT NULL,
                UNIQUE (entity_type, entity_sync_id, version, asset_type)
            );",
        )
        .execute(&pool)
        .await
        .expect("create asset tables");
        sqlx::query(
            "INSERT INTO sync_entities
             (entity_type, local_id, sync_id, revision, updated_at, deleted)
             VALUES ('film_photo', 1, 'photo-sync-1', 1, '2026-10-18T00:00:00Z', 0)",
        )
        .execute(&pool)
        .await
        .expect("insert sync entity");
        pool
    }

    #[tokio::test]
    async fn thumbnails_are_generated_outside_the_formal_gallery() {
        let root = std::env::temp_dir().join(format!(
            "goshootfilm-mobile-asset-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let media = root.join("media");
        let previews = root.join("previews");
        let source = media.join("rolls/1/edit/01.jpg");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        image::RgbImage::from_pixel(900, 600, image::Rgb([80, 100, 120]))
            .save_with_format(&source, image::ImageFormat::Jpeg)
            .unwrap();
        let source_bytes = fs::read(&source).unwrap();

        let pool = test_pool().await;
        let descriptors = ensure_descriptors(
            &pool,
            &media,
            "film_photo",
            "photo-sync-1",
            &[("edit".into(), "rolls/1/edit/01.jpg".into())],
        )
        .await
        .expect("create asset descriptors");
        assert_eq!(descriptors.len(), 2);
        let thumbnail = descriptors
            .iter()
            .find(|asset| asset.asset_type == "thumbnail")
            .expect("thumbnail descriptor");
        assert!(!thumbnail.available);

        let generated = generate_file(&pool, &media, &previews, &thumbnail.id, &Semaphore::new(1))
            .await
            .expect("generate thumbnail");
        assert!(generated
            .path
            .starts_with(previews.join("mobile").join("thumbs")));
        assert_eq!(generated.sha256.len(), 64);
        assert!(generated.byte_size > 0);
        let decoded = image::open(&generated.path).expect("open generated thumbnail");
        assert!(decoded.width() <= THUMBNAIL_MAX_EDGE);
        assert!(decoded.height() <= THUMBNAIL_MAX_EDGE);
        assert_eq!(fs::read(&source).unwrap(), source_bytes);

        let refreshed = ensure_descriptors(
            &pool,
            &media,
            "film_photo",
            "photo-sync-1",
            &[("edit".into(), "rolls/1/edit/01.jpg".into())],
        )
        .await
        .expect("refresh descriptors");
        let refreshed_thumbnail = refreshed
            .iter()
            .find(|asset| asset.asset_type == "thumbnail")
            .expect("thumbnail descriptor");
        assert!(refreshed_thumbnail.available);
        assert_eq!(
            refreshed_thumbnail.sha256.as_deref(),
            Some(generated.sha256.as_str())
        );

        pool.close().await;
        fs::remove_dir_all(root).expect("remove asset test directory");
    }
}
