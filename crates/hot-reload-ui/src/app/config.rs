use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use crate::utils::Language;

#[derive(Serialize, Deserialize, Clone)]
pub struct ConnectionProfile {
    pub name: String,
    pub ws_url: String,
    pub api_key: String,
    pub is_local: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub resources_path: Option<PathBuf>,
    pub current_profile: Option<String>,
    pub profiles: Vec<ConnectionProfile>,
    pub language: Language,
    pub theme: String,
}

impl ConnectionProfile {
    fn new(name: String, ws_url: String, api_key: String) -> Self {
        let is_local = ws_url.contains("localhost") || ws_url.contains("127.0.0.1");
        Self {
            name,
            ws_url,
            api_key,
            is_local,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            resources_path: None,
            current_profile: None,
            profiles: vec![ConnectionProfile {
                name: "localhost".to_string(),
                ws_url: String::from("ws://localhost:3090"),
                api_key: String::new(),
                is_local: true,
            }],
            language: Language::English,
            theme: "dark".to_string(),
        }
    }
}

impl ServerConfig {
    pub fn add_profile(&mut self, name: String, ws_url: String, api_key: String) {
        self.profiles.push(ConnectionProfile::new(name, ws_url, api_key));
    }

    pub fn remove_profile(&mut self, name: &str) {
        if let Some(profile) = self.profiles.iter().find(|p| p.name == name) {
            if profile.is_local {
                return;
            }
        }
        self.profiles.retain(|p| p.name != name);
    }

    pub fn get_current_profile(&self) -> Option<&ConnectionProfile> {
        self.current_profile
            .as_ref()
            .and_then(|name| self.profiles.iter().find(|p| p.name == *name))
    }
}