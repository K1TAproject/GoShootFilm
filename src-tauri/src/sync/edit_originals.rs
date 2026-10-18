use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditOriginalDescriptor {
    pub id: String,
    pub entity_type: String,
    pub entity_sync_id: String,
    pub mime_type: String,
    pub byte_size: Option<i64>,
    pub sha256: Option<String>,
    pub available: bool,
}

pub struct EditOriginalFile {
    pub path: PathBuf,
    pub mime_type: String,
    pub sha256: String,
    pub byte_size: i64,
}

type ExistingEditRow = (
    String,
    String,
    String,
    Option<i64>,
    Option<String>,
    Option<i64>,
);

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn random_id() -> String {
    let mut bytes = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut bytes);
    hex(&bytes)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|_| "edit original is unreadable".to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| "edit original read failed".to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

fn mime_for_path(path: &Path) -> Option<&'static str> {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

fn edit_path_parts(path: &Path) -> Option<Vec<&std::ffi::OsStr>> {
    if path.is_absolute() {
        return None;
    }
    path.components()
        .map(|component| match component {
            Component::Normal(value) => Some(value),
            _ => None,
        })
        .collect()
}

fn resolve_edit_path(
    media_dir: &Path,
    entity_type: &str,
    source_path: &str,
) -> Result<PathBuf, String> {
    let path = Path::new(source_path);
    let parts = edit_path_parts(path).ok_or_else(|| "invalid edit original path".to_string())?;
    if parts.len() < 4 || parts[2] != std::ffi::OsStr::new("edit") {
        return Err("edit original must stay in an edit directory".into());
    }
    let valid_root = (entity_type == "film_photo" && parts[0] == std::ffi::OsStr::new("rolls"))
        || (entity_type == "digital_photo" && parts[0] == std::ffi::OsStr::new("digital"));
    if !valid_root {
        return Err("edit original entity/path mismatch".into());
    }
    let numeric: i64 = parts[1]
        .to_str()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    if numeric <= 0 {
        return Err("edit original parent id is invalid".into());
    }
    if mime_for_path(path).is_none() {
        return Err("edit original format is not supported".into());
    }
    Ok(media_dir.join(path))
}

fn metadata(path: &Path) -> (Option<i64>, Option<i64>) {
    let metadata = fs::metadata(path).ok();
    let size = metadata.as_ref().map(|value| value.len() as i64);
    let mtime = metadata
        .and_then(|value| value.modified().ok())
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_secs() as i64);
    (size, mtime)
}

