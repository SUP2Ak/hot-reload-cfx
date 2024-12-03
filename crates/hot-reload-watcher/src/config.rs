use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct WatcherConfig {
    pub ws_host: String,
    pub ws_port: u16,
    pub fivem_port: u16,
    pub resources_path: String,
    pub api_key: String,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            ws_host: "localhost".to_string(),
            ws_port: 3091,
            fivem_port: 3090,
            resources_path: String::new(),
            api_key: String::new(),
        }
    }
}

impl WatcherConfig {
    pub fn load_or_create() -> Self {
        if let Ok(content) = std::fs::read_to_string("config_hot_reload.json") {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            let config = Self::default();
            let _ = std::fs::write(
                "config_hot_reload.json",
                serde_json::to_string_pretty(&config).unwrap(),
            );
            config
        }
    }

    pub fn ws_url(&self) -> String {
        format!("ws://{}:{}", self.ws_host, self.ws_port)
    }
}