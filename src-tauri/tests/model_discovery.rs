use std::{
    io::{Read, Write},
    net::TcpListener,
};
use supergpt_connect_lib::models::{self, Client, DiscoveryInput};

#[test]
fn url_normalization_and_model_catalog_validation() {
    assert_eq!(
        models::normalize_root("https://api.example.test/prefix/v1/").unwrap(),
        "https://api.example.test/prefix"
    );
    for url in [
        "http://example.test",
        "https://user:pass@example.test",
        "https://example.test?key=bad",
        "https://example.test/v1/responses",
        "file:///tmp/key",
    ] {
        assert!(models::normalize_root(url).is_err());
    }
    let models = models::catalog_models(
        &serde_json::json!({"data":[{"id":"claude-sonnet-test"},{"id":"gpt-test-codex"},{"id":"gpt-test-codex"}]}),
    );
    assert_eq!(models.len(), 2);
}

fn server(status: &str, headers: &str, body: &str) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let root = format!("http://{}", listener.local_addr().unwrap());
    let response=format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}",body.len());
    let handle = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .unwrap();
        let mut buffer = [0u8; 8192];
        let len = socket.read(&mut buffer).unwrap();
        socket.write_all(response.as_bytes()).unwrap();
        String::from_utf8_lossy(&buffer[..len]).into_owned()
    });
    (root, handle)
}

fn request(root: String) -> DiscoveryInput {
    DiscoveryInput {
        client: Client::Codex,
        api_key: "sk-test-not-a-real-key".into(),
        base_url: root,
    }
}

