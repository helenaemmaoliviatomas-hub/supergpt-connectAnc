use serde_json::{json, Value};
use std::fs;
use supergpt_connect_lib::{
    adapter,
    models::{self, Client, ValidatedInput},
    storage::{self, FileKind, Paths},
};

fn catalog(id: &str, metadata: Value) -> Result<Value, String> {
    let mut row = metadata;
    row["id"] = json!(id);
    models::build_codex_catalog(&json!({"data":[row]}), &[id.into()])
        .map(|text| serde_json::from_str(&text).unwrap())
}

fn import(paths: &Paths, id: &str, metadata: Value) -> Result<(), String> {
    let catalog = catalog(id, metadata)?;
    adapter::apply(
        paths,
        ValidatedInput {
            client: Client::CodexDesktop,
            api_key: "fake-test-only".into(),
            root: "https://example.test".into(),
            model: id.into(),
            selected_models: vec![id.into()],
            catalog: Some(catalog.to_string()),
        },
    )
    .map(|_| ())
}

fn read(paths: &Paths) -> Value {
    serde_json::from_str(&fs::read_to_string(paths.target(FileKind::CodexCatalog)).unwrap())
        .unwrap()
}

#[test]
fn partial_gateway_context_metadata_does_not_erase_a_local_window_repair() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    import(&paths, "gpt-6.1-sol", json!({})).unwrap();
    let mut old = read(&paths);
    old["models"][0]["context_window"] = json!(500000);
    old["models"][0]["auto_compact_token_limit"] = json!(440000);
    storage::atomic_write(
        &paths.target(FileKind::CodexCatalog),
        old.to_string().as_bytes(),
    )
    .unwrap();
    for _ in 0..2 {
        import(
            &paths,
            "gpt-6.1-sol",
            json!({"effective_context_window_percent":90}),
        )
        .unwrap();
        let row = &read(&paths)["models"][0];
        assert_eq!(row["context_window"], 500000);
        assert_eq!(row["max_context_window"], 872000);
        assert_eq!(row["auto_compact_token_limit"], 440000);
        assert_eq!(row["effective_context_window_percent"], 90);
    }
}

#[test]
fn a_gateway_percentage_can_combine_with_a_cached_unknown_model_capacity() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    storage::atomic_write(
        &paths.codex.join("models_cache.json"),
        br#"{"models":[{"slug":"alias","context_window":128000,"max_context_window":128000}]}"#,
    )
    .unwrap();
    import(
        &paths,
        "alias",
        json!({"effective_context_window_percent":90}),
    )
    .unwrap();
    let row = &read(&paths)["models"][0];
    assert_eq!(row["context_window"], 128000);
    assert_eq!(row["effective_context_window_percent"], 90);
}

#[test]
fn numeric_reasoning_options_are_serialized_as_codex_catalog_strings() {
    let generated = catalog(
        "alias",
        json!({"context_window":128000,
        "supported_reasoning_levels":[{"effort":1024},{"effort":4096}],
        "default_reasoning_level":4096}),
    )
    .unwrap();
    let row = &generated["models"][0];
    assert_eq!(row["supported_reasoning_levels"][0]["effort"], "1024");
    assert_eq!(row["default_reasoning_level"], "4096");
}

#[test]
fn null_context_metadata_means_unknown_not_an_erased_known_profile() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    import(
        &paths,
        "gpt-6.1-sol",
        json!({"context_window":null,"max_context_window":null}),
    )
    .unwrap();
    let row = &read(&paths)["models"][0];
    assert_eq!(row["context_window"], 272000);
    assert_eq!(row["max_context_window"], 872000);
}

#[test]
fn bundled_ultra_mapping_matches_the_official_model_catalog() {
    for id in ["gpt-6.1-sol", "gpt-6-astra"] {
        let generated = catalog(id, json!({})).unwrap();
        assert_eq!(
            generated["models"][0]["multi_agent_reasoning_effort"],
            "xhigh"
        );
    }
    let generated = catalog(
        "gpt-6.1-sol",
        json!({"multi_agent_reasoning_effort":"high"}),
    )
    .unwrap();
    assert_eq!(
        generated["models"][0]["multi_agent_reasoning_effort"],
        "high"
    );
}

#[test]
fn malformed_reasoning_metadata_never_changes_client_files() {
    for metadata in [
        json!({"supported_reasoning_levels":[{"effort":""}]}),
        json!({"supported_reasoning_levels":[{"effort":"high","description":false}]}),
        json!({"supports_reasoning_summaries":"true"}),
        json!({"supports_reasoning_summaries":false,"supported_reasoning_levels":[{"effort":"high"}]}),
        json!({"multi_agent_reasoning_effort":false}),
    ] {
        let home = tempfile::tempdir().unwrap();
        let paths = Paths::for_home(home.path());
        assert!(import(&paths, "gpt-6.1-sol", metadata).is_err());
        for kind in [
            FileKind::CodexConfig,
            FileKind::CodexAuth,
            FileKind::CodexCatalog,
        ] {
            assert!(!paths.target(kind).exists());
        }
        assert!(!paths.has_backup(Client::Codex));
    }
}
