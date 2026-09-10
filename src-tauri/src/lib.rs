mod db;
mod library;
mod models;
mod validation;

use models::{
    CameraDetailResponse, CameraResponse, CameraRollResponse, DashboardStatsResponse, FilmResponse,
    ImportAnalysisItemResponse, ImportDraft, ImportResultResponse, LabPreviewResponse,
    PhotoImportEntry, PhotoResponse, RollDetailResponse, RollSummaryResponse,
};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
#[cfg(test)]
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;
use tauri_plugin_opener::OpenerExt;
use validation::{
    clean_optional, clean_required, database_error, ensure_changed, ensure_positive_id,
    validate_date, validate_film_target_status, validate_shot_month,
};

// 定义一个结构体用来在全局保存数据库连接池
pub struct AppState {
    pub db: SqlitePool,
    pub app_data_dir: PathBuf,
    pub library: RwLock<library::LibraryRuntime>,
    pub gallery_operation_lock: tokio::sync::RwLock<()>,
    pub preview_locks: tokio::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

fn library_directories(state: &AppState) -> Result<(PathBuf, PathBuf), String> {
    state
        .library
        .read()
        .map_err(|_| "图库运行状态不可用".to_string())?
        .directories()
}

async fn preview_task_lock(state: &AppState, key: String) -> Arc<tokio::sync::Mutex<()>> {
    let mut locks = state.preview_locks.lock().await;
    locks
        .entry(key)
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

#[tauri::command]
fn get_library_status(
    state: tauri::State<'_, AppState>,
) -> Result<library::LibraryStatusResponse, String> {
    Ok(state
        .library
        .read()
        .map_err(|_| "图库运行状态不可用".to_string())?
        .status())
}

#[tauri::command]
async fn set_library_path(
    state: tauri::State<'_, AppState>,
    library_path: String,
) -> Result<library::LibraryMigrationResponse, String> {
    let _gallery_guard = state.gallery_operation_lock.write().await;
    let current = state
        .library
        .read()
        .map_err(|_| "图库运行状态不可用".to_string())?
        .clone();
    let (next, result) = library::configure_library(
        &state.app_data_dir,
        &state.db,
        &current,
        PathBuf::from(library_path),
    )
    .await?;
    *state
        .library
        .write()
        .map_err(|_| "无法切换图库运行路径".to_string())? = next;
    Ok(result)
}

type CameraRow = (
    i64,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
);
type FilmRow = (
    i64,
    String,
    String,
    i64,
    String,
    Option<String>,
    Option<String>,
);
type RollListRow = (
    i64,
    i64,
    i64,
    i32,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    i64,
);
type RollDetailRow = (
    i64,
    i64,
    i64,
    i32,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    String,
    String,
    String,
    String,
);
type PhotoRow = (
    i64,
    Option<i32>,
    Option<String>,
    Option<String>,
    Option<i32>,
);
type CameraRollRow = (
    i64,
    i32,
    Option<String>,
    Option<String>,
    String,
    String,
    i64,
);

async fn validate_purchase_date(
    pool: &SqlitePool,
    purchase_date: Option<String>,
) -> Result<Option<String>, String> {
    let purchase_date = validate_date(purchase_date, "购买日期")?;
    let Some(value) = purchase_date.as_deref() else {
        return Ok(None);
    };
    let is_not_future: i64 = sqlx::query_scalar("SELECT date(?) <= date('now', 'localtime')")
        .bind(value)
        .fetch_one(pool)
        .await
        .map_err(|error| format!("校验购买日期失败: {error}"))?;
    if is_not_future == 0 {
        return Err("购买日期不能晚于今天".into());
    }
    Ok(purchase_date)
}

fn film_display_name(brand: &str, name: &str) -> String {
    let brand = brand.split_whitespace().collect::<Vec<_>>().join(" ");
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower_brand = brand.to_lowercase();
    let lower_name = name.to_lowercase();
    if brand.is_empty()
        || lower_name == lower_brand
        || lower_name.starts_with(&format!("{lower_brand} "))
    {
        name
    } else if name.is_empty() {
        brand
    } else {
        format!("{brand} {name}")
    }
}

fn safe_media_path(media_dir: &Path, stored_path: &Path) -> Option<PathBuf> {
    // 数据库只允许保存当前图库层级的相对路径，避免把任意绝对路径暴露给文件命令。
    if stored_path.is_absolute() {
        return None;
    }

    let parts = stored_path
        .components()
        .map(|component| match component {
            Component::Normal(value) => Some(value),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    if parts.len() != 4
        || parts[0] != OsStr::new("rolls")
        || parts[1]
            .to_str()
            .and_then(|value| value.parse::<i64>().ok())
            .is_none_or(|roll_id| roll_id <= 0)
        || !matches!(parts[2].to_str(), Some("lab" | "edit"))
    {
        return None;
    }
    Some(media_dir.join(stored_path))
}

fn resolve_stored_path(media_dir: &Path, stored_path: Option<String>) -> Option<String> {
    stored_path.and_then(|stored_path| {
        let path = Path::new(&stored_path);
        safe_media_path(media_dir, path).map(|path| path.to_string_lossy().into_owned())
    })
}

fn resolve_stored_path_buf(media_dir: &Path, stored_path: &str) -> Option<PathBuf> {
    safe_media_path(media_dir, Path::new(stored_path))
}

#[tauri::command]
async fn get_cameras(state: tauri::State<'_, AppState>) -> Result<Vec<CameraResponse>, String> {
    let rows: Vec<CameraRow> = sqlx::query_as(
            "SELECT id, brand, model, status, format, purchase_date, note FROM cameras ORDER BY brand COLLATE NOCASE, model COLLATE NOCASE",
        )
        .fetch_all(&state.db)
        .await
        .map_err(|e| format!("读取相机失败: {e}"))?;

    Ok(rows
        .into_iter()
        .map(
            |(id, brand, model, status, format, purchase_date, note)| CameraResponse {
                id,
                brand,
                model,
                status,
                format,
                purchase_date,
                note,
            },
        )
        .collect())
}

#[tauri::command]
async fn get_dashboard_stats(
    state: tauri::State<'_, AppState>,
) -> Result<DashboardStatsResponse, String> {
    let cameras = get_cameras(state.clone()).await?;
    let (film_count, shot_film_count, roll_count, photo_count, favorite_photo_count): (
        i64,
        i64,
        i64,
        i64,
        i64,
    ) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM film_stocks),
            (SELECT COUNT(DISTINCT film_stock_id) FROM rolls),
            (SELECT COUNT(*) FROM rolls),
            (SELECT COUNT(*) FROM photos),
            (SELECT COUNT(*) FROM photos WHERE is_favorite = 1)
        "#,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| format!("读取首页统计失败: {e}"))?;

    Ok(DashboardStatsResponse {
        camera_count: cameras.len() as i64,
        cameras,
        film_count,
        shot_film_count,
        roll_count,
        photo_count,
        favorite_photo_count,
    })
}

#[tauri::command]
async fn get_films(state: tauri::State<'_, AppState>) -> Result<Vec<FilmResponse>, String> {
    let rows: Vec<FilmRow> = sqlx::query_as(
            "SELECT id, brand, name, iso, type, target_status, note FROM film_stocks ORDER BY brand COLLATE NOCASE, name COLLATE NOCASE",
        )
        .fetch_all(&state.db)
        .await
        .map_err(|e| format!("读取胶片型号失败: {e}"))?;

    Ok(rows
        .into_iter()
        .map(
            |(id, brand, name, iso, film_type, target_status, note)| FilmResponse {
                id,
                brand,
                name,
                iso,
                film_type,
                target_status,
                note,
            },
        )
        .collect())
}

#[tauri::command]
async fn get_rolls(state: tauri::State<'_, AppState>) -> Result<Vec<RollSummaryResponse>, String> {
    let rows: Vec<RollListRow> = sqlx::query_as(
        r#"
        SELECT
            r.id, r.camera_id, r.film_stock_id, r.roll_index, r.shot_month, r.city, r.note,
            c.brand, c.model, f.brand, f.name, f.type,
            (
                SELECT p.edit_scan_path
                FROM photos p
                WHERE p.roll_id = r.id AND p.edit_scan_path IS NOT NULL
                ORDER BY p.frame_number, p.id
                LIMIT 1
            ) AS cover_path,
            (SELECT COUNT(*) FROM photos p WHERE p.roll_id = r.id) AS photo_count
        FROM rolls r
        JOIN cameras c ON c.id = r.camera_id
        JOIN film_stocks f ON f.id = r.film_stock_id
        ORDER BY r.shot_month DESC, r.id DESC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("读取拍摄卷失败: {e}"))?;

    Ok(rows
        .into_iter()
        .map(
            |(
                id,
                camera_id,
                film_id,
                index,
                shot_month,
                city,
                note,
                camera_brand,
                camera_model,
                film_brand,
                film_name,
                film_type,
                cover_path,
                photo_count,
            )| {
                let film_info = film_display_name(&film_brand, &film_name);
                RollSummaryResponse {
                    id,
                    camera_id,
                    film_id,
                    index,
                    shot_month,
                    city,
                    note,
                    camera_info: format!("{} {}", camera_brand, camera_model),
                    film_info,
                    camera_brand,
                    camera_model,
                    film_brand,
                    film_name,
                    film_type,
                    // 列表仅使用该值判断是否有调色封面，实际图片由按需缩略图命令提供。
                    cover_path,
                    photo_count,
                }
            },
        )
        .collect())
}

#[tauri::command]
async fn get_roll_detail(
    id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<RollDetailResponse, String> {
    ensure_positive_id(id, "拍摄卷编号")?;
    let roll: Option<RollDetailRow> = sqlx::query_as(
        r#"
        SELECT r.id, r.camera_id, r.film_stock_id, r.roll_index, r.shot_month, r.city, r.note,
               c.brand, c.model, f.brand, f.name, f.type
        FROM rolls r
        JOIN cameras c ON c.id = r.camera_id
        JOIN film_stocks f ON f.id = r.film_stock_id
        WHERE r.id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| format!("读取拍摄卷详情失败: {e}"))?;
    let (
        id,
        camera_id,
        film_id,
        index,
        shot_month,
        city,
        note,
        camera_brand,
        camera_model,
        film_brand,
        film_name,
        film_type,
    ) = roll.ok_or_else(|| "未找到拍摄卷".to_string())?;

    let photos: Vec<PhotoRow> = sqlx::query_as(
            "SELECT id, frame_number, lab_scan_path, edit_scan_path, is_favorite FROM photos WHERE roll_id = ? ORDER BY frame_number, id",
        )
        .bind(id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| format!("读取照片失败: {e}"))?;

    let photo_count = photos.len() as i64;
    let media_dir = photos
        .iter()
        .any(|(_, _, lab, edit, _)| lab.is_some() || edit.is_some())
        .then(|| library_directories(&state).map(|directories| directories.0))
        .transpose()?;
    let photos = photos
        .into_iter()
        .map(
            |(id, frame_number, lab_scan_path, edit_scan_path, is_favorite)| PhotoResponse {
                id,
                frame_number,
                lab_scan_path: media_dir
                    .as_deref()
                    .and_then(|media_dir| resolve_stored_path(media_dir, lab_scan_path)),
                edit_scan_path: media_dir
                    .as_deref()
                    .and_then(|media_dir| resolve_stored_path(media_dir, edit_scan_path)),
                is_favorite: is_favorite.unwrap_or(0) != 0,
            },
        )
        .collect();

    let film_info = film_display_name(&film_brand, &film_name);
    Ok(RollDetailResponse {
        summary: RollSummaryResponse {
            id,
            camera_id,
            film_id,
            index,
            shot_month,
            city,
            note,
            camera_info: format!("{} {}", camera_brand, camera_model),
            film_info,
            camera_brand,
            camera_model,
            film_brand,
            film_name,
            film_type,
            cover_path: None,
            photo_count,
        },
        photos,
    })
}

// 2. 新增相机
#[tauri::command]
async fn add_camera(
    brand: String,
    model: String,
    format: Option<String>,
    purchase_date: Option<String>,
    note: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let brand = clean_required(brand, "品牌", 80)?;
    let model = clean_required(model, "型号", 120)?;
    let fmt = clean_optional(format, "画幅", 30)?.unwrap_or_else(|| "135".to_string());
    let purchase_date = validate_purchase_date(&state.db, purchase_date).await?;
    let note = clean_optional(note, "备注", 2000)?;
    sqlx::query(
        "INSERT INTO cameras (brand, model, format, purchase_date, note) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(brand)
    .bind(model)
    .bind(fmt)
    .bind(purchase_date)
    .bind(note)
    .execute(&state.db)
    .await
    .map_err(|e| database_error("新增相机", e))?;

    Ok("Camera added successfully!".into())
}

#[tauri::command]
async fn add_film_stock(
    brand: String,
    name: String,
    iso: i64,
    film_type: String,
    target_status: Option<String>,
    note: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<i64, String> {
    let brand = clean_required(brand, "品牌", 80)?;
    let name = clean_required(name, "名称", 120)?;
    let film_type = clean_required(film_type, "胶片类型", 50)?;
    if !(1..=12800).contains(&iso) {
        return Err("ISO 必须在 1 到 12800 之间".into());
    }
    let target_status = validate_film_target_status(target_status)?;
    let note = clean_optional(note, "备注", 2000)?;
    let result = sqlx::query(
        "INSERT INTO film_stocks (brand, name, iso, type, target_status, note) VALUES (?, ?, ?, ?, ?, ?)"
    )
        .bind(brand)
        .bind(name)
        .bind(iso)
        .bind(film_type)
        .bind(target_status)
        .bind(note)
        .execute(&state.db)
        .await
        .map_err(|e| database_error("新增胶片型号", e))?;

    Ok(result.last_insert_rowid())
}

#[tauri::command]
async fn add_roll(
    camera_id: i64,
    film_stock_id: i64,
    shot_month: Option<String>,
    city: Option<String>,
    note: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<i64, String> {
    ensure_positive_id(camera_id, "相机编号")?;
    ensure_positive_id(film_stock_id, "胶片编号")?;
    let shot_month = validate_shot_month(shot_month)?;
    let city = clean_optional(city, "地点", 120)?;
    let note = clean_optional(note, "备注", 2000)?;
    let camera_exists: Option<i64> = sqlx::query_scalar("SELECT id FROM cameras WHERE id = ?")
        .bind(camera_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| format!("读取相机失败: {e}"))?;
    if camera_exists.is_none() {
        return Err("所选相机不存在".into());
    }

    let film_exists: Option<i64> = sqlx::query_scalar("SELECT id FROM film_stocks WHERE id = ?")
        .bind(film_stock_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| format!("读取胶片型号失败: {e}"))?;
    if film_exists.is_none() {
        return Err("所选胶片型号不存在".into());
    }

    let mut transaction = state
        .db
        .begin()
        .await
        .map_err(|e| format!("无法开始新增拍摄卷事务: {e}"))?;
    db::reindex_rolls(&mut transaction)
        .await
        .map_err(|e| format!("整理全局拍摄卷序号失败: {e}"))?;
    let next_roll_index = db::next_roll_index(&mut transaction)
        .await
        .map_err(|e| format!("计算全局拍摄卷序号失败: {e}"))?;

    let result = sqlx::query(
        "INSERT INTO rolls (camera_id, film_stock_id, roll_index, shot_month, city, note) VALUES (?, ?, ?, ?, ?, ?)"
    )
        .bind(camera_id)
        .bind(film_stock_id)
        .bind(next_roll_index)
        .bind(shot_month)
        .bind(city)
        .bind(note)
        .execute(&mut *transaction)
        .await
        .map_err(|e| database_error("新增拍摄卷", e))?;

    transaction
        .commit()
        .await
        .map_err(|e| format!("提交新增拍摄卷失败: {e}"))?;

    Ok(result.last_insert_rowid())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn update_film_stock(
    id: i64,
    brand: String,
    name: String,
    iso: i64,
    film_type: String,
    target_status: Option<String>,
    note: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    ensure_positive_id(id, "胶片编号")?;
    let brand = clean_required(brand, "品牌", 80)?;
    let name = clean_required(name, "名称", 120)?;
    let film_type = clean_required(film_type, "胶片类型", 50)?;
    if !(1..=12800).contains(&iso) {
        return Err("ISO 必须在 1 到 12800 之间".into());
    }
    let target_status = validate_film_target_status(target_status)?;
    let note = clean_optional(note, "备注", 2000)?;
    let result = sqlx::query(
        "UPDATE film_stocks SET brand = ?, name = ?, iso = ?, type = ?, target_status = ?, note = ? WHERE id = ?",
    )
    .bind(brand)
    .bind(name)
    .bind(iso)
    .bind(film_type)
    .bind(target_status)
    .bind(note)
    .bind(id)
    .execute(&state.db)
    .await
    .map_err(|e| database_error("更新胶片型号", e))?
    .rows_affected();

    ensure_changed(result, "胶片型号")?;

    Ok("Film stock updated successfully!".into())
}

async fn cleanup_preview_records(
    preview_dir: &Path,
    preview_records: &[(i64, i64)],
    deleted_entity: &str,
) -> Result<(), String> {
    let mut roll_ids = HashSet::new();
    for (photo_id, roll_id) in preview_records {
        roll_ids.insert(*roll_id);
        let preview_path = preview_file_path(preview_dir, *roll_id, *photo_id);
        if let Err(error) = tokio::fs::remove_file(preview_path).await {
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err(format!(
                    "{deleted_entity}已删除，但关联预览资源清理失败: {error}"
                ));
            }
        }
    }
    for roll_id in roll_ids {
        let cover_path = roll_cover_preview_path(preview_dir, roll_id);
        if let Err(error) = tokio::fs::remove_file(cover_path).await {
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err(format!(
                    "{deleted_entity}已删除，但封面缩略图清理失败: {error}"
                ));
            }
        }
    }
    Ok(())
}

#[tauri::command]
async fn delete_film_stock(id: i64, state: tauri::State<'_, AppState>) -> Result<String, String> {
    ensure_positive_id(id, "胶片编号")?;
    let _gallery_guard = state.gallery_operation_lock.read().await;
    let mut transaction = state
        .db
        .begin()
        .await
        .map_err(|e| format!("无法开始删除胶片型号事务: {e}"))?;
    let preview_records: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT p.id, p.roll_id FROM photos p JOIN rolls r ON r.id = p.roll_id WHERE r.film_stock_id = ?",
    )
    .bind(id)
    .fetch_all(&mut *transaction)
    .await
    .map_err(|error| format!("读取关联照片预览失败: {error}"))?;
    let preview_dir = (!preview_records.is_empty())
        .then(|| library_directories(&state).map(|directories| directories.1))
        .transpose()?;
    let result = sqlx::query("DELETE FROM film_stocks WHERE id = ?")
        .bind(id)
        .execute(&mut *transaction)
        .await
        .map_err(|e| database_error("删除胶片型号", e))?;
    ensure_changed(result.rows_affected(), "胶片型号")?;
    db::reindex_rolls(&mut transaction)
        .await
        .map_err(|e| format!("重排全局拍摄卷序号失败: {e}"))?;
    transaction
        .commit()
        .await
        .map_err(|e| format!("提交删除胶片型号失败: {e}"))?;

    if let Some(preview_dir) = preview_dir {
        cleanup_preview_records(&preview_dir, &preview_records, "胶片型号").await?;
    }
    Ok("Film stock deleted successfully!".into())
}

#[tauri::command]
async fn update_roll(
    id: i64,
    camera_id: i64,
    film_stock_id: i64,
    shot_month: Option<String>,
    city: Option<String>,
    note: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    ensure_positive_id(id, "拍摄卷编号")?;
    ensure_positive_id(camera_id, "相机编号")?;
    ensure_positive_id(film_stock_id, "胶片编号")?;
    let shot_month = validate_shot_month(shot_month)?;
    let city = clean_optional(city, "地点", 120)?;
    let note = clean_optional(note, "备注", 2000)?;
    let roll_exists: Option<i64> = sqlx::query_scalar("SELECT id FROM rolls WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| format!("读取拍摄卷失败: {e}"))?;

    if roll_exists.is_none() {
        return Err("拍摄卷不存在".into());
    }

    let camera_exists: Option<i64> = sqlx::query_scalar("SELECT id FROM cameras WHERE id = ?")
        .bind(camera_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| format!("读取相机失败: {e}"))?;
    if camera_exists.is_none() {
        return Err("所选相机不存在".into());
    }

    let film_exists: Option<i64> = sqlx::query_scalar("SELECT id FROM film_stocks WHERE id = ?")
        .bind(film_stock_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| format!("读取胶片型号失败: {e}"))?;
    if film_exists.is_none() {
        return Err("所选胶片型号不存在".into());
    }

    let result = sqlx::query(
        "UPDATE rolls SET camera_id = ?, film_stock_id = ?, shot_month = ?, city = ?, note = ? WHERE id = ?",
    )
    .bind(camera_id)
    .bind(film_stock_id)
    .bind(shot_month)
    .bind(city)
    .bind(note)
    .bind(id)
    .execute(&state.db)
    .await
    .map_err(|e| database_error("更新拍摄卷", e))?;

    ensure_changed(result.rows_affected(), "拍摄卷")?;

    Ok("Roll updated successfully!".into())
}

#[tauri::command]
async fn delete_roll(id: i64, state: tauri::State<'_, AppState>) -> Result<String, String> {
    ensure_positive_id(id, "拍摄卷编号")?;
    let _gallery_guard = state.gallery_operation_lock.read().await;
    let mut transaction = state
        .db
        .begin()
        .await
        .map_err(|e| format!("无法开始删除拍摄卷事务: {e}"))?;
    let preview_records: Vec<(i64, i64)> =
        sqlx::query_as("SELECT id, roll_id FROM photos WHERE roll_id = ?")
            .bind(id)
            .fetch_all(&mut *transaction)
            .await
            .map_err(|error| format!("读取关联照片预览失败: {error}"))?;
    let preview_dir = (!preview_records.is_empty())
        .then(|| library_directories(&state).map(|directories| directories.1))
        .transpose()?;
    let result = sqlx::query("DELETE FROM rolls WHERE id = ?")
        .bind(id)
        .execute(&mut *transaction)
        .await
        .map_err(|e| database_error("删除拍摄卷", e))?;
    ensure_changed(result.rows_affected(), "拍摄卷")?;
    db::reindex_rolls(&mut transaction)
        .await
        .map_err(|e| format!("重排全局拍摄卷序号失败: {e}"))?;
    transaction
        .commit()
        .await
        .map_err(|e| format!("提交删除拍摄卷失败: {e}"))?;

    if let Some(preview_dir) = preview_dir {
        cleanup_preview_records(&preview_dir, &preview_records, "拍摄卷").await?;
    }
    Ok("Roll deleted successfully!".into())
}

// 3. 更新相机信息
#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn update_camera(
    id: i64,
    brand: String,
    model: String,
    status: String,
    format: Option<String>,
    purchase_date: Option<String>,
    note: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    ensure_positive_id(id, "相机编号")?;
    let brand = clean_required(brand, "品牌", 80)?;
    let model = clean_required(model, "型号", 120)?;
    if !matches!(status.as_str(), "active" | "inactive") {
        return Err("相机状态无效".into());
    }
    let format = clean_optional(format, "画幅", 30)?.or_else(|| Some("135".to_string()));
    let purchase_date = validate_purchase_date(&state.db, purchase_date).await?;
    let note = clean_optional(note, "备注", 2000)?;
    let result = sqlx::query(
        "UPDATE cameras SET brand = ?, model = ?, status = ?, format = ?, purchase_date = ?, note = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
    )
        .bind(brand)
        .bind(model)
        .bind(status)
        .bind(format)
        .bind(purchase_date)
        .bind(note)
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| database_error("更新相机", e))?;

    ensure_changed(result.rows_affected(), "相机")?;

    Ok("Camera updated successfully!".into())
}

// 4. 删除相机
#[tauri::command]
async fn delete_camera(id: i64, state: tauri::State<'_, AppState>) -> Result<String, String> {
    ensure_positive_id(id, "相机编号")?;
    let _gallery_guard = state.gallery_operation_lock.read().await;
    let mut transaction = state
        .db
        .begin()
        .await
        .map_err(|e| format!("无法开始删除相机事务: {e}"))?;
    let preview_records: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT p.id, p.roll_id FROM photos p JOIN rolls r ON r.id = p.roll_id WHERE r.camera_id = ?",
    )
    .bind(id)
    .fetch_all(&mut *transaction)
    .await
    .map_err(|error| format!("读取关联照片预览失败: {error}"))?;
    let preview_dir = (!preview_records.is_empty())
        .then(|| library_directories(&state).map(|directories| directories.1))
        .transpose()?;
    let result = sqlx::query("DELETE FROM cameras WHERE id = ?")
        .bind(id)
        .execute(&mut *transaction)
        .await
        .map_err(|e| database_error("删除相机", e))?;

    ensure_changed(result.rows_affected(), "相机")?;
    db::reindex_rolls(&mut transaction)
        .await
        .map_err(|e| format!("重排全局拍摄卷序号失败: {e}"))?;
    transaction
        .commit()
        .await
        .map_err(|e| format!("提交删除相机失败: {e}"))?;

    if let Some(preview_dir) = preview_dir {
        cleanup_preview_records(&preview_dir, &preview_records, "相机").await?;
    }

    Ok("Camera deleted successfully!".into())
}

// 5. 获取单个相机的详细信息及拍摄卷
#[tauri::command]
async fn get_camera_detail(
    id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<CameraDetailResponse, String> {
    ensure_positive_id(id, "相机编号")?;
    let camera_row: Option<CameraRow> = sqlx::query_as(
        "SELECT id, brand, model, status, format, purchase_date, note FROM cameras WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let camera = match camera_row {
        Some((id, brand, model, status, format, purchase_date, note)) => CameraResponse {
            id,
            brand,
            model,
            status,
            format,
            purchase_date,
            note,
        },
        None => return Err("未找到相机".into()),
    };

    let rolls_rows: Vec<CameraRollRow> = sqlx::query_as(
        r#"
        SELECT r.id, r.roll_index, r.shot_month, r.city, f.brand as film_brand, f.name as film_name, f.iso as film_iso
        FROM rolls r
        JOIN film_stocks f ON r.film_stock_id = f.id
        WHERE r.camera_id = ?
        ORDER BY r.shot_month IS NULL, r.shot_month DESC, r.roll_index DESC, r.id DESC
        "#
    )
        .bind(id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    let rolls = rolls_rows
        .into_iter()
        .map(
            |(rid, idx, month, city, f_brand, f_name, f_iso)| CameraRollResponse {
                id: rid,
                roll_index: idx,
                shot_month: month,
                city,
                film_info: format!("{} (ISO {})", film_display_name(&f_brand, &f_name), f_iso),
                film_brand: f_brand,
                film_name: f_name,
                film_iso: f_iso,
            },
        )
        .collect::<Vec<_>>();

    Ok(CameraDetailResponse { camera, rolls })
}

#[tauri::command]
async fn toggle_photo_favorite(
    state: tauri::State<'_, AppState>,
    photo_id: i64,
    is_favorite: bool,
) -> Result<(), String> {
    ensure_positive_id(photo_id, "照片编号")?;
    let val = if is_favorite { 1 } else { 0 };
    let result = sqlx::query("UPDATE photos SET is_favorite = ? WHERE id = ?")
        .bind(val)
        .bind(photo_id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    ensure_changed(result.rows_affected(), "照片")?;
    Ok(())
}

fn preview_file_path(preview_dir: &Path, roll_id: i64, photo_id: i64) -> PathBuf {
    // TIFF 预览是可再生缓存，必须与 media 下不可误删的正式图库分开。
    preview_dir
        .join("rolls")
        .join(roll_id.to_string())
        .join(format!("{photo_id}.png"))
}

fn roll_cover_preview_path(preview_dir: &Path, roll_id: i64) -> PathBuf {
    preview_dir
        .join("rolls")
        .join(roll_id.to_string())
        .join("cover.png")
}

fn build_lab_preview(source: &Path, target: &Path) -> Result<(), String> {
    let image = image::open(source)
        .map_err(|error| format!("原始扫描图片无法读取（{}）: {error}", source.display()))?;
    let preview = if image.width() > 1800 || image.height() > 1800 {
        image.thumbnail(1800, 1800).to_rgb8()
    } else {
        image.to_rgb8()
    };
    preview
        .save_with_format(target, image::ImageFormat::Png)
        .map_err(|error| format!("生成原始扫描预览失败: {error}"))
}

fn build_roll_cover_preview(source: &Path, target: &Path) -> Result<(), String> {
    let image = image::open(source)
        .map_err(|error| format!("拍摄卷封面无法读取（{}）: {error}", source.display()))?;
    image
        .thumbnail(640, 480)
        .to_rgb8()
        .save_with_format(target, image::ImageFormat::Png)
        .map_err(|error| format!("生成拍摄卷封面缩略图失败: {error}"))
}

async fn preview_is_fresh(source: &Path, preview: &Path) -> bool {
    match (
        tokio::fs::metadata(source).await,
        tokio::fs::metadata(preview).await,
    ) {
        (Ok(source), Ok(preview)) if preview.len() > 0 => {
            match (source.modified(), preview.modified()) {
                (Ok(source_time), Ok(preview_time)) => preview_time >= source_time,
                _ => true,
            }
        }
        _ => false,
    }
}

fn unique_preview_temp_path(preview: &Path) -> Result<PathBuf, String> {
    let parent = preview
        .parent()
        .ok_or_else(|| "无法确定预览图库目录".to_string())?;
    let file_name = preview
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| "预览文件名无效".to_string())?;
    let sequence = PREVIEW_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    Ok(parent.join(format!(
        ".{file_name}.building-{}-{sequence}",
        std::process::id()
    )))
}

async fn build_preview_file(
    source: PathBuf,
    preview: PathBuf,
    builder: fn(&Path, &Path) -> Result<(), String>,
    task_label: &str,
) -> Result<(), String> {
    let parent = preview
        .parent()
        .ok_or_else(|| "无法确定预览图库目录".to_string())?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|error| format!("无法创建预览图库目录: {error}"))?;
    let temporary = unique_preview_temp_path(&preview)?;
    let target = temporary.clone();
    let result = tauri::async_runtime::spawn_blocking(move || builder(&source, &target))
        .await
        .map_err(|error| format!("{task_label}任务失败: {error}"))?;
    if let Err(error) = result {
        let _ = tokio::fs::remove_file(&temporary).await;
        return Err(error);
    }
    if let Err(error) = library::atomic_replace(&temporary, &preview) {
        let _ = tokio::fs::remove_file(&temporary).await;
        return Err(format!("保存{task_label}失败: {error}"));
    }
    Ok(())
}

fn validate_photo_version(version: &str) -> Result<(), String> {
    if matches!(version, "lab" | "edit") {
        Ok(())
    } else {
        Err("图片版本无效，必须是原始扫描或调色图".into())
    }
}

fn roll_version_relative_dir(roll_id: i64, version: &str) -> PathBuf {
    PathBuf::from("rolls")
        .join(roll_id.to_string())
        .join(version)
}

#[tauri::command]
async fn get_roll_media_directory(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    roll_id: i64,
    version: String,
) -> Result<String, String> {
    ensure_positive_id(roll_id, "拍摄卷编号")?;
    validate_photo_version(&version)?;

    let roll_exists: Option<i64> = sqlx::query_scalar("SELECT id FROM rolls WHERE id = ?")
        .bind(roll_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|error| format!("确认拍摄卷失败: {error}"))?;

    if roll_exists.is_none() {
        return Err("拍摄卷不存在".into());
    }

    let (media_dir, _) = library_directories(&state)?;
    let directory = media_dir.join(roll_version_relative_dir(roll_id, &version));

    if !directory.is_dir() {
        return Err("当前图片版本还没有图库目录".into());
    }

    let directory = directory.to_string_lossy().into_owned();

    app.opener()
        .open_path(directory.clone(), None::<String>)
        .map_err(|error| format!("无法打开图库目录: {error}"))?;

    Ok(directory)
}

fn extension_allowed(version: &str, extension: &str) -> bool {
    match version {
        "lab" => matches!(
            extension,
            "png" | "jpg" | "jpeg" | "tif" | "tiff" | "webp" | "bmp" | "gif"
        ),
        "edit" => matches!(extension, "png" | "jpg" | "jpeg" | "webp"),
        _ => false,
    }
}

fn detect_frame_number(path: &Path) -> Option<i32> {
    // 扫描仪文件名前缀不稳定，业务约定只认文件名末尾两位，且 00 不属于有效 Frame。
    let stem = path.file_stem()?.to_str()?;
    let suffix = stem.chars().rev().take(2).collect::<Vec<_>>();
    if suffix.len() != 2 || !suffix.iter().all(char::is_ascii_digit) {
        return None;
    }
    let suffix: String = suffix.into_iter().rev().collect();
    suffix
        .parse::<i32>()
        .ok()
        .filter(|frame| (1..=99).contains(frame))
}

#[derive(Debug)]
struct NormalizedImport {
    source_path: PathBuf,
    source_display: String,
    file_name: String,
}

fn normalize_import_source(raw_path: &str, version: &str) -> Result<NormalizedImport, String> {
    let path = Path::new(raw_path);
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| format!("无法识别文件类型: {raw_path}"))?;
    if !extension_allowed(version, &extension) {
        let version_name = if version == "lab" {
            "原始扫描"
        } else {
            "调色图"
        };
        return Err(format!("{version_name}不支持此图片格式: {raw_path}"));
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| format!("照片文件不存在或无法访问: {raw_path}"))?;
    if !canonical.is_file() {
        return Err(format!("路径不是文件: {raw_path}"));
    }
    let file_name = canonical
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("无法读取照片文件名: {raw_path}"))?
        .to_string();
    let source_display = canonical.to_string_lossy().into_owned();
    Ok(NormalizedImport {
        source_path: canonical,
        source_display,
        file_name,
    })
}

type ExistingPhotoRow = (i64, i32, Option<String>, Option<String>);
static PREVIEW_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

async fn roll_photo_rows(
    pool: &SqlitePool,
    roll_id: i64,
) -> Result<HashMap<i32, Vec<ExistingPhotoRow>>, String> {
    let rows: Vec<ExistingPhotoRow> = sqlx::query_as(
        "SELECT id, frame_number, lab_scan_path, edit_scan_path FROM photos WHERE roll_id = ? AND frame_number IS NOT NULL",
    )
    .bind(roll_id)
    .fetch_all(pool)
    .await
    .map_err(|error| format!("读取已有照片失败: {error}"))?;
    let mut by_frame: HashMap<i32, Vec<ExistingPhotoRow>> = HashMap::new();
    for row in rows {
        by_frame.entry(row.1).or_default().push(row);
    }
    Ok(by_frame)
}

#[tauri::command]
async fn analyze_photo_import(
    state: tauri::State<'_, AppState>,
    roll_id: i64,
    version: String,
    drafts: Vec<ImportDraft>,
) -> Result<Vec<ImportAnalysisItemResponse>, String> {
    ensure_positive_id(roll_id, "拍摄卷编号")?;
    validate_photo_version(&version)?;
    if drafts.is_empty() {
        return Err("没有可导入的照片".into());
    }
    let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM rolls WHERE id = ?")
        .bind(roll_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|error| format!("确认拍摄卷失败: {error}"))?;
    if exists.is_none() {
        return Err("拍摄卷不存在".into());
    }

    let existing = roll_photo_rows(&state.db, roll_id).await?;
    let mut normalized = Vec::with_capacity(drafts.len());
    let mut seen_sources = HashSet::new();
    for draft in drafts {
        let source = normalize_import_source(&draft.source_path, &version)?;
        if !seen_sources.insert(source.source_path.clone()) {
            return Err(format!("同一文件被重复选择: {}", source.file_name));
        }
        let frame_number = draft
            .frame_number
            .or_else(|| detect_frame_number(&source.source_path));
        normalized.push((source, frame_number));
    }

    let mut selected_frames: HashMap<i32, usize> = HashMap::new();
    for (_, frame_number) in &normalized {
        if let Some(frame) = frame_number.filter(|frame| (1..=99).contains(frame)) {
            *selected_frames.entry(frame).or_default() += 1;
        }
    }

    Ok(normalized
        .into_iter()
        .map(|(source, frame_number)| {
            let mut issue = match frame_number {
                None => Some("未识别到文件名末尾的有效两位 Frame，请手动填写 01–99".into()),
                Some(frame) if !(1..=99).contains(&frame) => {
                    Some("Frame 必须是 01–99，00 无效".into())
                }
                _ => None,
            };
            let rows = frame_number.and_then(|frame| existing.get(&frame));
            if issue.is_none()
                && frame_number
                    .and_then(|frame| selected_frames.get(&frame))
                    .is_some_and(|count| *count > 1)
            {
                issue = Some("所选文件中存在重复 Frame，请修改后重新检查".into());
            }
            if issue.is_none() && rows.is_some_and(|rows| rows.len() > 1) {
                issue = Some("数据库中存在重复 Frame，无法自动配对".into());
            }
            let row = rows.and_then(|rows| rows.first());
            let existing_version = row.is_some_and(|row| {
                if version == "lab" {
                    row.2.is_some()
                } else {
                    row.3.is_some()
                }
            });
            let paired_version = row.is_some_and(|row| {
                if version == "lab" {
                    row.3.is_some()
                } else {
                    row.2.is_some()
                }
            });
            ImportAnalysisItemResponse {
                source_path: source.source_display,
                file_name: source.file_name,
                frame_number,
                existing_version,
                paired_version,
                issue,
            }
        })
        .collect())
}

// 删除照片记录时保留正式图库文件，仅清理可再生预览。
#[tauri::command]
async fn delete_photo(state: tauri::State<'_, AppState>, photo_id: i64) -> Result<(), String> {
    ensure_positive_id(photo_id, "照片编号")?;
    let _gallery_guard = state.gallery_operation_lock.read().await;
    let roll_id: Option<i64> = sqlx::query_scalar("SELECT roll_id FROM photos WHERE id = ?")
        .bind(photo_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| format!("读取照片记录失败: {e}"))?;
    let Some(roll_id) = roll_id else {
        return Err("未找到要操作的照片".into());
    };
    let (_, preview_dir) = library_directories(&state)?;

    let result = sqlx::query("DELETE FROM photos WHERE id = ?")
        .bind(photo_id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    ensure_changed(result.rows_affected(), "照片")?;

    let preview_path = preview_file_path(&preview_dir, roll_id, photo_id);
    if let Err(error) = tokio::fs::remove_file(&preview_path).await {
        if error.kind() != std::io::ErrorKind::NotFound {
            return Err(format!("照片记录已移除，但预览资源清理失败: {error}"));
        }
    }
    let _ = tokio::fs::remove_file(roll_cover_preview_path(&preview_dir, roll_id)).await;
    Ok(())
}

async fn cleanup_copied_files(paths: &[PathBuf]) {
    for path in paths {
        let _ = tokio::fs::remove_file(path).await;
    }
}

fn numbered_copy_name(file_name: &str, copy_number: usize) -> String {
    let path = Path::new(file_name);
    let stem = path
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or(file_name);
    match path.extension().and_then(OsStr::to_str) {
        Some(extension) if !extension.is_empty() => {
            format!("{stem} ({copy_number}).{extension}")
        }
        _ => format!("{stem} ({copy_number})"),
    }
}

async fn copy_to_unique_gallery_path(
    source_path: &Path,
    destination_dir: &Path,
    file_name: &str,
) -> Result<PathBuf, String> {
    let mut copy_number = 1_usize;
    loop {
        let destination_name = if copy_number == 1 {
            file_name.to_string()
        } else {
            numbered_copy_name(file_name, copy_number)
        };
        let destination_path = destination_dir.join(destination_name);
        let mut destination = match tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination_path)
            .await
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                copy_number = copy_number
                    .checked_add(1)
                    .ok_or_else(|| "无法生成唯一的图库文件名".to_string())?;
                continue;
            }
            Err(error) => return Err(format!("无法创建图库文件: {error}")),
        };

        let copy_result = async {
            let mut source = tokio::fs::File::open(source_path).await?;
            tokio::io::copy(&mut source, &mut destination).await?;
            destination.sync_all().await
        }
        .await;
        if let Err(error) = copy_result {
            drop(destination);
            let _ = tokio::fs::remove_file(&destination_path).await;
            return Err(format!("复制图片失败: {error}"));
        }
        return Ok(destination_path);
    }
}

