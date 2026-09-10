use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub library_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct LibraryRuntime {
    pub configured_path: Option<PathBuf>,
    pub active_root: Option<PathBuf>,
    pub legacy_root: Option<PathBuf>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStatusResponse {
    pub library_path: Option<String>,
    pub available: bool,
    pub needs_migration: bool,
    pub legacy_data_detected: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryMigrationResponse {
    pub library_path: String,
    pub file_count: u64,
    pub total_bytes: u64,
    pub migrated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileRecord {
    relative_path: PathBuf,
    size: u64,
}

impl LibraryRuntime {
    pub fn status(&self) -> LibraryStatusResponse {
        let available = self.directories().is_ok();
        LibraryStatusResponse {
            library_path: self
                .configured_path
                .as_ref()
                .or(self.active_root.as_ref())
                .map(|path| path.to_string_lossy().into_owned()),
            available,
            needs_migration: self.configured_path.is_none() && self.legacy_root.is_some(),
            legacy_data_detected: self.legacy_root.is_some(),
            error: self.error.clone().or_else(|| {
                (!available && self.configured_path.is_some())
                    .then(|| "图库目录当前不可用，请重新连接磁盘或选择图库位置。".into())
            }),
        }
    }

    pub fn directories(&self) -> Result<(PathBuf, PathBuf), String> {
        let root = self.active_root.as_ref().ok_or_else(|| {
            self.error
                .clone()
                .unwrap_or_else(|| "尚未设置图库目录，请先选择图库位置后再导入照片。".into())
        })?;
        if !root.is_dir() {
            return Err("图库目录当前不可用，请重新连接磁盘或选择图库位置。".into());
        }
        let media = root.join("media");
        let previews = root.join("previews");
        if !media.is_dir() || !previews.is_dir() {
            return Err("图库目录当前不可用，请重新连接磁盘或选择图库位置。".into());
        }
        Ok((media, previews))
    }
}

pub fn load_runtime(app_data_dir: &Path) -> LibraryRuntime {
    let legacy_has_data = directory_has_entries(&app_data_dir.join("media"))
        || directory_has_entries(&app_data_dir.join("previews"));
    let legacy_root = legacy_has_data.then(|| app_data_dir.to_path_buf());
    let settings_path = app_data_dir.join(SETTINGS_FILE);
    if !settings_path.exists() {
        return LibraryRuntime {
            configured_path: None,
            active_root: legacy_root.clone(),
            legacy_root,
            error: None,
        };
    }

    let settings = fs::read_to_string(&settings_path)
        .map_err(|error| format!("读取图库设置失败: {error}"))
        .and_then(|content| {
            serde_json::from_str::<AppSettings>(&content)
                .map_err(|error| format!("图库设置格式无效: {error}"))
        });
    match settings {
        Ok(settings) if settings.library_path.is_absolute() => {
            let available = settings.library_path.join("media").is_dir()
                && settings.library_path.join("previews").is_dir();
            LibraryRuntime {
                configured_path: Some(settings.library_path.clone()),
                active_root: available.then_some(settings.library_path),
                legacy_root,
                error: (!available)
                    .then(|| "图库目录当前不可用，请重新连接磁盘或选择图库位置。".into()),
            }
        }
        Ok(_) => LibraryRuntime {
            configured_path: None,
            active_root: legacy_root.clone(),
            legacy_root,
            error: Some("图库设置中的路径不是绝对目录，请重新选择图库位置。".into()),
        },
        Err(error) => LibraryRuntime {
            configured_path: None,
            active_root: legacy_root.clone(),
            legacy_root,
            error: Some(error),
        },
    }
}

pub async fn configure_library(
    app_data_dir: &Path,
    pool: &SqlitePool,
    runtime: &LibraryRuntime,
    requested_path: PathBuf,
) -> Result<(LibraryRuntime, LibraryMigrationResponse), String> {
    validate_target_path(&requested_path, runtime)?;
    let requested_is_ready =
        requested_path.join("media").is_dir() && requested_path.join("previews").is_dir();
    if runtime.configured_path.as_deref() == Some(requested_path.as_path()) && requested_is_ready {
        write_settings_atomic(app_data_dir, &requested_path)?;
        let response = LibraryMigrationResponse {
            library_path: requested_path.to_string_lossy().into_owned(),
            file_count: 0,
            total_bytes: 0,
            migrated: false,
        };
        return Ok((
            LibraryRuntime {
                configured_path: Some(requested_path.clone()),
                active_root: Some(requested_path),
                legacy_root: runtime.legacy_root.clone(),
                error: None,
            },
            response,
        ));
    }
    let source_root = runtime
        .active_root
        .clone()
        .or_else(|| runtime.legacy_root.clone())
        .filter(|path| path.is_dir());
    if source_root.as_deref() == Some(requested_path.as_path()) {
        write_settings_atomic(app_data_dir, &requested_path)?;
        let response = LibraryMigrationResponse {
            library_path: requested_path.to_string_lossy().into_owned(),
            file_count: 0,
            total_bytes: 0,
            migrated: false,
        };
        return Ok((
            LibraryRuntime {
                configured_path: Some(requested_path.clone()),
                active_root: Some(requested_path),
                legacy_root: runtime.legacy_root.clone(),
                error: None,
            },
            response,
        ));
    }

    let target_existed = requested_path.exists();
    let source_records = source_root
        .as_deref()
        .map(collect_library_records)
        .transpose()?
        .unwrap_or_default();
    let source_bytes = source_records.iter().map(|record| record.size).sum::<u64>();
    let space_probe = requested_path
        .parent()
        .filter(|path| path.exists())
        .unwrap_or(app_data_dir);
    let available = fs2::available_space(space_probe)
        .map_err(|error| format!("无法读取目标磁盘可用空间: {error}"))?;
    if available < source_bytes {
        return Err(format!(
            "目标磁盘空间不足：需要至少 {source_bytes} 字节，当前可用 {available} 字节"
        ));
    }

    let temporary_root = migration_temp_path(&requested_path)?;
    if temporary_root.exists() {
        fs::remove_dir_all(&temporary_root)
            .map_err(|error| format!("无法清理旧迁移临时目录: {error}"))?;
    }
    fs::create_dir_all(temporary_root.join("media/rolls"))
        .and_then(|_| fs::create_dir_all(temporary_root.join("previews/rolls")))
        .map_err(|error| format!("无法创建迁移临时目录: {error}"))?;

    let copy_source = source_root.clone();
    let copy_target = temporary_root.clone();
    let expected_records = source_records.clone();
    let copy_result = tokio::task::spawn_blocking(move || {
        if let Some(source_root) = copy_source {
            copy_tree(&source_root.join("media"), &copy_target.join("media"))?;
            copy_tree(&source_root.join("previews"), &copy_target.join("previews"))?;
        }
        validate_copied_records(&copy_target, &expected_records)
    })
    .await
    .map_err(|error| format!("图库迁移任务失败: {error}"))?;
    if let Err(error) = copy_result {
        let _ = fs::remove_dir_all(&temporary_root);
        return Err(error);
    }
    if let Err(error) = validate_database_paths(pool, &temporary_root.join("media")).await {
        let _ = fs::remove_dir_all(&temporary_root);
        return Err(error);
    }

    if target_existed {
        fs::remove_dir(&requested_path)
            .map_err(|error| format!("无法准备空的目标图库目录: {error}"))?;
    }
    if let Err(error) = fs::rename(&temporary_root, &requested_path) {
        let _ = fs::remove_dir_all(&temporary_root);
        if target_existed {
            let _ = fs::create_dir(&requested_path);
        }
        return Err(format!("无法启用已迁移图库目录: {error}"));
    }

    if let Err(error) = write_settings_atomic(app_data_dir, &requested_path) {
        let _ = fs::remove_dir_all(&requested_path);
        if target_existed {
            let _ = fs::create_dir(&requested_path);
        }
        return Err(error);
    }

    let response = LibraryMigrationResponse {
        library_path: requested_path.to_string_lossy().into_owned(),
        file_count: source_records.len() as u64,
        total_bytes: source_bytes,
        migrated: !source_records.is_empty(),
    };
    Ok((
        LibraryRuntime {
            configured_path: Some(requested_path.clone()),
            active_root: Some(requested_path),
            legacy_root: runtime.legacy_root.clone(),
            error: None,
        },
        response,
    ))
}

fn validate_target_path(path: &Path, runtime: &LibraryRuntime) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("图库位置必须是绝对目录".into());
    }
    if path.is_file() {
        return Err("图库位置不能选择单个文件".into());
    }
    if path.exists()
        && !is_directory_empty(path)?
        && runtime.active_root.as_deref() != Some(path)
        && runtime.configured_path.as_deref() != Some(path)
    {
        return Err("所选目录包含其他文件，请选择空目录或新目录，避免覆盖现有数据。".into());
    }
    let executable =
        std::env::current_exe().map_err(|error| format!("无法确认安装目录: {error}"))?;
    if let Some(install_dir) = executable.parent() {
        let install_dir = install_dir
            .canonicalize()
            .unwrap_or_else(|_| install_dir.to_path_buf());
        let comparable = comparable_path(path)?;
        if comparable == install_dir || comparable.starts_with(&install_dir) {
            return Err("图库不能放在应用安装目录中".into());
        }
    }
    let writable_parent = if path.exists() {
        path
    } else {
        path.parent()
            .filter(|parent| parent.exists())
            .ok_or_else(|| "图库父目录不存在或不可访问".to_string())?
    };
    let probe = writable_parent.join(format!(".goshootfilm-write-test-{}", std::process::id()));
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .and_then(|mut file| file.write_all(b"ok"))
        .map_err(|error| format!("所选目录不可写: {error}"))?;
    fs::remove_file(probe).map_err(|error| format!("无法清理目录写入测试文件: {error}"))?;
    Ok(())
}

fn comparable_path(path: &Path) -> Result<PathBuf, String> {
    if path.exists() {
        return path
            .canonicalize()
            .map_err(|error| format!("无法解析图库路径: {error}"));
    }
    let parent = path
        .parent()
        .ok_or_else(|| "图库路径缺少父目录".to_string())?;
    let parent = parent
        .canonicalize()
        .map_err(|error| format!("图库父目录不存在或不可访问: {error}"))?;
    let name = path.file_name().ok_or_else(|| "图库路径无效".to_string())?;
    Ok(parent.join(name))
}

fn directory_has_entries(path: &Path) -> bool {
    fs::read_dir(path)
        .ok()
        .and_then(|mut entries| entries.next())
        .is_some()
}

fn is_directory_empty(path: &Path) -> Result<bool, String> {
    Ok(fs::read_dir(path)
        .map_err(|error| format!("无法读取所选目录: {error}"))?
        .next()
        .is_none())
}

fn migration_temp_path(target: &Path) -> Result<PathBuf, String> {
    let parent = target
        .parent()
        .ok_or_else(|| "图库路径缺少父目录".to_string())?;
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "图库目录名称无效".to_string())?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("系统时间无效: {error}"))?
        .as_nanos();
    Ok(parent.join(format!(".{name}.migrating-{}-{nonce}", std::process::id())))
}

