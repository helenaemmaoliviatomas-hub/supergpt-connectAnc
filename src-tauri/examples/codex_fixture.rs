//! Test driver for the real production discovery/import/restore path. Fake credentials only.
use serde::Deserialize;
use std::{env, fs, path::PathBuf};
use supergpt_connect_lib::{adapter, models, restore, storage::Paths};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    home: PathBuf,
    root: String,
    client: models::Client,
    selected_models: Vec<String>,
    default_model: String,
    #[serde(default)]
    restore: bool,
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let file = env::args().nth(1).ok_or("fixture JSON path required")?;
    let spec: Fixture = serde_json::from_str(&fs::read_to_string(file).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if !spec.home.is_absolute()
        || !spec.home.join(".connect-runtime-fixture").is_file()
        || !spec.root.starts_with("http://127.0.0.1:")
        || !spec.client.is_codex()
    {
        return Err("Only marked temporary homes and a local fake gateway are allowed".into());
    }
    let paths = Paths::for_home(&spec.home);
    if spec.restore {
        let point = restore::list(&paths)
            .into_iter()
            .find(|p| p.client == models::Client::Codex)
            .ok_or("restore point missing")?;
        let result = restore::apply(
            &paths,
            spec.client,
            point.review_id.as_deref().ok_or("no backup")?,
            false,
        )?;
        println!("{}", serde_json::to_string(&result).unwrap());
    } else {
        let discovered = models::discover(models::DiscoveryInput {
            client: spec.client,
            api_key: "sk-connect-fixture-only".into(),
            base_url: spec.root,
        })
        .await?;
        let session = models::DiscoverySession::new(discovered);
        let input = session.prepare(&models::SelectionInput {
            tool_search: Default::default(),
            options: Default::default(),
            discovery_id: session.response().discovery_id,
            selected_models: spec.selected_models,
            default_model: spec.default_model,
        })?;
        let result = adapter::apply(&paths, input)?;
        println!("{}", serde_json::to_string(&result).unwrap());
    }
    Ok(())
}
