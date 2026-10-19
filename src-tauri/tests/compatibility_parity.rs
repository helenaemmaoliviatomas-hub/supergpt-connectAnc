use serde_json::{json, Value};
use std::fs;
use supergpt_connect_lib::{
    adapter, capabilities, claude_config,
    models::{Client, ValidatedInput},
    setup_options::{ArtifactMode, WebSearchMode},
    storage::{self, FileKind, Paths},
};

fn input(client: Client) -> ValidatedInput {
    ValidatedInput {
        client,
        api_key: "sk-new-route-fixture".into(),
        root: "https://gateway.example.test".into(),
        model: "claude-opus-5-5".into(),
        selected_models: vec![
            "claude-opus-5-5".into(),
            "claude-haiku-4-5".into(),
            "claude-sonnet-4-6".into(),
        ],
        catalog: None,
        capabilities: Default::default(),
        tool_search: Default::default(),
        options: Default::default(),
    }
}
fn write(paths: &Paths, kind: FileKind, text: &str) {
    storage::atomic_write(&paths.target(kind), text.as_bytes()).unwrap();
}
fn read(paths: &Paths, kind: FileKind) -> String {
    fs::read_to_string(paths.target(kind)).unwrap()
}

#[test]
fn lightweight_roles_use_only_selected_models_and_subagents_can_follow_the_session() {
    let mut request = input(Client::Claude);
    let roles = claude_config::role_models(&request);
    assert_eq!(roles["haiku"], "claude-haiku-4-5");
    assert_eq!(roles["sonnet"], "claude-sonnet-4-6");
    assert_eq!(roles["opus"], request.model);
    assert_eq!(roles["subagent"], "");
    request.selected_models.push("claude-haiku-other".into());
    assert_eq!(claude_config::role_models(&request)["haiku"], request.model);
    request
        .options
        .claude_models
        .insert("haiku".into(), "claude-haiku-4-5".into());
    assert_eq!(
        claude_config::role_models(&request)["haiku"],
        "claude-haiku-4-5"
    );
    request
        .options
        .claude_models
        .insert("subagent".into(), "unselected-expensive-model".into());
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    assert!(capabilities::preview(&paths, &request).is_err());
    assert!(adapter::apply(&paths, request).is_err());
    assert!(!paths.target(FileKind::ClaudeSettings).exists());
}

#[test]
fn cli_and_extension_share_compatibility_values_without_changing_permissions() {
    for client in [Client::Claude, Client::ClaudeVscode] {
        let home = tempfile::tempdir().unwrap();
        let paths = Paths::for_home(home.path());
        let original = "{\"permissions\":{\"defaultMode\":\"plan\",\"deny\":[\"Bash(rm *)\"]},\"hooks\":{},\"env\":{\"CLAUDE_CODE_DISABLE_ARTIFACT\":\"1\",\"KEEP\":\"yes\"}}";
        write(&paths, FileKind::ClaudeSettings, original);
        adapter::apply(&paths, input(client)).unwrap();
        let doc: Value = serde_json::from_str(&read(&paths, FileKind::ClaudeSettings)).unwrap();
        assert_eq!(doc["permissions"]["defaultMode"], "plan");
        assert_eq!(doc["env"]["CLAUDE_CODE_DISABLE_ARTIFACT"], "1");
        assert_eq!(doc["env"]["CLAUDE_CODE_AUTO_MODE_SERVER"], "0");
        assert_eq!(doc["env"]["ANTHROPIC_API_KEY"], "");
        assert_eq!(doc["env"]["CLAUDE_CODE_USE_VERTEX"], "");
        if client == Client::ClaudeVscode {
            let vs: Value = jsonc_parser::parse_to_serde_value(
                &read(&paths, FileKind::ClaudeVscode),
                &Default::default(),
            )
            .unwrap();
            assert_eq!(vs["claudeCode.disableLoginPrompt"], true);
            for field in vs["claudeCode.environmentVariables"].as_array().unwrap() {
                assert_eq!(field["value"], doc["env"][field["name"].as_str().unwrap()]);
            }
        }
        let mut request = input(client);
        request.options.artifact_mode = ArtifactMode::Enabled;
        adapter::apply(&paths, request).unwrap();
        let doc: Value = serde_json::from_str(&read(&paths, FileKind::ClaudeSettings)).unwrap();
        assert_eq!(doc["env"]["CLAUDE_CODE_DISABLE_ARTIFACT"], "0");
        storage::restore(&paths, client).unwrap();
        assert_eq!(read(&paths, FileKind::ClaudeSettings), original);
    }
}

