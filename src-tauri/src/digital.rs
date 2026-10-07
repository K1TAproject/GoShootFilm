use crate::library;
use crate::models::{
    DigitalAlbumDetailResponse, DigitalAlbumResponse, DigitalImportAnalysisItemResponse,
    DigitalImportEntry, DigitalPhotoResponse, ImportResultResponse, LabPreviewResponse,
};
use crate::validation::{
    clean_optional, clean_required, database_error, ensure_changed, ensure_positive_id,
    validate_shot_month,
};
use crate::{library_directories, preview_task_lock, AppState, PHOTO_THUMBNAIL_MAX_EDGE};
use image::ImageFormat;
use sqlx::{Row, Sqlite, Transaction};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use tauri_plugin_opener::OpenerExt;

type AlbumRow = (
    i64,
    String,
    Option<i64>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    i64,
    i64,
    i64,
    Option<i64>,
    Option<String>,
);

type PhotoRow = (i64, String, Option<String>, Option<String>, i64);

fn album_response(row: AlbumRow) -> DigitalAlbumResponse {
    let (
        id,
        _title,
        camera_id,
        shot_date,
        city,
        note,
        camera_brand,
        camera_model,
        photo_count,
        raw_count,
        edit_count,
        cover_photo_id,
        cover_version,
    ) = row;
    DigitalAlbumResponse {
        id,
        title: album_display_title(shot_date.as_deref(), city.as_deref()),
        camera_id,
        shot_date,
        city,
        note,
        camera_brand,
        camera_model,
        photo_count,
        raw_count,
        edit_count,
        cover_photo_id,
        cover_version,
    }
}

fn album_display_title(shot_month: Option<&str>, city: Option<&str>) -> String {
    let month = shot_month
        .map(|value| value.get(..7).unwrap_or(value))
        .map(|value| {
            value
                .split_once('-')
                .map(|(year, month)| format!("{year}年{month}月"))
                .unwrap_or_else(|| value.to_string())
        });
    match (month, city.filter(|value| !value.is_empty())) {
        (Some(month), Some(city)) => format!("{month} · {city}"),
        (Some(month), None) => month,
        (None, Some(city)) => city.to_string(),
        (None, None) => "未命名相册".into(),
    }
}

fn photo_response(row: PhotoRow) -> DigitalPhotoResponse {
    DigitalPhotoResponse {
        id: row.0,
        pairing_key: row.1,
        raw_path: row.2,
        edit_path: row.3,
        is_favorite: row.4 != 0,
    }
}

const ALBUM_SELECT: &str = r#"
    SELECT a.id, a.title, a.camera_id, a.shot_date, a.city, a.note,
           c.brand, c.model,
           COUNT(p.id),
           COALESCE(SUM(CASE WHEN p.raw_path IS NOT NULL THEN 1 ELSE 0 END), 0),
           COALESCE(SUM(CASE WHEN p.edit_path IS NOT NULL THEN 1 ELSE 0 END), 0),
           COALESCE(
               (SELECT ep.id FROM digital_photos ep WHERE ep.album_id = a.id AND ep.edit_path IS NOT NULL ORDER BY ep.created_at, ep.id LIMIT 1),
               (SELECT rp.id FROM digital_photos rp WHERE rp.album_id = a.id AND rp.raw_path IS NOT NULL ORDER BY rp.created_at, rp.id LIMIT 1)
           ),
           CASE
               WHEN EXISTS(SELECT 1 FROM digital_photos ep WHERE ep.album_id = a.id AND ep.edit_path IS NOT NULL) THEN 'edit'
               WHEN EXISTS(SELECT 1 FROM digital_photos rp WHERE rp.album_id = a.id AND rp.raw_path IS NOT NULL) THEN 'raw'
               ELSE NULL
           END
    FROM digital_albums a
    LEFT JOIN cameras c ON c.id = a.camera_id
    LEFT JOIN digital_photos p ON p.album_id = a.id
"#;

#[tauri::command]
pub(crate) async fn get_digital_albums(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<DigitalAlbumResponse>, String> {
    let sql = format!(
        "{ALBUM_SELECT} GROUP BY a.id ORDER BY a.shot_date IS NULL, a.shot_date DESC, a.created_at DESC, a.id DESC"
    );
    let rows: Vec<AlbumRow> = sqlx::query_as(&sql)
        .fetch_all(&state.db)
        .await
        .map_err(|error| database_error("读取数码相册", error))?;
    Ok(rows.into_iter().map(album_response).collect())
}

async fn checked_camera_id(
    state: &AppState,
    camera_id: Option<i64>,
) -> Result<Option<i64>, String> {
    let Some(camera_id) = camera_id else {
        return Ok(None);
    };
    ensure_positive_id(camera_id, "相机 ID")?;
    let exists: i64 = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cameras WHERE id = ?)")
        .bind(camera_id)
        .fetch_one(&state.db)
        .await
        .map_err(|error| database_error("确认相机", error))?;
    if exists == 0 {
        return Err("所选相机不存在".into());
    }
    Ok(Some(camera_id))
}

