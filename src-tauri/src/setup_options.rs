//! Import choices are validated once and shared by preview and every writer.
use crate::models::{Client, ValidatedInput};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactMode {
    #[default]
    Preserve,
    Disabled,
    Enabled,
}

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WebSearchMode {
    #[default]
    Preserve,
    Disabled,
    Cached,
    Live,
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupOptions {
    #[serde(default)]
    pub claude_models: BTreeMap<String, String>,
    #[serde(default)]
    pub artifact_mode: ArtifactMode,
    #[serde(default)]
    pub web_search: WebSearchMode,
}

impl SetupOptions {
    pub fn web_search_value(&self, doc: &toml_edit::DocumentMut) -> Result<&'static str, String> {
        Ok(match self.web_search {
            WebSearchMode::Disabled => "disabled",
            WebSearchMode::Cached => "cached",
            WebSearchMode::Live => "live",
            WebSearchMode::Preserve => {
                match doc.get("web_search").and_then(toml_edit::Item::as_str) {
                    None if !doc.contains_key("web_search") => "disabled",
                    Some("disabled") => "disabled",
                    Some("cached") => "cached",
                    Some("live") => "live",
                    _ => {
                        return Err(
                            "原 web_search 配置无效，请选择关闭、缓存或实时搜索后重新导入。".into(),
                        )
                    }
                }
            }
        })
    }
    pub fn validate(&self, input: &ValidatedInput) -> Result<(), String> {
        for (role, model) in &self.claude_models {
            if !["haiku", "sonnet", "opus", "fable", "subagent"].contains(&role.as_str())
                || !input.selected_models.contains(model)
            {
                return Err("用途模型必须来自本次已勾选的模型，请重新选择。".into());
            }
        }
        if input.client.is_codex() {
            if !self.claude_models.is_empty() || self.artifact_mode != ArtifactMode::Preserve {
                return Err("Codex 不接受 Claude 专用配置。".into());
            }
        } else if self.web_search != WebSearchMode::Preserve {
            return Err("此联网搜索设置仅适用于 Codex。".into());
        }
        if input.client == Client::ClaudeDesktop
            && (!self.claude_models.is_empty() || self.artifact_mode != ArtifactMode::Preserve)
        {
            return Err("Claude 桌面版不读取 Claude Code 的用途模型与 Artifact 开关。".into());
        }
        Ok(())
    }
}