#[test]
fn extension_config_directory_is_used_for_preview_write_and_restore() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    let other = home.path().join("工作 Profile/.claude");
    let vscode = format!("{{\n// Keep this comment\n\"claudeCode.environmentVariables\":[{{\"name\":\"CLAUDE_CONFIG_DIR\",\"value\":{}}}],\"claudeCode.disableLoginPrompt\":false}}", serde_json::to_string(&other).unwrap());
    write(&paths, FileKind::ClaudeVscode, &vscode);
    write(
        &paths,
        FileKind::ClaudeSettings,
        "{\"model\":\"untouched-cli\"}",
    );
    storage::atomic_write(&other.join("settings.json"), b"{\"hooks\":{}}").unwrap();
    let preview = capabilities::preview(&paths, &input(Client::ClaudeVscode)).unwrap();
    assert!(preview
        .details
        .iter()
        .any(|s| s.contains(other.to_str().unwrap())));
    adapter::apply(&paths, input(Client::ClaudeVscode)).unwrap();
    assert!(fs::read_to_string(other.join("settings.json"))
        .unwrap()
        .contains("sk-new-route-fixture"));
    assert_eq!(
        read(&paths, FileKind::ClaudeSettings),
        "{\"model\":\"untouched-cli\"}"
    );
    storage::restore(&paths, Client::ClaudeVscode).unwrap();
    assert_eq!(
        fs::read_to_string(other.join("settings.json")).unwrap(),
        "{\"hooks\":{}}"
    );
    assert_eq!(read(&paths, FileKind::ClaudeVscode), vscode);
}

#[test]
fn invalid_or_duplicate_extension_paths_never_write_the_wrong_home() {
    for vars in [
        json!([{"name":"CLAUDE_CONFIG_DIR","value":"~/.claude"}]),
        json!([{"name":"CLAUDE_CONFIG_DIR","value":""},{"name":"CLAUDE_CONFIG_DIR","value":""}]),
    ] {
        let home = tempfile::tempdir().unwrap();
        let paths = Paths::for_home(home.path());
        write(
            &paths,
            FileKind::ClaudeVscode,
            &json!({"claudeCode.environmentVariables":vars}).to_string(),
        );
        assert!(adapter::apply(&paths, input(Client::ClaudeVscode)).is_err());
        assert!(!paths.target(FileKind::ClaudeSettings).exists());
    }
}