#[tauri::command]
pub(crate) async fn add_digital_album(
    title: Option<String>,
    camera_id: Option<i64>,
    shot_date: Option<String>,
    city: Option<String>,
    note: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<i64, String> {
    let _ = title;
    let shot_date = validate_shot_month(shot_date)?;
    let city = clean_optional(city, "地点", 160)?;
    let generated_title = album_display_title(shot_date.as_deref(), city.as_deref());
    let result = sqlx::query(
        "INSERT INTO digital_albums (title, camera_id, shot_date, city, note) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(generated_title)
    .bind(checked_camera_id(&state, camera_id).await?)
    .bind(shot_date)
    .bind(city)
    .bind(clean_optional(note, "备注", 2000)?)
    .execute(&state.db)
    .await
    .map_err(|error| database_error("新建数码相册", error))?;
    Ok(result.last_insert_rowid())
}

#[tauri::command]
pub(crate) async fn update_digital_album(
    id: i64,
    title: Option<String>,
    camera_id: Option<i64>,
    shot_date: Option<String>,
    city: Option<String>,
    note: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    ensure_positive_id(id, "相册 ID")?;
    let _ = title;
    let shot_date = validate_shot_month(shot_date)?;
    let city = clean_optional(city, "地点", 160)?;
    let generated_title = album_display_title(shot_date.as_deref(), city.as_deref());
    let result = sqlx::query(
        "UPDATE digital_albums SET title = ?, camera_id = ?, shot_date = ?, city = ?, note = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(generated_title)
    .bind(checked_camera_id(&state, camera_id).await?)
    .bind(shot_date)
    .bind(city)
    .bind(clean_optional(note, "备注", 2000)?)
    .bind(id)
    .execute(&state.db)
    .await
    .map_err(|error| database_error("更新数码相册", error))?;
    ensure_changed(result.rows_affected(), "数码相册")?;
    Ok("相册已更新".into())
}

#[tauri::command]
pub(crate) async fn get_digital_album_detail(
    id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<DigitalAlbumDetailResponse, String> {
    ensure_positive_id(id, "相册 ID")?;
    let sql = format!("{ALBUM_SELECT} WHERE a.id = ? GROUP BY a.id");
    let album: AlbumRow = sqlx::query_as(&sql)
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|error| database_error("读取数码相册", error))?
        .ok_or_else(|| "数码相册不存在".to_string())?;
    let photos: Vec<PhotoRow> = sqlx::query_as(
        "SELECT id, pairing_key, raw_path, edit_path, is_favorite FROM digital_photos WHERE album_id = ? ORDER BY pairing_key COLLATE NOCASE, id",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|error| database_error("读取数码照片", error))?;
    Ok(DigitalAlbumDetailResponse {
        album: album_response(album),
        photos: photos.into_iter().map(photo_response).collect(),
    })
}

#[tauri::command]
pub(crate) async fn delete_digital_album(
    id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    ensure_positive_id(id, "相册 ID")?;
    let _guard = state.gallery_operation_lock.read().await;
    let result = sqlx::query("DELETE FROM digital_albums WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|error| database_error("删除数码相册", error))?;
    ensure_changed(result.rows_affected(), "数码相册")?;
    let (_, previews) = library_directories(&state)?;
    let directory = previews.join("digital").join(id.to_string());
    if let Err(error) = tokio::fs::remove_dir_all(directory).await {
        if error.kind() != std::io::ErrorKind::NotFound {
            return Err(format!("相册记录已删除，但预览资源清理失败: {error}"));
        }
    }
    Ok("相册记录已删除；正式图库文件已保留".into())
}

fn validate_version(version: &str) -> Result<(), String> {
    if matches!(version, "raw" | "edit") {
        Ok(())
    } else {
        Err("数码图片版本无效".into())
    }
}

fn relative_directory(album_id: i64, version: &str) -> PathBuf {
    PathBuf::from("digital")
        .join(album_id.to_string())
        .join(version)
}

fn safe_digital_path(media_dir: &Path, stored_path: &str) -> Option<PathBuf> {
    let path = Path::new(stored_path);
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
    if parts.len() != 4
        || parts[0] != OsStr::new("digital")
        || parts[1]
            .to_str()
            .and_then(|value| value.parse::<i64>().ok())
            .is_none_or(|id| id <= 0)
        || !matches!(parts[2].to_str(), Some("raw" | "edit"))
    {
        return None;
    }
    Some(media_dir.join(path))
}

#[tauri::command]
pub(crate) async fn get_digital_media_directory(
    app: tauri::AppHandle,
    album_id: i64,
    version: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    ensure_positive_id(album_id, "相册 ID")?;
    validate_version(&version)?;
    let exists: i64 =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM digital_albums WHERE id = ?)")
            .bind(album_id)
            .fetch_one(&state.db)
            .await
            .map_err(|error| database_error("确认数码相册", error))?;
    if exists == 0 {
        return Err("数码相册不存在".into());
    }
    let (media, _) = library_directories(&state)?;
    let directory = media.join(relative_directory(album_id, &version));
    if !directory.is_dir() {
        return Err("当前版本还没有图库目录".into());
    }
    let display = directory.to_string_lossy().into_owned();
    app.opener()
        .open_path(display.clone(), None::<String>)
        .map_err(|error| format!("无法打开图库目录: {error}"))?;
    Ok(display)
}

struct NormalizedSource {
    path: PathBuf,
    display: String,
    file_name: String,
    pairing_key: String,
}

fn normalize_source(
    raw: &str,
    version: &str,
    requested_key: Option<String>,
) -> Result<NormalizedSource, String> {
    validate_version(version)?;
    let path = Path::new(raw);
    let extension = path
        .extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| format!("无法识别文件类型: {raw}"))?;
    let allowed = if version == "raw" {
        extension == "cr2"
    } else {
        extension == "png"
    };
    if !allowed {
        return Err(if version == "raw" {
            format!("RAW 导入仅支持 CR2: {raw}")
        } else {
            format!("调色图导入仅支持 PNG: {raw}")
        });
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| format!("图片文件不存在或无法访问: {raw}"))?;
    if !canonical.is_file() {
        return Err(format!("路径不是文件: {raw}"));
    }
    let file_name = canonical
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| format!("无法读取文件名: {raw}"))?
        .trim()
        .to_string();
    let detected = canonical
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or_else(|| format!("无法识别配对名称: {raw}"))?;
    let pairing_key = clean_required(
        requested_key.unwrap_or_else(|| detected.to_string()),
        "配对名称",
        240,
    )?;
    Ok(NormalizedSource {
        display: canonical.to_string_lossy().into_owned(),
        path: canonical,
        file_name,
        pairing_key,
    })
}

#[tauri::command]
pub(crate) async fn analyze_digital_import(
    album_id: i64,
    version: String,
    source_paths: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<DigitalImportAnalysisItemResponse>, String> {
    ensure_positive_id(album_id, "相册 ID")?;
    validate_version(&version)?;
    if source_paths.is_empty() {
        return Err("没有可导入的图片".into());
    }
    let rows = sqlx::query(
        "SELECT pairing_key, raw_path, edit_path FROM digital_photos WHERE album_id = ?",
    )
    .bind(album_id)
    .fetch_all(&state.db)
    .await
    .map_err(|error| database_error("读取已有数码照片", error))?;
    let mut normalized = Vec::new();
    let mut source_seen = HashSet::new();
    let mut key_seen = HashSet::new();
    for source in source_paths {
        let item = normalize_source(&source, &version, None)?;
        if !source_seen.insert(item.path.clone()) {
            return Err(format!("同一文件被重复选择: {}", item.file_name));
        }
        normalized.push(item);
    }
    Ok(normalized
        .into_iter()
        .map(|item| {
            let normalized_key = item.pairing_key.to_lowercase();
            let mut issue = None;
            if !key_seen.insert(normalized_key.clone()) {
                issue = Some("本批次存在重复配对名称，请修改后重试".into());
            }
            let existing = rows
                .iter()
                .find(|row| row.get::<String, _>(0).to_lowercase() == normalized_key);
            let existing_version = existing.is_some_and(|row| {
                if version == "raw" {
                    row.get::<Option<String>, _>(1).is_some()
                } else {
                    row.get::<Option<String>, _>(2).is_some()
                }
            });
            let paired_version = existing.is_some_and(|row| {
                if version == "raw" {
                    row.get::<Option<String>, _>(2).is_some()
                } else {
                    row.get::<Option<String>, _>(1).is_some()
                }
            });
            DigitalImportAnalysisItemResponse {
                source_path: item.display,
                file_name: item.file_name,
                pairing_key: item.pairing_key,
                existing_version,
                paired_version,
                issue,
            }
        })
        .collect())
}

fn numbered_name(file_name: &str, number: usize) -> String {
    let path = Path::new(file_name);
    let stem = path
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or(file_name);
    match path.extension().and_then(OsStr::to_str) {
        Some(extension) => format!("{stem} ({number}).{extension}"),
        None => format!("{stem} ({number})"),
    }
}

async fn copy_unique(source: &Path, directory: &Path, file_name: &str) -> Result<PathBuf, String> {
    tokio::fs::create_dir_all(directory)
        .await
        .map_err(|error| format!("无法创建数码图库目录: {error}"))?;
    for number in 1.. {
        let name = if number == 1 {
            file_name.to_string()
        } else {
            numbered_name(file_name, number)
        };
        let target = directory.join(name);
        let mut output = match tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .await
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("无法创建图库文件: {error}")),
        };
        let result = async {
            let mut input = tokio::fs::File::open(source).await?;
            tokio::io::copy(&mut input, &mut output).await?;
            output.sync_all().await
        }
        .await;
        if let Err(error) = result {
            drop(output);
            let _ = tokio::fs::remove_file(&target).await;
            return Err(format!("复制图片失败: {error}"));
        }
        return Ok(target);
    }
    unreachable!()
}

