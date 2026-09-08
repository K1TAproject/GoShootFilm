pub(crate) fn clean_required(value: String, field: &str, max_len: usize) -> Result<String, String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(format!("{field}不能为空"));
    }
    if value.chars().count() > max_len {
        return Err(format!("{field}不能超过{max_len}个字符"));
    }
    Ok(value)
}

pub(crate) fn clean_optional(
    value: Option<String>,
    field: &str,
    max_len: usize,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().to_string();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max_len {
        return Err(format!("{field}不能超过{max_len}个字符"));
    }
    Ok(Some(value))
}

pub(crate) fn validate_shot_month(value: Option<String>) -> Result<Option<String>, String> {
    let value = clean_optional(value, "拍摄月份", 7)?;
    let Some(value) = value else {
        return Ok(None);
    };
    let bytes = value.as_bytes();
    let valid_shape = bytes.len() == 7
        && bytes[4] == b'-'
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..].iter().all(u8::is_ascii_digit);
    let valid_month = value[5..]
        .parse::<u8>()
        .is_ok_and(|month| (1..=12).contains(&month));
    if !valid_shape || !valid_month {
        return Err("拍摄月份必须采用 YYYY-MM 格式".into());
    }
    Ok(Some(value))
}

pub(crate) fn validate_date(value: Option<String>, field: &str) -> Result<Option<String>, String> {
    let value = clean_optional(value, field, 10)?;
    let Some(value) = value else {
        return Ok(None);
    };
    let parts: Vec<_> = value.split('-').collect();
    let valid_shape = parts.len() == 3
        && parts[0].len() == 4
        && parts[1].len() == 2
        && parts[2].len() == 2
        && parts
            .iter()
            .all(|part| part.chars().all(|ch| ch.is_ascii_digit()));
    let valid_date = if valid_shape {
        let year = parts[0].parse::<u32>().unwrap_or_default();
        let month = parts[1].parse::<u8>().unwrap_or_default();
        let day = parts[2].parse::<u8>().unwrap_or_default();
        let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let max_day = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap_year => 29,
            2 => 28,
            _ => 0,
        };
        year > 0 && day > 0 && day <= max_day
    } else {
        false
    };
    if !valid_date {
        return Err(format!("{field}必须采用 YYYY-MM-DD 格式"));
    }
    Ok(Some(value))
}

pub(crate) fn database_error(action: &str, error: sqlx::Error) -> String {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.is_unique_violation() {
            return format!("{action}失败：相同记录已存在");
        }
        if database_error.is_foreign_key_violation() {
            return format!("{action}失败：关联的记录不存在或仍被使用");
        }
    }
    format!("{action}失败: {error}")
}

pub(crate) fn ensure_positive_id(id: i64, field: &str) -> Result<(), String> {
    if id <= 0 {
        return Err(format!("{field}无效"));
    }
    Ok(())
}

pub(crate) fn ensure_changed(rows_affected: u64, entity: &str) -> Result<(), String> {
    if rows_affected == 0 {
        return Err(format!("未找到要操作的{entity}"));
    }
    Ok(())
}

pub(crate) fn validate_film_target_status(target_status: Option<String>) -> Result<String, String> {
    let target_status = target_status.unwrap_or_else(|| "unshot".to_string());
    if matches!(target_status.as_str(), "unshot" | "shot") {
        Ok(target_status)
    } else {
        Err("胶片状态无效，仅支持未拍摄或已拍摄".into())
    }
}
