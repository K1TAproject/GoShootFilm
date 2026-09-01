use sqlx::SqlitePool;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use tauri::Manager;

// 定义一个结构体用来在全局保存数据库连接池
pub struct AppState {
    pub db: SqlitePool,
}

// 1. 获取全量数据（供首页仪表盘统计使用）
#[tauri::command]
async fn get_all_data(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let cameras: Vec<(
        i64,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    )> =
        sqlx::query_as("SELECT id, brand, model, status, format, purchase_date, note FROM cameras")
            .fetch_all(&state.db)
            .await
            .map_err(|e| e.to_string())?;

    let films: Vec<(
        i64,
        String,
        String,
        i64,
        String,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as("SELECT id, brand, name, iso, type, target_status, note FROM film_stocks")
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    // 联合查询 rolls，附带相机和胶卷的名称
    let rolls_rows: Vec<(
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
    )> = sqlx::query_as(
        r#"
        SELECT
            r.id, r.camera_id, r.film_stock_id, r.roll_index, r.shot_month, r.city, r.note,
            c.brand as c_brand, c.model as c_model,
            f.brand as f_brand, f.name as f_name
        FROM rolls r
        JOIN cameras c ON r.camera_id = c.id
        JOIN film_stocks f ON r.film_stock_id = f.id
        ORDER BY r.shot_month DESC, r.id DESC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    // 组装 rolls 及其对应的 photos
    let mut rolls_json = Vec::new();
    for r in rolls_rows {
        let (
            id,
            camera_id,
            film_stock_id,
            roll_index,
            shot_month,
            city,
            note,
            c_brand,
            c_model,
            f_brand,
            f_name,
        ) = r;

        let photos: Vec<(i64, Option<i32>, Option<String>, Option<String>, Option<i32>)> = sqlx::query_as(
            "SELECT id, frame_number, lab_scan_path, edit_scan_path, is_favorite FROM photos WHERE roll_id = ?"
        )
            .bind(id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

        let photos_val: Vec<serde_json::Value> = photos
            .into_iter()
            .map(|(pid, frame, lab, edit, is_fav)| {
                serde_json::json!({
                    "id": pid,
                    "frame_number": frame,
                    "lab_scan_path": lab,
                    "edit_scan_path": edit,
                    "is_favorite": is_fav.unwrap_or(0) // 传给前端数字 0 或 1
                })
            })
            .collect();

        rolls_json.push(serde_json::json!({
            "id": id,
            "cameraId": camera_id,
            "filmId": film_stock_id,
            "index": roll_index,
            "shot_month": shot_month,
            "city": city,
            "note": note,
            "camera_brand": c_brand,
            "camera_model": c_model,
            "film_brand": f_brand,
            "film_name": f_name,
            "camera_info": format!("{} {}", c_brand, c_model),
            "film_info": format!("{} {}", f_brand, f_name),
            "photos": photos_val
        }));
    }

    let result = serde_json::json!({
        "cameras": cameras.into_iter().map(|(id, b, m, s, fmt, pd, note)| serde_json::json!({
            "id": id, "brand": b, "model": m, "status": s, "format": fmt, "purchase_date": pd, "note": note
        })).collect::<Vec<_>>(),
        "films": films.into_iter().map(|(id, b, n, iso, t, target_status, note)| serde_json::json!({
            "id": id, "brand": b, "name": n, "iso": iso, "type": t,
            "target_status": target_status, "note": note
        })).collect::<Vec<_>>(),
        "rolls": rolls_json,
    });

    Ok(result)
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
    let fmt = format.unwrap_or_else(|| "135".to_string());
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
    .map_err(|e| e.to_string())?;

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
    let result = sqlx::query(
        "INSERT INTO film_stocks (brand, name, iso, type, target_status, note) VALUES (?, ?, ?, ?, ?, ?)"
    )
        .bind(brand)
        .bind(name)
        .bind(iso)
        .bind(film_type)
        .bind(target_status.unwrap_or_else(|| "untested".to_string()))
        .bind(note)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;

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
    let camera_exists: Option<i64> = sqlx::query_scalar("SELECT id FROM cameras WHERE id = ?")
        .bind(camera_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    if camera_exists.is_none() {
        return Err("Selected camera does not exist".into());
    }

    let film_exists: Option<i64> = sqlx::query_scalar("SELECT id FROM film_stocks WHERE id = ?")
        .bind(film_stock_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    if film_exists.is_none() {
        return Err("Selected film does not exist".into());
    }

    let next_roll_index: i32 = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT MAX(roll_index) FROM rolls WHERE camera_id = ?",
    )
    .bind(camera_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?
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
        .map_err(|e| e.to_string())?;

    Ok(result.last_insert_rowid())
}

#[tauri::command]
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
    sqlx::query(
        "UPDATE film_stocks SET brand = ?, name = ?, iso = ?, type = ?, target_status = ?, note = ? WHERE id = ?",
    )
    .bind(brand)
    .bind(name)
    .bind(iso)
    .bind(film_type)
    .bind(target_status.unwrap_or_else(|| "untested".to_string()))
    .bind(note)
    .bind(id)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;

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
    let roll_row: Option<(i64, i64, i64)> =
        sqlx::query_as("SELECT id, camera_id, film_stock_id FROM rolls WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| e.to_string())?;

    let Some((_roll_id, old_camera_id, _old_film_stock_id)) = roll_row else {
        return Err("Roll not found".into());
    };

    let camera_exists: Option<i64> = sqlx::query_scalar("SELECT id FROM cameras WHERE id = ?")
        .bind(camera_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    if camera_exists.is_none() {
        return Err("Selected camera does not exist".into());
    }

    let film_exists: Option<i64> = sqlx::query_scalar("SELECT id FROM film_stocks WHERE id = ?")
        .bind(film_stock_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    if film_exists.is_none() {
        return Err("Selected film does not exist".into());
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

    sqlx::query(
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
    .map_err(|e| e.to_string())?;

    Ok("Roll updated successfully!".into())
}

// 3. 更新相机信息
#[tauri::command]
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
    sqlx::query(
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
        .map_err(|e| e.to_string())?;

    Ok("Camera updated successfully!".into())
}

// 4. 删除相机
#[tauri::command]
async fn delete_camera(id: i64, state: tauri::State<'_, AppState>) -> Result<String, String> {
    sqlx::query("DELETE FROM cameras WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    Ok("Camera deleted successfully!".into())
}

// 5. 获取单个相机的详细信息及拍摄卷
#[tauri::command]
async fn get_camera_detail(
    id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let camera_row: Option<(
        i64,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT id, brand, model, status, format, purchase_date, note FROM cameras WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let camera = match camera_row {
        Some((id, b, m, s, fmt, pd, note)) => serde_json::json!({
            "id": id, "brand": b, "model": m, "status": s, "format": fmt, "purchase_date": pd, "note": note
        }),
        None => return Err("Camera not found".into()),
    };

    let rolls_rows: Vec<(i64, i32, Option<String>, Option<String>, String, String, i64)> = sqlx::query_as(
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
        .map(|(rid, idx, month, city, f_brand, f_name, f_iso)| {
            serde_json::json!({
                "id": rid,
                "roll_index": idx,
                "shot_month": month,
                "city": city,
                "film_brand": f_brand,
                "film_name": f_name,
                "film_iso": f_iso,
                "film_info": format!("{} {} (ISO {})", f_brand, f_name, f_iso)
            })
        })
        .collect::<Vec<_>>();

    Ok(serde_json::json!({
        "camera": camera,
        "rolls": rolls
    }))
}

#[tauri::command]
async fn toggle_photo_favorite(
    state: tauri::State<'_, AppState>, // 假设你的数据库连接池存在 AppState 中
    photo_id: i64,
    is_favorite: bool,
) -> Result<(), String> {
    let val = if is_favorite { 1 } else { 0 };
    sqlx::query("UPDATE photos SET is_favorite = ? WHERE id = ?")
        .bind(val)
        .bind(photo_id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// 删除照片
#[tauri::command]
async fn delete_photo(state: tauri::State<'_, AppState>, photo_id: i64) -> Result<(), String> {
    sqlx::query("DELETE FROM photos WHERE id = ?")
        .bind(photo_id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// 拖拽或对话框导入照片到指定胶卷（自动计算并填充连续的 frame_number）
#[tauri::command]
async fn import_photos(
    state: tauri::State<'_, AppState>,
    roll_id: i64,
    file_paths: Vec<String>,
) -> Result<(), String> {
    let roll_exists: Option<i64> = sqlx::query_scalar("SELECT id FROM rolls WHERE id = ?")
        .bind(roll_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    if roll_exists.is_none() {
        return Err("Roll does not exist".into());
    }

    let existing_frames: Vec<i32> = sqlx::query_scalar(
        "SELECT frame_number FROM photos WHERE roll_id = ? AND frame_number IS NOT NULL",
    )
    .bind(roll_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;
    let mut used_frames: HashSet<i32> = existing_frames.into_iter().collect();

    let max_frame: Option<i32> =
        sqlx::query_scalar("SELECT MAX(frame_number) FROM photos WHERE roll_id = ?")
            .bind(roll_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| e.to_string())?
            .flatten();

    let mut next_frame = max_frame.unwrap_or(0) + 1;

    for path in file_paths {
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
            .filter(|frame| !used_frames.contains(frame));

        let frame_number = filename_frame.unwrap_or_else(|| {
            while used_frames.contains(&next_frame) {
                next_frame += 1;
            }
            next_frame
        });

        sqlx::query(
            "INSERT INTO photos (roll_id, frame_number, edit_scan_path, is_favorite) VALUES (?, ?, ?, 0)"
        )
            .bind(roll_id)
            .bind(frame_number)
            .bind(path)
            .execute(&state.db)
            .await
            .map_err(|e| e.to_string())?;

        used_frames.insert(frame_number);
        if frame_number >= next_frame {
            next_frame = frame_number + 1;
        }
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();

            tauri::async_runtime::block_on(async move {
                let app_dir = match handle.path().app_data_dir() {
                    Ok(dir) => dir,
                    Err(e) => {
                        eprintln!("Failed to get app data dir: {}", e);
                        return;
                    }
                };

                if !app_dir.exists() {
                    let _ = fs::create_dir_all(&app_dir);
                }

                let db_path = app_dir.join("goshootfilm.db");
                let db_url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());

                match SqlitePool::connect(&db_url).await {
                    Ok(pool) => {
                        // 1. 创建 cameras 表
                        let _ = sqlx::query(
                            r#"
                            CREATE TABLE IF NOT EXISTS cameras (
                                id INTEGER PRIMARY KEY AUTOINCREMENT,
                                brand TEXT NOT NULL,
                                model TEXT NOT NULL,
                                format TEXT DEFAULT '135',
                                purchase_date TEXT DEFAULT NULL,
                                status TEXT DEFAULT 'active',
                                note TEXT DEFAULT NULL,
                                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
                            )
                            "#
                        ).execute(&pool).await;

                        // 2. 创建 film_stocks 表
                        let _ = sqlx::query(
                            r#"
                            CREATE TABLE IF NOT EXISTS film_stocks (
                                id INTEGER PRIMARY KEY AUTOINCREMENT,
                                brand TEXT NOT NULL,
                                name TEXT NOT NULL,
                                iso INTEGER NOT NULL,
                                type TEXT NOT NULL,
                                target_status TEXT DEFAULT 'untested',
                                note TEXT DEFAULT NULL,
                                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                                UNIQUE(brand, name)
                            )
                            "#
                        ).execute(&pool).await;

                        // 3. 创建 rolls 表
                        let _ = sqlx::query(
                            r#"
                            CREATE TABLE IF NOT EXISTS rolls (
                                id INTEGER PRIMARY KEY AUTOINCREMENT,
                                camera_id INTEGER NOT NULL,
                                film_stock_id INTEGER NOT NULL,
                                roll_index INTEGER NOT NULL,
                                shot_month TEXT DEFAULT NULL,
                                city TEXT DEFAULT NULL,
                                note TEXT DEFAULT NULL,
                                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                                FOREIGN KEY (camera_id) REFERENCES cameras(id) ON DELETE CASCADE,
                                FOREIGN KEY (film_stock_id) REFERENCES film_stocks(id) ON DELETE CASCADE
                            )
                            "#
                        ).execute(&pool).await;

                        // 4. 创建 photos 表
                        let _ = sqlx::query(
                            r#"
                            CREATE TABLE IF NOT EXISTS photos (
                                id INTEGER PRIMARY KEY AUTOINCREMENT,
                                roll_id INTEGER NOT NULL,
                                frame_number INTEGER DEFAULT NULL,
                                lab_scan_path TEXT DEFAULT NULL,
                                edit_scan_path TEXT DEFAULT NULL,
                                is_favorite INTEGER DEFAULT 0,
                                note TEXT DEFAULT NULL,
                                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                                FOREIGN KEY (roll_id) REFERENCES rolls(id) ON DELETE CASCADE
                            )
                            "#
                        ).execute(&pool).await;

                        // 5. 仅初始化固定的胶卷图鉴库 (film_stocks) 数据，相机和拍摄卷保持为空由用户添加
                        let film_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM film_stocks")
                            .fetch_one(&pool)
                            .await
                            .unwrap_or((0,));

                        if film_count.0 == 0 {
                            let _ = sqlx::query("INSERT INTO film_stocks (id, brand, name, iso, type, target_status, created_at) VALUES
                                (1, 'Foma', 'Fomapan 100 Classic', 100, 'B&W', 'shot', '2026-08-30 08:30:09'),
                                (2, 'FUJIFILM', 'Fujicolor 100', 100, 'Color Negative', 'unshot', '2026-08-30 08:36:28'),
                                (3, 'Kodak', 'Professional Ektachrome E100', 100, 'Slide', 'shot', '2026-08-30 08:36:28'),
                                (4, 'Kodak', 'Professional PORTRA 160', 160, 'Color Negative', 'shot', '2026-08-30 08:36:28'),
                                (5, 'Wolfen', 'NC500 Color Negative Film', 500, 'Color Negative', 'unshot', '2026-08-30 08:36:28'),
                                (6, 'kodak', 'Gold 200', 200, 'Color Negative', 'shot', '2026-08-31 08:50:00')").execute(&pool).await;

                            println!("Default film stocks injected successfully!");
                        }

                        handle.manage(AppState { db: pool });
                    }
                    Err(e) => {
                        eprintln!("Failed to connect to SQLite: {}", e);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_all_data,
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