async fn remove_files(paths: &[PathBuf]) {
    for path in paths {
        let _ = tokio::fs::remove_file(path).await;
    }
}

async fn upsert_photo(
    transaction: &mut Transaction<'_, Sqlite>,
    album_id: i64,
    version: &str,
    key: &str,
    stored_path: &str,
) -> Result<bool, String> {
    let existing: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM digital_photos WHERE album_id = ? AND pairing_key = ? COLLATE NOCASE",
    )
    .bind(album_id)
    .bind(key)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|error| database_error("配对数码照片", error))?;
    if let Some(id) = existing {
        let sql = if version == "raw" {
            "UPDATE digital_photos SET raw_path = ? WHERE id = ?"
        } else {
            "UPDATE digital_photos SET edit_path = ? WHERE id = ?"
        };
        sqlx::query(sql)
            .bind(stored_path)
            .bind(id)
            .execute(&mut **transaction)
            .await
            .map_err(|error| database_error("更新数码照片版本", error))?;
        Ok(true)
    } else {
        let (raw, edit) = if version == "raw" {
            (Some(stored_path), None)
        } else {
            (None, Some(stored_path))
        };
        sqlx::query("INSERT INTO digital_photos (album_id, pairing_key, raw_path, edit_path) VALUES (?, ?, ?, ?)")
            .bind(album_id)
            .bind(key)
            .bind(raw)
            .bind(edit)
            .execute(&mut **transaction)
            .await
            .map_err(|error| database_error("新增数码照片", error))?;
        Ok(false)
    }
}

