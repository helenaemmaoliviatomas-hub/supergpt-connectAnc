use crate::{
    models::ValidatedInput,
    provider_fields,
    storage::{self, Change, FileKind, Paths},
};
use jsonc_parser::{
    cst::{CstInputValue, CstRootNode},
    ParseOptions,
};
use serde_json::{json, Value};

fn cst(value: Value) -> CstInputValue {
    match value {
        Value::Null => CstInputValue::Null,
        Value::Bool(v) => CstInputValue::Bool(v),
        Value::Number(v) => CstInputValue::Number(v.to_string()),
        Value::String(v) => CstInputValue::String(v),
        Value::Array(v) => CstInputValue::Array(v.into_iter().map(cst).collect()),
        Value::Object(v) => {
            CstInputValue::Object(v.into_iter().map(|(k, v)| (k, cst(v))).collect())
        }
    }
}

pub fn read(paths: &Paths) -> Result<Value, String> {
    let text = storage::read_optional(&paths.target(FileKind::ClaudeVscode))?;
    let text = text
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("{}");
    let doc: Value =
        jsonc_parser::parse_to_serde_value(text.trim_start_matches('\u{feff}'), &options())
            .map_err(|_| "VS Code 用户设置不是有效 JSONC，未修改配置。")?;
    if !doc.is_object() {
        return Err("VS Code 用户设置必须是对象，未修改配置。".into());
    }
    Ok(doc)
}

fn options() -> ParseOptions {
    ParseOptions {
        allow_comments: true,
        allow_trailing_commas: true,
        allow_loose_object_property_names: false,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: false,
        allow_unary_plus_numbers: false,
        allow_bare_decimal_point_numbers: false,
        allow_non_finite_numbers: false,
        allow_extended_string_escapes: false,
    }
}

pub fn change(
    paths: &Paths,
    input: &ValidatedInput,
    tool_search_enabled: bool,
) -> Result<Change, String> {
    let kind = FileKind::ClaudeVscode;
    let before = storage::read_optional(&paths.target(kind))?;
    let text = before
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("{}");
    // VS Code supports JSON + comments/trailing commas, not JSON5.
    let options = options();
    let doc: Value = jsonc_parser::parse_to_serde_value(text, &options)
        .map_err(|_| "VS Code 用户设置不是有效 JSONC，未修改配置。")?;
    if !doc.is_object() {
        return Err("VS Code 用户设置必须是对象，未修改配置。".into());
    }
    let key = "claudeCode.environmentVariables";
    let mut vars = match doc.get(key) {
        None => vec![],
        Some(Value::Array(values)) => values.clone(),
        _ => return Err("claudeCode.environmentVariables 必须是数组，未修改配置。".into()),
    };
    if vars.iter().any(|v| {
        v.get("name").and_then(Value::as_str).is_none()
            || v.get("value").and_then(Value::as_str).is_none()
    }) {
        return Err("Claude VS Code 环境变量格式无效，未修改配置。".into());
    }
    vars.retain(|v| {
        !provider_fields::claude_floor_env(v["name"].as_str().unwrap())
            && v["name"] != "ENABLE_TOOL_SEARCH"
    });
    for (name, value) in crate::claude_config::environment(input, tool_search_enabled) {
        vars.retain(|v| v["name"] != name);
        vars.push(json!({"name": name, "value": value}));
    }
    // CST replaces only the extension's property; all surrounding comments and
    // settings stay byte-for-byte intact, including trailing commas and URLs.
    let root = CstRootNode::parse(text, &options).map_err(|_| "无法解析 VS Code 设置。")?;
    let obj = root.object_value_or_set();
    for owned in [key, "claudeCode.disableLoginPrompt"] {
        if obj
            .properties()
            .iter()
            .filter(|prop| prop.decoded_name().as_deref() == Some(owned))
            .count()
            > 1
        {
            return Err(format!("VS Code 中有重复的 {owned} 设置，请合并后再导入。"));
        }
    }
    if let Some(property) = obj.get(key) {
        property.set_value(cst(json!(vars)));
    } else {
        obj.append(key, cst(json!(vars)));
    }
    if let Some(property) = obj.get("claudeCode.disableLoginPrompt") {
        property.set_value(CstInputValue::Bool(true));
    } else {
        obj.append("claudeCode.disableLoginPrompt", CstInputValue::Bool(true));
    }
    Ok(Change {
        kind,
        before,
        after: root.to_string(),
    })
}
