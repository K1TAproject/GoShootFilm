use super::assets::{self, AssetDescriptor};
use crate::digital;
use crate::film_display_name;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncStats {
    pub camera_count: i64,
    pub film_count: i64,
    pub shot_film_count: i64,
    pub roll_count: i64,
    pub photo_count: i64,
    pub favorite_photo_count: i64,
    pub equipment_count: i64,
    pub digital_album_count: i64,
    pub digital_photo_count: i64,
    pub archived_photo_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncChange {
    pub entity_type: String,
    pub sync_id: String,
    pub revision: i64,
    pub deleted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assets: Vec<AssetDescriptor>,
}

#[derive(Debug, Clone)]
pub struct ChangeRow {
    pub seq: i64,
    pub entity_type: &'static str,
    pub sync_id: String,
    pub revision: i64,
    pub deleted: bool,
}

pub struct ChangesPage {
    pub changes: Vec<ChangeRow>,
    pub next_cursor: i64,
    pub max_seq: i64,
    pub reset_required: bool,
}

pub async fn stats(db: &SqlitePool) -> Result<SyncStats, String> {
    let row = sqlx::query(
        r#"
        SELECT
            (SELECT COUNT(*) FROM cameras),
            (SELECT COUNT(*) FROM film_stocks),
            (SELECT COUNT(DISTINCT film_stock_id) FROM rolls),
            (SELECT COUNT(*) FROM rolls),
            (SELECT COUNT(*) FROM photos),
            (SELECT COUNT(*) FROM photos WHERE is_favorite = 1)
                + (SELECT COUNT(*) FROM digital_photos WHERE is_favorite = 1),
            (SELECT COUNT(*) FROM cameras) + (SELECT COUNT(*) FROM equipment_items),
            (SELECT COUNT(*) FROM digital_albums),
            (SELECT COUNT(*) FROM digital_photos),
            (SELECT COUNT(*) FROM photos) + (SELECT COUNT(*) FROM digital_photos)
        "#,
    )
    .fetch_one(db)
    .await
    .map_err(|error| error.to_string())?;
    let camera_count: i64 = row.get(0);
    let film_count: i64 = row.get(1);
    let shot_film_count: i64 = row.get(2);
    let roll_count: i64 = row.get(3);
    let photo_count: i64 = row.get(4);
    let favorite_photo_count: i64 = row.get(5);
    let equipment_count: i64 = row.get(6);
    let digital_album_count: i64 = row.get(7);
    let digital_photo_count: i64 = row.get(8);
    let archived_photo_count: i64 = row.get(9);
    Ok(SyncStats {
        camera_count,
        film_count,
        shot_film_count,
        roll_count,
        photo_count,
        favorite_photo_count,
        equipment_count,
        digital_album_count,
        digital_photo_count,
        archived_photo_count,
    })
}

pub async fn changes_since(
    db: &SqlitePool,
    cursor: i64,
    limit: usize,
) -> Result<ChangesPage, String> {
    let max_seq: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(seq), 0) FROM sync_changes")
        .fetch_one(db)
        .await
        .map_err(|error| error.to_string())?;
    if cursor > max_seq {
        return Ok(ChangesPage {
            changes: Vec::new(),
            next_cursor: max_seq,
            max_seq,
            reset_required: true,
        });
    }

    let rows = sqlx::query(
        "SELECT seq, entity_type, sync_id, revision, deleted
         FROM sync_changes
         WHERE seq > ?
         ORDER BY seq ASC
         LIMIT ?",
    )
    .bind(cursor)
    .bind(limit as i64)
    .fetch_all(db)
    .await
    .map_err(|error| error.to_string())?;

    let next_cursor = rows
        .last()
        .map(|row| row.get::<i64, _>("seq"))
        .unwrap_or(cursor);
    let mut parsed = Vec::with_capacity(rows.len());
    for row in rows {
        let entity_type: String = row.get("entity_type");
        let entity_type: &'static str = match entity_type.as_str() {
            "camera" => "camera",
            "equipment" => "equipment",
            "film" => "film",
            "roll" => "roll",
            "film_photo" => "film_photo",
            "digital_album" => "digital_album",
            "digital_photo" => "digital_photo",
            _ => continue,
        };
        parsed.push(ChangeRow {
            seq: row.get("seq"),
            entity_type,
            sync_id: row.get("sync_id"),
            revision: row.get("revision"),
            deleted: row.get::<i64, _>("deleted") != 0,
        });
    }
    // Keep only the latest change per entity in this batch. The cursor still
    // advances past every raw change row.
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for row in parsed.into_iter().rev() {
        if seen.insert((row.entity_type, row.sync_id.clone())) {
            deduped.push(row);
        }
    }
    deduped.reverse();

    Ok(ChangesPage {
        changes: deduped,
        next_cursor,
        max_seq,
        reset_required: false,
    })
}