#[tauri::command]
pub(crate) async fn import_digital_photos(
    album_id: i64,
    version: String,
    entries: Vec<DigitalImportEntry>,
    state: tauri::State<'_, AppState>,
) -> Result<ImportResultResponse, String> {
    let _guard = state.gallery_operation_lock.read().await;
    import_digital_photos_inner(&state, album_id, version, entries).await
}

async fn import_digital_photos_inner(
    state: &AppState,
    album_id: i64,
    version: String,
    entries: Vec<DigitalImportEntry>,
) -> Result<ImportResultResponse, String> {
    ensure_positive_id(album_id, "相册 ID")?;
    validate_version(&version)?;
    if entries.is_empty() {
        return Err("没有可导入的图片".into());
    }
    let (media, _) = library_directories(state)?;
    let directory = media.join(relative_directory(album_id, &version));
    let mut normalized = Vec::new();
    let mut sources = HashSet::new();
    let mut keys = HashSet::new();
    let mut skipped = 0;
    for entry in entries {
        if entry.conflict_action == "cancel" {
            return Err("已取消整批导入，未写入任何数据".into());
        }
        if !matches!(entry.conflict_action.as_str(), "add" | "skip" | "replace") {
            return Err("导入冲突处理方式无效".into());
        }
        if entry.conflict_action == "skip" {
            skipped += 1;
            continue;
        }
        let source = normalize_source(&entry.source_path, &version, entry.pairing_key)?;
        if !sources.insert(source.path.clone()) {
            return Err(format!("同一文件被重复选择: {}", source.file_name));
        }
        if !keys.insert(source.pairing_key.to_lowercase()) {
            return Err(format!("本批次存在重复配对名称: {}", source.pairing_key));
        }
        normalized.push((source, entry.conflict_action));
    }
    let mut copied = Vec::new();
    let mut transaction = state
        .db
        .begin()
        .await
        .map_err(|error| database_error("开始导入", error))?;
    let exists: i64 =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM digital_albums WHERE id = ?)")
            .bind(album_id)
            .fetch_one(&mut *transaction)
            .await
            .map_err(|error| database_error("确认数码相册", error))?;
    if exists == 0 {
        return Err("数码相册不存在".into());
    }
    let mut imported = 0;
    let mut updated = 0;
    for (source, action) in normalized {
        let existing_path: Option<Option<String>> = sqlx::query_scalar(if version == "raw" {
            "SELECT raw_path FROM digital_photos WHERE album_id = ? AND pairing_key = ? COLLATE NOCASE"
        } else {
            "SELECT edit_path FROM digital_photos WHERE album_id = ? AND pairing_key = ? COLLATE NOCASE"
        })
        .bind(album_id)
        .bind(&source.pairing_key)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| database_error("检查数码照片冲突", error))?;
        if existing_path.flatten().is_some() && action != "replace" {
            remove_files(&copied).await;
            return Err(format!(
                "{} 已存在当前版本，请选择跳过、替换或取消",
                source.pairing_key
            ));
        }
        let target = match copy_unique(&source.path, &directory, &source.file_name).await {
            Ok(target) => target,
            Err(error) => {
                remove_files(&copied).await;
                return Err(error);
            }
        };
        copied.push(target.clone());
        let stored = target
            .strip_prefix(&media)
            .map_err(|_| "无法生成安全的图库相对路径".to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        match upsert_photo(
            &mut transaction,
            album_id,
            &version,
            &source.pairing_key,
            &stored,
        )
        .await
        {
            Ok(true) => updated += 1,
            Ok(false) => imported += 1,
            Err(error) => {
                remove_files(&copied).await;
                return Err(error);
            }
        }
    }
    if let Err(error) = transaction.commit().await {
        remove_files(&copied).await;
        return Err(database_error("提交数码照片导入", error));
    }
    Ok(ImportResultResponse {
        imported_count: imported,
        updated_count: updated,
        skipped_count: skipped,
    })
}

