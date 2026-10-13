//! Context limits are capabilities, not a guessed universal default.
use serde_json::{json, Value};

pub(crate) const FIELDS: [&str; 4] = [
    "context_window",
    "max_context_window",
    "auto_compact_token_limit",
    "effective_context_window_percent",
];

pub(crate) fn limits(row: &Value) -> Value {
    let mut result = json!({});
    for key in FIELDS {
        result[key] = row.get(key).cloned().unwrap_or_else(|| {
            if key == "effective_context_window_percent" {
                json!(95)
            } else {
                Value::Null
            }
        });
    }
    result
}

pub(crate) fn validate(row: &Value, required: bool) -> Result<(), String> {
    for key in FIELDS {
        let v = &row[key];
        if v.is_null() && key != "effective_context_window_percent" {
            continue;
        }
        // Bound multiplication used by Codex for percentages, as well as signed parsing.
        let upper = if key == "effective_context_window_percent" {
            100
        } else {
            i64::MAX / 100
        };
        if !v.as_i64().is_some_and(|n| n > 0 && n <= upper) {
            return Err("模型的上下文容量或压缩参数格式不正确，配置尚未写入。".into());
        }
    }
    if let (Some(window), Some(max)) = (
        row["context_window"].as_i64(),
        row["max_context_window"].as_i64(),
    ) {
        if window > max {
            return Err("模型的上下文容量超过最大容量，配置尚未写入。".into());
        }
    }
    if required
        && row["context_window"]
            .as_i64()
            .or(row["max_context_window"].as_i64())
            .is_none()
    {
        return Err(format!(
            "模型 {} 未提供上下文容量。请让中转站在模型目录中提供 context_window，或取消勾选该模型后重试；尚未修改客户端配置。",
            row["slug"].as_str().unwrap_or("未知模型")
        ));
    }
    Ok(())
}

fn find_model<'a>(catalog: &'a Value, id: &str) -> Option<&'a Value> {
    catalog
        .get("models")?
        .as_array()?
        .iter()
        .find(|row| row["slug"] == id)
}

fn legacy_default(row: &Value) -> bool {
    // Exact 0.1.0/0.1.1 generated defaults; never preserve this known bad fallback.
    row["context_window"] == 32768
        && row["max_context_window"] == 32768
        && row["auto_compact_token_limit"].is_null()
        && row["effective_context_window_percent"] == 95
}

/// Only call with a previous catalog belonging to this active provider and endpoint.
/// Gateway limits win; otherwise preserve explicit local repairs across reimports.
pub(crate) fn prepare_import(
    generated: &str,
    previous: Option<&str>,
    cached: Option<&str>,
) -> Result<String, String> {
    let mut catalog: Value =
        serde_json::from_str(generated).map_err(|_| "模型目录不是有效 JSON，配置尚未写入。")?;
    let previous: Option<Value> = previous
        .map(serde_json::from_str)
        .transpose()
        .map_err(|_| "现有模型目录不是有效 JSON，配置尚未写入。")?;
    // A cache is an optional hint; a missing or malformed cache must not break known models.
    let cached: Option<Value> = cached.and_then(|s| serde_json::from_str(s).ok());
    let sources = catalog["_supergpt_connect"]["context_sources"].clone();
    let gateway_fields = catalog["_supergpt_connect"]["gateway_context_fields"].clone();
    let mut defaults = json!({});
    let rows = catalog["models"]
        .as_array_mut()
        .ok_or("模型目录缺少 models，配置尚未写入。")?;
    for row in rows {
        let id = row["slug"]
            .as_str()
            .ok_or("模型目录缺少模型名。")?
            .to_string();
        let gateway_owns = |key: &str| {
            gateway_fields[&id]
                .as_array()
                .map_or(sources[&id] == "gateway", |fields| {
                    fields.iter().any(|field| field == key)
                })
        };
        if row["context_window"].is_null()
            && row["max_context_window"].is_null()
            && !gateway_owns("context_window")
            && !gateway_owns("max_context_window")
        {
            if let Some(cached_row) = cached.as_ref().and_then(|c| find_model(c, &id)) {
                let context = limits(cached_row);
                if validate(&context, true).is_ok() {
                    for key in FIELDS {
                        if !gateway_owns(key) {
                            row[key] = context[key].clone();
                        }
                    }
                }
            }
        }
        defaults[&id] = limits(row);
        if let Some(old) = previous.as_ref().and_then(|c| find_model(c, &id)) {
            let old_context = limits(old);
            let recorded =
                &previous.as_ref().unwrap()["_supergpt_connect"]["generated_context"][&id];
            for key in FIELDS {
                let manually_changed = if recorded.is_object() {
                    old_context[key] != recorded[key]
                } else {
                    !legacy_default(&old_context)
                };
                if manually_changed && !gateway_owns(key) {
                    row[key] = old_context[key].clone();
                }
            }
        }
        validate(row, true)?;
    }
    // Store only nonsecret generation metadata inside the already-backed-up catalog.
    // Comparing against it on the next import distinguishes user edits from our defaults.
    catalog["_supergpt_connect"]["schema_version"] = json!(2);
    catalog["_supergpt_connect"]["generated_context"] = defaults;
    serde_json::to_string_pretty(&catalog).map_err(|_| "无法生成模型目录。".into())
}
