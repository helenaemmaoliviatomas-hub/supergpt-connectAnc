use serde_json::{json, Value};
use std::fs;
use supergpt_connect_lib::{
    adapter,
    models::{self, Client, ValidatedInput},
    storage::{self, FileKind, Paths},
};

fn input(id: &str, metadata: Value) -> ValidatedInput {
    let mut row = metadata;
    row["id"] = json!(id);
    let selected_models = vec![id.into()];
    ValidatedInput {
        capabilities: Default::default(),
        tool_search: Default::default(),
        client: Client::CodexDesktop,
        api_key: "fake-fixture-only".into(),
        root: "https://example.test".into(),
        model: id.into(),
        catalog: Some(
            models::build_codex_catalog(&json!({"data":[row]}), &selected_models).unwrap(),
        ),
        selected_models,
    }
}

fn read(paths: &Paths) -> Value {
    serde_json::from_str(&fs::read_to_string(paths.target(FileKind::CodexCatalog)).unwrap())
        .unwrap()
}

fn write(paths: &Paths, catalog: &Value) {
    storage::atomic_write(
        &paths.target(FileKind::CodexCatalog),
        &serde_json::to_vec(catalog).unwrap(),
    )
    .unwrap();
}

#[test]
fn sol_context_and_maximum_match_official_catalog_without_changing_user_overrides() {
    for client in [Client::Codex, Client::CodexDesktop] {
        let home = tempfile::tempdir().unwrap();
        let paths = Paths::for_home(home.path());
        let original = "model_context_window = 500000 # my window\nmodel_auto_compact_token_limit = 450000 # my threshold\n";
        storage::atomic_write(&paths.target(FileKind::CodexConfig), original.as_bytes()).unwrap();
        let mut data = input("gpt-6.1-sol", json!({}));
        data.client = client;
        adapter::apply(&paths, data).unwrap();
        let catalog = read(&paths);
        assert_eq!(catalog["models"][0]["context_window"], 272000);
        assert_eq!(catalog["models"][0]["max_context_window"], 872000);
        let config = fs::read_to_string(paths.target(FileKind::CodexConfig)).unwrap();
        assert!(config.starts_with(original));
        assert!(
            !adapter::apply(&paths, input("gpt-6.1-sol", json!({})))
                .unwrap()
                .changed
        );
        storage::restore(&paths, client).unwrap();
        assert_eq!(
            fs::read_to_string(paths.target(FileKind::CodexConfig)).unwrap(),
            original
        );
        assert!(!paths.target(FileKind::CodexCatalog).exists());
    }
}

#[test]
fn legacy_32k_defaults_are_repaired_but_explicit_local_repairs_survive_repeated_imports() {
    for repaired in [false, true] {
        let home = tempfile::tempdir().unwrap();
        let paths = Paths::for_home(home.path());
        adapter::apply(&paths, input("gpt-6.1-sol", json!({}))).unwrap();
        let mut legacy = read(&paths);
        legacy.as_object_mut().unwrap().remove("_supergpt_connect");
        legacy["models"][0]["context_window"] = json!(if repaired { 500000 } else { 32768 });
        legacy["models"][0]["max_context_window"] = json!(if repaired { 872000 } else { 32768 });
        if repaired {
            legacy["models"][0]["auto_compact_token_limit"] = json!(440000);
        }
        write(&paths, &legacy);
        for _ in 0..2 {
            adapter::apply(&paths, input("gpt-6.1-sol", json!({}))).unwrap();
            let catalog = read(&paths);
            assert_eq!(
                catalog["models"][0]["context_window"],
                if repaired { 500000 } else { 272000 }
            );
            assert_eq!(catalog["models"][0]["max_context_window"], 872000);
            if repaired {
                assert_eq!(catalog["models"][0]["auto_compact_token_limit"], 440000);
            }
        }
    }
}

#[test]
fn new_local_edits_persist_but_unchanged_generated_defaults_can_upgrade() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    adapter::apply(&paths, input("gpt-6.1-sol", json!({}))).unwrap();
    let mut catalog = read(&paths);
    // Simulate an older generated profile rather than a user edit.
    catalog["models"][0]["context_window"] = json!(200000);
    catalog["_supergpt_connect"]["generated_context"]["gpt-6.1-sol"]["context_window"] =
        json!(200000);
    write(&paths, &catalog);
    adapter::apply(&paths, input("gpt-6.1-sol", json!({}))).unwrap();
    assert_eq!(read(&paths)["models"][0]["context_window"], 272000);
    catalog = read(&paths);
    catalog["models"][0]["context_window"] = json!(500000);
    catalog["models"][0]["auto_compact_token_limit"] = json!(430000);
    write(&paths, &catalog);
    for _ in 0..2 {
        adapter::apply(&paths, input("gpt-6.1-sol", json!({}))).unwrap();
        assert_eq!(read(&paths)["models"][0]["context_window"], 500000);
        assert_eq!(
            read(&paths)["models"][0]["auto_compact_token_limit"],
            430000
        );
    }
    let mut other = input("gpt-6.1-sol", json!({}));
    other.root = "https://other.example.test".into();
    adapter::apply(&paths, other).unwrap();
    assert_eq!(read(&paths)["models"][0]["context_window"], 272000);
}