fn collect_library_records(root: &Path) -> Result<Vec<FileRecord>, String> {
    let mut records = Vec::new();
    collect_records(root, &root.join("media"), &mut records)?;
    collect_records(root, &root.join("previews"), &mut records)?;
    records.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(records)
}

fn collect_records(
    root: &Path,
    current: &Path,
    records: &mut Vec<FileRecord>,
) -> Result<(), String> {
    if !current.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(current).map_err(|error| format!("读取迁移源目录失败: {error}"))?
    {
        let entry = entry.map_err(|error| format!("读取迁移源文件失败: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("读取迁移源类型失败: {error}"))?;
        if file_type.is_symlink() {
            return Err(format!(
                "迁移源包含不支持的符号链接: {}",
                entry.path().display()
            ));
        }
        if file_type.is_dir() {
            collect_records(root, &entry.path(), records)?;
        } else if file_type.is_file() {
            records.push(FileRecord {
                relative_path: entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|_| "迁移源路径无效")?
                    .to_path_buf(),
                size: entry
                    .metadata()
                    .map_err(|error| format!("读取迁移源大小失败: {error}"))?
                    .len(),
            });
        }
    }
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.exists() {
        return Ok(());
    }
    fs::create_dir_all(destination).map_err(|error| format!("创建迁移目标目录失败: {error}"))?;
    for entry in fs::read_dir(source).map_err(|error| format!("读取迁移源目录失败: {error}"))?
    {
        let entry = entry.map_err(|error| format!("读取迁移源文件失败: {error}"))?;
        let target = destination.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|error| format!("读取迁移源类型失败: {error}"))?;
        if file_type.is_symlink() {
            return Err(format!(
                "迁移源包含不支持的符号链接: {}",
                entry.path().display()
            ));
        }
        if file_type.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target).map_err(|error| {
                format!("复制图库文件失败（{}）: {error}", entry.path().display())
            })?;
        }
    }
    Ok(())
}