#[test]
fn codex_keeps_official_login_and_keyring_and_cleans_reserved_provider_names() {
    for auth in [
        None,
        Some("{\"tokens\":{\"refresh_token\":\"fake-official-login\"}}"),
        Some("{\"OPENAI_API_KEY\":\"old-official-key\"}"),
    ] {
        let home = tempfile::tempdir().unwrap();
        let paths = Paths::for_home(home.path());
        let config = "# Keep login and comments\ncli_auth_credentials_store = \"keyring\"\nweb_search = \"live\"\n[model_providers.openai]\nbase_url = \"https://old.example\"\n[model_providers.other]\nname = \"Keep me\"\n";
        write(&paths, FileKind::CodexConfig, config);
        if let Some(auth) = auth {
            write(&paths, FileKind::CodexAuth, auth);
        }
        let preview = capabilities::preview(&paths, &input(Client::Codex)).unwrap();
        assert!(preview.summary.contains("实时"));
        assert!(preview.warnings.iter().any(|s| s.contains("保留名称")));
        adapter::apply(&paths, input(Client::Codex)).unwrap();
        assert_eq!(
            storage::read_optional(&paths.target(FileKind::CodexAuth))
                .unwrap()
                .as_deref(),
            auth
        );
        let doc = read(&paths, FileKind::CodexConfig)
            .parse::<toml_edit::DocumentMut>()
            .unwrap();
        assert_eq!(doc["cli_auth_credentials_store"].as_str(), Some("keyring"));
        assert_eq!(doc["web_search"].as_str(), Some("live"));
        assert!(doc["model_providers"].get("openai").is_none());
        assert_eq!(
            doc["model_providers"]["other"]["name"].as_str(),
            Some("Keep me")
        );
        assert_eq!(
            doc["model_providers"]["supergpt-connect"]["experimental_bearer_token"].as_str(),
            Some("sk-new-route-fixture")
        );
        let mut next = input(Client::Codex);
        next.options.web_search = WebSearchMode::Cached;
        adapter::apply(&paths, next).unwrap();
        let doc = read(&paths, FileKind::CodexConfig)
            .parse::<toml_edit::DocumentMut>()
            .unwrap();
        assert_eq!(doc["web_search"].as_str(), Some("cached"));
        storage::restore(&paths, Client::Codex).unwrap();
        assert_eq!(read(&paths, FileKind::CodexConfig), config);
        assert_eq!(
            storage::read_optional(&paths.target(FileKind::CodexAuth))
                .unwrap()
                .as_deref(),
            auth
        );
    }
}

#[test]
fn legacy_desktop_routing_is_replaced_but_restrictions_and_backups_survive() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    let original = "{\"enterpriseConfig\":{\"inferenceProvider\":\"gateway\",\"inferenceGatewayApiKey\":\"old\",\"coworkEgressAllowedHosts\":[\"allowed.example\"]},\"mcpServers\":{}}";
    write(&paths, FileKind::ClaudeDesktopConfig, original);
    adapter::apply(&paths, input(Client::ClaudeDesktop)).unwrap();
    let doc: Value = serde_json::from_str(&read(&paths, FileKind::ClaudeDesktopConfig)).unwrap();
    assert!(doc["enterpriseConfig"].get("inferenceProvider").is_none());
    assert_eq!(
        doc["enterpriseConfig"]["coworkEgressAllowedHosts"],
        json!(["allowed.example"])
    );
    storage::restore(&paths, Client::ClaudeDesktop).unwrap();
    assert_eq!(read(&paths, FileKind::ClaudeDesktopConfig), original);
}

#[test]
fn windows_desktop_directory_selection_rejects_ambiguous_installations() {
    let home = tempfile::tempdir().unwrap();
    let pick = || storage::claude_desktop_directory(home.path(), true);
    assert_eq!(pick().unwrap(), home.path().join("Claude-3p"));
    for name in ["Claude-3p-A", "Claude-3p-B"] {
        fs::create_dir_all(home.path().join(name).join("configLibrary")).unwrap();
        if name.ends_with('A') {
            assert_eq!(pick().unwrap(), home.path().join(name));
        }
    }
    assert!(pick().is_err());
    fs::create_dir(home.path().join("Claude-3p")).unwrap();
    assert_eq!(pick().unwrap(), home.path().join("Claude-3p"));
}

#[test]
fn client_version_gates_compare_numeric_components() {
    assert!(capabilities::version_before("2.1.7", (2, 1, 14)));
    assert!(!capabilities::version_before("2.1.14", (2, 1, 14)));
    assert!(capabilities::version_before("2.1.99", (2, 1, 242)));
    assert!(!capabilities::version_before("2.1.292", (2, 1, 242)));
    assert!(!capabilities::version_before("2.2.0-beta", (2, 1, 242)));
    assert!(capabilities::version_before("1.40608.9", (1, 40609, 0)));
}
