use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraResponse {
    pub id: i64,
    pub brand: String,
    pub model: String,
    pub status: String,
    pub format: Option<String>,
    pub purchase_date: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilmResponse {
    pub id: i64,
    pub brand: String,
    pub name: String,
    pub iso: i64,
    #[serde(rename = "type")]
    pub film_type: String,
    pub target_status: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollSummaryResponse {
    pub id: i64,
    pub camera_id: i64,
    pub film_id: i64,
    pub index: i32,
    pub shot_month: Option<String>,
    pub city: Option<String>,
    pub note: Option<String>,
    pub camera_brand: String,
    pub camera_model: String,
    pub film_brand: String,
    pub film_name: String,
    pub camera_info: String,
    pub film_info: String,
    pub cover_path: Option<String>,
    pub photo_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhotoResponse {
    pub id: i64,
    pub frame_number: Option<i32>,
    pub lab_scan_path: Option<String>,
    pub edit_scan_path: Option<String>,
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDraft {
    pub source_path: String,
    pub frame_number: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhotoImportEntry {
    pub source_path: String,
    pub frame_number: i32,
    pub conflict_action: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAnalysisItemResponse {
    pub source_path: String,
    pub file_name: String,
    pub frame_number: Option<i32>,
    pub existing_version: bool,
    pub paired_version: bool,
    pub issue: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResultResponse {
    pub imported_count: usize,
    pub updated_count: usize,
    pub skipped_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LabPreviewResponse {
    pub photo_id: i64,
    pub preview_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LabOriginalResponse {
    pub photo_id: i64,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollDetailResponse {
    #[serde(flatten)]
    pub summary: RollSummaryResponse,
    pub photos: Vec<PhotoResponse>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraRollResponse {
    pub id: i64,
    pub roll_index: i32,
    pub shot_month: Option<String>,
    pub city: Option<String>,
    pub film_brand: String,
    pub film_name: String,
    pub film_iso: i64,
    pub film_info: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraDetailResponse {
    pub camera: CameraResponse,
    pub rolls: Vec<CameraRollResponse>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStatsResponse {
    pub cameras: Vec<CameraResponse>,
    pub camera_count: i64,
    pub film_count: i64,
    pub shot_film_count: i64,
    pub roll_count: i64,
    pub photo_count: i64,
    pub favorite_photo_count: i64,
}