fn validate_copied_records(root: &Path, expected: &[FileRecord]) -> Result<(), String> {
    let actual = collect_library_records(root)?;
    if actual != expected {
        return Err("迁移校验失败：文件数量、相对路径或大小不一致".into());
    }
    for record in &actual {
        let path = root.join(&record.relative_path);
        let mut file = fs::File::open(&path)
            .map_err(|error| format!("迁移文件不可读（{}）: {error}", path.display()))?;
        if record.size > 0 {
            let mut byte = [0_u8; 1];
            file.read_exact(&mut byte)
                .map_err(|error| format!("迁移文件读取失败（{}）: {error}", path.display()))?;
        }
    }
    Ok(())
}

async fn validate_database_paths(pool: &SqlitePool, media_dir: &Path) -> Result<(), String> {
    let rows: Vec<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT lab_scan_path, edit_scan_path FROM photos")
            .fetch_all(pool)
            .await
            .map_err(|error| format!("读取照片路径用于迁移校验失败: {error}"))?;
    for stored_path in rows
        .into_iter()
        .flat_map(|(lab, edit)| [lab, edit])
        .flatten()
    {
        let relative = Path::new(&stored_path);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(format!("数据库包含无效图库相对路径: {stored_path}"));
        }
        let resolved = media_dir.join(relative);
        if !resolved.is_file() {
            return Err(format!(
                "迁移后找不到数据库照片文件: {}",
                resolved.display()
            ));
        }
    }
    Ok(())
}