#[tokio::test]
async fn discovery_authenticates_to_models_without_paid_inference() {
    let (root, server) = server("200 OK", "", r#"{"data":[{"id":"gpt-test-codex"}]}"#);
    let data = models::discover(request(root)).await.unwrap();
    assert_eq!(data.choices[0].id, "gpt-test-codex");
    let safe = serde_json::to_string(&models::DiscoverySession::new(data).response()).unwrap();
    assert!(!safe.contains("sk-test-not-a-real-key"));
    let received = server.join().unwrap();
    assert!(received.starts_with("GET /v1/models "));
    assert!(received
        .to_lowercase()
        .contains("authorization: bearer sk-test-not-a-real-key"));
}

#[tokio::test]
async fn bad_keys_and_redirects_return_redacted_errors() {
    for (status, headers) in [
        ("401 Unauthorized", ""),
        (
            "302 Found",
            "Location: http://127.0.0.1:9/must-not-receive-key\r\n",
        ),
    ] {
        let (root, server) = server(status, headers, r#"{"echo":"sk-test-not-a-real-key"}"#);
        let error = models::discover(request(root)).await.err().unwrap();
        assert!(!error.contains("sk-test"));
        assert!(error.contains("验证") || error.contains("302"));
        server.join().unwrap();
    }
}

fn discovered(client: Client) -> models::DiscoveredModels {
    models::DiscoveredModels {
        client,
        api_key: "sk-only-in-memory".into(),
        root: "https://example.test".into(),
        payload: serde_json::json!({"data":[{"id":"custom-a"},{"id":"custom-b"},{"id":"excluded"}]}),
        choices: ["custom-a", "custom-b", "excluded"]
            .into_iter()
            .map(|id| models::ModelChoice {
                id: id.into(),
                name: id.into(),
            })
            .collect(),
    }
}

#[test]
fn selection_accepts_only_fetched_models_and_a_selected_default() {
    let data = discovered(Client::Codex);
    for (ids, default) in [
        (vec![], ""),
        (vec!["custom-a"], "custom-b"),
        (vec!["invented"], "invented"),
        (vec!["custom-a", "custom-a"], "custom-a"),
    ] {
        assert!(models::prepare_selection(
            &data,
            &ids.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            default
        )
        .is_err());
    }
    let chosen =
        models::prepare_selection(&data, &["custom-a".into(), "custom-b".into()], "custom-b")
            .unwrap();
    assert_eq!(chosen.model, "custom-b");
    let catalog: serde_json::Value =
        serde_json::from_str(chosen.catalog.as_ref().unwrap()).unwrap();
    let ids: Vec<_> = catalog["models"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["slug"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["custom-a", "custom-b"]);
    assert_eq!(
        catalog["models"][0]["input_modalities"],
        serde_json::json!(["text"])
    );
    assert!(catalog["models"][0]["context_window"].is_null());
    assert!(catalog["models"][0]["default_reasoning_level"].is_null());
}

#[test]
fn model_selection_is_bound_to_the_discovery_session() {
    let session = models::DiscoverySession::new(discovered(Client::Claude));
    let response = session.response();
    let mut request = models::SelectionInput {
        discovery_id: response.discovery_id,
        selected_models: vec!["custom-a".into()],
        default_model: "custom-a".into(),
    };
    assert!(session.prepare(&request).unwrap().catalog.is_none());
    request.discovery_id = "previous-session".into();
    assert!(session.prepare(&request).is_err());
}

#[test]
fn gateway_native_metadata_survives_while_unchecked_models_are_removed() {
    let payload = serde_json::json!({"models":[{"slug":"custom-a","display_name":"Team A","context_window":123456,"input_modalities":["text","image"],"base_instructions":"Gateway instructions"},{"slug":"excluded"}]});
    let result = models::build_codex_catalog(&payload, &["custom-a".into()]).unwrap();
    let catalog: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(catalog["models"].as_array().unwrap().len(), 1);
    assert_eq!(catalog["models"][0]["display_name"], "Team A");
    assert_eq!(catalog["models"][0]["context_window"], 123456);
    assert_eq!(
        catalog["models"][0]["base_instructions"],
        "Gateway instructions"
    );
}

#[test]
fn slug_only_catalogs_do_not_imply_vision_or_large_context_support() {
    let payload = serde_json::json!({"models":[{"slug":"custom-a"}]});
    let generated = models::build_codex_catalog(&payload, &["custom-a".into()]).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&generated).unwrap();
    assert!(parsed["models"][0]["context_window"].is_null());
    assert_eq!(
        parsed["models"][0]["input_modalities"],
        serde_json::json!(["text"])
    );
    assert_eq!(
        parsed["models"][0]["supported_reasoning_levels"],
        serde_json::json!([])
    );
}

#[test]
fn known_codex_models_keep_their_reasoning_options_from_name_only_lists() {
    let ids = vec![
        "gpt-5.4".into(),
        "gpt-5.1-codex".into(),
        "gpt-6.1-sol".into(),
    ];
    for payload in [
        serde_json::json!({"data":[{"id":"gpt-5.4"},{"id":"gpt-5.1-codex"},{"id":"gpt-6.1-sol"}]}),
        serde_json::json!({"models":[{"slug":"gpt-5.4"},{"slug":"gpt-5.1-codex"},{"slug":"gpt-6.1-sol"}]}),
    ] {
        let generated = models::build_codex_catalog(&payload, &ids).unwrap();
        let catalog: serde_json::Value = serde_json::from_str(&generated).unwrap();
        let efforts = |index: usize| -> Vec<&str> {
            catalog["models"][index]["supported_reasoning_levels"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v["effort"].as_str().unwrap())
                .collect()
        };
        assert_eq!(efforts(0), ["low", "medium", "high", "xhigh"]);
        assert_eq!(efforts(1), ["low", "medium", "high"]);
        assert_eq!(
            efforts(2),
            ["low", "medium", "high", "xhigh", "max", "ultra"]
        );
        for model in catalog["models"].as_array().unwrap() {
            assert_eq!(model["supports_reasoning_summaries"], true);
            assert!(model["supported_reasoning_levels"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["effort"] == model["default_reasoning_level"]));
            assert_eq!(model["context_window"], 272000);
            assert_eq!(model["input_modalities"], serde_json::json!(["text"]));
        }
    }
}

#[test]
fn gateway_reasoning_metadata_in_data_is_used_for_custom_aliases() {
    let payload = serde_json::json!({"data":[{
        "id":" company-model ",
        "supported_reasoning_levels":[{"effort":"low"},{"effort":"high","description":"Deep"}],
        "default_reasoning_level":"high"
    }]});
    let generated = models::build_codex_catalog(&payload, &["company-model".into()]).unwrap();
    let catalog: serde_json::Value = serde_json::from_str(&generated).unwrap();
    let model = &catalog["models"][0];
    assert_eq!(model["slug"], "company-model");
    assert_eq!(model["default_reasoning_level"], "high");
    assert_eq!(
        model["supported_reasoning_levels"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(model["supported_reasoning_levels"][0]["description"].is_string());
    assert_eq!(
        model["supported_reasoning_levels"][1]["description"],
        "Deep"
    );
    assert_eq!(model["supports_reasoning_summaries"], true);
}

#[test]
fn gateway_reasoning_restrictions_override_known_models_and_keep_default_valid() {
    for field in ["data", "models"] {
        let payload = serde_json::json!({field:[{
            "id":"gpt-5.4", "slug":"gpt-5.4",
            "supported_reasoning_levels":[{"effort":"high","description":"Only supported effort"}]
        }]});
        let generated = models::build_codex_catalog(&payload, &["gpt-5.4".into()]).unwrap();
        let catalog: serde_json::Value = serde_json::from_str(&generated).unwrap();
        assert_eq!(catalog["models"][0]["default_reasoning_level"], "high");
        assert_eq!(
            catalog["models"][0]["supported_reasoning_levels"]
                .as_array()
                .unwrap()
                .len(),
            1
        );

        let payload = serde_json::json!({field:[{
            "id":"gpt-5.4", "slug":"gpt-5.4", "supported_reasoning_levels":[]
        }]});
        let generated = models::build_codex_catalog(&payload, &["gpt-5.4".into()]).unwrap();
        let catalog: serde_json::Value = serde_json::from_str(&generated).unwrap();
        assert!(catalog["models"][0]["default_reasoning_level"].is_null());
        assert_eq!(
            catalog["models"][0]["supported_reasoning_levels"],
            serde_json::json!([])
        );
        assert_eq!(catalog["models"][0]["supports_reasoning_summaries"], false);
    }
}

#[test]
fn reasoning_is_not_guessed_for_similarly_named_aliases_or_non_reasoning_models() {
    let ids = vec![
        "gpt-4.1".into(),
        "company-gpt-5.4".into(),
        "gpt-5.4-fast-custom".into(),
    ];
    let generated = models::build_codex_catalog(&serde_json::json!({}), &ids).unwrap();
    let catalog: serde_json::Value = serde_json::from_str(&generated).unwrap();
    for model in catalog["models"].as_array().unwrap() {
        assert!(model["default_reasoning_level"].is_null());
        assert_eq!(model["supported_reasoning_levels"], serde_json::json!([]));
        assert_eq!(model["supports_reasoning_summaries"], false);
    }
}

#[test]
fn malformed_gateway_reasoning_options_do_not_produce_a_broken_catalog() {
    for levels in [
        serde_json::json!("high"),
        serde_json::json!(["high"]),
        serde_json::json!([{}]),
    ] {
        let payload =
            serde_json::json!({"data":[{"id":"custom", "supported_reasoning_levels":levels}]});
        assert!(models::build_codex_catalog(&payload, &["custom".into()]).is_err());
    }
}