async fn local_row(
    db: &SqlitePool,
    entity_type: &str,
    sync_id: &str,
) -> Result<Option<(i64, bool)>, String> {
    let row: Option<(i64, i64)> = sqlx::query_as(
        "SELECT local_id, deleted FROM sync_entities WHERE entity_type = ? AND sync_id = ?",
    )
    .bind(entity_type)
    .bind(sync_id)
    .fetch_optional(db)
    .await
    .map_err(|error| error.to_string())?;
    Ok(row.map(|(id, deleted)| (id, deleted != 0)))
}

async fn sync_id_for(
    db: &SqlitePool,
    entity_type: &str,
    local_id: i64,
) -> Result<Option<String>, String> {
    sqlx::query_scalar("SELECT sync_id FROM sync_entities WHERE entity_type = ? AND local_id = ?")
        .bind(entity_type)
        .bind(local_id)
        .fetch_optional(db)
        .await
        .map_err(|error| error.to_string())
}

async fn photo_versions(
    db: &SqlitePool,
    entity_type: &str,
    local_id: i64,
) -> Result<Vec<(String, String)>, String> {
    let mut versions = Vec::new();
    if entity_type == "film_photo" {
        let row: Option<(Option<String>, Option<String>)> =
            sqlx::query_as("SELECT lab_scan_path, edit_scan_path FROM photos WHERE id = ?")
                .bind(local_id)
                .fetch_optional(db)
                .await
                .map_err(|error| error.to_string())?;
        if let Some((lab, edit)) = row {
            if let Some(path) = edit {
                versions.push(("edit".into(), path));
            }
            if let Some(path) = lab {
                versions.push(("lab".into(), path));
            }
        }
    } else if entity_type == "digital_photo" {
        let row: Option<(Option<String>, Option<String>)> =
            sqlx::query_as("SELECT raw_path, edit_path FROM digital_photos WHERE id = ?")
                .bind(local_id)
                .fetch_optional(db)
                .await
                .map_err(|error| error.to_string())?;
        if let Some((raw, edit)) = row {
            if let Some(path) = edit {
                versions.push(("edit".into(), path));
            }
            if let Some(path) = raw {
                versions.push(("raw".into(), path));
            }
        }
    }
    Ok(versions)
}

