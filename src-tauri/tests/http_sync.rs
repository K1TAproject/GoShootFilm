use goshootfilm_lib::sync::SyncManager;
use serde_json::{json, Value};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use std::fs;
use std::net::{IpAddr, Ipv4Addr};
use std::time::{SystemTime, UNIX_EPOCH};

async fn test_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("create http test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("apply migrations");
    pool
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_pairing_sync_and_revocation_flow() {
    let pool = test_pool().await;
    let root = std::env::temp_dir().join(format!(
        "goshootfilm-http-sync-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let media = root.join("media");
    let previews = root.join("previews");
    let source = media.join("rolls/1/edit/08.jpg");
    fs::create_dir_all(source.parent().unwrap()).expect("create media directory");
    fs::create_dir_all(&previews).expect("create preview directory");
    image::RgbImage::from_pixel(800, 600, image::Rgb([90, 110, 130]))
        .save_with_format(&source, image::ImageFormat::Jpeg)
        .expect("write source image");
    let source_bytes = fs::read(&source).expect("read source image");

    let camera_id: i64 = sqlx::query("INSERT INTO cameras (brand, model) VALUES ('Nikon', 'F2')")
        .execute(&pool)
        .await
        .expect("insert camera")
        .last_insert_rowid();
    let film_id: i64 = sqlx::query_scalar("SELECT id FROM film_stocks LIMIT 1")
        .fetch_one(&pool)
        .await
        .expect("seeded film");
    let roll_id: i64 = sqlx::query(
        "INSERT INTO rolls (camera_id, film_stock_id, roll_index, shot_month, city)
         VALUES (?, ?, 1, '2026-10', '杭州')",
    )
    .bind(camera_id)
    .bind(film_id)
    .execute(&pool)
    .await
    .expect("insert roll")
    .last_insert_rowid();
    sqlx::query(
        "INSERT INTO photos (roll_id, frame_number, edit_scan_path, is_favorite)
         VALUES (?, 8, 'rolls/1/edit/08.jpg', 1)",
    )
    .bind(roll_id)
    .execute(&pool)
    .await
    .expect("insert photo");

    let manager = SyncManager::new_without_app(pool.clone());
    manager.set_media_directories_for_test(media.clone(), previews.clone());
    let status = manager
        .start_on_for_test(IpAddr::V4(Ipv4Addr::LOCALHOST))
        .await
        .expect("start loopback sync server");
    let base = status.address.expect("sync address");

    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("build http client");
    let health_response = client
        .get(format!("{base}/api/v1/health"))
        .send()
        .await
        .expect("health request");
    let health_status = health_response.status();
    let health_text = health_response.text().await.expect("health text");
    assert!(
        health_status.is_success(),
        "health status {health_status}: {health_text}"
    );
    let health: Value = serde_json::from_str(&health_text).expect("health json");
    assert_eq!(health["protocolVersion"], 1);
    assert!(health["serverFingerprint"].as_str().is_some());

    let pairing = manager
        .create_pairing_session()
        .await
        .expect("create pairing session");
    let payload: Value = serde_json::from_str(&pairing.payload).expect("pairing payload json");
    let token = payload["token"].as_str().expect("pair token").to_string();

    let request: Value = client
        .post(format!("{base}/api/v1/pair/request"))
        .json(&json!({
            "token": token,
            "deviceName": "Integration Phone",
            "platform": "android",
            "protocolVersion": 1,
        }))
        .send()
        .await
        .expect("pair request")
        .json()
        .await
        .expect("pair request json");
    let request_id = request["requestId"]
        .as_str()
        .expect("request id")
        .to_string();

    let pending: Value = client
        .get(format!("{base}/api/v1/pair/status/{request_id}"))
        .send()
        .await
        .expect("pending status")
        .json()
        .await
        .expect("pending json");
    assert_eq!(pending["status"], "pending");

    manager
        .approve_pairing(&request_id)
        .await
        .expect("approve pairing");

    let approved: Value = client
        .get(format!("{base}/api/v1/pair/status/{request_id}"))
        .send()
        .await
        .expect("approved status")
        .json()
        .await
        .expect("approved json");
    assert_eq!(approved["status"], "approved");
    let credential = approved["credential"]
        .as_str()
        .expect("credential")
        .to_string();
    let device_id = approved["deviceId"]
        .as_str()
        .expect("device id")
        .to_string();

    let changes: Value = client
        .get(format!("{base}/api/v1/sync/changes?cursor=0"))
        .bearer_auth(&credential)
        .send()
        .await
        .expect("sync changes")
        .json()
        .await
        .expect("sync changes json");
    assert_eq!(changes["protocolVersion"], 1);
    assert!(changes["changes"].as_array().is_some());
    assert_eq!(changes["stats"]["cameraCount"].as_i64(), Some(1));
    assert_eq!(changes["stats"]["photoCount"].as_i64(), Some(1));

    let photo_change = changes["changes"]
        .as_array()
        .expect("changes array")
        .iter()
        .find(|change| change["entityType"] == "film_photo")
        .expect("film photo change");
    assert_eq!(photo_change["data"]["frameNumber"].as_i64(), Some(8));
    let thumbnail = photo_change["assets"]
        .as_array()
        .expect("photo assets")
        .iter()
        .find(|asset| asset["assetType"] == "thumbnail")
        .expect("thumbnail descriptor");
    let thumbnail_id = thumbnail["id"].as_str().expect("thumbnail id");

    let edit_original = photo_change["editOriginal"]
        .as_object()
        .expect("edit original descriptor");
    let edit_original_id = edit_original["id"].as_str().expect("edit original id");
    assert_eq!(edit_original["mimeType"], "image/jpeg");
    assert_eq!(edit_original["available"], true);

    let edit_original_response = client
        .get(format!("{base}/api/v1/edit-originals/{edit_original_id}"))
        .bearer_auth(&credential)
        .send()
        .await
        .expect("download edit original");
    assert!(edit_original_response.status().is_success());
    let edit_original_hash = edit_original_response
        .headers()
        .get("x-goshootfilm-sha256")
        .and_then(|value| value.to_str().ok())
        .expect("edit original hash")
        .to_string();
    assert_eq!(edit_original_hash.len(), 64);
    let edit_original_bytes = edit_original_response
        .bytes()
        .await
        .expect("edit original bytes");
    assert_eq!(edit_original_bytes.as_ref(), source_bytes.as_slice());

    let resource = client
        .get(format!("{base}/api/v1/resources/{thumbnail_id}"))
        .bearer_auth(&credential)
        .send()
        .await
        .expect("download thumbnail");
    assert!(resource.status().is_success());
    let resource_hash = resource
        .headers()
        .get("x-goshootfilm-sha256")
        .and_then(|value| value.to_str().ok())
        .expect("resource hash")
        .to_string();
    assert_eq!(resource_hash.len(), 64);
    let resource_bytes = resource.bytes().await.expect("thumbnail bytes");
    assert!(!resource_bytes.is_empty());
    let generated_count = fs::read_dir(previews.join("mobile").join("thumbs"))
        .expect("read generated thumbnails")
        .count();
    assert!(generated_count > 0);
    assert_eq!(fs::read(&source).expect("read source again"), source_bytes);

    manager
        .revoke_device(&device_id)
        .await
        .expect("revoke device");
    let unauthorized = client
        .get(format!("{base}/api/v1/sync/changes?cursor=0"))
        .bearer_auth(&credential)
        .send()
        .await
        .expect("post-revoke request");
    assert_eq!(unauthorized.status(), reqwest::StatusCode::UNAUTHORIZED);

    manager.stop().await.expect("stop sync server");
    pool.close().await;
    fs::remove_dir_all(root).expect("remove http sync test directory");
}
