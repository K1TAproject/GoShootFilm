pub mod assets;
pub mod data;
pub mod edit_originals;
pub mod server;

use crate::startup;
use crate::AppState;
use qrcode::render::svg;
use qrcode::QrCode;
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex, Semaphore};

pub const PROTOCOL_VERSION: u32 = 1;
pub const DEFAULT_PORT: u16 = 47_821;
const PAIRING_TTL_SECONDS: i64 = 120;
const CREDENTIAL_TTL_SECONDS: i64 = 120;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileSyncStatus {
    pub running: bool,
    pub address: Option<String>,
    pub port: Option<u16>,
    pub fingerprint: String,
    pub protocol_version: u32,
    pub paired_device_count: i64,
    pub pending_request_count: i64,
    pub default_port: u16,
    pub local_only: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingSessionResponse {
    pub qr_svg: String,
    pub payload: String,
    pub url: String,
    pub expires_at: i64,
    pub fingerprint: String,
    pub server_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingRequestInfo {
    pub id: String,
    pub device_name: String,
    pub platform: String,
    pub created_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileDeviceInfo {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub paired_at: String,
    pub last_sync_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeviceRecord {
    pub id: String,
}

#[derive(Debug)]
pub enum PairState {
    Pending,
    Approved {
        device_id: String,
        credential: String,
    },
    Delivered,
    CredentialExpired,
    Denied,
    Expired,
}

struct RunningServer {
    address: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<()>,
}

struct PairingSession {
    token_hash: String,
    expires_at: i64,
}

struct PendingCredential {
    credential: String,
    expires_at: i64,
}

#[derive(Default)]
struct PairingState {
    session: Option<PairingSession>,
    credentials: HashMap<String, PendingCredential>,
}

pub struct SyncManager {
    app: Option<AppHandle>,
    db: SqlitePool,
    runtime: Mutex<Option<RunningServer>>,
    start_stop: Mutex<()>,
    pairing: Mutex<PairingState>,
    asset_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    media_override: OnceLock<(PathBuf, PathBuf)>,
    pub(crate) asset_semaphore: Semaphore,
}

impl SyncManager {
    pub fn new(app: AppHandle, db: SqlitePool) -> Arc<Self> {
        Arc::new(Self {
            app: Some(app),
            db,
            runtime: Mutex::new(None),
            start_stop: Mutex::new(()),
            pairing: Mutex::new(PairingState::default()),
            asset_locks: Mutex::new(HashMap::new()),
            media_override: OnceLock::new(),
            asset_semaphore: Semaphore::new(2),
        })
    }

    #[doc(hidden)]
    pub fn new_without_app(db: SqlitePool) -> Arc<Self> {
        Arc::new(Self {
            app: None,
            db,
            runtime: Mutex::new(None),
            start_stop: Mutex::new(()),
            pairing: Mutex::new(PairingState::default()),
            asset_locks: Mutex::new(HashMap::new()),
            media_override: OnceLock::new(),
            asset_semaphore: Semaphore::new(1),
        })
    }

    #[doc(hidden)]
    pub fn set_media_directories_for_test(&self, media: PathBuf, previews: PathBuf) {
        let _ = self.media_override.set((media, previews));
    }

    #[doc(hidden)]
    pub async fn begin_pairing_for_test(&self, token: &str, expires_at: i64) {
        self.pairing.lock().await.session = Some(PairingSession {
            token_hash: sha256_hex(token),
            expires_at,
        });
    }

    pub(crate) fn db(&self) -> &SqlitePool {
        &self.db
    }

    pub(crate) async fn media_directories(&self) -> Result<(PathBuf, PathBuf), String> {
        if let Some((media, previews)) = self.media_override.get() {
            return Ok((media.clone(), previews.clone()));
        }
        let app = self
            .app
            .as_ref()
            .ok_or_else(|| "同步服务未连接桌面应用".to_string())?;
        let state = app.state::<AppState>();
        crate::library_directories(&state)
    }

    pub(crate) async fn asset_lock(&self, asset_id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.asset_locks.lock().await;
        locks
            .entry(asset_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    pub async fn fingerprint(&self) -> Result<String, String> {
        let secret: Option<String> =
            sqlx::query_scalar("SELECT value FROM app_metadata WHERE key = 'mobile_sync_secret'")
                .fetch_optional(&self.db)
                .await
                .map_err(|error| error.to_string())?;
        let secret = match secret {
            Some(secret) => secret,
            None => {
                let generated = random_hex(32);
                sqlx::query(
                    "INSERT OR IGNORE INTO app_metadata (key, value) VALUES ('mobile_sync_secret', ?)",
                )
                .bind(&generated)
                .execute(&self.db)
                .await
                .map_err(|error| error.to_string())?;
                sqlx::query_scalar(
                    "SELECT value FROM app_metadata WHERE key = 'mobile_sync_secret'",
                )
                .fetch_one(&self.db)
                .await
                .map_err(|error| error.to_string())?
            }
        };
        Ok(sha256_hex(&secret)[..32].to_string())
    }

    pub async fn status(&self) -> Result<MobileSyncStatus, String> {
        let (running, address, port, local_only) = {
            let runtime = self.runtime.lock().await;
            match runtime.as_ref() {
                Some(server) => (
                    true,
                    Some(format!("http://{}", server.address)),
                    Some(server.address.port()),
                    server.address.ip().is_loopback(),
                ),
                None => (false, None, None, false),
            }
        };
        let fingerprint = self.fingerprint().await?;
        let paired_device_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM mobile_devices WHERE revoked_at IS NULL")
                .fetch_one(&self.db)
                .await
                .map_err(|error| error.to_string())?;
        let pending_request_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM mobile_pair_requests
             WHERE status = 'pending' AND expires_at > strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .fetch_one(&self.db)
        .await
        .map_err(|error| error.to_string())?;
        Ok(MobileSyncStatus {
            running,
            address,
            port,
            fingerprint,
            protocol_version: PROTOCOL_VERSION,
            paired_device_count,
            pending_request_count,
            default_port: DEFAULT_PORT,
            local_only,
        })
    }

    pub async fn start(self: &Arc<Self>) -> Result<MobileSyncStatus, String> {
        let ip = detect_lan_ip().unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
        self.start_on(ip).await
    }

    #[doc(hidden)]
    pub async fn start_on_for_test(
        self: &Arc<Self>,
        ip: IpAddr,
    ) -> Result<MobileSyncStatus, String> {
        self.start_on(ip).await
    }

    async fn start_on(self: &Arc<Self>, ip: IpAddr) -> Result<MobileSyncStatus, String> {
        let _guard = self.start_stop.lock().await;
        if self.runtime.lock().await.is_some() {
            return self.status().await;
        }

        let mut listener = None;
        let mut last_error = String::new();
        for offset in 0..10 {
            let port = DEFAULT_PORT.saturating_add(offset);
            match TcpListener::bind(SocketAddr::new(ip, port)).await {
                Ok(value) => {
                    listener = Some(value);
                    break;
                }
                Err(error) => {
                    last_error = error.to_string();
                }
            }
        }
        let listener = listener.ok_or_else(|| format!("无法启动移动端同步服务: {last_error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("无法读取同步服务地址: {error}"))?;
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let manager = self.clone();
        let task = tokio::spawn(async move {
            let app = manager.clone();
            let router = server::router(app);
            let result = axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await;
            if let Err(error) = result {
                startup::record("mobile sync", &format!("server stopped: {error}"));
            }
        });
        *self.runtime.lock().await = Some(RunningServer {
            address,
            shutdown: Some(shutdown_tx),
            task,
        });
        drop(_guard);
        self.status().await
    }

    pub async fn stop(&self) -> Result<MobileSyncStatus, String> {
        let _guard = self.start_stop.lock().await;
        let running = self.runtime.lock().await.take();
        if let Some(mut running) = running {
            if let Some(shutdown) = running.shutdown.take() {
                let _ = shutdown.send(());
            }
            let mut task = running.task;
            if tokio::time::timeout(Duration::from_secs(3), &mut task)
                .await
                .is_err()
            {
                task.abort();
                let _ = task.await;
            }
        }
        drop(_guard);
        self.status().await
    }

    pub async fn create_pairing_session(
        self: &Arc<Self>,
    ) -> Result<PairingSessionResponse, String> {
        let status = self.start().await?;
        let url = status
            .address
            .ok_or_else(|| "同步服务尚未启动".to_string())?;
        let token = random_hex(32);
        let expires_at = now_epoch() + PAIRING_TTL_SECONDS;
        {
            let mut pairing = self.pairing.lock().await;
            pairing.session = Some(PairingSession {
                token_hash: sha256_hex(&token),
                expires_at,
            });
        }
        let payload = json!({
            "v": PROTOCOL_VERSION,
            "url": url,
            "token": token,
            "fingerprint": status.fingerprint,
            "expiresAt": expires_at,
            "serverName": "GoShootFilm",
        })
        .to_string();
        let code = QrCode::new(payload.as_bytes())
            .map_err(|error| format!("无法生成配对二维码: {error}"))?;
        let qr_svg = code.render::<svg::Color>().min_dimensions(240, 240).build();
        Ok(PairingSessionResponse {
            qr_svg,
            payload,
            url,
            expires_at,
            fingerprint: status.fingerprint,
            server_name: "GoShootFilm".into(),
        })
    }

    pub async fn pairing_available(&self) -> bool {
        self.pairing
            .lock()
            .await
            .session
            .as_ref()
            .is_some_and(|session| session.expires_at >= now_epoch())
    }

    pub async fn consume_pair_token(&self, token: &str) -> bool {
        let token_hash = sha256_hex(token);
        let mut pairing = self.pairing.lock().await;
        let valid = pairing.session.as_ref().is_some_and(|session| {
            session.token_hash == token_hash && session.expires_at >= now_epoch()
        });
        if valid {
            pairing.session = None;
        }
        valid
    }

    pub async fn create_pair_request(
        &self,
        token: &str,
        device_name: &str,
        platform: &str,
    ) -> Result<String, String> {
        if !self.consume_pair_token(token).await {
            return Err("配对令牌无效或已过期".into());
        }
        let request_id = random_hex(16);
        sqlx::query(
            "INSERT INTO mobile_pair_requests
             (id, device_name, platform, status, created_at, expires_at)
             VALUES (?, ?, ?, 'pending',
                     strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                     strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '+2 minutes'))",
        )
        .bind(&request_id)
        .bind(device_name)
        .bind(platform)
        .execute(&self.db)
        .await
        .map_err(|error| error.to_string())?;
        if let Some(app) = &self.app {
            let _ = app.emit(
                "mobile-sync://pair-request",
                json!({
                    "id": request_id,
                    "deviceName": device_name,
                    "platform": platform,
                }),
            );
        }
        Ok(request_id)
    }

    pub async fn pair_state(&self, request_id: &str) -> Result<PairState, String> {
        let row: Option<(String, Option<String>, Option<String>, i64)> = sqlx::query_as(
            "SELECT status, approved_device_id, credential_delivered_at,
                    CASE WHEN expires_at <= strftime('%Y-%m-%dT%H:%M:%fZ', 'now') THEN 1 ELSE 0 END
             FROM mobile_pair_requests WHERE id = ?",
        )
        .bind(request_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|error| error.to_string())?;
        let Some((status, device_id, delivered_at, expired)) = row else {
            return Err("配对请求不存在".into());
        };
        match status.as_str() {
            "pending" if expired != 0 => {
                sqlx::query("UPDATE mobile_pair_requests SET status = 'expired' WHERE id = ? AND status = 'pending'")
                    .bind(request_id)
                    .execute(&self.db)
                    .await
                    .map_err(|error| error.to_string())?;
                Ok(PairState::Expired)
            }
            "pending" => Ok(PairState::Pending),
            "denied" => Ok(PairState::Denied),
            "expired" => Ok(PairState::Expired),
            "approved" => {
                if delivered_at.is_some() {
                    return Ok(PairState::Delivered);
                }
                let mut pairing = self.pairing.lock().await;
                let Some(pending) = pairing.credentials.remove(request_id) else {
                    return Ok(PairState::CredentialExpired);
                };
                if pending.expires_at < now_epoch() {
                    return Ok(PairState::CredentialExpired);
                }
                sqlx::query(
                    "UPDATE mobile_pair_requests
                     SET credential_delivered_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE id = ?",
                )
                .bind(request_id)
                .execute(&self.db)
                .await
                .map_err(|error| error.to_string())?;
                Ok(PairState::Approved {
                    device_id: device_id.unwrap_or_default(),
                    credential: pending.credential,
                })
            }
            _ => Ok(PairState::Expired),
        }
    }

    pub async fn approve_pairing(&self, request_id: &str) -> Result<String, String> {
        let mut transaction = self.db.begin().await.map_err(|error| error.to_string())?;
        let row: Option<(String, String, String, i64)> = sqlx::query_as(
            "SELECT status, device_name, platform,
                    CASE WHEN expires_at <= strftime('%Y-%m-%dT%H:%M:%fZ', 'now') THEN 1 ELSE 0 END
             FROM mobile_pair_requests WHERE id = ?",
        )
        .bind(request_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;
        let Some((status, device_name, platform, expired)) = row else {
            return Err("配对请求不存在".into());
        };
        if status != "pending" {
            return Err("该配对请求已处理".into());
        }
        if expired != 0 {
            sqlx::query("UPDATE mobile_pair_requests SET status = 'expired' WHERE id = ?")
                .bind(request_id)
                .execute(&mut *transaction)
                .await
                .map_err(|error| error.to_string())?;
            transaction
                .commit()
                .await
                .map_err(|error| error.to_string())?;
            return Err("配对请求已过期，请在电脑端重新生成二维码".into());
        }
        let device_id = random_hex(16);
        let credential = random_hex(32);
        let credential_hash = sha256_hex(&credential);
        sqlx::query(
            "INSERT INTO mobile_devices (id, name, platform, credential_hash, paired_at)
             VALUES (?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(&device_id)
        .bind(&device_name)
        .bind(&platform)
        .bind(&credential_hash)
        .execute(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;
        sqlx::query(
            "UPDATE mobile_pair_requests
             SET status = 'approved', approved_device_id = ?,
                 approved_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 expires_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '+2 minutes')
             WHERE id = ?",
        )
        .bind(&device_id)
        .bind(request_id)
        .execute(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;
        transaction
            .commit()
            .await
            .map_err(|error| error.to_string())?;
        let mut pairing = self.pairing.lock().await;
        pairing.credentials.insert(
            request_id.to_string(),
            PendingCredential {
                credential,
                expires_at: now_epoch() + CREDENTIAL_TTL_SECONDS,
            },
        );
        if let Some(app) = &self.app {
            let _ = app.emit("mobile-sync://pairing-changed", json!({}));
        }
        Ok("设备已批准".into())
    }

    pub async fn deny_pairing(&self, request_id: &str) -> Result<String, String> {
        let result = sqlx::query(
            "UPDATE mobile_pair_requests SET status = 'denied'
             WHERE id = ? AND status = 'pending'",
        )
        .bind(request_id)
        .execute(&self.db)
        .await
        .map_err(|error| error.to_string())?;
        if result.rows_affected() == 0 {
            return Err("配对请求不存在或已处理".into());
        }
        if let Some(app) = &self.app {
            let _ = app.emit("mobile-sync://pairing-changed", json!({}));
        }
        Ok("已拒绝配对请求".into())
    }

    pub async fn pairing_requests(&self) -> Result<Vec<PairingRequestInfo>, String> {
        sqlx::query(
            "UPDATE mobile_pair_requests SET status = 'expired'
             WHERE status = 'pending'
               AND expires_at <= strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .execute(&self.db)
        .await
        .map_err(|error| error.to_string())?;
        let rows = sqlx::query(
            "SELECT id, device_name, platform, created_at, expires_at
             FROM mobile_pair_requests WHERE status = 'pending'
             ORDER BY created_at ASC",
        )
        .fetch_all(&self.db)
        .await
        .map_err(|error| error.to_string())?;
        Ok(rows
            .into_iter()
            .map(|row| PairingRequestInfo {
                id: sqlx::Row::get(&row, "id"),
                device_name: sqlx::Row::get(&row, "device_name"),
                platform: sqlx::Row::get(&row, "platform"),
                created_at: sqlx::Row::get(&row, "created_at"),
                expires_at: sqlx::Row::get(&row, "expires_at"),
            })
            .collect())
    }

    pub async fn devices(&self) -> Result<Vec<MobileDeviceInfo>, String> {
        let rows = sqlx::query(
            "SELECT id, name, platform, paired_at, last_sync_at
             FROM mobile_devices WHERE revoked_at IS NULL
             ORDER BY paired_at DESC",
        )
        .fetch_all(&self.db)
        .await
        .map_err(|error| error.to_string())?;
        Ok(rows
            .into_iter()
            .map(|row| MobileDeviceInfo {
                id: sqlx::Row::get(&row, "id"),
                name: sqlx::Row::get(&row, "name"),
                platform: sqlx::Row::get(&row, "platform"),
                paired_at: sqlx::Row::get(&row, "paired_at"),
                last_sync_at: sqlx::Row::get(&row, "last_sync_at"),
            })
            .collect())
    }

    pub async fn revoke_device(&self, device_id: &str) -> Result<String, String> {
        let result = sqlx::query(
            "UPDATE mobile_devices SET revoked_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ? AND revoked_at IS NULL",
        )
        .bind(device_id)
        .execute(&self.db)
        .await
        .map_err(|error| error.to_string())?;
        if result.rows_affected() == 0 {
            return Err("设备不存在或已撤销".into());
        }
        Ok("设备已撤销".into())
    }

    pub async fn authenticate(&self, token: &str) -> Option<DeviceRecord> {
        if token.is_empty() {
            return None;
        }
        let hash = sha256_hex(token);
        let row: Option<String> = sqlx::query_scalar(
            "SELECT id FROM mobile_devices
             WHERE credential_hash = ? AND revoked_at IS NULL",
        )
        .bind(hash)
        .fetch_optional(&self.db)
        .await
        .ok()?;
        row.map(|id| DeviceRecord { id })
    }

    pub async fn touch_device(&self, device_id: &str) {
        let _ = sqlx::query(
            "UPDATE mobile_devices
             SET last_sync_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?",
        )
        .bind(device_id)
        .execute(&self.db)
        .await;
    }
}

pub(crate) fn sha256_hex(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn random_hex(bytes: usize) -> String {
    let mut buffer = vec![0u8; bytes];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut buffer);
    let mut output = String::with_capacity(bytes * 2);
    for byte in buffer {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

pub(crate) fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn is_benchmark_address(address: Ipv4Addr) -> bool {
    let octets = address.octets();
    octets[0] == 198 && (18..=19).contains(&octets[1])
}

fn address_rank(address: Ipv4Addr) -> u8 {
    let octets = address.octets();
    if octets[0] == 192 && octets[1] == 168 {
        0
    } else if octets[0] == 10 {
        1
    } else {
        2
    }
}

fn interface_rank(name: &str) -> Option<u8> {
    let name = name.to_lowercase();
    let virtual_like = [
        "virtualbox",
        "vmware",
        "hyper-v",
        "vethernet",
        "loopback",
        "tap",
        "tun",
        "clash",
        "vpn",
        "zerotier",
        "tailscale",
    ]
    .iter()
    .any(|keyword| name.contains(keyword));
    if virtual_like {
        return None;
    }
    if name.contains("wlan")
        || name.contains("wi-fi")
        || name.contains("wifi")
        || name.contains("wireless")
        || name.contains("无线")
    {
        Some(0)
    } else if name.contains("ethernet") || name.contains("以太网") {
        Some(1)
    } else {
        Some(2)
    }
}

fn detect_lan_ip() -> Option<IpAddr> {
    if let Ok(interfaces) = if_addrs::get_if_addrs() {
        let mut candidates = interfaces
            .into_iter()
            .filter(|interface| !interface.is_loopback())
            .filter_map(|interface| {
                let rank = interface_rank(&interface.name)?;
                match interface.ip() {
                    IpAddr::V4(address)
                        if address.is_private()
                            && !address.is_link_local()
                            && !is_benchmark_address(address) =>
                    {
                        Some((rank, address_rank(address), address))
                    }
                    _ => None,
                }
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|(interface, address, _)| (*interface, *address));
        if let Some((_, _, address)) = candidates.first() {
            return Some(IpAddr::V4(*address));
        }
    }

    // Fallback for systems where interface enumeration is unavailable.
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(address)
            if !address.is_loopback()
                && !address.is_unspecified()
                && !is_benchmark_address(address) =>
        {
            Some(IpAddr::V4(address))
        }
        _ => None,
    }
}

#[tauri::command]
pub async fn get_mobile_sync_status(
    manager: tauri::State<'_, Arc<SyncManager>>,
) -> Result<MobileSyncStatus, String> {
    manager.status().await
}

#[tauri::command]
pub async fn start_mobile_sync(
    manager: tauri::State<'_, Arc<SyncManager>>,
) -> Result<MobileSyncStatus, String> {
    manager.inner().clone().start().await
}

#[tauri::command]
pub async fn stop_mobile_sync(
    manager: tauri::State<'_, Arc<SyncManager>>,
) -> Result<MobileSyncStatus, String> {
    manager.stop().await
}

#[tauri::command]
pub async fn create_mobile_pairing(
    manager: tauri::State<'_, Arc<SyncManager>>,
) -> Result<PairingSessionResponse, String> {
    manager.inner().clone().create_pairing_session().await
}

#[tauri::command]
pub async fn get_mobile_pairing_requests(
    manager: tauri::State<'_, Arc<SyncManager>>,
) -> Result<Vec<PairingRequestInfo>, String> {
    manager.pairing_requests().await
}

#[tauri::command]
pub async fn approve_mobile_pairing(
    request_id: String,
    manager: tauri::State<'_, Arc<SyncManager>>,
) -> Result<String, String> {
    manager.approve_pairing(&request_id).await
}

#[tauri::command]
pub async fn deny_mobile_pairing(
    request_id: String,
    manager: tauri::State<'_, Arc<SyncManager>>,
) -> Result<String, String> {
    manager.deny_pairing(&request_id).await
}

#[tauri::command]
pub async fn get_mobile_devices(
    manager: tauri::State<'_, Arc<SyncManager>>,
) -> Result<Vec<MobileDeviceInfo>, String> {
    manager.devices().await
}

#[tauri::command]
pub async fn revoke_mobile_device(
    device_id: String,
    manager: tauri::State<'_, Arc<SyncManager>>,
) -> Result<String, String> {
    manager.revoke_device(&device_id).await
}