#[tauri::command]
async fn import_photo_versions(
    state: tauri::State<'_, AppState>,
    roll_id: i64,
    version: String,
    entries: Vec<PhotoImportEntry>,
) -> Result<ImportResultResponse, String> {
    let _gallery_guard = state.gallery_operation_lock.read().await;
    import_photo_versions_inner(&state, roll_id, version, entries).await
}

async fn import_photo_versions_inner(
    state: &AppState,
    roll_id: i64,
    version: String,
    entries: Vec<PhotoImportEntry>,
) -> Result<ImportResultResponse, String> {
    ensure_positive_id(roll_id, "拍摄卷编号")?;
    validate_photo_version(&version)?;
    if entries.is_empty() {
        return Err("没有可导入的照片".into());
    }
    let (media_dir, preview_dir) = library_directories(state)?;

    let mut normalized = Vec::with_capacity(entries.len());
    let mut unique_sources = HashSet::new();
    let mut unique_frames = HashSet::new();
    for entry in entries {
        if !(1..=99).contains(&entry.frame_number) {
            return Err(format!(
                "Frame {:02} 无效，必须是 01–99",
                entry.frame_number
            ));
        }
        if !matches!(
            entry.conflict_action.as_str(),
            "add" | "skip" | "replace" | "cancel"
        ) {
            return Err(format!(
                "Frame {:02} 的冲突处理方式无效",
                entry.frame_number
            ));
        }
        if entry.conflict_action == "cancel" {
            return Err("已取消整批导入，未写入任何数据".into());
        }
        let source = normalize_import_source(&entry.source_path, &version)?;
        if !unique_sources.insert(source.source_path.clone()) {
            return Err(format!("同一文件被重复选择: {}", source.file_name));
        }
        if entry.conflict_action != "skip" && !unique_frames.insert(entry.frame_number) {
            return Err(format!(
                "所选文件中存在重复 Frame {:02}，未写入任何数据",
                entry.frame_number
            ));
        }
        normalized.push((entry, source));
    }

    // 原子性同时跨越数据库和文件系统：提交前任一失败都回滚事务并删除本批已复制文件。
    let mut transaction = state
        .db
        .begin()
        .await
        .map_err(|e| format!("无法开始照片导入事务: {e}"))?;
    let roll_exists: Option<i64> = sqlx::query_scalar("SELECT id FROM rolls WHERE id = ?")
        .bind(roll_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|e| e.to_string())?;
    if roll_exists.is_none() {
        return Err("拍摄卷不存在".into());
    }

    let rows: Vec<ExistingPhotoRow> = sqlx::query_as(
        "SELECT id, frame_number, lab_scan_path, edit_scan_path FROM photos WHERE roll_id = ? AND frame_number IS NOT NULL",
    )
    .bind(roll_id)
    .fetch_all(&mut *transaction)
    .await
    .map_err(|error| format!("读取已有照片失败: {error}"))?;
    let mut existing: HashMap<i32, Vec<ExistingPhotoRow>> = HashMap::new();
    for row in rows {
        existing.entry(row.1).or_default().push(row);
    }

    for (entry, _) in &normalized {
        let rows = existing.get(&entry.frame_number);
        if rows.is_some_and(|rows| rows.len() > 1) {
            let _ = transaction.rollback().await;
            return Err(format!(
                "数据库中存在重复 Frame {:02}，无法确认配对关系",
                entry.frame_number
            ));
        }
        let has_same_version = rows.and_then(|rows| rows.first()).is_some_and(|row| {
            if version == "lab" {
                row.2.is_some()
            } else {
                row.3.is_some()
            }
        });
        if has_same_version && !matches!(entry.conflict_action.as_str(), "skip" | "replace") {
            let _ = transaction.rollback().await;
            return Err(format!(
                "Frame {:02} 已有同版本图片，请选择跳过、替换或取消",
                entry.frame_number
            ));
        }
    }

    let relative_dir = roll_version_relative_dir(roll_id, &version);
    let absolute_dir = media_dir.join(&relative_dir);
    tokio::fs::create_dir_all(&absolute_dir)
        .await
        .map_err(|e| format!("无法创建照片存储目录: {e}"))?;

    let mut imported_count = 0_usize;
    let mut updated_count = 0_usize;
    let mut skipped_count = 0_usize;
    let mut copied_files = Vec::new();
    let mut preview_invalidations = Vec::new();
    for (entry, source) in normalized {
        if entry.conflict_action == "skip" {
            skipped_count += 1;
            continue;
        }
        let absolute_path = match copy_to_unique_gallery_path(
            &source.source_path,
            &absolute_dir,
            &source.file_name,
        )
        .await
        {
            Ok(path) => path,
            Err(error) => {
                cleanup_copied_files(&copied_files).await;
                let _ = transaction.rollback().await;
                return Err(format!(
                    "复制照片到正式图库失败（{}）: {error}",
                    source.source_display
                ));
            }
        };
        copied_files.push(absolute_path.clone());
        let stored_path = match absolute_path.strip_prefix(&media_dir) {
            Ok(path) => path.to_string_lossy().replace('\\', "/"),
            Err(_) => {
                cleanup_copied_files(&copied_files).await;
                let _ = transaction.rollback().await;
                return Err("图库文件路径超出应用数据目录，已取消本批次导入".into());
            }
        };
        let existing_row = existing
            .get(&entry.frame_number)
            .and_then(|rows| rows.first());
        let operation = if let Some((photo_id, _, _, _)) = existing_row {
            let query = if version == "lab" {
                "UPDATE photos SET lab_scan_path = ? WHERE id = ?"
            } else {
                "UPDATE photos SET edit_scan_path = ? WHERE id = ?"
            };
            let result = sqlx::query(query)
                .bind(&stored_path)
                .bind(photo_id)
                .execute(&mut *transaction)
                .await;
            if version == "lab" {
                preview_invalidations.push((*photo_id, roll_id));
            }
            updated_count += 1;
            result.map(|_| ())
        } else {
            let query = if version == "lab" {
                "INSERT INTO photos (roll_id, frame_number, lab_scan_path, is_favorite) VALUES (?, ?, ?, 0)"
            } else {
                "INSERT INTO photos (roll_id, frame_number, edit_scan_path, is_favorite) VALUES (?, ?, ?, 0)"
            };
            let result = sqlx::query(query)
                .bind(roll_id)
                .bind(entry.frame_number)
                .bind(&stored_path)
                .execute(&mut *transaction)
                .await;
            if let Ok(result) = &result {
                if version == "lab" {
                    preview_invalidations.push((result.last_insert_rowid(), roll_id));
                }
            }
            imported_count += 1;
            result.map(|_| ())
        };
        if let Err(error) = operation {
            cleanup_copied_files(&copied_files).await;
            let _ = transaction.rollback().await;
            return Err(format!("写入照片记录失败，已取消本批次导入: {error}"));
        }
    }
    if let Err(error) = transaction.commit().await {
        cleanup_copied_files(&copied_files).await;
        return Err(format!("提交照片导入事务失败: {error}"));
    }
    // 正式图库和数据库已提交后，只失效可再生预览；预览可在下次查看时重新生成。
    for (photo_id, roll_id) in preview_invalidations {
        let preview_path = preview_file_path(&preview_dir, roll_id, photo_id);
        let _ = tokio::fs::remove_file(preview_path).await;
    }
    if version == "edit" {
        let _ = tokio::fs::remove_file(roll_cover_preview_path(&preview_dir, roll_id)).await;
    }
    Ok(ImportResultResponse {
        imported_count,
        updated_count,
        skipped_count,
    })
}

