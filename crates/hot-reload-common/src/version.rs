/* */
use serde::Deserialize;
use tracing;
use serde_json;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct VersionFile {
    latest: String,
    versions: std::collections::HashMap<String, VersionInfo>,
}

#[derive(Deserialize)]
pub struct VersionInfo {
    requires: HashMap<String, String>,
    changelog: Vec<String>,
    download: String,
}

#[derive(Deserialize)]
pub struct Requirements {
    ui: String,
    resource: Option<String>,
    watcher: Option<String>,
}

impl VersionFile {
    pub fn get_latest(&self) -> &str {
        &self.latest
    }

    pub fn get_version_info(&self, version: &str) -> Option<&VersionInfo> {
        self.versions.get(version)
    }

    pub fn get_latest_info(&self) -> Option<&VersionInfo> {
        self.versions.get(&self.latest)
    }

    pub fn has_update(&self, current_version: &str) -> bool {
        self.latest != current_version
    }
}

pub struct VersionChecker;

impl VersionChecker {
    pub async fn check_all() -> Result<(Option<VersionFile>, Option<VersionFile>, Option<VersionFile>), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let watcher = Self::check_component(&client, "watcher").await;
        let ui = Self::check_component(&client, "ui").await;
        let fx = Self::check_component(&client, "fx").await;
        
        Ok((
            watcher.ok(),
            ui.ok(),
            fx.ok()
        ))
    }

    pub async fn check_watcher() -> Result<VersionFile, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        Self::check_component(&client, "watcher").await
    }

    pub async fn check_fx() -> Result<VersionFile, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        Self::check_component(&client, "fx").await
    }

    pub async fn check_ui() -> Result<VersionFile, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        Self::check_component(&client, "ui").await
    }

    async fn check_component(client: &reqwest::Client, id: &str) -> Result<VersionFile, Box<dyn std::error::Error>> {
        let response = client
            .get(format!("https://raw.githubusercontent.com/SUP2Ak/hot-reload-cfx/main/version.{}.json", id))
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err("Version file not found".into());
        }

        let text = response.text().await?;

        match serde_json::from_str::<VersionFile>(&text) {
            Ok(version_file) => Ok(version_file),
            Err(e) => {
                tracing::error!("Failed to parse JSON response: {}", e);
                tracing::error!("Received content: {}", text);
                Err(format!("Invalid JSON format: {}", e).into())
            }
        }
    }
}