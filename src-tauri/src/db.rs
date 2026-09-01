use sqlx::{SqlitePool, Sqlite, Pool};
use std::fs;
use tauri::Manager;

pub async fn init_db(app: &tauri::App) -> Result<Pool<Sqlite>, Box<dyn std::error::Error>> {
    // 获取 Tauri 推荐的应用本地数据目录（例如 AppData 路径）
    let app_dir = app.path().app_data_dir()?;
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir)?;
    }

    // 拼接出 SQLite 数据库文件的完整路径
    let db_path = app_dir.join("goshootfilm.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());

    println!("Database path: {}", db_url);

    // 连接或创建 SQLite 数据库
    let pool = SqlitePool::connect(&db_url).await?;

    // 初始化建表语句（你可以根据以前 MySQL 的表结构改写为 SQLite 语法）
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS cameras (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            brand TEXT NOT NULL,
            model TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}