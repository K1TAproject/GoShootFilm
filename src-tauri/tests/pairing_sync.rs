use goshootfilm_lib::sync::{PairState, SyncManager};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

async fn test_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("create pairing test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("apply migrations");
    pool
}

#[tokio::test]
async fn pairing_is_one_time_and_revocation_is_immediate() {
    let pool = test_pool().await;
    let manager = SyncManager::new_without_app(pool.clone());
    let token = "a".repeat(64);
    manager
        .begin_pairing_for_test(&token, now_epoch() + 120)
        .await;
    let request_id = manager
        .create_pair_request(&token, "Test Phone", "android")
        .await
        .expect("create pair request");
    assert!(manager
        .create_pair_request(&token, "Second Phone", "android")
        .await
        .is_err());
    assert!(matches!(
        manager.pair_state(&request_id).await.unwrap(),
        PairState::Pending
    ));
    manager
        .approve_pairing(&request_id)
        .await
        .expect("approve pairing");
    let state = manager.pair_state(&request_id).await.unwrap();
    let (device_id, credential) = match state {
        PairState::Approved {
            device_id,
            credential,
        } => (device_id, credential),
        _ => panic!("pairing should be approved"),
    };
    assert!(manager.authenticate(&credential).await.is_some());
    assert!(matches!(
        manager.pair_state(&request_id).await.unwrap(),
        PairState::Delivered
    ));
    manager
        .revoke_device(&device_id)
        .await
        .expect("revoke device");
    assert!(manager.authenticate(&credential).await.is_none());
    let status = manager.status().await.expect("read status");
    assert_eq!(status.paired_device_count, 0);
    pool.close().await;
}

#[tokio::test]
async fn expired_pairing_token_is_rejected_without_creating_request() {
    let pool = test_pool().await;
    let manager = SyncManager::new_without_app(pool.clone());
    let token = "b".repeat(64);
    manager
        .begin_pairing_for_test(&token, now_epoch() - 1)
        .await;
    assert!(manager
        .create_pair_request(&token, "Expired Phone", "android")
        .await
        .is_err());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mobile_pair_requests")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    pool.close().await;
}
