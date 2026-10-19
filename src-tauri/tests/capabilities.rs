use serde_json::{json, Value};
use supergpt_connect_lib::{
    adapter,
    capabilities::{self, State, ToolSearchMode},
    models::{Client, ValidatedInput},
    restore,
    storage::{self, FileKind, Paths},
};
fn input(client: Client, status: &str) -> ValidatedInput {
    ValidatedInput { client, api_key:"sk-fixture-only".into(), root:"https://gateway.example.test".into(),model:"claude-sonnet-4-6".into(),selected_models:vec!["claude-sonnet-4-6".into()],catalog:None,
        capabilities: capabilities::parse(&json!({"object":"supergpt.connect_capabilities","schema_version":1,"models":{"claude-sonnet-4-6":{"messages":{"status":"supported"},"claude_tool_search":{"status":status}}}})).unwrap(),tool_search:ToolSearchMode::Auto }
}
fn read(paths: &Paths, kind: FileKind) -> Value {
    serde_json::from_str(&std::fs::read_to_string(paths.target(kind)).unwrap()).unwrap()
}
#[test]
fn defaults_follow_verified_route_and_switching_clears_prior_flag() {
    for client in [Client::Claude, Client::ClaudeVscode, Client::ClaudeDesktop] {
        let home = tempfile::tempdir().unwrap();
        let paths = Paths::for_home(home.path());
        assert!(
            capabilities::preview(&paths, &input(client, "supported"))
                .unwrap()
                .tool_search_enabled
        );
        adapter::apply(&paths, input(client, "supported")).unwrap();
        if client == Client::ClaudeDesktop {
            assert_eq!(
                read(&paths, FileKind::ClaudeDesktopProfile)["toolSearchEnabled"],
                true
            );
        } else {
            assert_eq!(
                read(&paths, FileKind::ClaudeSettings)["env"]["ENABLE_TOOL_SEARCH"],
                "true"
            );
            if client == Client::ClaudeVscode {
                let v = read(&paths, FileKind::ClaudeVscode);
                assert!(v["claudeCode.environmentVariables"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|x| x["name"] == "ENABLE_TOOL_SEARCH" && x["value"] == "true"));
            }
        }
        adapter::apply(&paths, input(client, "unknown")).unwrap();
        if client == Client::ClaudeDesktop {
            assert_eq!(
                read(&paths, FileKind::ClaudeDesktopProfile)["toolSearchEnabled"],
                false
            );
        } else {
            assert_eq!(
                read(&paths, FileKind::ClaudeSettings)["env"]["ENABLE_TOOL_SEARCH"],
                "false"
            );
        }
        let point = restore::list(&paths)
            .into_iter()
            .find(|p| p.client == client.backup_client())
            .unwrap();
        restore::apply(&paths, client, point.review_id.as_deref().unwrap(), false).unwrap();
        assert!(!paths.target(FileKind::primary(client)).exists());
    }
}
#[test]
fn permissions_and_beta_opt_out_are_preserved_and_reported_before_write() {
    for original in [
        r#"{"env":{"CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS":"1"},"hooks":{"keep":[]}}"#,
        r#"{"permissions":{"deny":["ToolSearch"]}}"#,
    ] {
        let home = tempfile::tempdir().unwrap();
        let paths = Paths::for_home(home.path());
        storage::atomic_write(&paths.target(FileKind::ClaudeSettings), original.as_bytes())
            .unwrap();
        let preview = capabilities::preview(&paths, &input(Client::Claude, "supported")).unwrap();
        assert!(!preview.tool_search_enabled);
        assert!(!preview.warnings.is_empty());
        adapter::apply(&paths, input(Client::Claude, "supported")).unwrap();
        let current = read(&paths, FileKind::ClaudeSettings);
        let old: Value = serde_json::from_str(original).unwrap();
        for (key, value) in old.as_object().unwrap() {
            if key == "env" {
                assert_eq!(
                    current["env"]["CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS"],
                    value["CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS"]
                );
            } else {
                assert_eq!(&current[key], value);
            }
        }
    }
}
#[test]
fn vscode_jsonc_conflict_is_visible_and_overrides_stale_true() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    storage::atomic_write(&paths.target(FileKind::ClaudeVscode),b"{ // keep\n\"claudeCode.environmentVariables\":[{\"name\":\"CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS\",\"value\":\"true\"},{\"name\":\"ENABLE_TOOL_SEARCH\",\"value\":\"true\"},],}").unwrap();
    assert!(
        !capabilities::preview(&paths, &input(Client::ClaudeVscode, "supported"))
            .unwrap()
            .tool_search_enabled
    );
    adapter::apply(&paths, input(Client::ClaudeVscode, "supported")).unwrap();
    let text = std::fs::read_to_string(paths.target(FileKind::ClaudeVscode)).unwrap();
    assert!(text.contains("// keep"));
    let doc: Value = jsonc_parser::parse_to_serde_value(&text, &Default::default()).unwrap();
    let flags: Vec<_> = doc["claudeCode.environmentVariables"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|x| x["name"] == "ENABLE_TOOL_SEARCH")
        .collect();
    assert_eq!(flags.len(), 1);
    assert_eq!(flags[0]["value"], "false");
}
#[test]
fn mixed_models_and_manual_modes_do_not_silently_enable_unsupported_routes() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    let mut request = input(Client::Claude, "supported");
    request.selected_models.push("unverified".into());
    assert_eq!(
        capabilities::preview(&paths, &request)
            .unwrap()
            .tool_search_state,
        State::Unknown
    );
    request.tool_search = ToolSearchMode::Enabled;
    assert!(
        capabilities::preview(&paths, &request)
            .unwrap()
            .tool_search_enabled
    );
    let mut request = input(Client::Claude, "unsupported");
    request.tool_search = ToolSearchMode::Enabled;
    assert!(adapter::apply(&paths, request).is_err());
    assert!(!paths.has_backup(Client::Claude));
    let mut request = input(Client::Claude, "supported");
    request.tool_search = ToolSearchMode::Disabled;
    assert!(
        !capabilities::preview(&paths, &request)
            .unwrap()
            .tool_search_enabled
    );
    assert!(capabilities::parse(
        &json!({"object":"supergpt.connect_capabilities","schema_version":2,"models":{}})
    )
    .is_none());
    assert!(capabilities::parse(&json!({"object":"supergpt.connect_capabilities","schema_version":1,"models":{"x":{"messages":{"status":"invented"}}}})).is_none());
}