async fn lab_original_for_photo(state: &AppState, photo_id: i64) -> Result<(i64, PathBuf), String> {
    ensure_positive_id(photo_id, "照片编号")?;
    let row: Option<(i64, Option<String>)> =
        sqlx::query_as("SELECT roll_id, lab_scan_path FROM photos WHERE id = ?")
            .bind(photo_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|error| format!("读取原始扫描记录失败: {error}"))?;
    let (roll_id, stored_path) = row.ok_or_else(|| "照片记录不存在".to_string())?;
    let stored_path = stored_path.ok_or_else(|| "该 Frame 没有原始扫描图".to_string())?;
    let (media_dir, _) = library_directories(state)?;
    let original_path = resolve_stored_path_buf(&media_dir, &stored_path)
        .ok_or_else(|| "原始扫描路径无效".to_string())?;
    if !original_path.is_file() {
        return Err(format!("原始扫描文件不存在: {}", original_path.display()));
    }
    Ok((roll_id, original_path))
}

#[tauri::command]
async fn get_lab_preview(
    state: tauri::State<'_, AppState>,
    photo_id: i64,
) -> Result<LabPreviewResponse, String> {
    get_lab_preview_inner(&state, photo_id).await
}

async fn get_lab_preview_inner(
    state: &AppState,
    photo_id: i64,
) -> Result<LabPreviewResponse, String> {
    let _gallery_guard = state.gallery_operation_lock.read().await;
    let (roll_id, original_path) = lab_original_for_photo(state, photo_id).await?;
    let (_, preview_dir) = library_directories(state)?;
    let preview_path = preview_file_path(&preview_dir, roll_id, photo_id);
    let task_lock = preview_task_lock(state, format!("lab:{photo_id}")).await;
    let _task_guard = task_lock.lock().await;
    if !preview_is_fresh(&original_path, &preview_path).await {
        build_preview_file(
            original_path,
            preview_path.clone(),
            build_lab_preview,
            "原始扫描预览",
        )
        .await?;
    }

    Ok(LabPreviewResponse {
        photo_id,
        preview_path: preview_path.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
async fn get_roll_cover_preview(
    state: tauri::State<'_, AppState>,
    roll_id: i64,
) -> Result<String, String> {
    ensure_positive_id(roll_id, "拍摄卷编号")?;
    let _gallery_guard = state.gallery_operation_lock.read().await;
    let stored_path: Option<String> = sqlx::query_scalar(
        "SELECT edit_scan_path FROM photos WHERE roll_id = ? AND edit_scan_path IS NOT NULL ORDER BY frame_number, id LIMIT 1",
    )
    .bind(roll_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|error| format!("读取拍摄卷封面失败: {error}"))?
    .flatten();
    let stored_path = stored_path.ok_or_else(|| "该拍摄卷没有可用的调色封面".to_string())?;
    let (media_dir, preview_dir) = library_directories(&state)?;
    let original_path = resolve_stored_path_buf(&media_dir, &stored_path)
        .ok_or_else(|| "拍摄卷封面路径无效".to_string())?;
    if !original_path.is_file() {
        return Err(format!("拍摄卷封面文件不存在: {}", original_path.display()));
    }
    let preview_path = roll_cover_preview_path(&preview_dir, roll_id);
    let task_lock = preview_task_lock(&state, format!("cover:{roll_id}")).await;
    let _task_guard = task_lock.lock().await;
    if !preview_is_fresh(&original_path, &preview_path).await {
        build_preview_file(
            original_path,
            preview_path.clone(),
            build_roll_cover_preview,
            "拍摄卷封面缩略图",
        )
        .await?;
    }
    Ok(preview_path.to_string_lossy().into_owned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let resources = tauri::async_runtime::block_on(db::init_db(app))
                .map_err(|e| format!("数据库初始化失败: {e}"))?;
            let library = library::load_runtime(&resources.app_data_dir);
            app.manage(AppState {
                db: resources.pool,
                app_data_dir: resources.app_data_dir,
                library: RwLock::new(library),
                gallery_operation_lock: tokio::sync::RwLock::new(()),
                preview_locks: tokio::sync::Mutex::new(HashMap::new()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_dashboard_stats,
            get_cameras,
            get_films,
            get_rolls,
            get_roll_detail,
            add_camera,
            add_film_stock,
            add_roll,
            update_film_stock,
            delete_film_stock,
            update_roll,
            delete_roll,
            update_camera,
            delete_camera,
            get_camera_detail,
            toggle_photo_favorite,
            delete_photo,
            analyze_photo_import,
            import_photo_versions,
            get_roll_media_directory,
            get_lab_preview,
            get_roll_cover_preview,
            get_library_status,
            set_library_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn required_text_is_trimmed_and_checked() {
        assert_eq!(
            clean_required("  Nikon  ".into(), "品牌", 80).unwrap(),
            "Nikon"
        );
        assert!(clean_required("   ".into(), "品牌", 80).is_err());
    }

    #[test]
    fn film_status_rejects_removed_untested_value() {
        assert_eq!(validate_film_target_status(None).unwrap(), "unshot");
        assert_eq!(
            validate_film_target_status(Some("shot".into())).unwrap(),
            "shot"
        );
        assert!(validate_film_target_status(Some("untested".into())).is_err());
    }

    #[test]
    fn film_display_name_adds_brand_only_once() {
        assert_eq!(film_display_name("Kodak", "Ektar 100"), "Kodak Ektar 100");
        assert_eq!(
            film_display_name("Kodak", "Kodak Gold 200"),
            "Kodak Gold 200"
        );
    }

    #[test]
    fn dates_and_months_reject_invalid_values() {
        assert_eq!(
            validate_date(Some("2024-02-29".into()), "购买日期").unwrap(),
            Some("2024-02-29".into())
        );
        assert!(validate_date(Some("2023-02-29".into()), "购买日期").is_err());
        assert!(validate_shot_month(Some("2026-13".into())).is_err());
    }

    #[tokio::test]
    async fn purchase_date_rejects_future_values() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("open test database");
        assert!(validate_purchase_date(&pool, Some("9999-12-31".into()))
            .await
            .is_err());
        assert!(validate_purchase_date(&pool, None).await.unwrap().is_none());
    }

    #[test]
    fn media_paths_cannot_escape_the_application_gallery() {
        let media_dir = Path::new("C:/app/media");
        assert!(safe_media_path(media_dir, Path::new("rolls/1/edit/photo.jpg")).is_some());
        assert!(safe_media_path(media_dir, Path::new("../private.jpg")).is_none());
        assert!(safe_media_path(media_dir, Path::new("C:/outside.jpg")).is_none());
        assert!(safe_media_path(media_dir, Path::new("rolls/1/edit/08/batch/photo.jpg")).is_none());
        assert!(safe_media_path(media_dir, Path::new("legacy/photo.jpg")).is_none());
    }

    #[test]
    fn frame_number_uses_the_last_two_filename_digits() {
        assert_eq!(
            detect_frame_number(Path::new("260203000031430001.tif")),
            Some(1)
        );
        assert_eq!(
            detect_frame_number(Path::new("260203000031430024.tif")),
            Some(24)
        );
        assert_eq!(detect_frame_number(Path::new("scan_08.png")), Some(8));
        assert_eq!(detect_frame_number(Path::new("scan_00.tif")), None);
        assert_eq!(detect_frame_number(Path::new("scan.tif")), None);
    }

    #[test]
    fn tiff_preview_is_generated_separately_from_the_original() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time after epoch")
            .as_nanos();
        let test_dir = std::env::temp_dir().join(format!(
            "goshootfilm-preview-test-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&test_dir).expect("create preview test directory");
        let original = test_dir.join("scan_08.tif");
        let preview = test_dir.join("preview.png");
        let image = image::RgbImage::from_pixel(4, 3, image::Rgb([20, 40, 60]));
        image
            .save_with_format(&original, image::ImageFormat::Tiff)
            .expect("create TIFF fixture");

        build_lab_preview(&original, &preview).expect("build TIFF preview");

        assert!(original.is_file());
        assert!(preview.is_file());
        assert_ne!(original, preview);
        let decoded_preview = image::open(&preview).expect("read preview");
        assert_eq!((decoded_preview.width(), decoded_preview.height()), (4, 3));
        std::fs::remove_dir_all(&test_dir).expect("clean preview test directory");
    }

    #[tokio::test]
    async fn concurrent_requests_share_one_complete_tiff_preview() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time after epoch")
            .as_nanos();
        let test_dir = std::env::temp_dir().join(format!(
            "goshootfilm-preview-race-test-{}-{unique}",
            std::process::id()
        ));
        let original_dir = test_dir.join("media/rolls/1/lab");
        std::fs::create_dir_all(&original_dir).expect("create original directory");
        std::fs::create_dir_all(test_dir.join("previews/rolls")).expect("create preview directory");
        let original = original_dir.join("scan_08.tif");
        image::RgbImage::from_pixel(16, 12, image::Rgb([20, 40, 60]))
            .save_with_format(&original, image::ImageFormat::Tiff)
            .expect("create TIFF fixture");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(2)
            .connect("sqlite::memory:")
            .await
            .expect("create database");
        sqlx::query("CREATE TABLE photos (id INTEGER PRIMARY KEY, roll_id INTEGER NOT NULL, lab_scan_path TEXT)")
            .execute(&pool)
            .await
            .expect("create photos table");
        sqlx::query("INSERT INTO photos (id, roll_id, lab_scan_path) VALUES (1, 1, 'rolls/1/lab/scan_08.tif')")
            .execute(&pool)
            .await
            .expect("create photo row");
        let state = AppState {
            db: pool.clone(),
            app_data_dir: test_dir.clone(),
            library: RwLock::new(library::LibraryRuntime {
                configured_path: Some(test_dir.clone()),
                active_root: Some(test_dir.clone()),
                legacy_root: None,
                error: None,
            }),
            gallery_operation_lock: tokio::sync::RwLock::new(()),
            preview_locks: tokio::sync::Mutex::new(HashMap::new()),
        };

        let (first, second) = tokio::join!(
            get_lab_preview_inner(&state, 1),
            get_lab_preview_inner(&state, 1)
        );
        let first = first.expect("first preview request");
        let second = second.expect("second preview request");
        assert_eq!(first.preview_path, second.preview_path);
        let preview = image::open(&first.preview_path).unwrap();
        assert_eq!((preview.width(), preview.height()), (16, 12));
        let preview_dir = test_dir.join("previews/rolls/1");
        assert!(std::fs::read_dir(&preview_dir).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("building")));
        pool.close().await;
        std::fs::remove_dir_all(&test_dir).expect("clean preview race test directory");
    }

    #[tokio::test]
    async fn lab_and_edit_imports_pair_atomically_on_the_same_frame() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time after epoch")
            .as_nanos();
        let test_dir = std::env::temp_dir().join(format!(
            "goshootfilm-import-test-{}-{unique}",
            std::process::id()
        ));
        let media_dir = test_dir.join("media");
        let preview_dir = test_dir.join("previews");
        std::fs::create_dir_all(&media_dir).expect("create media directory");
        std::fs::create_dir_all(&preview_dir).expect("create preview directory");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create database");
        sqlx::query("CREATE TABLE rolls (id INTEGER PRIMARY KEY)")
            .execute(&pool)
            .await
            .expect("create rolls table");
        sqlx::query(
            "CREATE TABLE photos (id INTEGER PRIMARY KEY AUTOINCREMENT, roll_id INTEGER NOT NULL, frame_number INTEGER, lab_scan_path TEXT, edit_scan_path TEXT, is_favorite INTEGER DEFAULT 0)",
        )
        .execute(&pool)
        .await
        .expect("create photos table");
        sqlx::query("INSERT INTO rolls (id) VALUES (1)")
            .execute(&pool)
            .await
            .expect("create roll");

        let lab_source = test_dir.join("260203000031430008.tif");
        let edit_source = test_dir.join("edit_08.png");
        let image = image::RgbImage::from_pixel(4, 3, image::Rgb([30, 60, 90]));
        image
            .save_with_format(&lab_source, image::ImageFormat::Tiff)
            .expect("create TIFF source");
        image
            .save_with_format(&edit_source, image::ImageFormat::Png)
            .expect("create edit source");
        let state = AppState {
            db: pool.clone(),
            app_data_dir: test_dir.clone(),
            library: RwLock::new(library::LibraryRuntime {
                configured_path: Some(test_dir.clone()),
                active_root: Some(test_dir.clone()),
                legacy_root: None,
                error: None,
            }),
            gallery_operation_lock: tokio::sync::RwLock::new(()),
            preview_locks: tokio::sync::Mutex::new(HashMap::new()),
        };

        let lab_result = import_photo_versions_inner(
            &state,
            1,
            "lab".into(),
            vec![PhotoImportEntry {
                source_path: lab_source.to_string_lossy().into_owned(),
                frame_number: 8,
                conflict_action: "add".into(),
            }],
        )
        .await
        .expect("import lab scan");
        assert_eq!(
            (lab_result.imported_count, lab_result.updated_count),
            (1, 0)
        );

        let edit_result = import_photo_versions_inner(
            &state,
            1,
            "edit".into(),
            vec![PhotoImportEntry {
                source_path: edit_source.to_string_lossy().into_owned(),
                frame_number: 8,
                conflict_action: "add".into(),
            }],
        )
        .await
        .expect("pair edit scan");
        assert_eq!(
            (edit_result.imported_count, edit_result.updated_count),
            (0, 1)
        );

        let row: (i64, Option<String>, Option<String>) = sqlx::query_as(
            "SELECT COUNT(*), MAX(lab_scan_path), MAX(edit_scan_path) FROM photos WHERE roll_id = 1 AND frame_number = 8",
        )
        .fetch_one(&pool)
        .await
        .expect("read paired photo");
        assert_eq!(row.0, 1);
        assert_eq!(row.1.as_deref(), Some("rolls/1/lab/260203000031430008.tif"));
        assert_eq!(row.2.as_deref(), Some("rolls/1/edit/edit_08.png"));
        assert!(media_dir
            .join("rolls/1/lab/260203000031430008.tif")
            .is_file());
        assert!(media_dir.join("rolls/1/edit/edit_08.png").is_file());

        let duplicate_source_dir = test_dir.join("duplicate-source");
        std::fs::create_dir_all(&duplicate_source_dir).expect("create duplicate source directory");
        let duplicate_name_source = duplicate_source_dir.join("260203000031430008.tif");
        image
            .save_with_format(&duplicate_name_source, image::ImageFormat::Tiff)
            .expect("create same-name TIFF source");
        import_photo_versions_inner(
            &state,
            1,
            "lab".into(),
            vec![PhotoImportEntry {
                source_path: duplicate_name_source.to_string_lossy().into_owned(),
                frame_number: 9,
                conflict_action: "add".into(),
            }],
        )
        .await
        .expect("import readable unique copy name");
        let duplicate_path: String =
            sqlx::query_scalar("SELECT lab_scan_path FROM photos WHERE frame_number = 9")
                .fetch_one(&pool)
                .await
                .expect("read unique copy path");
        assert_eq!(duplicate_path, "rolls/1/lab/260203000031430008 (2).tif");

        sqlx::query(
            "CREATE TRIGGER fail_frame_11 BEFORE INSERT ON photos WHEN NEW.frame_number = 11 BEGIN SELECT RAISE(ABORT, 'forced failure'); END",
        )
        .execute(&pool)
        .await
        .expect("create failure trigger");
        let first_batch_source = test_dir.join("atomic_10.png");
        let second_batch_source = test_dir.join("atomic_11.png");
        image
            .save_with_format(&first_batch_source, image::ImageFormat::Png)
            .expect("create first atomic source");
        image
            .save_with_format(&second_batch_source, image::ImageFormat::Png)
            .expect("create second atomic source");
        let failed_batch = import_photo_versions_inner(
            &state,
            1,
            "edit".into(),
            vec![
                PhotoImportEntry {
                    source_path: first_batch_source.to_string_lossy().into_owned(),
                    frame_number: 10,
                    conflict_action: "add".into(),
                },
                PhotoImportEntry {
                    source_path: second_batch_source.to_string_lossy().into_owned(),
                    frame_number: 11,
                    conflict_action: "add".into(),
                },
            ],
        )
        .await;
        assert!(failed_batch.is_err());
        let failed_rows: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM photos WHERE frame_number IN (10, 11)")
                .fetch_one(&pool)
                .await
                .expect("count rolled-back rows");
        assert_eq!(failed_rows, 0);
        assert!(!media_dir.join("rolls/1/edit/atomic_10.png").exists());
        assert!(!media_dir.join("rolls/1/edit/atomic_11.png").exists());

        let conflict = import_photo_versions_inner(
            &state,
            1,
            "edit".into(),
            vec![PhotoImportEntry {
                source_path: edit_source.to_string_lossy().into_owned(),
                frame_number: 8,
                conflict_action: "add".into(),
            }],
        )
        .await;
        assert!(conflict.is_err());
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM photos")
            .fetch_one(&pool)
            .await
            .expect("count photos");
        assert_eq!(count, 2);

        pool.close().await;
        std::fs::remove_dir_all(&test_dir).expect("clean import test directory");
    }
}