fn write_settings_atomic(app_data_dir: &Path, library_path: &Path) -> Result<(), String> {
    let settings_path = app_data_dir.join(SETTINGS_FILE);
    let temporary_path = app_data_dir.join(format!("{SETTINGS_FILE}.tmp-{}", std::process::id()));
    let content = serde_json::to_vec_pretty(&AppSettings {
        library_path: library_path.to_path_buf(),
    })
    .map_err(|error| format!("序列化图库设置失败: {error}"))?;
    let mut temporary = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temporary_path)
        .map_err(|error| format!("创建图库设置临时文件失败: {error}"))?;
    temporary
        .write_all(&content)
        .and_then(|_| temporary.sync_all())
        .map_err(|error| format!("写入图库设置失败: {error}"))?;
    drop(temporary);
    atomic_replace(&temporary_path, &settings_path).map_err(|error| {
        let _ = fs::remove_file(&temporary_path);
        format!("保存图库设置失败: {error}")
    })
}

#[cfg(windows)]
pub(crate) fn atomic_replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
pub(crate) fn atomic_replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_uses_legacy_gallery_until_settings_are_written() {
        let root =
            std::env::temp_dir().join(format!("goshootfilm-library-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("media/rolls/1/edit")).unwrap();
        fs::create_dir_all(root.join("previews/rolls")).unwrap();
        fs::write(root.join("media/rolls/1/edit/photo.jpg"), b"image").unwrap();
        let runtime = load_runtime(&root);
        assert!(runtime.status().needs_migration);
        assert_eq!(runtime.directories().unwrap().0, root.join("media"));
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn migration_copies_validates_and_switches_without_deleting_source() {
        let root = std::env::temp_dir().join(format!(
            "goshootfilm-library-migration-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let app_data = root.join("app-data");
        let target = root.join("library");
        fs::create_dir_all(app_data.join("media/rolls/1/edit")).unwrap();
        fs::create_dir_all(app_data.join("previews/rolls")).unwrap();
        fs::write(app_data.join("media/rolls/1/edit/photo.jpg"), b"image").unwrap();
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query("CREATE TABLE photos (lab_scan_path TEXT, edit_scan_path TEXT)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO photos (edit_scan_path) VALUES ('rolls/1/edit/photo.jpg')")
            .execute(&pool)
            .await
            .unwrap();
        let runtime = load_runtime(&app_data);

        let (next, result) = configure_library(&app_data, &pool, &runtime, target.clone())
            .await
            .unwrap();

        assert!(result.migrated);
        assert_eq!(result.file_count, 1);
        assert!(app_data.join("media/rolls/1/edit/photo.jpg").is_file());
        assert!(target.join("media/rolls/1/edit/photo.jpg").is_file());
        assert_eq!(next.directories().unwrap().0, target.join("media"));
        let saved: AppSettings =
            serde_json::from_slice(&fs::read(app_data.join(SETTINGS_FILE)).unwrap()).unwrap();
        assert_eq!(saved.library_path, target);
        pool.close().await;
        fs::remove_dir_all(root).unwrap();
    }
}
