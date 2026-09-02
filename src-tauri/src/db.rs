use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tauri::Manager;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub struct DatabaseResources {
    pub pool: SqlitePool,
    pub media_dir: PathBuf,
    pub preview_dir: PathBuf,
}

pub async fn init_db(app: &tauri::App) -> Result<DatabaseResources, Box<dyn std::error::Error>> {
    let app_dir = app.path().app_data_dir()?;
    fs::create_dir_all(&app_dir)?;

    let db_path = app_dir.join("goshootfilm.db");
    let options = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    MIGRATOR.run(&pool).await?;
    repair_legacy_camera_schema(&pool).await?;

    let media_dir = app_dir.join("media");
    fs::create_dir_all(&media_dir)?;
    let preview_dir = app_dir.join("previews");
    fs::create_dir_all(&preview_dir)?;

    Ok(DatabaseResources {
        pool,
        media_dir,
        preview_dir,
    })
}

async fn repair_legacy_camera_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let rows = sqlx::query("PRAGMA table_info(cameras)")
        .fetch_all(pool)
        .await?;
    let columns: HashSet<String> = rows
        .iter()
        .map(|row| row.get::<String, _>("name"))
        .collect();

    let additions = [
        (
            "format",
            "ALTER TABLE cameras ADD COLUMN format TEXT DEFAULT '135'",
        ),
        (
            "purchase_date",
            "ALTER TABLE cameras ADD COLUMN purchase_date TEXT DEFAULT NULL",
        ),
        (
            "status",
            "ALTER TABLE cameras ADD COLUMN status TEXT DEFAULT 'active'",
        ),
        (
            "note",
            "ALTER TABLE cameras ADD COLUMN note TEXT DEFAULT NULL",
        ),
        (
            "updated_at",
            "ALTER TABLE cameras ADD COLUMN updated_at DATETIME DEFAULT NULL",
        ),
    ];

    for (name, statement) in additions {
        if !columns.contains(name) {
            sqlx::query(statement).execute(pool).await?;
        }
    }

    sqlx::query(
        "UPDATE cameras SET updated_at = COALESCE(updated_at, created_at, CURRENT_TIMESTAMP)",
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn memory_pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create in-memory database")
    }

    #[tokio::test]
    async fn new_database_contains_only_seed_films() {
        let pool = memory_pool().await;
        MIGRATOR.run(&pool).await.expect("run migrations");
        repair_legacy_camera_schema(&pool)
            .await
            .expect("repair schema");

        let film_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM film_stocks")
            .fetch_one(&pool)
            .await
            .expect("count films");
        let camera_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cameras")
            .fetch_one(&pool)
            .await
            .expect("count cameras");
        let roll_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rolls")
            .fetch_one(&pool)
            .await
            .expect("count rolls");

        assert_eq!(film_count, 6);
        assert_eq!(camera_count, 0);
        assert_eq!(roll_count, 0);
    }

    #[tokio::test]
    async fn legacy_camera_table_is_upgraded_without_losing_rows() {
        let pool = memory_pool().await;
        sqlx::query(
            "CREATE TABLE cameras (id INTEGER PRIMARY KEY AUTOINCREMENT, brand TEXT NOT NULL, model TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP)",
        )
        .execute(&pool)
        .await
        .expect("create legacy table");
        sqlx::query("INSERT INTO cameras (brand, model) VALUES ('Nikon', 'FM2')")
            .execute(&pool)
            .await
            .expect("insert legacy camera");

        MIGRATOR.run(&pool).await.expect("run migrations");
        repair_legacy_camera_schema(&pool)
            .await
            .expect("repair schema");

        let row: (String, String, String, String) =
            sqlx::query_as("SELECT brand, model, status, format FROM cameras WHERE id = 1")
                .fetch_one(&pool)
                .await
                .expect("read upgraded camera");
        assert_eq!(
            row,
            ("Nikon".into(), "FM2".into(), "active".into(), "135".into())
        );
    }

    #[tokio::test]
    async fn camera_delete_cascades_to_rolls_and_photo_records() {
        let pool = memory_pool().await;
        MIGRATOR.run(&pool).await.expect("run migrations");
        repair_legacy_camera_schema(&pool)
            .await
            .expect("repair schema");
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .expect("enable foreign keys");

        let camera_id = sqlx::query("INSERT INTO cameras (brand, model) VALUES ('Nikon', 'FM2')")
            .execute(&pool)
            .await
            .expect("insert camera")
            .last_insert_rowid();
        let roll_id = sqlx::query(
            "INSERT INTO rolls (camera_id, film_stock_id, roll_index) VALUES (?, 1, 1)",
        )
        .bind(camera_id)
        .execute(&pool)
        .await
        .expect("insert roll")
        .last_insert_rowid();
        sqlx::query("INSERT INTO photos (roll_id, frame_number) VALUES (?, 1)")
            .bind(roll_id)
            .execute(&pool)
            .await
            .expect("insert photo");

        sqlx::query("DELETE FROM cameras WHERE id = ?")
            .bind(camera_id)
            .execute(&pool)
            .await
            .expect("delete camera");

        let roll_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rolls")
            .fetch_one(&pool)
            .await
            .expect("count rolls");
        let photo_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM photos")
            .fetch_one(&pool)
            .await
            .expect("count photos");
        assert_eq!((roll_count, photo_count), (0, 0));
    }
}
