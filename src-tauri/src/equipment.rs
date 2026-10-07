use crate::models::EquipmentItemResponse;
use crate::validation::{
    clean_optional, clean_required, database_error, ensure_changed, ensure_positive_id,
};
use crate::{validate_purchase_date, AppState};

type EquipmentRow = (
    i64,
    String,
    Option<String>,
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
);

fn response(row: EquipmentRow) -> EquipmentItemResponse {
    let (id, category, subtype, brand, model, mount, status, purchase_date, note) = row;
    EquipmentItemResponse {
        id,
        category,
        subtype,
        brand,
        model,
        mount,
        status,
        purchase_date,
        note,
    }
}

fn validate_kind_fields(
    category: String,
    subtype: Option<String>,
    mount: Option<String>,
) -> Result<(String, Option<String>, Option<String>), String> {
    let category = clean_required(category, "器材分类", 16)?;
    if !matches!(category.as_str(), "lens" | "other") {
        return Err("器材分类只支持镜头或其他器材".into());
    }
    let subtype = clean_optional(subtype, "具体类型", 80)?;
    let mount = clean_optional(mount, "卡口", 80)?;
    if category == "lens" && mount.is_none() {
        return Err("镜头卡口不能为空".into());
    }
    if category == "other" && subtype.is_none() {
        return Err("其他器材的具体类型不能为空".into());
    }
    Ok((category, subtype, mount))
}

fn validate_status(status: String) -> Result<String, String> {
    let status = clean_required(status, "状态", 16)?;
    if matches!(status.as_str(), "active" | "inactive") {
        Ok(status)
    } else {
        Err("状态只支持在用或闲置".into())
    }
}

#[tauri::command]
pub(crate) async fn get_equipment_items(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<EquipmentItemResponse>, String> {
    let rows: Vec<EquipmentRow> = sqlx::query_as(
        "SELECT id, category, subtype, brand, model, mount, status, purchase_date, note FROM equipment_items ORDER BY created_at ASC, id ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|error| database_error("读取器材", error))?;
    Ok(rows.into_iter().map(response).collect())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn add_equipment_item(
    category: String,
    subtype: Option<String>,
    brand: String,
    model: String,
    mount: Option<String>,
    status: String,
    purchase_date: Option<String>,
    note: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<i64, String> {
    let (category, subtype, mount) = validate_kind_fields(category, subtype, mount)?;
    let brand = clean_required(brand, "品牌", 120)?;
    let model = clean_required(model, "型号", 160)?;
    let status = validate_status(status)?;
    let purchase_date = validate_purchase_date(&state.db, purchase_date).await?;
    let note = clean_optional(note, "备注", 2000)?;
    let result = sqlx::query(
        "INSERT INTO equipment_items (category, subtype, brand, model, mount, status, purchase_date, note) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(category)
    .bind(subtype)
    .bind(brand)
    .bind(model)
    .bind(mount)
    .bind(status)
    .bind(purchase_date)
    .bind(note)
    .execute(&state.db)
    .await
    .map_err(|error| database_error("新增器材", error))?;
    Ok(result.last_insert_rowid())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn update_equipment_item(
    id: i64,
    category: String,
    subtype: Option<String>,
    brand: String,
    model: String,
    mount: Option<String>,
    status: String,
    purchase_date: Option<String>,
    note: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    ensure_positive_id(id, "器材 ID")?;
    let (category, subtype, mount) = validate_kind_fields(category, subtype, mount)?;
    let result = sqlx::query(
        "UPDATE equipment_items SET category = ?, subtype = ?, brand = ?, model = ?, mount = ?, status = ?, purchase_date = ?, note = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(category)
    .bind(subtype)
    .bind(clean_required(brand, "品牌", 120)?)
    .bind(clean_required(model, "型号", 160)?)
    .bind(mount)
    .bind(validate_status(status)?)
    .bind(validate_purchase_date(&state.db, purchase_date).await?)
    .bind(clean_optional(note, "备注", 2000)?)
    .bind(id)
    .execute(&state.db)
    .await
    .map_err(|error| database_error("更新器材", error))?;
    ensure_changed(result.rows_affected(), "器材")?;
    Ok("器材已更新".into())
}

#[tauri::command]
pub(crate) async fn delete_equipment_item(
    id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    ensure_positive_id(id, "器材 ID")?;
    let result = sqlx::query("DELETE FROM equipment_items WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|error| database_error("删除器材", error))?;
    ensure_changed(result.rows_affected(), "器材")?;
    Ok("器材档案已删除".into())
}
