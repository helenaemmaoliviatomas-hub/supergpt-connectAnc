//! One connection environment for Claude Code CLI and its VS Code extension.
use crate::{models::ValidatedInput, provider_fields, setup_options::ArtifactMode};
use std::collections::BTreeMap;

pub fn role_models(input: &ValidatedInput) -> BTreeMap<String, String> {
    let mut roles = BTreeMap::new();
    for role in ["haiku", "sonnet", "opus", "fable"] {
        let model = input
            .options
            .claude_models
            .get(role)
            .cloned()
            .unwrap_or_else(|| {
                let family = format!("claude-{role}-");
                let matches: Vec<_> = input
                    .selected_models
                    .iter()
                    .filter(|id| {
                        id.strip_prefix("anthropic/")
                            .unwrap_or(id)
                            .starts_with(&family)
                    })
                    .collect();
                // Never guess a price or choose arbitrarily between versions/aliases.
                if matches.iter().any(|id| **id == input.model) {
                    input.model.clone()
                } else if matches.len() == 1 {
                    matches[0].clone()
                } else {
                    input.model.clone()
                }
            });
        roles.insert(role.into(), model);
    }
    roles.insert(
        "subagent".into(),
        input
            .options
            .claude_models
            .get("subagent")
            .cloned()
            .unwrap_or_default(),
    );
    roles
}

pub fn environment(input: &ValidatedInput, tool_search: bool) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    // Removing a JSON field does not unset an inherited shell/OS variable.
    // Explicit empty values neutralize old credentials, model pins and cloud routing.
    for key in provider_fields::CLAUDE_PROTOCOL_SELECTORS
        .iter()
        .copied()
        .chain([
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_MODEL",
            "ANTHROPIC_CUSTOM_HEADERS",
            "ANTHROPIC_SMALL_FAST_MODEL",
            "CLAUDE_CODE_OAUTH_TOKEN",
            "CLAUDE_CODE_OAUTH_REFRESH_TOKEN",
            "CLAUDE_CODE_OAUTH_SCOPES",
            "CLAUDE_CODE_SUBAGENT_MODEL_FORCE",
        ])
    {
        env.insert(key.into(), String::new());
    }
    for (key, val) in [
        ("ANTHROPIC_BASE_URL", input.root.as_str()),
        ("ANTHROPIC_AUTH_TOKEN", input.api_key.as_str()),
        ("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1"),
        ("CLAUDE_CODE_AUTO_MODE_SERVER", "0"),
        (
            "ENABLE_TOOL_SEARCH",
            if tool_search { "true" } else { "false" },
        ),
    ] {
        env.insert(key.into(), val.into());
    }
    for (role, model) in role_models(input) {
        let key = if role == "subagent" {
            "CLAUDE_CODE_SUBAGENT_MODEL".into()
        } else {
            format!("ANTHROPIC_DEFAULT_{}_MODEL", role.to_uppercase())
        };
        env.insert(key, model);
    }
    match input.options.artifact_mode {
        ArtifactMode::Preserve => {}
        ArtifactMode::Disabled => {
            env.insert("CLAUDE_CODE_DISABLE_ARTIFACT".into(), "1".into());
        }
        ArtifactMode::Enabled => {
            env.insert("CLAUDE_CODE_DISABLE_ARTIFACT".into(), "0".into());
        }
    }
    env
}