pub async fn ensure_descriptor(
    db: &SqlitePool,
    media_dir: &Path,
    entity_type: &str,
    entity_sync_id: &str,
    source_path: &str,
) -> Result<EditOriginalDescriptor, String> {
    let source = resolve_edit_path(media_dir, entity_type, source_path)?;
    let mime_type = mime_for_path(&source)
        .ok_or_else(|| "edit original format is not supported".to_string())?;
    let (size, mtime) = metadata(&source);
    let existing: Option<ExistingEditRow> = sqlx::query_as(
        "SELECT id, source_path, mime_type, byte_size, sha256, source_mtime
             FROM mobile_edit_originals
             WHERE entity_type = ? AND entity_sync_id = ?",
    )
    .bind(entity_type)
    .bind(entity_sync_id)
    .fetch_optional(db)
    .await
    .map_err(|error| error.to_string())?;

    if let Some((id, old_path, old_mime, old_size, old_hash, old_mtime)) = existing {
        let changed = old_path != source_path
            || old_mime != mime_type
            || old_size != size
            || (mtime.is_some() && old_mtime != mtime);
        if changed {
            sqlx::query(
                "UPDATE mobile_edit_originals
                 SET source_path = ?, mime_type = ?, byte_size = ?, source_mtime = ?,
                     sha256 = NULL, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?",
            )
            .bind(source_path)
            .bind(mime_type)
            .bind(size)
            .bind(mtime)
            .bind(&id)
            .execute(db)
            .await
            .map_err(|error| error.to_string())?;
            return Ok(EditOriginalDescriptor {
                id,
                entity_type: entity_type.into(),
                entity_sync_id: entity_sync_id.into(),
                mime_type: mime_type.into(),
                byte_size: size,
                sha256: None,
                available: size.is_some(),
            });
        }
        return Ok(EditOriginalDescriptor {
            id,
            entity_type: entity_type.into(),
            entity_sync_id: entity_sync_id.into(),
            mime_type: mime_type.into(),
            byte_size: size,
            sha256: old_hash,
            available: size.is_some(),
        });
    }

    let id = random_id();
    sqlx::query(
        "INSERT INTO mobile_edit_originals
         (id, entity_type, entity_sync_id, source_path, mime_type, byte_size, source_mtime,
          created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?,
                 strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
    )
    .bind(&id)
    .bind(entity_type)
    .bind(entity_sync_id)
    .bind(source_path)
    .bind(mime_type)
    .bind(size)
    .bind(mtime)
    .execute(db)
    .await
    .map_err(|error| error.to_string())?;
    Ok(EditOriginalDescriptor {
        id,
        entity_type: entity_type.into(),
        entity_sync_id: entity_sync_id.into(),
        mime_type: mime_type.into(),
        byte_size: size,
        sha256: None,
        available: size.is_some(),
    })
}

pub async fn prepare_download(
    db: &SqlitePool,
    media_dir: &Path,
    asset_id: &str,
) -> Result<EditOriginalFile, String> {
    let row = sqlx::query(
        "SELECT id, entity_type, entity_sync_id, source_path, mime_type, byte_size, sha256, source_mtime
         FROM mobile_edit_originals WHERE id = ?",
    )
    .bind(asset_id)
    .fetch_optional(db)
    .await
    .map_err(|error| error.to_string())?
    .ok_or_else(|| "edit original not found".to_string())?;
    let entity_type: String = row.get("entity_type");
    let entity_sync_id: String = row.get("entity_sync_id");
    let source_path: String = row.get("source_path");
    let mime_type: String = row.get("mime_type");
    let old_size: Option<i64> = row.get("byte_size");
    let old_hash: Option<String> = row.get("sha256");
    let old_mtime: Option<i64> = row.get("source_mtime");

    let live: Option<i64> = sqlx::query_scalar(
        "SELECT deleted FROM sync_entities WHERE entity_type = ? AND sync_id = ?",
    )
    .bind(&entity_type)
    .bind(&entity_sync_id)
    .fetch_optional(db)
    .await
    .map_err(|error| error.to_string())?;
    if live != Some(0) {
        return Err("edit original entity no longer exists".into());
    }

    let path = resolve_edit_path(media_dir, &entity_type, &source_path)?;
    if !path.is_file() {
        return Err("edit original file is missing".into());
    }
    let (size, mtime) = metadata(&path);
    let size = size.ok_or_else(|| "edit original file is missing".to_string())?;

    let hash = if old_hash.is_some()
        && old_size == Some(size)
        && (mtime.is_none() || old_mtime == mtime)
    {
        old_hash.unwrap_or_default()
    } else {
        let hash = sha256_file(&path)?;
        sqlx::query(
            "UPDATE mobile_edit_originals
             SET byte_size = ?, source_mtime = ?, sha256 = ?,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?",
        )
        .bind(size)
        .bind(mtime)
        .bind(&hash)
        .bind(asset_id)
        .execute(db)
        .await
        .map_err(|error| error.to_string())?;
        hash
    };

    Ok(EditOriginalFile {
        path,
        mime_type,
        sha256: hash,
        byte_size: size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[test]
    fn edit_paths_must_remain_in_edit_directory() {
        let media = Path::new("C:/library/media");
        assert!(resolve_edit_path(media, "film_photo", "rolls/1/edit/01.jpg").is_ok());
        assert!(resolve_edit_path(media, "digital_photo", "digital/2/edit/IMG_1.png").is_ok());
        assert!(resolve_edit_path(media, "film_photo", "rolls/1/lab/01.jpg").is_err());
        assert!(resolve_edit_path(media, "film_photo", "../edit/01.jpg").is_err());
        assert!(resolve_edit_path(media, "digital_photo", "digital/2/raw/IMG_1.cr2").is_err());
        assert!(resolve_edit_path(media, "film_photo", "rolls/1/edit/01.tif").is_err());
    }

    #[tokio::test]
    async fn edit_original_metadata_is_cached_without_exposing_a_path() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
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
            INSERT INTO sync_entities
                (entity_type, local_id, sync_id, revision, updated_at, deleted)
                VALUES ('film_photo', 1, 'photo-sync-1', 1, '2026-10-18T00:00:00Z', 0);
            ",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::raw_sql(include_str!(
            "../../migrations/0009_mobile_edit_originals.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        let root =
            std::env::temp_dir().join(format!("goshootfilm-edit-original-{}", std::process::id()));
        let media = root.join("media");
        let file = media.join("rolls/1/edit/01.png");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, b"edited image bytes").unwrap();
        let descriptor = ensure_descriptor(
            &pool,
            &media,
            "film_photo",
            "photo-sync-1",
            "rolls/1/edit/01.png",
        )
        .await
        .unwrap();
        assert!(descriptor.available);
        assert!(descriptor.sha256.is_none());
        let downloaded = prepare_download(&pool, &media, &descriptor.id)
            .await
            .unwrap();
        assert_eq!(downloaded.mime_type, "image/png");
        assert_eq!(downloaded.byte_size, 18);
        assert_eq!(downloaded.sha256.len(), 64);
        assert_eq!(downloaded.path, file);
        let refreshed = ensure_descriptor(
            &pool,
            &media,
            "film_photo",
            "photo-sync-1",
            "rolls/1/edit/01.png",
        )
        .await
        .unwrap();
        assert_eq!(
            refreshed.sha256.as_deref(),
            Some(downloaded.sha256.as_str())
        );
        pool.close().await;
        fs::remove_dir_all(root).unwrap();
    }
}