async fn payload(
    db: &SqlitePool,
    entity_type: &str,
    local_id: i64,
) -> Result<Option<Value>, String> {
    match entity_type {
        "camera" => {
            let row = sqlx::query(
                "SELECT brand, model, status, camera_type, format, sensor_format, purchase_date, note
                 FROM cameras WHERE id = ?",
            )
            .bind(local_id)
            .fetch_optional(db)
            .await
            .map_err(|error| error.to_string())?;
            Ok(row.map(|row| {
                json!({
                    "brand": row.get::<String, _>("brand"),
                    "model": row.get::<String, _>("model"),
                    "status": row
                        .get::<Option<String>, _>("status")
                        .unwrap_or_else(|| "active".into()),
                    "cameraType": row
                        .get::<Option<String>, _>("camera_type")
                        .unwrap_or_else(|| "film".into()),
                    "format": row.get::<Option<String>, _>("format"),
                    "sensorFormat": row.get::<Option<String>, _>("sensor_format"),
                    "purchaseDate": row.get::<Option<String>, _>("purchase_date"),
                    "note": row.get::<Option<String>, _>("note"),
                })
            }))
        }
        "equipment" => {
            let row = sqlx::query(
                "SELECT category, subtype, brand, model, mount, status, purchase_date, note
                 FROM equipment_items WHERE id = ?",
            )
            .bind(local_id)
            .fetch_optional(db)
            .await
            .map_err(|error| error.to_string())?;
            Ok(row.map(|row| {
                json!({
                    "category": row.get::<String, _>("category"),
                    "subtype": row.get::<Option<String>, _>("subtype"),
                    "brand": row.get::<String, _>("brand"),
                    "model": row.get::<String, _>("model"),
                    "mount": row.get::<Option<String>, _>("mount"),
                    "status": row.get::<String, _>("status"),
                    "purchaseDate": row.get::<Option<String>, _>("purchase_date"),
                    "note": row.get::<Option<String>, _>("note"),
                })
            }))
        }
        "film" => {
            let row = sqlx::query(
                "SELECT brand, name, iso, type, target_status, note FROM film_stocks WHERE id = ?",
            )
            .bind(local_id)
            .fetch_optional(db)
            .await
            .map_err(|error| error.to_string())?;
            Ok(row.map(|row| {
                json!({
                    "brand": row.get::<String, _>("brand"),
                    "name": row.get::<String, _>("name"),
                    "iso": row.get::<i64, _>("iso"),
                    "type": row.get::<String, _>("type"),
                    "targetStatus": row.get::<Option<String>, _>("target_status"),
                    "note": row.get::<Option<String>, _>("note"),
                })
            }))
        }
        "roll" => {
            let row = sqlx::query(
                r#"
                SELECT r.camera_id, r.film_stock_id, r.roll_index, r.shot_month, r.city, r.note,
                       c.brand AS camera_brand, c.model AS camera_model,
                       f.brand AS film_brand, f.name AS film_name, f.type AS film_type,
                       (
                           SELECT p.id FROM photos p
                           WHERE p.roll_id = r.id AND (p.edit_scan_path IS NOT NULL OR p.lab_scan_path IS NOT NULL)
                           ORDER BY p.is_favorite DESC,
                                    CASE WHEN p.is_favorite = 0 AND p.edit_scan_path IS NULL THEN 1 ELSE 0 END,
                                    p.frame_number, p.id
                           LIMIT 1
                       ) AS cover_photo_id,
                       (
                           SELECT CASE WHEN p.edit_scan_path IS NOT NULL THEN 'edit' ELSE 'lab' END
                           FROM photos p
                           WHERE p.roll_id = r.id AND (p.edit_scan_path IS NOT NULL OR p.lab_scan_path IS NOT NULL)
                           ORDER BY p.is_favorite DESC,
                                    CASE WHEN p.is_favorite = 0 AND p.edit_scan_path IS NULL THEN 1 ELSE 0 END,
                                    p.frame_number, p.id
                           LIMIT 1
                       ) AS cover_version,
                       (SELECT COUNT(*) FROM photos p WHERE p.roll_id = r.id) AS photo_count
                FROM rolls r
                JOIN cameras c ON c.id = r.camera_id
                JOIN film_stocks f ON f.id = r.film_stock_id
                WHERE r.id = ?
                "#,
            )
            .bind(local_id)
            .fetch_optional(db)
            .await
            .map_err(|error| error.to_string())?;
            let Some(row) = row else {
                return Ok(None);
            };
            let camera_id: i64 = row.get("camera_id");
            let film_id: i64 = row.get("film_stock_id");
            let cover_photo_id: Option<i64> = row.get("cover_photo_id");
            let cover_photo_sync_id = match cover_photo_id {
                Some(id) => sync_id_for(db, "film_photo", id).await?,
                None => None,
            };
            let camera_sync_id = sync_id_for(db, "camera", camera_id).await?;
            let film_sync_id = sync_id_for(db, "film", film_id).await?;
            let brand: String = row.get("film_brand");
            let name: String = row.get("film_name");
            Ok(Some(json!({
                "cameraSyncId": camera_sync_id,
                "filmSyncId": film_sync_id,
                "cameraBrand": row.get::<String, _>("camera_brand"),
                "cameraModel": row.get::<String, _>("camera_model"),
                "filmBrand": brand,
                "filmName": name,
                "filmType": row.get::<String, _>("film_type"),
                "cameraInfo": format!("{} {}", row.get::<String, _>("camera_brand"), row.get::<String, _>("camera_model")),
                "filmInfo": film_display_name(&brand, &name),
                "index": row.get::<i32, _>("roll_index"),
                "shotMonth": row.get::<Option<String>, _>("shot_month"),
                "city": row.get::<Option<String>, _>("city"),
                "note": row.get::<Option<String>, _>("note"),
                "coverPhotoSyncId": cover_photo_sync_id,
                "coverVersion": row.get::<Option<String>, _>("cover_version"),
                "photoCount": row.get::<i64, _>("photo_count"),
            })))
        }
        "film_photo" => {
            let row = sqlx::query(
                "SELECT roll_id, frame_number, lab_scan_path, edit_scan_path, is_favorite
                 FROM photos WHERE id = ?",
            )
            .bind(local_id)
            .fetch_optional(db)
            .await
            .map_err(|error| error.to_string())?;
            let Some(row) = row else {
                return Ok(None);
            };
            let roll_id: i64 = row.get("roll_id");
            Ok(Some(json!({
                "rollSyncId": sync_id_for(db, "roll", roll_id).await?,
                "frameNumber": row.get::<Option<i32>, _>("frame_number"),
                "displayName": row.get::<Option<i32>, _>("frame_number").map(|value| format!("Frame {value:02}")),
                "labAvailable": row.get::<Option<String>, _>("lab_scan_path").is_some(),
                "editAvailable": row.get::<Option<String>, _>("edit_scan_path").is_some(),
                "isFavorite": row.get::<i64, _>("is_favorite") != 0,
            })))
        }
        "digital_album" => {
            let row = sqlx::query(
                r#"
                SELECT a.title, a.camera_id, a.shot_date, a.city, a.note,
                       c.brand AS camera_brand, c.model AS camera_model,
                       COUNT(p.id) AS photo_count,
                       COALESCE(SUM(CASE WHEN p.raw_path IS NOT NULL THEN 1 ELSE 0 END), 0) AS raw_count,
                       COALESCE(SUM(CASE WHEN p.edit_path IS NOT NULL THEN 1 ELSE 0 END), 0) AS edit_count,
                       (
                           SELECT cp.id FROM digital_photos cp
                           WHERE cp.album_id = a.id AND (cp.raw_path IS NOT NULL OR cp.edit_path IS NOT NULL)
                           ORDER BY cp.is_favorite DESC,
                                    CASE WHEN cp.is_favorite = 0 AND cp.edit_path IS NULL THEN 1 ELSE 0 END,
                                    cp.created_at, cp.id
                           LIMIT 1
                       ) AS cover_photo_id,
                       (
                           SELECT CASE WHEN cp.edit_path IS NOT NULL THEN 'edit' ELSE 'raw' END
                           FROM digital_photos cp
                           WHERE cp.album_id = a.id AND (cp.raw_path IS NOT NULL OR cp.edit_path IS NOT NULL)
                           ORDER BY cp.is_favorite DESC,
                                    CASE WHEN cp.is_favorite = 0 AND cp.edit_path IS NULL THEN 1 ELSE 0 END,
                                    cp.created_at, cp.id
                           LIMIT 1
                       ) AS cover_version
                FROM digital_albums a
                LEFT JOIN cameras c ON c.id = a.camera_id
                LEFT JOIN digital_photos p ON p.album_id = a.id
                WHERE a.id = ?
                GROUP BY a.id
                "#,
            )
            .bind(local_id)
            .fetch_optional(db)
            .await
            .map_err(|error| error.to_string())?;
            let Some(row) = row else {
                return Ok(None);
            };
            let camera_id: Option<i64> = row.get("camera_id");
            let camera_sync_id = match camera_id {
                Some(id) => sync_id_for(db, "camera", id).await?,
                None => None,
            };
            let cover_photo_id: Option<i64> = row.get("cover_photo_id");
            let cover_photo_sync_id = match cover_photo_id {
                Some(id) => sync_id_for(db, "digital_photo", id).await?,
                None => None,
            };
            let shot_date: Option<String> = row.get("shot_date");
            let city: Option<String> = row.get("city");
            Ok(Some(json!({
                "title": digital::album_display_title(shot_date.as_deref(), city.as_deref()),
                "cameraSyncId": camera_sync_id,
                "cameraBrand": row.get::<Option<String>, _>("camera_brand"),
                "cameraModel": row.get::<Option<String>, _>("camera_model"),
                "shotDate": shot_date,
                "city": city,
                "note": row.get::<Option<String>, _>("note"),
                "photoCount": row.get::<i64, _>("photo_count"),
                "rawCount": row.get::<i64, _>("raw_count"),
                "editCount": row.get::<i64, _>("edit_count"),
                "coverPhotoSyncId": cover_photo_sync_id,
                "coverVersion": row.get::<Option<String>, _>("cover_version"),
            })))
        }
        "digital_photo" => {
            let row = sqlx::query(
                "SELECT album_id, pairing_key, raw_path, edit_path, is_favorite
                 FROM digital_photos WHERE id = ?",
            )
            .bind(local_id)
            .fetch_optional(db)
            .await
            .map_err(|error| error.to_string())?;
            let Some(row) = row else {
                return Ok(None);
            };
            let album_id: i64 = row.get("album_id");
            Ok(Some(json!({
                "albumSyncId": sync_id_for(db, "digital_album", album_id).await?,
                "pairingKey": row.get::<String, _>("pairing_key"),
                "rawAvailable": row.get::<Option<String>, _>("raw_path").is_some(),
                "editAvailable": row.get::<Option<String>, _>("edit_path").is_some(),
                "isFavorite": row.get::<i64, _>("is_favorite") != 0,
            })))
        }
        _ => Ok(None),
    }
}