fn raw_preview_path(previews: &Path, album_id: i64, photo_id: i64) -> PathBuf {
    previews
        .join("digital")
        .join(album_id.to_string())
        .join("raw")
        .join(format!("{photo_id}.jpg"))
}

fn thumb_path(previews: &Path, album_id: i64, photo_id: i64, version: &str) -> PathBuf {
    previews
        .join("digital")
        .join(album_id.to_string())
        .join("thumbs")
        .join(format!("{version}-{photo_id}.png"))
}

#[derive(Clone, Copy)]
enum TiffEndian {
    Little,
    Big,
}

fn tiff_u16(bytes: &[u8], offset: usize, endian: TiffEndian) -> Option<u16> {
    let value: [u8; 2] = bytes.get(offset..offset + 2)?.try_into().ok()?;
    Some(match endian {
        TiffEndian::Little => u16::from_le_bytes(value),
        TiffEndian::Big => u16::from_be_bytes(value),
    })
}

fn tiff_u32(bytes: &[u8], offset: usize, endian: TiffEndian) -> Option<u32> {
    let value: [u8; 4] = bytes.get(offset..offset + 4)?.try_into().ok()?;
    Some(match endian {
        TiffEndian::Little => u32::from_le_bytes(value),
        TiffEndian::Big => u32::from_be_bytes(value),
    })
}

fn tiff_entry_values(
    bytes: &[u8],
    entry: usize,
    value_type: u16,
    count: u32,
    endian: TiffEndian,
) -> Vec<u32> {
    let unit_size = match value_type {
        3 => 2_usize,
        4 | 13 => 4_usize,
        _ => return Vec::new(),
    };
    let Ok(count) = usize::try_from(count) else {
        return Vec::new();
    };
    let Some(byte_count) = unit_size.checked_mul(count) else {
        return Vec::new();
    };
    let value_field = entry + 8;
    let start = if byte_count <= 4 {
        value_field
    } else {
        let Some(offset) =
            tiff_u32(bytes, value_field, endian).and_then(|v| usize::try_from(v).ok())
        else {
            return Vec::new();
        };
        offset
    };
    (0..count)
        .filter_map(|index| {
            let offset = start.checked_add(index.checked_mul(unit_size)?)?;
            if unit_size == 2 {
                tiff_u16(bytes, offset, endian).map(u32::from)
            } else {
                tiff_u32(bytes, offset, endian)
            }
        })
        .collect()
}

fn cr2_embedded_jpeg(bytes: &[u8]) -> Result<&[u8], String> {
    let endian = match bytes.get(0..2) {
        Some(b"II") => TiffEndian::Little,
        Some(b"MM") => TiffEndian::Big,
        _ => return Err("CR2 的 TIFF 字节序标记无效".into()),
    };
    if tiff_u16(bytes, 2, endian) != Some(42) {
        return Err("文件不是可识别的 CR2/TIFF".into());
    }
    let first_ifd = tiff_u32(bytes, 4, endian)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| "CR2 缺少 TIFF 目录".to_string())?;
    let mut pending = vec![first_ifd];
    let mut visited = HashSet::new();
    let mut jpeg_candidates = Vec::new();
    let mut strip_candidates = Vec::new();
    while let Some(ifd) = pending.pop() {
        if ifd == 0 || !visited.insert(ifd) || visited.len() > 32 {
            continue;
        }
        let Some(entry_count) = tiff_u16(bytes, ifd, endian).map(usize::from) else {
            continue;
        };
        let Some(entries_size) = entry_count.checked_mul(12) else {
            continue;
        };
        let Some(entries_end) = ifd.checked_add(2).and_then(|v| v.checked_add(entries_size)) else {
            continue;
        };
        if entries_end + 4 > bytes.len() {
            continue;
        }
        let mut jpeg_offsets = Vec::new();
        let mut jpeg_lengths = Vec::new();
        let mut strip_offsets = Vec::new();
        let mut strip_lengths = Vec::new();
        for index in 0..entry_count {
            let entry = ifd + 2 + index * 12;
            let Some(tag) = tiff_u16(bytes, entry, endian) else {
                continue;
            };
            let Some(value_type) = tiff_u16(bytes, entry + 2, endian) else {
                continue;
            };
            let Some(count) = tiff_u32(bytes, entry + 4, endian) else {
                continue;
            };
            let values = tiff_entry_values(bytes, entry, value_type, count, endian);
            match tag {
                0x0201 => jpeg_offsets = values,
                0x0202 => jpeg_lengths = values,
                0x0111 => strip_offsets = values,
                0x0117 => strip_lengths = values,
                // 子 IFD 与 Exif IFD 也可能保存预览位置。
                0x014a | 0x8769 => {
                    pending.extend(values.into_iter().filter_map(|v| usize::try_from(v).ok()))
                }
                _ => {}
            }
        }
        for (offsets, lengths, candidates) in [
            (&jpeg_offsets, &jpeg_lengths, &mut jpeg_candidates),
            (&strip_offsets, &strip_lengths, &mut strip_candidates),
        ] {
            for (offset, length) in offsets.iter().zip(lengths.iter()) {
                let (Ok(offset), Ok(length)) = (usize::try_from(*offset), usize::try_from(*length))
                else {
                    continue;
                };
                let Some(end) = offset.checked_add(length) else {
                    continue;
                };
                let Some(candidate) = bytes.get(offset..end) else {
                    continue;
                };
                if candidate.starts_with(&[0xff, 0xd8]) && candidate.ends_with(&[0xff, 0xd9]) {
                    candidates.push(candidate);
                }
            }
        }
        if let Some(next) =
            tiff_u32(bytes, entries_end, endian).and_then(|v| usize::try_from(v).ok())
        {
            pending.push(next);
        }
    }
    jpeg_candidates
        .into_iter()
        .max_by_key(|candidate| candidate.len())
        .or_else(|| {
            strip_candidates
                .into_iter()
                .min_by_key(|candidate| candidate.len())
        })
        .ok_or_else(|| "CR2 中没有可识别的内嵌 JPEG 预览".into())
}

