//! Create isolated real-client fixtures through the same import engine as the UI.
use serde_json::json;
use supergpt_connect_lib::{
    adapter, capabilities,
    models::{Client, ValidatedInput},
    storage::Paths,
};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert!(args.len() == 5 || args.len() == 6);
    let selected_models: Vec<String> = if args.len() == 6 {
        serde_json::from_str(&args[5]).unwrap()
    } else {
        vec![args[3].clone()]
    };
    let paths = Paths::for_home(std::path::Path::new(&args[1]));
    let mut verified = serde_json::Map::new();
    for model in &selected_models {
        verified.insert(
            model.clone(),
            json!({"messages":{"status":"supported"},"claude_tool_search":{"status":args[4]}}),
        );
    }
    let caps = capabilities::parse(
        &json!({"object":"supergpt.connect_capabilities","schema_version":1,"models":verified}),
    )
    .unwrap();
    adapter::apply(
        &paths,
        ValidatedInput {
            client: Client::Claude,
            api_key: "sk-local-fixture".into(),
            root: args[2].clone(),
            model: args[3].clone(),
            selected_models,
            catalog: None,
            capabilities: caps,
            tool_search: Default::default(),
            options: Default::default(),
        },
    )
    .unwrap();
    println!("{}", paths.claude.display());
}
