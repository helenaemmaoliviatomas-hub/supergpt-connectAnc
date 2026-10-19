//! Create isolated real-client fixtures through the same import engine as the UI.
use serde_json::json;
use supergpt_connect_lib::{
    adapter, capabilities,
    models::{Client, ValidatedInput},
    storage::Paths,
};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 5);
    let paths = Paths::for_home(std::path::Path::new(&args[1]));
    let caps=capabilities::parse(&json!({"object":"supergpt.connect_capabilities","schema_version":1,"models":{args[3].clone():{"messages":{"status":"supported"},"claude_tool_search":{"status":args[4]}}}})).unwrap();
    adapter::apply(
        &paths,
        ValidatedInput {
            client: Client::Claude,
            api_key: "sk-local-fixture".into(),
            root: args[2].clone(),
            model: args[3].clone(),
            selected_models: vec![args[3].clone()],
            catalog: None,
            capabilities: caps,
            tool_search: Default::default(),
        },
    )
    .unwrap();
    println!("{}", paths.claude.display());
}