fn extract_cr2_preview(source: &Path, target: &Path) -> Result<(), String> {
    // 第一版只提取 CR2 中已有的 JPEG，不解析或显影 RAW 像素；这样不会触碰正式原件。
    let bytes = fs::read(source)
        .map_err(|error| format!("CR2 文件不存在或无法读取（{}）: {error}", source.display()))?;
    let jpeg = cr2_embedded_jpeg(&bytes)?;
    let parent = target
        .parent()
        .ok_or_else(|| "无法确定 RAW 预览目录".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("无法创建 RAW 预览目录: {error}"))?;
    let temporary = target.with_extension(format!("tmp-{}", std::process::id()));
    let mut file =
        fs::File::create(&temporary).map_err(|error| format!("无法创建 RAW 预览: {error}"))?;
    file.write_all(jpeg)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("无法写入 RAW 预览: {error}"))?;
    library::atomic_replace(&temporary, target)
        .map_err(|error| format!("无法保存 RAW 预览: {error}"))
}

fn build_thumbnail(source: &Path, target: &Path) -> Result<(), String> {
    let image = image::open(source)
        .map_err(|error| format!("图片无法读取（{}）: {error}", source.display()))?;
    let thumbnail = image.thumbnail(PHOTO_THUMBNAIL_MAX_EDGE, PHOTO_THUMBNAIL_MAX_EDGE);
    let parent = target
        .parent()
        .ok_or_else(|| "无法确定缩略图目录".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("无法创建缩略图目录: {error}"))?;
    let temporary = target.with_extension(format!("tmp-{}", std::process::id()));
    thumbnail
        .save_with_format(&temporary, ImageFormat::Png)
        .map_err(|error| format!("生成缩略图失败: {error}"))?;
    library::atomic_replace(&temporary, target).map_err(|error| format!("保存缩略图失败: {error}"))
}

async fn source_for_photo(
    state: &AppState,
    photo_id: i64,
    version: &str,
) -> Result<(i64, PathBuf), String> {
    let row: (i64, Option<String>, Option<String>) =
        sqlx::query_as("SELECT album_id, raw_path, edit_path FROM digital_photos WHERE id = ?")
            .bind(photo_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|error| database_error("读取数码照片", error))?
            .ok_or_else(|| "数码照片不存在".to_string())?;
    let stored = if version == "raw" { row.1 } else { row.2 }
        .ok_or_else(|| "当前照片没有这个版本".to_string())?;
    let (media, _) = library_directories(state)?;
    let path =
        safe_digital_path(&media, &stored).ok_or_else(|| "数码照片图库路径无效".to_string())?;
    if !path.is_file() {
        return Err(format!("图片文件不存在: {}", path.display()));
    }
    Ok((row.0, path))
}

