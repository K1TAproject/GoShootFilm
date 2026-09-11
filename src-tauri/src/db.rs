use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, Sqlite, SqlitePool, Transaction};
use std::collections::HashSet;
use std::fs;
use std::io::{Error as IoError, ErrorKind};
use std::path::PathBuf;
use std::time::Duration;
use tauri::Manager;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");
const FILM_CATALOG: &str = include_str!("../../src/data/film-catalog.csv");

#[derive(Debug)]
struct CatalogFilm {
    brand: String,
    name: String,
    iso: i64,
    film_type: String,
    legacy_brand: String,
    legacy_name: String,
}

pub struct DatabaseResources {
    pub pool: SqlitePool,
    pub app_data_dir: PathBuf,
}

pub async fn init_db(app: &tauri::App) -> Result<DatabaseResources, Box<dyn std::error::Error>> {
    init_db_at(app.path().app_data_dir()?).await
}

async fn init_db_at(app_dir: PathBuf) -> Result<DatabaseResources, Box<dyn std::error::Error>> {
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
    sync_film_catalog(&pool).await?;
    let mut transaction = pool.begin().await?;
    reindex_rolls(&mut transaction).await?;
    transaction.commit().await?;

    Ok(DatabaseResources {
        pool,
        app_data_dir: app_dir,
    })
}

fn invalid_catalog(message: impl Into<String>) -> IoError {
    IoError::new(ErrorKind::InvalidData, message.into())
}

fn normalized_film_key(brand: &str, name: &str) -> String {
    format!("{} {}", brand.trim(), name.trim())
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn parse_film_catalog() -> Result<(String, Vec<CatalogFilm>), IoError> {
    let mut lines = FILM_CATALOG.lines();
    let version_line = lines
        .next()
        .ok_or_else(|| invalid_catalog("胶卷目录缺少版本行"))?;
    let version_fields: Vec<_> = version_line.split(';').collect();
    if version_fields.len() != 7
        || version_fields.first() != Some(&"version")
        || version_fields.get(1).is_none_or(|v| v.is_empty())
    {
        return Err(invalid_catalog("胶卷目录版本行无效"));
    }
    let header = lines
        .next()
        .ok_or_else(|| invalid_catalog("胶卷目录缺少表头"))?;
    if header != "brand;name;iso;type;image;legacy_brand;legacy_name" {
        return Err(invalid_catalog("胶卷目录表头无效"));
    }

    let mut seen = HashSet::new();
    let mut seen_images = HashSet::new();
    let mut catalog = Vec::new();
    for (offset, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split(';').collect();
        if fields.len() != 7 {
            return Err(invalid_catalog(format!(
                "胶卷目录第 {} 行字段数量无效",
                offset + 3
            )));
        }
        let iso = fields[2]
            .parse::<i64>()
            .map_err(|_| invalid_catalog(format!("胶卷目录第 {} 行 ISO 无效", offset + 3)))?;
        let image = fields[4].trim();
        let image_is_safe = image.ends_with(".png")
            && image.chars().all(|ch| {
                ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '-' | '.')
            });
        let incomplete_legacy_key = fields[5].trim().is_empty() != fields[6].trim().is_empty();
        if fields[0].trim().is_empty()
            || fields[1].trim().is_empty()
            || !(1..=12800).contains(&iso)
            || !matches!(fields[3], "B&W" | "Color Negative" | "Slide")
            || !image_is_safe
            || incomplete_legacy_key
        {
            return Err(invalid_catalog(format!(
                "胶卷目录第 {} 行数据无效",
                offset + 3
            )));
        }
        let key = normalized_film_key(fields[0], fields[1]);
        if !seen.insert(key) {
            return Err(invalid_catalog(format!(
                "胶卷目录第 {} 行存在重复品牌与型号",
                offset + 3
            )));
        }
        if !seen_images.insert(image) {
            return Err(invalid_catalog(format!(
                "胶卷目录第 {} 行存在重复图片文件名",
                offset + 3
            )));
        }
        catalog.push(CatalogFilm {
            brand: fields[0].trim().to_string(),
            name: fields[1].trim().to_string(),
            iso,
            film_type: fields[3].to_string(),
            legacy_brand: fields[5].trim().to_string(),
            legacy_name: fields[6].trim().to_string(),
        });
    }
    Ok((version_fields[1].to_string(), catalog))
}

