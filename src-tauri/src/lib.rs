mod db;
mod models;

use models::{
    CameraDetailResponse, CameraResponse, CameraRollResponse, DashboardStatsResponse, FilmResponse,
    PhotoResponse, RollDetailResponse, RollSummaryResponse,
};
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use tauri::Manager;

// 定义一个结构体用来在全局保存数据库连接池
pub struct AppState {
    pub db: SqlitePool,
    pub media_dir: PathBuf,
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

fn clean_required(value: String, field: &str, max_len: usize) -> Result<String, String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(format!("{field}不能为空"));
    }
    if value.chars().count() > max_len {
        return Err(format!("{field}不能超过{max_len}个字符"));
    }
    Ok(value)
}

fn clean_optional(
    value: Option<String>,
    field: &str,
    max_len: usize,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().to_string();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max_len {
        return Err(format!("{field}不能超过{max_len}个字符"));
    }
    Ok(Some(value))
}

fn validate_shot_month(value: Option<String>) -> Result<Option<String>, String> {
    let value = clean_optional(value, "拍摄月份", 7)?;
    let Some(value) = value else {
        return Ok(None);
    };
    let bytes = value.as_bytes();
    let valid_shape = bytes.len() == 7
        && bytes[4] == b'-'
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..].iter().all(u8::is_ascii_digit);
    let valid_month = value[5..]
        .parse::<u8>()
        .is_ok_and(|month| (1..=12).contains(&month));
    if !valid_shape || !valid_month {
        return Err("拍摄月份必须采用 YYYY-MM 格式".into());
    }
    Ok(Some(value))
}

fn validate_date(value: Option<String>, field: &str) -> Result<Option<String>, String> {
    let value = clean_optional(value, field, 10)?;
    let Some(value) = value else {
        return Ok(None);
    };
    let parts: Vec<_> = value.split('-').collect();
    let valid_shape = parts.len() == 3
        && parts[0].len() == 4
        && parts[1].len() == 2
        && parts[2].len() == 2
        && parts
            .iter()
            .all(|part| part.chars().all(|ch| ch.is_ascii_digit()));
    let valid_date = if valid_shape {
        let year = parts[0].parse::<u32>().unwrap_or_default();
        let month = parts[1].parse::<u8>().unwrap_or_default();
        let day = parts[2].parse::<u8>().unwrap_or_default();
        let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let max_day = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap_year => 29,
            2 => 28,
            _ => 0,
        };
        year > 0 && day > 0 && day <= max_day
    } else {
        false
    };
    if !valid_date {
        return Err(format!("{field}必须采用 YYYY-MM-DD 格式"));
    }
    Ok(Some(value))
}

fn database_error(action: &str, error: sqlx::Error) -> String {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.is_unique_violation() {
            return format!("{action}失败：相同记录已存在");
        }
        if database_error.is_foreign_key_violation() {
            return format!("{action}失败：关联的记录不存在或仍被使用");
        }
    }
    format!("{action}失败: {error}")
}

fn ensure_positive_id(id: i64, field: &str) -> Result<(), String> {
    if id <= 0 {
        return Err(format!("{field}无效"));
    }
    Ok(())
}

fn ensure_changed(rows_affected: u64, entity: &str) -> Result<(), String> {
    if rows_affected == 0 {
        return Err(format!("未找到要操作的{entity}"));
    }
    Ok(())
}

fn safe_media_path(media_dir: &Path, stored_path: &Path) -> Option<PathBuf> {
    if stored_path.is_absolute()
        || stored_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return None;
    }
    Some(media_dir.join(stored_path))
}