#[tauri::command]
pub(crate) async fn get_digital_photo_preview(
    photo_id: i64,
    version: String,
    thumbnail: bool,
    state: tauri::State<'_, AppState>,
) -> Result<LabPreviewResponse, String> {
    ensure_positive_id(photo_id, "照片 ID")?;
    validate_version(&version)?;
    let (album_id, formal_source) = source_for_photo(&state, photo_id, &version).await?;
    let (_, previews) = library_directories(&state)?;
    let preview_source = if version == "raw" {
        let raw_preview = raw_preview_path(&previews, album_id, photo_id);
        let lock = preview_task_lock(&state, format!("digital-raw:{photo_id}")).await;
        let _lock = lock.lock().await;
        let fresh = super::preview_is_fresh(&formal_source, &raw_preview).await;
        if !fresh {
            let source = formal_source.clone();
            let target = raw_preview.clone();
            let _permit = state
                .preview_generation_limit
                .acquire()
                .await
                .map_err(|_| "预览队列不可用".to_string())?;
            tauri::async_runtime::spawn_blocking(move || extract_cr2_preview(&source, &target))
                .await
                .map_err(|error| format!("CR2 预览任务失败: {error}"))??;
        }
        raw_preview
    } else {
        formal_source.clone()
    };
    let target = if thumbnail {
        let target = thumb_path(&previews, album_id, photo_id, &version);
        let lock = preview_task_lock(&state, format!("digital-thumb:{version}:{photo_id}")).await;
        let _lock = lock.lock().await;
        if !super::preview_is_fresh(&preview_source, &target).await {
            let source = preview_source.clone();
            let destination = target.clone();
            let _permit = state
                .preview_generation_limit
                .acquire()
                .await
                .map_err(|_| "缩略图队列不可用".to_string())?;
            tauri::async_runtime::spawn_blocking(move || build_thumbnail(&source, &destination))
                .await
                .map_err(|error| format!("缩略图任务失败: {error}"))??;
        }
        target
    } else {
        preview_source
    };
    Ok(LabPreviewResponse {
        photo_id,
        preview_path: target.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
pub(crate) async fn toggle_digital_photo_favorite(
    photo_id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    ensure_positive_id(photo_id, "照片 ID")?;
    let value: i64 = sqlx::query_scalar(
        "UPDATE digital_photos SET is_favorite = CASE is_favorite WHEN 0 THEN 1 ELSE 0 END WHERE id = ? RETURNING is_favorite",
    )
    .bind(photo_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|error| database_error("更新收藏状态", error))?
    .ok_or_else(|| "数码照片不存在".to_string())?;
    Ok(value != 0)
}

#[tauri::command]
pub(crate) async fn delete_digital_photo(
    photo_id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    ensure_positive_id(photo_id, "照片 ID")?;
    let album_id: i64 = sqlx::query_scalar("SELECT album_id FROM digital_photos WHERE id = ?")
        .bind(photo_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|error| database_error("读取数码照片", error))?
        .ok_or_else(|| "数码照片不存在".to_string())?;
    sqlx::query("DELETE FROM digital_photos WHERE id = ?")
        .bind(photo_id)
        .execute(&state.db)
        .await
        .map_err(|error| database_error("删除数码照片", error))?;
    let (_, previews) = library_directories(&state)?;
    for path in [
        raw_preview_path(&previews, album_id, photo_id),
        thumb_path(&previews, album_id, photo_id, "raw"),
        thumb_path(&previews, album_id, photo_id, "edit"),
    ] {
        if let Err(error) = tokio::fs::remove_file(path).await {
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err(format!("记录已删除，但预览资源清理失败: {error}"));
            }
        }
    }
    Ok("照片记录已删除；正式 CR2 和 PNG 已保留".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use std::collections::HashMap;
    use std::sync::RwLock;

    #[test]
    fn pairing_key_keeps_complete_stem() {
        let path = Path::new("IMG_1234-final.png");
        assert_eq!(
            path.file_stem().and_then(OsStr::to_str),
            Some("IMG_1234-final")
        );
    }

    #[test]
    fn album_name_comes_from_month_and_city() {
        assert_eq!(
            album_display_title(Some("2026-10"), Some("上海")),
            "2026年10月 · 上海"
        );
        assert_eq!(album_display_title(Some("2026-10-18"), None), "2026年10月");
        assert_eq!(album_display_title(None, Some("北京")), "北京");
        assert_eq!(album_display_title(None, None), "未命名相册");
    }

    #[test]
    fn digital_paths_are_narrowly_scoped() {
        let root = Path::new("C:/library/media");
        assert!(safe_digital_path(root, "digital/2/raw/IMG_1.CR2").is_some());
        assert!(safe_digital_path(root, "digital/2/edit/IMG_1.png").is_some());
        assert!(safe_digital_path(root, "digital/2/other/IMG_1.png").is_none());
        assert!(safe_digital_path(root, "../secret.CR2").is_none());
    }

    #[tokio::test]
    async fn real_cr2_embedded_preview_extracts_when_fixture_is_provided() {
        let Some(source) = std::env::var_os("GOSHOOTFILM_CR2_SAMPLE").map(PathBuf::from) else {
            return;
        };
        let root =
            std::env::temp_dir().join(format!("goshootfilm-real-cr2-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let state = import_test_state(&root).await;
        let stem = source
            .file_stem()
            .and_then(OsStr::to_str)
            .unwrap()
            .to_string();
        let edit_source = root.join(format!("{stem}.png"));
        image::RgbImage::new(8, 8)
            .save_with_format(&edit_source, ImageFormat::Png)
            .unwrap();
        for (version, path) in [("raw", source), ("edit", edit_source)] {
            import_digital_photos_inner(
                &state,
                1,
                version.into(),
                vec![DigitalImportEntry {
                    source_path: path.to_string_lossy().into_owned(),
                    pairing_key: None,
                    conflict_action: "add".into(),
                }],
            )
            .await
            .expect("import and pair real CR2 with PNG");
        }
        let (pairing_key, raw_path, edit_path): (String, String, String) =
            sqlx::query_as("SELECT pairing_key, raw_path, edit_path FROM digital_photos")
                .fetch_one(&state.db)
                .await
                .unwrap();
        assert_eq!(pairing_key, stem);
        assert!(edit_path.ends_with(&format!("/{stem}.png")));
        let formal_raw = root.join("media").join(raw_path);
        let target = root.join("preview.jpg");
        extract_cr2_preview(&formal_raw, &target).expect("extract embedded JPEG from real CR2");
        image::open(&target).expect("open extracted embedded JPEG");
        state.db.close().await;
        fs::remove_dir_all(root).expect("remove real CR2 test directory");
    }

    async fn import_test_state(root: &Path) -> AppState {
        fs::create_dir_all(root.join("media")).unwrap();
        fs::create_dir_all(root.join("previews")).unwrap();
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query("CREATE TABLE cameras (id INTEGER PRIMARY KEY)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::raw_sql(include_str!("../migrations/0005_equipment_digital.sql"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO digital_albums (title) VALUES ('Import test')")
            .execute(&pool)
            .await
            .unwrap();
        AppState {
            db: pool,
            app_data_dir: root.to_path_buf(),
            library: RwLock::new(library::LibraryRuntime {
                configured_path: Some(root.to_path_buf()),
                active_root: Some(root.to_path_buf()),
                legacy_root: None,
                error: None,
            }),
            gallery_operation_lock: tokio::sync::RwLock::new(()),
            preview_locks: tokio::sync::Mutex::new(HashMap::new()),
            preview_generation_limit: tokio::sync::Semaphore::new(1),
        }
    }

    #[tokio::test]
    async fn raw_and_png_import_pair_atomically_and_keep_names() {
        let root = std::env::temp_dir().join(format!(
            "goshootfilm-digital-import-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let raw_source = root.join("IMG_1234.CR2");
        let edit_source = root.join("IMG_1234.png");
        fs::write(&raw_source, b"raw").unwrap();
        fs::write(&edit_source, b"png").unwrap();
        let state = import_test_state(&root).await;

        import_digital_photos_inner(
            &state,
            1,
            "raw".into(),
            vec![DigitalImportEntry {
                source_path: raw_source.to_string_lossy().into_owned(),
                pairing_key: None,
                conflict_action: "add".into(),
            }],
        )
        .await
        .unwrap();
        import_digital_photos_inner(
            &state,
            1,
            "edit".into(),
            vec![DigitalImportEntry {
                source_path: edit_source.to_string_lossy().into_owned(),
                pairing_key: None,
                conflict_action: "add".into(),
            }],
        )
        .await
        .unwrap();

        let row: (String, String, String) =
            sqlx::query_as("SELECT pairing_key, raw_path, edit_path FROM digital_photos")
                .fetch_one(&state.db)
                .await
                .unwrap();
        assert_eq!(row.0, "IMG_1234");
        assert_eq!(row.1, "digital/1/raw/IMG_1234.CR2");
        assert_eq!(row.2, "digital/1/edit/IMG_1234.png");
        assert!(root.join("media").join(&row.1).is_file());
        assert!(root.join("media").join(&row.2).is_file());

        import_digital_photos_inner(
            &state,
            1,
            "raw".into(),
            vec![DigitalImportEntry {
                source_path: raw_source.to_string_lossy().into_owned(),
                pairing_key: Some("IMG_1234".into()),
                conflict_action: "replace".into(),
            }],
        )
        .await
        .unwrap();
        let replaced: String = sqlx::query_scalar("SELECT raw_path FROM digital_photos")
            .fetch_one(&state.db)
            .await
            .unwrap();
        assert_eq!(replaced, "digital/1/raw/IMG_1234 (2).CR2");
        assert!(root.join("media").join(replaced).is_file());

        let new_source = root.join("IMG_9999.CR2");
        fs::write(&new_source, b"another raw").unwrap();
        let failed = import_digital_photos_inner(
            &state,
            1,
            "raw".into(),
            vec![
                DigitalImportEntry {
                    source_path: new_source.to_string_lossy().into_owned(),
                    pairing_key: None,
                    conflict_action: "add".into(),
                },
                DigitalImportEntry {
                    source_path: raw_source.to_string_lossy().into_owned(),
                    pairing_key: Some("IMG_1234".into()),
                    conflict_action: "add".into(),
                },
            ],
        )
        .await;
        assert!(failed.is_err());
        let rolled_back: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM digital_photos WHERE pairing_key = 'IMG_9999'",
        )
        .fetch_one(&state.db)
        .await
        .unwrap();
        assert_eq!(rolled_back, 0);
        assert!(!root.join("media/digital/1/raw/IMG_9999.CR2").exists());
        state.db.close().await;
        fs::remove_dir_all(root).unwrap();
    }
}
