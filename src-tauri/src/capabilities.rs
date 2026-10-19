//! Route capabilities and local constraints share one policy for preview and import.
use crate::{
    models::{Client, ValidatedInput},
    storage::{self, FileKind, Paths},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Supported,
    Unsupported,
    #[default]
    Unknown,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Capability {
    #[serde(default)]
    pub status: State,
    #[serde(default)]
    pub reason: String,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ModelCapabilities {
    #[serde(default)]
    pub messages: Capability,
    #[serde(default)]
    pub claude_tool_search: Capability,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RouteCapabilities {
    #[serde(default)]
    pub models: BTreeMap<String, ModelCapabilities>,
    #[serde(skip)]
    pub warning: Option<String>,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ToolSearchMode {
    #[default]
    Auto,
    Disabled,
    Enabled,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupPreview {
    pub tool_search_enabled: bool,
    pub tool_search_state: State,
    pub summary: String,
    pub warnings: Vec<String>,
    pub blocked: Option<String>,
}

pub fn parse(payload: &Value) -> Option<RouteCapabilities> {
    if payload.get("object")?.as_str()? != "supergpt.connect_capabilities"
        || payload.get("schema_version")?.as_u64()? != 1
    {
        return None;
    }
    serde_json::from_value(payload.clone()).ok()
}

fn on(value: &Value) -> bool {
    value.as_bool() == Some(true)
        || value.as_i64() == Some(1)
        || value
            .as_str()
            .is_some_and(|s| matches!(s.to_ascii_lowercase().as_str(), "true" | "1" | "yes" | "on"))
}
fn selected_state(input: &ValidatedInput) -> State {
    let mut state = State::Supported;
    for id in &input.selected_models {
        match input
            .capabilities
            .models
            .get(id)
            .map(|m| m.claude_tool_search.status)
            .unwrap_or_default()
        {
            State::Unsupported => return State::Unsupported,
            State::Unknown => state = State::Unknown,
            State::Supported => {}
        }
    }
    state
}

pub fn preview(paths: &Paths, input: &ValidatedInput) -> Result<SetupPreview, String> {
    let compatibility = crate::compatibility::inspect(paths, input.client);
    let mut result = SetupPreview {
        tool_search_enabled: false,
        tool_search_state: State::Unknown,
        summary: String::new(),
        warnings: compatibility.warnings,
        blocked: compatibility.blocked,
    };
    if let Some(warning) = &input.capabilities.warning {
        result.warnings.push(warning.clone());
    }
    if input.client.is_codex() {
        result.summary = "保留图片、上下文和思考能力；联网搜索沿用当前连接器的兼容设置。".into();
        return Ok(result);
    }
    if input.client == Client::Claude || input.client == Client::ClaudeVscode {
        result.warnings.push("项目设置、组织管理策略和启动命令仍可能覆盖本机设置；重新启动客户端后可在 /context 检查实际 MCP 占用。设置了 alwaysLoad 的工具仍会提前加载。".into());
        let other = if input.client == Client::Claude {
            Client::ClaudeVscode
        } else {
            Client::Claude
        };
        if paths.has_backup(other) {
            result.warnings.push("终端与 VS Code 共用 Claude Code 设置，另一入口也已有恢复点；恢复时请检查共享文件，避免覆盖后来导入的线路。".into());
        }
    }
    result.tool_search_state = selected_state(input);
    if input
        .selected_models
        .iter()
        .any(|id| id.contains("claude-3") || id.starts_with("claude-haiku-4-0"))
    {
        result.tool_search_state = State::Unsupported;
    }
    let unsupported_messages: Vec<_> = input
        .selected_models
        .iter()
        .filter(|id| {
            input
                .capabilities
                .models
                .get(*id)
                .is_some_and(|m| m.messages.status == State::Unsupported)
        })
        .cloned()
        .collect();
    if !unsupported_messages.is_empty() {
        result.blocked = Some(format!(
            "这些模型尚不支持当前客户端的对话协议：{}。请选择其他模型或客户端。",
            unsupported_messages.join("、")
        ));
    }
    result.tool_search_enabled = match input.tool_search {
        ToolSearchMode::Auto => result.tool_search_state == State::Supported,
        ToolSearchMode::Disabled => false,
        ToolSearchMode::Enabled if result.tool_search_state == State::Unsupported => {
            result.blocked =
                Some("所选模型包含已确认不支持工具按需加载的线路，请使用自动模式或关闭。".into());
            false
        }
        ToolSearchMode::Enabled => true,
    };
    if result.tool_search_state == State::Unknown {
        result.warnings.push("所选模型尚未全部完成工具按需加载验证。自动模式会完整加载 MCP 定义，可能增加起始上下文；连接器不会偷偷发送收费测试请求。".into());
    }
    if result.tool_search_state == State::Unsupported {
        result.warnings.push("所选模型包含不支持工具按需加载的线路，将完整加载 MCP 工具定义；工具较多时起始上下文会增大。".into());
    }
    if input.tool_search == ToolSearchMode::Enabled && result.tool_search_state == State::Unknown {
        result.warnings.push(
            "你已手动开启未经验证的工具按需加载；若对话或工具调用失败，请切回自动模式重新导入。"
                .into(),
        );
    }
    if input.client == Client::ClaudeDesktop {
        if compatibility
            .version
            .as_deref()
            .is_some_and(|v| version_before(v, (1, 40609, 0)))
        {
            result.tool_search_enabled = false;
            result.warnings.push("请升级 Claude 桌面版至 1.40609.0 或更新版本，再启用工具按需加载；旧版可能额外启用其他实验协议。".into());
        }
    } else {
        let text = storage::read_optional(&paths.target(FileKind::ClaudeSettings))?;
        let doc: Value = serde_json::from_str(
            text.as_deref()
                .unwrap_or("{}")
                .trim_start_matches('\u{feff}'),
        )
        .map_err(|_| "Claude Code 设置不是有效 JSON，配置尚未写入。")?;
        let mut beta_disabled = on(&doc["env"]["CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS"]);
        if input.client == Client::ClaudeVscode {
            if let Some(text) = storage::read_optional(&paths.target(FileKind::ClaudeVscode))? {
                let vscode: Value = jsonc_parser::parse_to_serde_value(&text, &Default::default())
                    .map_err(|_| "VS Code 用户设置不是有效 JSONC，配置尚未写入。")?;
                if let Some(vars) = vscode["claudeCode.environmentVariables"].as_array() {
                    for var in vars {
                        if var["name"] == "CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS" {
                            beta_disabled |= on(&var["value"]);
                        }
                    }
                }
            }
        }
        if !paths.sandbox {
            if let Ok(value) = std::env::var("CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS") {
                beta_disabled |= on(&Value::String(value));
            }
        }
        let denied = doc["permissions"]["deny"].as_array().is_some_and(|rules| {
            rules.iter().any(|rule| {
                rule.as_str()
                    .is_some_and(|s| s == "ToolSearch" || s.starts_with("ToolSearch("))
            })
        });
        if beta_disabled || denied {
            result.tool_search_enabled = false;
            result.warnings.push(if denied { "本机权限禁止了 ToolSearch，已保留该权限规则。工具将完整加载；如需按需加载，请先调整权限再重新导入。" } else { "本机禁用了实验性功能，工具按需加载无法生效。已保留你的设置；如需启用，请调整 CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS 后重新导入。" }.into());
        }
        if compatibility
            .version
            .as_deref()
            .is_some_and(|v| version_before(v, (2, 0, 0)))
        {
            result.tool_search_enabled = false;
            result.warnings.push(
                "此 Claude Code 版本不在原生工具搜索开关支持范围内，请升级到最新版本后重新导入。"
                    .into(),
            );
        }
    }
    result.summary = if result.tool_search_enabled {
        "工具按需加载已开启：先搜索，再加载需要的 MCP 工具。"
    } else {
        "工具按需加载已关闭：MCP 工具定义会完整进入上下文。"
    }
    .into();
    Ok(result)
}

pub fn version_before(text: &str, minimum: (u32, u32, u32)) -> bool {
    let parts: Vec<_> = text
        .split('.')
        .take(3)
        .map(|p| p.split('-').next().unwrap_or("").parse::<u32>())
        .collect();
    matches!(parts.as_slice(), [Ok(a),Ok(b),Ok(c)] if (*a,*b,*c) < minimum)
}

/// Read-only, bounded capability discovery on the same authenticated origin.
/// Missing/old/nonconforming endpoints never imply support and never cause paid probes.
pub async fn discover(client: &reqwest::Client, root: &str, key: &str) -> RouteCapabilities {
    async fn fetch(client: &reqwest::Client, root: &str, key: &str) -> Option<RouteCapabilities> {
        let mut response = client
            .get(format!("{root}/v1/sub2api/connect"))
            .bearer_auth(key)
            .header("Accept", "application/json")
            .send()
            .await
            .ok()?;
        if !response.status().is_success() {
            return None;
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.ok()? {
            if bytes.len() + chunk.len() > 2 * 1024 * 1024 {
                return None;
            }
            bytes.extend_from_slice(&chunk);
        }
        parse(&serde_json::from_slice::<Value>(&bytes).ok()?)
    }
    fetch(client, root, key)
        .await
        .unwrap_or_else(|| RouteCapabilities {
            warning: Some("中转站未返回有效的线路能力验证信息，按未验证处理。".into()),
            ..Default::default()
        })
}