#[test]
fn gateway_limits_override_profiles_and_local_repairs_in_both_catalog_formats() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    adapter::apply(&paths, input("gpt-6.1-sol", json!({}))).unwrap();
    let mut catalog = read(&paths);
    catalog["models"][0]["context_window"] = json!(500000);
    write(&paths, &catalog);
    for field in ["data", "models"] {
        let mut data = input("gpt-6.1-sol", json!({}));
        data.catalog = Some(
            models::build_codex_catalog(
                &json!({field:[{"slug":"gpt-6.1-sol","context_window":32768}]}),
                &data.selected_models,
            )
            .unwrap(),
        );
        adapter::apply(&paths, data).unwrap();
        assert_eq!(read(&paths)["models"][0]["context_window"], 32768);
        assert_eq!(read(&paths)["models"][0]["max_context_window"], 32768);
    }
    adapter::apply(
        &paths,
        input("gpt-6.1-sol", json!({"max_context_window":64000})),
    )
    .unwrap();
    assert_eq!(read(&paths)["models"][0]["context_window"], 64000);
    assert_eq!(read(&paths)["models"][0]["max_context_window"], 64000);
}

#[test]
fn unknown_capacity_is_never_guessed_or_committed_and_valid_cache_is_read_only() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    for cache in [
        None,
        Some("broken"),
        Some("{\"models\":[{\"slug\":\"alias\",\"context_window\":-1}]}"),
    ] {
        if let Some(text) = cache {
            storage::atomic_write(&paths.codex.join("models_cache.json"), text.as_bytes()).unwrap();
        }
        let error = adapter::apply(&paths, input("alias", json!({}))).unwrap_err();
        assert!(error.contains("alias"));
        assert!(!paths.target(FileKind::CodexConfig).exists());
        assert!(!paths.target(FileKind::CodexAuth).exists());
        assert!(!paths.target(FileKind::CodexCatalog).exists());
        assert!(!paths.has_backup(Client::Codex));
    }
    let cached = "{\"models\":[{\"slug\":\"alias\",\"context_window\":128000,\"base_instructions\":\"do not copy\"}]}";
    storage::atomic_write(&paths.codex.join("models_cache.json"), cached.as_bytes()).unwrap();
    adapter::apply(&paths, input("alias", json!({}))).unwrap();
    assert_eq!(read(&paths)["models"][0]["context_window"], 128000);
    assert_ne!(
        read(&paths)["models"][0]["base_instructions"],
        "do not copy"
    );
    assert_eq!(
        fs::read_to_string(paths.codex.join("models_cache.json")).unwrap(),
        cached
    );
    let mut repaired = read(&paths);
    repaired["models"][0]["context_window"] = json!(96000);
    write(&paths, &repaired);
    fs::remove_file(paths.codex.join("models_cache.json")).unwrap();
    adapter::apply(&paths, input("alias", json!({}))).unwrap();
    assert_eq!(read(&paths)["models"][0]["context_window"], 96000);
}

#[test]
fn invalid_gateway_context_is_rejected_instead_of_creating_an_unusable_catalog() {
    for metadata in [
        json!({"context_window":0}),
        json!({"context_window":-1}),
        json!({"context_window":"500000"}),
        json!({"context_window":1.5}),
        json!({"context_window":500000,"max_context_window":32768}),
        json!({"max_context_window":-2}),
        json!({"effective_context_window_percent":0}),
        json!({"effective_context_window_percent":101}),
        json!({"auto_compact_token_limit":-1}),
    ] {
        let mut row = metadata;
        row["id"] = json!("gpt-6.1-sol");
        assert!(
            models::build_codex_catalog(&json!({"data":[row]}), &["gpt-6.1-sol".into()]).is_err()
        );
    }
}

#[test]
fn broken_local_repairs_abort_without_overwriting_any_client_file() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    adapter::apply(&paths, input("gpt-6.1-sol", json!({}))).unwrap();
    let mut catalog = read(&paths);
    catalog["models"][0]["context_window"] = json!(500000);
    catalog["models"][0]["max_context_window"] = json!(32768);
    write(&paths, &catalog);
    let files = [
        FileKind::CodexConfig,
        FileKind::CodexAuth,
        FileKind::CodexCatalog,
    ];
    let before: Vec<_> = files
        .iter()
        .map(|k| fs::read(paths.target(*k)).unwrap())
        .collect();
    assert!(adapter::apply(&paths, input("gpt-6.1-sol", json!({}))).is_err());
    for (kind, bytes) in files.iter().zip(before) {
        assert_eq!(fs::read(paths.target(*kind)).unwrap(), bytes);
    }
}

#[test]
fn stale_cache_cannot_reintroduce_the_old_limit_for_a_known_model() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    storage::atomic_write(
        &paths.codex.join("models_cache.json"),
        br#"{"models":[{"slug":"gpt-6.1-sol","context_window":32768,"max_context_window":32768}]}"#,
    )
    .unwrap();
    adapter::apply(&paths, input("gpt-6.1-sol", json!({}))).unwrap();
    assert_eq!(read(&paths)["models"][0]["context_window"], 272000);
    assert_eq!(read(&paths)["models"][0]["max_context_window"], 872000);
}