async fn sync_film_catalog(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    let (version, catalog) = parse_film_catalog()?;
    let metadata_key = "film_catalog_version";
    let applied_version: Option<String> =
        sqlx::query_scalar("SELECT value FROM app_metadata WHERE key = ?")
            .bind(metadata_key)
            .fetch_optional(pool)
            .await?;
    if applied_version.as_deref() == Some(version.as_str()) {
        return Ok(());
    }

    let mut transaction = pool.begin().await?;
    let mut database_films: Vec<(i64, String, String)> =
        sqlx::query_as("SELECT id, brand, name FROM film_stocks")
            .fetch_all(&mut *transaction)
            .await?;

    for film in catalog {
        let canonical_key = normalized_film_key(&film.brand, &film.name);
        let legacy_key = if film.legacy_brand.is_empty() || film.legacy_name.is_empty() {
            None
        } else {
            Some(normalized_film_key(&film.legacy_brand, &film.legacy_name))
        };
        let mut matching_ids: Vec<i64> = database_films
            .iter()
            .filter_map(|(id, brand, name)| {
                let key = normalized_film_key(brand, name);
                (key == canonical_key || legacy_key.as_ref() == Some(&key)).then_some(*id)
            })
            .collect();
        matching_ids.sort_unstable();
        matching_ids.dedup();

        if matching_ids.len() > 1 {
            return Err(invalid_catalog(format!(
                "同步胶卷目录时发现冲突记录：{} {}",
                film.brand, film.name
            ))
            .into());
        }

        if let Some(id) = matching_ids.first().copied() {
            sqlx::query(
                "UPDATE film_stocks SET brand = ?, name = ?, iso = ?, type = ? WHERE id = ?",
            )
            .bind(&film.brand)
            .bind(&film.name)
            .bind(film.iso)
            .bind(&film.film_type)
            .bind(id)
            .execute(&mut *transaction)
            .await?;
            if let Some((_, brand, name)) = database_films.iter_mut().find(|row| row.0 == id) {
                *brand = film.brand;
                *name = film.name;
            }
        } else {
            let id = sqlx::query(
                "INSERT INTO film_stocks (brand, name, iso, type, target_status) VALUES (?, ?, ?, ?, 'unshot')",
            )
            .bind(&film.brand)
            .bind(&film.name)
            .bind(film.iso)
            .bind(&film.film_type)
            .execute(&mut *transaction)
            .await?
            .last_insert_rowid();
            database_films.push((id, film.brand, film.name));
        }
    }

    sqlx::query(
        "INSERT INTO app_metadata (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(metadata_key)
    .bind(version)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

pub(crate) async fn reindex_rolls(
    transaction: &mut Transaction<'_, Sqlite>,
) -> Result<(), sqlx::Error> {
    // roll_index 是面向用户的全局连续卷号，不是 SQLite 主键；删除级联后也必须消除空档。
    let roll_ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM rolls ORDER BY created_at, id")
        .fetch_all(&mut **transaction)
        .await?;
    for (offset, roll_id) in roll_ids.into_iter().enumerate() {
        sqlx::query("UPDATE rolls SET roll_index = ? WHERE id = ?")
            .bind((offset + 1) as i64)
            .bind(roll_id)
            .execute(&mut **transaction)
            .await?;
    }
    Ok(())
}

pub(crate) async fn next_roll_index(
    transaction: &mut Transaction<'_, Sqlite>,
) -> Result<i32, sqlx::Error> {
    Ok(
        sqlx::query_scalar::<_, Option<i32>>("SELECT MAX(roll_index) FROM rolls")
            .fetch_one(&mut **transaction)
            .await?
            .unwrap_or(0)
            + 1,
    )
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
    async fn database_initialization_does_not_create_gallery_directories() {
        let app_data =
            std::env::temp_dir().join(format!("goshootfilm-db-no-gallery-{}", std::process::id()));
        let _ = fs::remove_dir_all(&app_data);

        let resources = init_db_at(app_data.clone())
            .await
            .expect("initialize database");
        resources.pool.close().await;

        assert!(app_data.join("goshootfilm.db").is_file());
        assert!(!app_data.join("media").exists());
        assert!(!app_data.join("previews").exists());
        fs::remove_dir_all(app_data).expect("remove test app data");
    }

    #[tokio::test]
    async fn current_film_catalog_is_imported_once_with_canonical_values() {
        let pool = memory_pool().await;
        MIGRATOR.run(&pool).await.expect("run migrations");
        sync_film_catalog(&pool)
            .await
            .expect("sync current catalog");
        sync_film_catalog(&pool)
            .await
            .expect("rerun current catalog sync without duplicates");
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
        let photo_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM photos")
            .fetch_one(&pool)
            .await
            .expect("count photos");
        let non_unshot_film_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM film_stocks WHERE target_status != 'unshot'")
                .fetch_one(&pool)
                .await
                .expect("count films with a non-default status");
        let type_counts: Vec<(String, i64)> =
            sqlx::query_as("SELECT type, COUNT(*) FROM film_stocks GROUP BY type ORDER BY type")
                .fetch_all(&pool)
                .await
                .expect("count films by type");
        let sample: (i64, String, String, i64, String) =
            sqlx::query_as("SELECT id, brand, name, iso, type FROM film_stocks WHERE id = 5")
                .fetch_one(&pool)
                .await
                .expect("read canonicalized sample film");

        assert_eq!(film_count, 111);
        assert_eq!(camera_count, 0);
        assert_eq!(roll_count, 0);
        assert_eq!(photo_count, 0);
        assert_eq!(non_unshot_film_count, 0);
        assert_eq!(
            type_counts,
            vec![
                ("B&W".into(), 50),
                ("Color Negative".into(), 54),
                ("Slide".into(), 7),
            ]
        );
        assert_eq!(
            sample,
            (
                5,
                "ORWO".into(),
                "ORIGINAL WOLFEN NC500".into(),
                500,
                "Color Negative".into(),
            )
        );
    }

    #[tokio::test]
    async fn builtin_status_migration_preserves_user_statuses_notes_and_custom_films() {
        let pool = memory_pool().await;
        sqlx::raw_sql(include_str!("../migrations/0001_initial.sql"))
            .execute(&pool)
            .await
            .expect("create legacy database");
        sqlx::raw_sql(include_str!("../migrations/0002_import_official_films.sql"))
            .execute(&pool)
            .await
            .expect("import legacy official catalog");

        sqlx::query(
            "UPDATE film_stocks SET target_status = 'untested', note = '用户设置' WHERE id = 3",
        )
        .execute(&pool)
        .await
        .expect("customize an original seed film");
        sqlx::query(
            "UPDATE film_stocks SET target_status = 'shot', note = '用户设置' WHERE brand = 'ADOX' AND name = 'ADOX CHS 100 II Black & White Film'",
        )
        .execute(&pool)
        .await
        .expect("customize an official catalog film");
        sqlx::query(
            "INSERT INTO film_stocks (brand, name, iso, type, target_status, note) VALUES ('Custom', 'Personal Film', 200, 'Color Negative', 'untested', '用户胶卷')",
        )
        .execute(&pool)
        .await
        .expect("insert a user film");

        let camera_id = sqlx::query("INSERT INTO cameras (brand, model) VALUES ('Nikon', 'F2')")
            .execute(&pool)
            .await
            .expect("insert related camera")
            .last_insert_rowid();
        sqlx::query("INSERT INTO rolls (camera_id, film_stock_id, roll_index) VALUES (?, 3, 1)")
            .bind(camera_id)
            .execute(&pool)
            .await
            .expect("insert related roll");

        sqlx::raw_sql(include_str!(
            "../migrations/0003_normalize_builtin_film_status.sql"
        ))
        .execute(&pool)
        .await
        .expect("normalize untouched builtin statuses");
        sqlx::raw_sql(include_str!("../migrations/0004_sync_film_catalog.sql"))
            .execute(&pool)
            .await
            .expect("remove legacy status values");
        sync_film_catalog(&pool)
            .await
            .expect("sync renamed catalog without changing record ids");

        let corrected_seed: String =
            sqlx::query_scalar("SELECT target_status FROM film_stocks WHERE id = 1")
                .fetch_one(&pool)
                .await
                .expect("read untouched seed status");
        let customized_seed: (String, Option<String>) =
            sqlx::query_as("SELECT target_status, note FROM film_stocks WHERE id = 3")
                .fetch_one(&pool)
                .await
                .expect("read customized seed film");
        let customized_official: (String, Option<String>) = sqlx::query_as(
            "SELECT target_status, note FROM film_stocks WHERE brand = 'ADOX' AND name = 'CHS 100 II'",
        )
        .fetch_one(&pool)
        .await
        .expect("read customized official film");
        let custom_film: (String, Option<String>) = sqlx::query_as(
            "SELECT target_status, note FROM film_stocks WHERE brand = 'Custom' AND name = 'Personal Film'",
        )
            .fetch_one(&pool)
            .await
            .expect("read custom film");
        let related_film: (i64, String, String) = sqlx::query_as(
            "SELECT r.film_stock_id, f.brand, f.name FROM rolls r JOIN film_stocks f ON f.id = r.film_stock_id",
        )
        .fetch_one(&pool)
        .await
        .expect("read preserved roll relationship");

        assert_eq!(corrected_seed, "unshot");
        assert_eq!(customized_seed, ("unshot".into(), Some("用户设置".into())));
        assert_eq!(
            customized_official,
            ("shot".into(), Some("用户设置".into()))
        );
        assert_eq!(custom_film, ("unshot".into(), Some("用户胶卷".into())));
        assert_eq!(related_film, (3, "Kodak".into(), "Ektachrome E100".into()));
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

    #[tokio::test]
    async fn global_roll_indexes_span_cameras_and_close_gaps_after_cascades() {
        let pool = memory_pool().await;
        MIGRATOR.run(&pool).await.expect("run migrations");
        repair_legacy_camera_schema(&pool)
            .await
            .expect("repair schema");
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .expect("enable foreign keys");

        let first_camera = sqlx::query("INSERT INTO cameras (brand, model) VALUES ('Nikon', 'F2')")
            .execute(&pool)
            .await
            .expect("insert first camera")
            .last_insert_rowid();
        let second_camera =
            sqlx::query("INSERT INTO cameras (brand, model) VALUES ('Canon', 'New F-1')")
                .execute(&pool)
                .await
                .expect("insert second camera")
                .last_insert_rowid();
        for (camera_id, film_id, stale_index, created_at) in [
            (first_camera, 1_i64, 1_i64, "2026-01-01 00:00:00"),
            (second_camera, 2_i64, 1_i64, "2026-02-01 00:00:00"),
            (first_camera, 3_i64, 2_i64, "2026-03-01 00:00:00"),
        ] {
            sqlx::query(
                "INSERT INTO rolls (camera_id, film_stock_id, roll_index, created_at) VALUES (?, ?, ?, ?)",
            )
            .bind(camera_id)
            .bind(film_id)
            .bind(stale_index)
            .bind(created_at)
            .execute(&pool)
            .await
            .expect("insert roll");
        }

        let mut transaction = pool.begin().await.expect("begin reindex transaction");
        reindex_rolls(&mut transaction)
            .await
            .expect("reindex rolls");
        transaction.commit().await.expect("commit reindex");
        let indexes: Vec<i64> =
            sqlx::query_scalar("SELECT roll_index FROM rolls ORDER BY created_at, id")
                .fetch_all(&pool)
                .await
                .expect("read global indexes");
        assert_eq!(indexes, vec![1, 2, 3]);
        let next_index: i64 =
            sqlx::query_scalar::<_, Option<i64>>("SELECT MAX(roll_index) FROM rolls")
                .fetch_one(&pool)
                .await
                .expect("read next global index")
                .unwrap_or(0)
                + 1;
        assert_eq!(next_index, 4);

        let middle_roll_id: i64 = sqlx::query_scalar("SELECT id FROM rolls WHERE roll_index = 2")
            .fetch_one(&pool)
            .await
            .expect("read middle roll");
        let mut transaction = pool.begin().await.expect("begin delete transaction");
        sqlx::query("DELETE FROM rolls WHERE id = ?")
            .bind(middle_roll_id)
            .execute(&mut *transaction)
            .await
            .expect("delete middle roll");
        reindex_rolls(&mut transaction)
            .await
            .expect("close deleted roll gap");
        transaction.commit().await.expect("commit roll deletion");
        let indexes: Vec<i64> =
            sqlx::query_scalar("SELECT roll_index FROM rolls ORDER BY roll_index")
                .fetch_all(&pool)
                .await
                .expect("read indexes after roll deletion");
        assert_eq!(indexes, vec![1, 2]);

        let mut transaction = pool.begin().await.expect("begin cascade transaction");
        sqlx::query("DELETE FROM film_stocks WHERE id = 1")
            .execute(&mut *transaction)
            .await
            .expect("delete film and cascade roll");
        reindex_rolls(&mut transaction)
            .await
            .expect("close cascade gap");
        transaction.commit().await.expect("commit cascade deletion");
        let indexes: Vec<i64> =
            sqlx::query_scalar("SELECT roll_index FROM rolls ORDER BY roll_index")
                .fetch_all(&pool)
                .await
                .expect("read indexes after cascade");
        assert_eq!(indexes, vec![1]);
    }

    #[tokio::test]
    async fn first_business_roll_index_does_not_depend_on_sqlite_id_sequence() {
        let pool = memory_pool().await;
        MIGRATOR.run(&pool).await.expect("run migrations");
        repair_legacy_camera_schema(&pool)
            .await
            .expect("repair schema");
        let camera_id = sqlx::query("INSERT INTO cameras (brand, model) VALUES ('Nikon', 'F2')")
            .execute(&pool)
            .await
            .expect("insert camera")
            .last_insert_rowid();
        let old_id = sqlx::query(
            "INSERT INTO rolls (camera_id, film_stock_id, roll_index) VALUES (?, 1, 8)",
        )
        .bind(camera_id)
        .execute(&pool)
        .await
        .expect("insert old roll")
        .last_insert_rowid();
        sqlx::query("DELETE FROM rolls")
            .execute(&pool)
            .await
            .expect("clear old rolls without resetting sqlite sequence");

        let mut transaction = pool.begin().await.expect("begin insert transaction");
        reindex_rolls(&mut transaction)
            .await
            .expect("normalize empty rolls");
        let next_index = next_roll_index(&mut transaction)
            .await
            .expect("compute first business index");
        let new_id = sqlx::query(
            "INSERT INTO rolls (camera_id, film_stock_id, roll_index) VALUES (?, 2, ?)",
        )
        .bind(camera_id)
        .bind(next_index)
        .execute(&mut *transaction)
        .await
        .expect("insert new first roll")
        .last_insert_rowid();
        transaction.commit().await.expect("commit new roll");

        assert!(new_id > old_id);
        assert_eq!(next_index, 1);
        let stored_index: i64 = sqlx::query_scalar("SELECT roll_index FROM rolls WHERE id = ?")
            .bind(new_id)
            .fetch_one(&pool)
            .await
            .expect("read new business index");
        assert_eq!(stored_index, 1);
    }
}
