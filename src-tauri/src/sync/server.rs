use super::assets;
use super::data;
use super::{now_epoch, PairState, SyncManager, PROTOCOL_VERSION};
use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "bad_request", message)
    }

    fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "设备未授权或已被撤销",
        )
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", message)
    }

    fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "同步服务内部错误",
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({
                "code": self.code,
                "error": self.message,
            })),
        )
            .into_response()
    }
}

pub fn router(manager: Arc<SyncManager>) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/pair/request", post(pair_request))
        .route("/api/v1/pair/status/{request_id}", get(pair_status))
        .route("/api/v1/sync/changes", get(sync_changes))
        .route("/api/v1/resources/{asset_id}", get(resource))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(CorsLayer::permissive())
        .with_state(manager)
}

async fn health(State(manager): State<Arc<SyncManager>>) -> Result<Json<Value>, ApiError> {
    let fingerprint = manager
        .fingerprint()
        .await
        .map_err(|_| ApiError::internal())?;
    let pairing_available = manager.pairing_available().await;
    Ok(Json(json!({
        "protocolVersion": PROTOCOL_VERSION,
        "serverFingerprint": fingerprint,
        "appVersion": env!("CARGO_PKG_VERSION"),
        "pairingAvailable": pairing_available,
    })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PairRequestBody {
    token: String,
    device_name: String,
    platform: Option<String>,
    protocol_version: Option<u32>,
}

async fn pair_request(
    State(manager): State<Arc<SyncManager>>,
    Json(body): Json<PairRequestBody>,
) -> Result<Json<Value>, ApiError> {
    let version = body.protocol_version.unwrap_or(PROTOCOL_VERSION);
    if version != PROTOCOL_VERSION {
        return Err(ApiError::new(
            StatusCode::UPGRADE_REQUIRED,
            "protocol_incompatible",
            "手机端协议版本与电脑端不兼容",
        ));
    }
    let device_name: String = body.device_name.trim().chars().take(80).collect();
    if device_name.is_empty() {
        return Err(ApiError::bad_request("设备名称不能为空"));
    }
    let platform = match body.platform.as_deref() {
        Some("android") => "android",
        Some("ios") => "ios",
        Some("web") => "web",
        _ => "android",
    };
    let request_id = manager
        .create_pair_request(body.token.trim(), &device_name, platform)
        .await
        .map_err(|_| {
            ApiError::new(
                StatusCode::GONE,
                "pairing_token_invalid",
                "二维码已过期或已被使用，请在电脑端重新生成",
            )
        })?;
    Ok(Json(json!({
        "requestId": request_id,
        "expiresAt": now_epoch() + 120,
    })))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PairStatusBody {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    device_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    credential: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    server_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    protocol_version: Option<u32>,
}

async fn pair_status(
    State(manager): State<Arc<SyncManager>>,
    Path(request_id): Path<String>,
) -> Result<Json<PairStatusBody>, ApiError> {
    let state = manager
        .pair_state(&request_id)
        .await
        .map_err(|_| ApiError::not_found("配对请求不存在"))?;
    match state {
        PairState::Pending => Ok(Json(PairStatusBody {
            status: "pending".into(),
            request_id: Some(request_id),
            device_id: None,
            credential: None,
            server_fingerprint: None,
            protocol_version: None,
        })),
        PairState::Approved {
            device_id,
            credential,
        } => {
            let fingerprint = manager
                .fingerprint()
                .await
                .map_err(|_| ApiError::internal())?;
            Ok(Json(PairStatusBody {
                status: "approved".into(),
                request_id: Some(request_id),
                device_id: Some(device_id),
                credential: Some(credential),
                server_fingerprint: Some(fingerprint),
                protocol_version: Some(PROTOCOL_VERSION),
            }))
        }
        PairState::Delivered => Ok(Json(PairStatusBody {
            status: "delivered".into(),
            request_id: Some(request_id),
            device_id: None,
            credential: None,
            server_fingerprint: None,
            protocol_version: None,
        })),
        PairState::CredentialExpired => Ok(Json(PairStatusBody {
            status: "credentialExpired".into(),
            request_id: Some(request_id),
            device_id: None,
            credential: None,
            server_fingerprint: None,
            protocol_version: None,
        })),
        PairState::Denied => Ok(Json(PairStatusBody {
            status: "denied".into(),
            request_id: Some(request_id),
            device_id: None,
            credential: None,
            server_fingerprint: None,
            protocol_version: None,
        })),
        PairState::Expired => Ok(Json(PairStatusBody {
            status: "expired".into(),
            request_id: Some(request_id),
            device_id: None,
            credential: None,
            server_fingerprint: None,
            protocol_version: None,
        })),
    }
}

async fn authenticate(
    manager: &SyncManager,
    headers: &HeaderMap,
) -> Result<super::DeviceRecord, ApiError> {
    let value = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(ApiError::unauthorized)?;
    let token = value
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .ok_or_else(ApiError::unauthorized)?;
    manager
        .authenticate(token)
        .await
        .ok_or_else(ApiError::unauthorized)
}

#[derive(Debug, Deserialize)]
struct ChangesQuery {
    cursor: Option<i64>,
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChangesResponse {
    protocol_version: u32,
    cursor: i64,
    reset_required: bool,
    has_more: bool,
    server_time: String,
    stats: data::SyncStats,
    changes: Vec<data::SyncChange>,
}

async fn sync_changes(
    State(manager): State<Arc<SyncManager>>,
    headers: HeaderMap,
    Query(query): Query<ChangesQuery>,
) -> Result<Json<ChangesResponse>, ApiError> {
    let device = authenticate(&manager, &headers).await?;
    let cursor = query.cursor.unwrap_or(0);
    if cursor < 0 {
        return Err(ApiError::bad_request("同步游标无效"));
    }
    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    let page = data::changes_since(manager.db(), cursor, limit)
        .await
        .map_err(|_| ApiError::internal())?;

    let media_dir = match manager.media_directories().await {
        Ok((media, _)) => Some(media),
        Err(_) => None,
    };
    let mut changes = Vec::with_capacity(page.changes.len());
    for row in page.changes {
        changes.push(data::materialize(manager.db(), media_dir.as_deref(), row).await);
    }
    let stats = data::stats(manager.db())
        .await
        .map_err(|_| ApiError::internal())?;
    let server_time: String = sqlx::query_scalar("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')")
        .fetch_one(manager.db())
        .await
        .map_err(|_| ApiError::internal())?;
    manager.touch_device(&device.id).await;

    Ok(Json(ChangesResponse {
        protocol_version: PROTOCOL_VERSION,
        cursor: if page.reset_required {
            0
        } else {
            page.next_cursor
        },
        reset_required: page.reset_required,
        has_more: page.next_cursor < page.max_seq,
        server_time,
        stats,
        changes,
    }))
}

async fn resource(
    State(manager): State<Arc<SyncManager>>,
    headers: HeaderMap,
    Path(asset_id): Path<String>,
) -> Result<Response, ApiError> {
    let _device = authenticate(&manager, &headers).await?;
    let lock = manager.asset_lock(&asset_id).await;
    let _guard = lock.lock().await;
    let (media, previews) = manager
        .media_directories()
        .await
        .map_err(|_| ApiError::not_found("图库暂不可用"))?;
    let generated = assets::generate_file(
        manager.db(),
        &media,
        &previews,
        &asset_id,
        &manager.asset_semaphore,
    )
    .await
    .map_err(|error| {
        if error.contains("not found")
            || error.contains("no longer exists")
            || error.contains("missing")
        {
            ApiError::not_found("图片资源不存在或已失效")
        } else {
            ApiError::internal()
        }
    })?;
    let bytes = tokio::fs::read(&generated.path)
        .await
        .map_err(|_| ApiError::internal())?;
    let mut response_headers = HeaderMap::new();
    response_headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/jpeg"));
    // Asset IDs stay stable when a source image is replaced, so the response
    // must not be cached as immutable. The mobile client verifies the hash.
    response_headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    if let Ok(etag) = HeaderValue::from_str(&format!("\"{}\"", generated.sha256)) {
        response_headers.insert(header::ETAG, etag);
    }
    if let Ok(hash) = HeaderValue::from_str(&generated.sha256) {
        response_headers.insert("x-goshootfilm-sha256", hash);
    }
    if let Ok(length) = HeaderValue::from_str(&generated.byte_size.to_string()) {
        response_headers.insert(header::CONTENT_LENGTH, length);
    }
    Ok((response_headers, Body::from(bytes)).into_response())
}