fn resolve_stored_path(media_dir: &Path, stored_path: Option<String>) -> Option<String> {
    stored_path.and_then(|stored_path| {
        let path = Path::new(&stored_path);
        if path.is_absolute() {
            Some(stored_path)
        } else {
            safe_media_path(media_dir, path).map(|path| path.to_string_lossy().into_owned())
        }
    })
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
            c.brand, c.model, f.brand, f.name,
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
                cover_path,
                photo_count,
            )| {
                RollSummaryResponse {
                    id,
                    camera_id,
                    film_id,
                    index,
                    shot_month,
                    city,
                    note,
                    camera_info: format!("{} {}", camera_brand, camera_model),
                    film_info: format!("{} {}", film_brand, film_name),
                    camera_brand,
                    camera_model,
                    film_brand,
                    film_name,
                    cover_path: resolve_stored_path(&state.media_dir, cover_path),
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
               c.brand, c.model, f.brand, f.name
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
    ) = roll.ok_or_else(|| "未找到拍摄卷".to_string())?;

    let photos: Vec<PhotoRow> = sqlx::query_as(
            "SELECT id, frame_number, lab_scan_path, edit_scan_path, is_favorite FROM photos WHERE roll_id = ? ORDER BY frame_number, id",
        )
        .bind(id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| format!("读取照片失败: {e}"))?;

    let photo_count = photos.len() as i64;
    let photos = photos
        .into_iter()
        .map(
            |(id, frame_number, lab_scan_path, edit_scan_path, is_favorite)| PhotoResponse {
                id,
                frame_number,
                lab_scan_path: resolve_stored_path(&state.media_dir, lab_scan_path),
                edit_scan_path: resolve_stored_path(&state.media_dir, edit_scan_path),
                is_favorite: is_favorite.unwrap_or(0) != 0,
            },
        )
        .collect();

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
            film_info: format!("{} {}", film_brand, film_name),
            camera_brand,
            camera_model,
            film_brand,
            film_name,
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
    let purchase_date = validate_date(purchase_date, "购买日期")?;
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
    let target_status = target_status.unwrap_or_else(|| "untested".to_string());
    if !matches!(target_status.as_str(), "untested" | "unshot" | "shot") {
        return Err("胶片状态无效".into());
    }
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

    let next_roll_index: i32 = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT MAX(roll_index) FROM rolls WHERE camera_id = ?",
    )
    .bind(camera_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| format!("计算拍摄卷序号失败: {e}"))?
    .unwrap_or(0)
        + 1;

    let result = sqlx::query(
        "INSERT INTO rolls (camera_id, film_stock_id, roll_index, shot_month, city, note) VALUES (?, ?, ?, ?, ?, ?)"
    )
        .bind(camera_id)
        .bind(film_stock_id)
        .bind(next_roll_index)
        .bind(shot_month)
        .bind(city)
        .bind(note)
        .execute(&state.db)
        .await
        .map_err(|e| database_error("新增拍摄卷", e))?;

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
    let target_status = target_status.unwrap_or_else(|| "untested".to_string());
    if !matches!(target_status.as_str(), "untested" | "unshot" | "shot") {
        return Err("胶片状态无效".into());
    }
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
    let roll_row: Option<(i64, i64, i64)> =
        sqlx::query_as("SELECT id, camera_id, film_stock_id FROM rolls WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| format!("读取拍摄卷失败: {e}"))?;

    let Some((_roll_id, old_camera_id, _old_film_stock_id)) = roll_row else {
        return Err("拍摄卷不存在".into());
    };

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

    let next_roll_index = if camera_id != old_camera_id {
        sqlx::query_scalar::<_, Option<i32>>(
            "SELECT MAX(roll_index) FROM rolls WHERE camera_id = ?",
        )
        .bind(camera_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or(0)
            + 1
    } else {
        sqlx::query_scalar::<_, Option<i32>>("SELECT roll_index FROM rolls WHERE id = ?")
            .bind(id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| e.to_string())?
            .unwrap_or(1)
    };

    let result = sqlx::query(
        "UPDATE rolls SET camera_id = ?, film_stock_id = ?, roll_index = ?, shot_month = ?, city = ?, note = ? WHERE id = ?",
    )
    .bind(camera_id)
    .bind(film_stock_id)
    .bind(next_roll_index)
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
    let purchase_date = validate_date(purchase_date, "购买日期")?;
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
    let result = sqlx::query("DELETE FROM cameras WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| database_error("删除相机", e))?;

    ensure_changed(result.rows_affected(), "相机")?;

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
                film_info: format!("{} {} (ISO {})", f_brand, f_name, f_iso),
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

// 删除照片
#[tauri::command]
async fn delete_photo(state: tauri::State<'_, AppState>, photo_id: i64) -> Result<(), String> {
    ensure_positive_id(photo_id, "照片编号")?;
    let stored_paths: Option<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT lab_scan_path, edit_scan_path FROM photos WHERE id = ?")
            .bind(photo_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| format!("读取照片记录失败: {e}"))?;
    let Some((lab_scan_path, edit_scan_path)) = stored_paths else {
        return Err("未找到要操作的照片".into());
    };

    let result = sqlx::query("DELETE FROM photos WHERE id = ?")
        .bind(photo_id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    ensure_changed(result.rows_affected(), "照片")?;

    let mut removed_paths = HashSet::new();
    for stored_path in [lab_scan_path, edit_scan_path].into_iter().flatten() {
        let relative_path = Path::new(&stored_path);
        if !removed_paths.insert(stored_path.clone()) {
            continue;
        }
        let Some(absolute_path) = safe_media_path(&state.media_dir, relative_path) else {
            continue;
        };
        if let Err(error) = tokio::fs::remove_file(&absolute_path).await {
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err(format!(
                    "照片记录已移除，但应用内副本清理失败（{}）: {error}",
                    absolute_path.display()
                ));
            }
        }
    }
    Ok(())
}

async fn cleanup_copied_files(paths: &[PathBuf]) {
    for path in paths {
        let _ = tokio::fs::remove_file(path).await;
    }
}

// 拖拽或对话框导入照片到指定胶卷（自动计算并填充连续的 frame_number）
#[tauri::command]
async fn import_photos(
    state: tauri::State<'_, AppState>,
    roll_id: i64,
    file_paths: Vec<String>,
) -> Result<usize, String> {
    ensure_positive_id(roll_id, "拍摄卷编号")?;
    if file_paths.is_empty() {
        return Err("没有可导入的照片".into());
    }

    let allowed_extensions = ["png", "jpg", "jpeg", "tif", "tiff", "webp"];
    let mut normalized_paths = Vec::new();
    let mut unique_paths = HashSet::new();
    for raw_path in file_paths {
        let path = Path::new(&raw_path);
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .ok_or_else(|| format!("无法识别文件类型: {raw_path}"))?;
        if !allowed_extensions.contains(&extension.as_str()) {
            return Err(format!("不支持的图片格式: {raw_path}"));
        }
        let canonical = path
            .canonicalize()
            .map_err(|_| format!("照片文件不存在或无法访问: {raw_path}"))?;
        if !canonical.is_file() {
            return Err(format!("路径不是文件: {raw_path}"));
        }
        let canonical = canonical.to_string_lossy();
        let canonical = if let Some(path) = canonical.strip_prefix("\\\\?\\UNC\\") {
            format!("\\\\{path}")
        } else {
            canonical
                .strip_prefix("\\\\?\\")
                .unwrap_or(&canonical)
                .to_string()
        };
        if unique_paths.insert(canonical.clone()) {
            normalized_paths.push(canonical);
        }
    }

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

    let existing_frames: Vec<i32> = sqlx::query_scalar(
        "SELECT frame_number FROM photos WHERE roll_id = ? AND frame_number IS NOT NULL",
    )
    .bind(roll_id)
    .fetch_all(&mut *transaction)
    .await
    .map_err(|e| e.to_string())?;
    let mut used_frames: HashSet<i32> = existing_frames.into_iter().collect();

    let max_frame: Option<i32> =
        sqlx::query_scalar("SELECT MAX(frame_number) FROM photos WHERE roll_id = ?")
            .bind(roll_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|e| e.to_string())?
            .flatten();

    let mut next_frame = max_frame.unwrap_or(0) + 1;

    let relative_dir = PathBuf::from("rolls").join(roll_id.to_string());
    let absolute_dir = state.media_dir.join(&relative_dir);
    tokio::fs::create_dir_all(&absolute_dir)
        .await
        .map_err(|e| format!("无法创建照片存储目录: {e}"))?;

    let mut imported_count = 0;
    let mut copied_files = Vec::new();
    for path in normalized_paths {
        let filename_frame = Path::new(&path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .and_then(|stem| {
                let suffix: String = stem
                    .chars()
                    .rev()
                    .take(2)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                if suffix.len() == 2 && suffix.chars().all(|ch| ch.is_ascii_digit()) {
                    suffix.parse::<i32>().ok()
                } else {
                    None
                }
            })
            .filter(|frame| *frame > 0 && !used_frames.contains(frame));

        let frame_number = filename_frame.unwrap_or_else(|| {
            while used_frames.contains(&next_frame) {
                next_frame += 1;
            }
            next_frame
        });

        let original_name = Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("无法读取照片文件名: {path}"))?;
        let mut copy_index = 0_u32;
        let (relative_path, absolute_path) = loop {
            let target_name = if copy_index == 0 {
                format!("{frame_number:03}_{original_name}")
            } else {
                format!("{frame_number:03}_{copy_index}_{original_name}")
            };
            let relative_path = relative_dir.join(target_name);
            let absolute_path = state.media_dir.join(&relative_path);
            if !absolute_path.exists() {
                break (relative_path, absolute_path);
            }
            copy_index += 1;
        };

        if let Err(error) = tokio::fs::copy(&path, &absolute_path).await {
            cleanup_copied_files(&copied_files).await;
            let _ = transaction.rollback().await;
            return Err(format!("复制照片到应用目录失败（{path}）: {error}"));
        }
        copied_files.push(absolute_path);
        let stored_path = relative_path.to_string_lossy().replace('\\', "/");

        if let Err(error) = sqlx::query(
            "INSERT INTO photos (roll_id, frame_number, edit_scan_path, is_favorite) VALUES (?, ?, ?, 0)"
        )
            .bind(roll_id)
            .bind(frame_number)
            .bind(stored_path)
            .execute(&mut *transaction)
            .await
        {
            cleanup_copied_files(&copied_files).await;
            let _ = transaction.rollback().await;
            return Err(format!("写入照片记录失败，已取消本批次导入: {error}"));
        }

        imported_count += 1;
        used_frames.insert(frame_number);
        if frame_number >= next_frame {
            next_frame = frame_number + 1;
        }
    }
    if let Err(error) = transaction.commit().await {
        cleanup_copied_files(&copied_files).await;
        return Err(format!("提交照片导入事务失败: {error}"));
    }
    Ok(imported_count)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let resources = tauri::async_runtime::block_on(db::init_db(app))
                .map_err(|e| format!("数据库初始化失败: {e}"))?;
            app.manage(AppState {
                db: resources.pool,
                media_dir: resources.media_dir,
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
            update_roll,
            update_camera,
            delete_camera,
            get_camera_detail,
            toggle_photo_favorite,
            delete_photo,
            import_photos
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
    fn dates_and_months_reject_invalid_values() {
        assert_eq!(
            validate_date(Some("2024-02-29".into()), "购买日期").unwrap(),
            Some("2024-02-29".into())
        );
        assert!(validate_date(Some("2023-02-29".into()), "购买日期").is_err());
        assert!(validate_shot_month(Some("2026-13".into())).is_err());
    }

    #[test]
    fn media_paths_cannot_escape_the_application_gallery() {
        let media_dir = Path::new("C:/app/media");
        assert!(safe_media_path(media_dir, Path::new("rolls/1/photo.jpg")).is_some());
        assert!(safe_media_path(media_dir, Path::new("../private.jpg")).is_none());
        assert!(safe_media_path(media_dir, Path::new("C:/outside.jpg")).is_none());
    }
}