/// Resolve the current payload and asset descriptors for one change row.
pub async fn materialize(db: &SqlitePool, media_dir: Option<&Path>, row: ChangeRow) -> SyncChange {
    let live = local_row(db, row.entity_type, &row.sync_id)
        .await
        .ok()
        .flatten();
    let Some((local_id, deleted)) = live else {
        return SyncChange {
            entity_type: row.entity_type.into(),
            sync_id: row.sync_id,
            revision: row.revision,
            deleted: true,
            data: None,
            assets: Vec::new(),
        };
    };
    if row.deleted || deleted {
        return SyncChange {
            entity_type: row.entity_type.into(),
            sync_id: row.sync_id,
            revision: row.revision,
            deleted: true,
            data: None,
            assets: Vec::new(),
        };
    }
    let data = match payload(db, row.entity_type, local_id).await {
        Ok(Some(data)) => data,
        _ => {
            return SyncChange {
                entity_type: row.entity_type.into(),
                sync_id: row.sync_id,
                revision: row.revision,
                deleted: true,
                data: None,
                assets: Vec::new(),
            };
        }
    };
    let assets = if matches!(row.entity_type, "film_photo" | "digital_photo") {
        match media_dir {
            Some(media_dir) => {
                let versions = photo_versions(db, row.entity_type, local_id)
                    .await
                    .unwrap_or_default();
                if versions.is_empty() {
                    Vec::new()
                } else {
                    assets::ensure_descriptors(
                        db,
                        media_dir,
                        row.entity_type,
                        &row.sync_id,
                        &versions,
                    )
                    .await
                    .unwrap_or_default()
                }
            }
            None => Vec::new(),
        }
    } else {
        Vec::new()
    };
    SyncChange {
        entity_type: row.entity_type.into(),
        sync_id: row.sync_id,
        revision: row.revision,
        deleted: false,
        data: Some(data),
        assets,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sync test database");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("apply migrations");
        pool
    }

    #[tokio::test]
    async fn incremental_changes_include_payload_and_tombstone() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO cameras (brand, model) VALUES ('Nikon', 'F2')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO film_stocks (brand, name, iso, type) VALUES ('Kodak', 'Portra 400', 400, 'Color Negative')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO rolls (camera_id, film_stock_id, roll_index, shot_month, city) VALUES (1, 1, 1, '2026-10', '杭州')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO photos (roll_id, frame_number, edit_scan_path, is_favorite) VALUES (1, 8, 'rolls/1/edit/08.jpg', 1)")
            .execute(&pool)
            .await
            .unwrap();

        let photo_sync_id: String = sqlx::query_scalar(
            "SELECT sync_id FROM sync_entities WHERE entity_type = 'film_photo' AND local_id = 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let page = changes_since(&pool, 0, 500).await.unwrap();
        assert!(!page.reset_required);
        let row = page
            .changes
            .into_iter()
            .find(|row| row.sync_id == photo_sync_id)
            .expect("photo change present");
        let change = materialize(&pool, None, row).await;
        assert!(!change.deleted);
        let data = change.data.expect("photo payload");
        assert_eq!(data["frameNumber"].as_i64(), Some(8));
        assert_eq!(data["displayName"].as_str(), Some("Frame 08"));
        assert_eq!(data["isFavorite"].as_bool(), Some(true));
        assert_eq!(data["editAvailable"].as_bool(), Some(true));
        assert_eq!(data["labAvailable"].as_bool(), Some(false));
        assert!(data["rollSyncId"].as_str().is_some());

        sqlx::query("DELETE FROM photos WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();
        let page = changes_since(&pool, page.next_cursor, 500).await.unwrap();
        let row = page
            .changes
            .into_iter()
            .find(|row| row.sync_id == photo_sync_id)
            .expect("photo tombstone present");
        let change = materialize(&pool, None, row).await;
        assert!(change.deleted);
        assert!(change.data.is_none());

        let stats = stats(&pool).await.unwrap();
        assert_eq!(stats.photo_count, 0);
        assert_eq!(stats.roll_count, 1);
        assert_eq!(stats.camera_count, 1);

        let reset = changes_since(&pool, page.max_seq + 10, 100).await.unwrap();
        assert!(reset.reset_required);
    }
}
