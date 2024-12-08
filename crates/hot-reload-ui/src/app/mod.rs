mod config;
mod render;

use crate::utils::{generate_api_key, Translator};
use tokio::runtime::Runtime;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, error};
use chrono::Local;
use config::ServerConfig;
use eframe::egui::ImageSource;
use eframe::{egui, App, Theme};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::VecDeque;
use hot_reload_common::{AuthRequest, AuthResponse, InitialData, VersionChecker};

#[derive(Clone)]
struct Version {
    current: String,
    latest: String,
    message: String,
}

#[derive(Clone, PartialEq)]
enum ConnectionStatus {
    Disconnected,
    #[allow(dead_code)]
    Connecting,
    Connected,
    Error(String),
}

#[derive(Clone, PartialEq)]
enum ServerState {
    Disconnected,
    Connected,
    #[allow(dead_code)]
    Error(String),
}

pub struct HotReloadApp {
    config: ServerConfig,
    runtime: Arc<Runtime>,
    connection_status: Arc<Mutex<ConnectionStatus>>,
    server_status: Arc<Mutex<ServerState>>,
    resource_tree: Arc<Mutex<HashMap<String, Vec<String>>>>,
    resources_path: Arc<Mutex<Option<String>>>,
    show_add_profile_popup: bool,
    show_api_key_popup: bool,
    new_profile_name: String,
    new_profile_url: String,
    new_profile_api_key: String,
    show_ignored_files: bool,
    #[allow(dead_code)]
    show_hidden_files: bool,
    show_about_popup: bool,
    tree_state: ResourceTreeState,
    icons: Option<FileIcons>,
    translator: Translator,
    is_connected: bool,
    theme: Theme,
    logs: Arc<Mutex<VecDeque<String>>>,
    pending_messages: Arc<Mutex<Vec<String>>>,
    last_update: Arc<Mutex<std::time::Instant>>,
    version: Version,
}

#[derive(Default, Clone, Serialize, Debug)]
struct ResourceTreeState {
    expanded: HashMap<String, bool>,
    checked: HashMap<String, bool>,
}

struct FileIcons {
    lua: ImageSource<'static>,
    javascript: ImageSource<'static>,
    csharp: ImageSource<'static>,
    default: ImageSource<'static>,
}


#[derive(Debug, serde::Serialize)]
struct DebugResourceData {
    resources: HashMap<String, Vec<String>>,
    tree_state: ResourceTreeState,
    timestamp: String,
}

impl HotReloadApp {
    pub async fn new(cc: &eframe::CreationContext<'_>, runtime: Arc<Runtime>) -> Self {
        let connection_status = Arc::new(Mutex::new(ConnectionStatus::Disconnected));
        let resource_tree = Arc::new(Mutex::new(HashMap::new()));
        let resources_path = Arc::new(Mutex::new(None));
        let server_status = Arc::new(Mutex::new(ServerState::Disconnected));
        let mut translator = Translator::new();

        if let Ok(config_str) = std::fs::read_to_string("server_config.json") {
            let config: ServerConfig = serde_json::from_str(&config_str).unwrap_or_default();
            let _ = translator.set_language(config.language);
        }

        let current_version = env!("CARGO_PKG_VERSION").to_string();
        let version_checker_ui = VersionChecker::check_ui().await;      
        let version = match &version_checker_ui {
            Ok(v) => {
                let latest = v.get_latest().to_string();
                let message = if v.has_update(&current_version) {
                    tracing::info!("New version available: {}", latest);
                    translator.t_args(
                        "new_version_available",
                        &[("version", &latest)]
                    )
                } else {
                    tracing::info!("UI is up to date");
                    translator.t("ui_up_to_date").to_string()
                };
                Version {
                    current: current_version,
                    latest,
                    message,
                }
            },
            Err(e) => {
                tracing::warn!("Failed to check UI version: {}", e);
                Version {
                    current: current_version.clone(),
                    latest: current_version,
                    message: translator.t_args(
                        "failed_check_updates",
                        &[("error", &e.to_string())]
                    ),
                }
            }
        };

        egui_extras::install_image_loaders(&cc.egui_ctx);

        let icons = FileIcons {
            lua: ImageSource::Bytes {
                uri: "lua.svg".into(),
                bytes: include_bytes!("../../assets/lua.svg").as_ref().into(),
            },
            javascript: ImageSource::Bytes {
                uri: "js.svg".into(),
                bytes: include_bytes!("../../assets/js.svg").as_ref().into(),
            },
            csharp: ImageSource::Bytes {
                uri: "csharp.svg".into(),
                bytes: include_bytes!("../../assets/csharp.svg").as_ref().into(),
            },
            default: ImageSource::Bytes {
                uri: "file.svg".into(),
                bytes: include_bytes!("../../assets/file.svg").as_ref().into(),
            },
        };

        let mut app = Self {
            config: ServerConfig::default(),
            runtime,
            connection_status,
            server_status,
            resource_tree,
            resources_path,
            show_add_profile_popup: false,
            show_api_key_popup: false,
            new_profile_name: String::new(),
            new_profile_url: String::new(),
            new_profile_api_key: String::new(),
            show_ignored_files: false,
            show_hidden_files: false,
            show_about_popup: false,
            tree_state: ResourceTreeState::default(),
            icons: Some(icons),
            translator,
            theme: Theme::Dark,
            is_connected: false,
            logs: Arc::new(Mutex::new(VecDeque::with_capacity(100))),
            pending_messages: Arc::new(Mutex::new(Vec::with_capacity(100))),
            last_update: Arc::new(Mutex::new(std::time::Instant::now())),
            version,
        };

        if let Ok(config_str) = std::fs::read_to_string("server_config.json") {
            app.config = serde_json::from_str(&config_str).unwrap_or_default();
            app.theme = match app.config.theme.as_str() {
                "light" => {
                    cc.egui_ctx.set_visuals(egui::Visuals::light());
                    Theme::Light
                },
                "dark" | _ => {
                    cc.egui_ctx.set_visuals(egui::Visuals::dark());
                    Theme::Dark
                },
            };
        }

        app
    }

    fn save_config(&self) {
        if let Ok(config_str) = serde_json::to_string_pretty(&self.config) {
            let _ = std::fs::write("server_config.json", config_str);
        }
    }

    fn set_is_connected(&mut self, is_connected: bool) {
        self.is_connected = is_connected;
    }

    fn start_websocket(&mut self, ws_url: String, api_key: Option<String>) {
        let rt = self.runtime.clone();
        let status = self.connection_status.clone();
        let resource_tree = self.resource_tree.clone();
        let resources_path = self.resources_path.clone();
        let logs = self.logs.clone();
        let pending_messages = self.pending_messages.clone();
        let server_status = self.server_status.clone();

        rt.spawn(async move {
            info!("🔌 Tentative de connexion à {}", ws_url);
            match connect_async(&ws_url).await {
                Ok((mut ws_stream, _)) => {
                    if let Some(key) = api_key {
                        let auth = AuthRequest { api_key: key };
                        if let Ok(auth_msg) = serde_json::to_string(&auth) {
                            if let Err(e) = ws_stream.send(Message::Text(auth_msg)).await {
                                error!("❌ Authentication error: {}", e);
                                if let Ok(mut status) = status.lock() {
                                    *status = ConnectionStatus::Error(e.to_string());
                                }
                                return;
                            }

                            match ws_stream.next().await {
                                Some(Ok(response)) => {
                                    if let Ok(auth_response) = serde_json::from_str::<AuthResponse>(&response.to_string()) {
                                        match auth_response {
                                            AuthResponse::Failed(reason) => {
                                                if let Ok(mut status) = status.lock() {
                                                    *status = ConnectionStatus::Error(reason);
                                                }
                                                return;
                                            }
                                            AuthResponse::Success => {
                                                info!("✅ Authentication successful");
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    error!("❌ Pas de réponse d'authentification");
                                    return;
                                }
                            }
                        }
                    }

                    // if let Ok(mut server_status) = server_status.lock() {
                    //     *server_status = ServerState::Connected;
                    // }

                    info!("📡 WebSocket connection established");

                    if let Ok(mut status) = status.lock() {
                        *status = ConnectionStatus::Connected;
                    }

                    // Boucle de réception des messages
                    while let Some(msg) = ws_stream.next().await {
                        match msg {
                            Ok(msg) => {
                                if let Ok(text) = msg.to_text() {
                                    info!("📨 Message brut reçu: {}", text);
                                    
                                    // Essayer de parser en tant que données initiales
                                    match serde_json::from_str::<InitialData>(text) {
                                        Ok(initial_data) => {
                                            info!("📥 Initial data received successfully");
                                            info!("📂 Resources path: {}", initial_data.resources_path);
                                            info!("📚 Number of resources: {}", initial_data.resources.len());
                                            Self::handle_initial_data(&resources_path, &resource_tree, initial_data).await;
                                            continue;
                                        }
                                        Err(_) => {
                                            match serde_json::from_str::<serde_json::Value>(text) {
                                                Ok(json) => {
                                                    match json.get("type").and_then(|t| t.as_str()) {
                                                        Some("server_status") => {
                                                            info!("🔍 Status reçu: {}", json["status"]);
                                                            if let Ok(mut server_status) = server_status.lock() {
                                                                *server_status = match json["status"].as_str() {
                                                                    Some("online") => ServerState::Connected,
                                                                    Some("offline") => ServerState::Disconnected,
                                                                    _ => ServerState::Error(json["status"].as_str().unwrap_or("unknown").to_string()),
                                                                };
                                                            }
                                                        }
                                                        Some("fx_resource_action") => {
                                                            info!("🔍 Action reçue: {}", json["action"]);
                                                        }
                                                        _ => {}
                                                    }

                                                    // info!("🔍 Type de message reçu: {}", json["type"]);
                                                    // if json["type"] == "batch" {
                                                    //     let batch_type = json["type"].as_str().unwrap_or("unknown");
                                                    //     info!("🔍 Type de batch reçu: {}", batch_type);
                                                    //     match batch_type {
                                                    //         "server_status" => {
                                                    //             if let Some(status) = json["status"].as_str() {
                                                    //                 info!("🔍 Status reçu: {}", status);
                                                    //                 if let Ok(mut server_status) = server_status.lock() {
                                                    //                     *server_status = match status {
                                                    //                         "online" => ServerState::Connected,
                                                    //                         "offline" => ServerState::Disconnected,
                                                    //                         _ => ServerState::Error(status.to_string()),
                                                    //                     };
                                                    //                 }
                                                    //             }
                                                    //         }
                                                    //         "fx_resource_action" => {
                                                    //             json["action"].as_str().map(|action| {
                                                    //                 info!("🔍 Action reçue: {}", action);
                                                    //             });
                                                    //         }
                                                    //         _ => {}
                                                    //     }

                                                    //     if let Some(messages) = json["messages"].as_array() {
                                                    //         info!("📦 Batch received with {} messages", messages.len());
                                                    //         let message_strings: Vec<String> = messages.iter()
                                                    //             .filter_map(|msg| msg["message"].as_str().map(String::from))
                                                    //             .collect();
                                                    //         Self::process_message_batch(&message_strings, &logs, &pending_messages, &server_status).await;
                                                    //     }
                                                    // }
                                                }
                                                Err(e) => {
                                                    error!("❌ JSON parsing error: {}", e);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("❌ WebSocket error: {}", e);
                                if let Ok(mut status) = status.lock() {
                                    *status = ConnectionStatus::Error(e.to_string());
                                }
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("❌ Connection error: {}", e);
                    if let Ok(mut status) = status.lock() {
                        *status = ConnectionStatus::Error(e.to_string());
                    }
                }
            }
        });
    }

    async fn handle_initial_data(
        resources_path: &Arc<Mutex<Option<String>>>,
        resource_tree: &Arc<Mutex<HashMap<String, Vec<String>>>>,
        initial_data: InitialData,
    ) {
        info!("🔄 Processing initial data");
        if let Ok(mut path) = resources_path.lock() {
            *path = Some(initial_data.resources_path.clone());
            info!("📂 Resources path updated: {}", initial_data.resources_path);
        }
        if let Ok(mut tree) = resource_tree.lock() {
            *tree = initial_data.resources.clone();
            info!("🌳 Resources tree updated with {} resources", tree.len());
        }
    }

    async fn process_message_batch(
        messages: &[String],
        logs: &Arc<Mutex<VecDeque<String>>>,
        pending: &Arc<Mutex<Vec<String>>>,
        server_status: &Arc<Mutex<ServerState>>,
    ) {
        info!("🔄 Processing a batch of {} messages", messages.len());
        if let Ok(mut pending_messages) = pending.lock() {
            for message in messages {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(message) {
                    match json.get("type").and_then(|t| t.as_str()) {
                        Some("fx_response") => {
                            if let Some(message) = json.get("message").and_then(|m| m.as_str()) {
                                pending_messages.push(message.to_string());
                                info!("📨 Message added to pending messages: {}", message);
                                
                                if let Ok(mut logs) = logs.lock() {
                                    if logs.len() >= 100 {
                                        logs.pop_front();
                                    }
                                    logs.push_back(message.to_string());
                                    info!("📝 Message added to logs");
                                }
                            }
                        },
                        Some("server_status") => {
                            if let Some(status) = json.get("status").and_then(|s| s.as_str()) {
                                if let Ok(mut current_status) = server_status.lock() {
                                    *current_status = match status {
                                        "online" => ServerState::Connected,
                                        "offline" => ServerState::Disconnected,
                                        _ => ServerState::Error(status.to_string()),
                                    };
                                    info!("🔄 Server status updated: {}", status);
                                }
                            }
                        },
                        _ => {}
                    }
                }
            }
        }
    }

    // Propager l'état de vérification aux fichiers et sous-dossiers ( utile plus tard à finir [checkbox hot-reload])
    #[allow(dead_code)]
    fn propagate_check_state(&mut self, parent: &str, checked: bool) {
        // Collecter les données nécessaires d'abord
        let (files, subfolders) = if let Ok(tree) = self.resource_tree.lock() {
            if let Some(files) = tree.get(parent) {
                (files.clone(), tree.keys().cloned().collect::<Vec<_>>())
            } else {
                (Vec::new(), Vec::new())
            }
        } else {
            (Vec::new(), Vec::new())
        };

        // Propager aux fichiers
        for file in files {
            let file_id = format!("{}/{}", parent, file);
            self.tree_state.checked.insert(file_id, checked);
        }

        // Propager aux sous-dossiers
        for subfolder_name in subfolders {
            let subfolder_id = format!("{}/{}", parent, subfolder_name);
            self.tree_state.checked.insert(subfolder_id.clone(), checked);
            self.propagate_check_state(&subfolder_id, checked);
        }
    }

    #[allow(dead_code)]
    fn update_parent_state(&mut self, parent: &str) {
        if let Ok(tree) = self.resource_tree.lock() {
            if let Some(files) = tree.get(parent) {
                let all_checked = files.iter().all(|file| {
                    let file_id = format!("{}/{}", parent, file);
                    self.tree_state.checked.get(&file_id).copied().unwrap_or(true)
                }) && tree.iter().all(|(name, _)| {
                    let subfolder_id = format!("{}/{}", parent, name);
                    self.tree_state.checked.get(&subfolder_id).copied().unwrap_or(true)
                });
                self.tree_state.checked.insert(parent.to_string(), all_checked);
            }
        }
    }

    fn all_checked(&self) -> bool {
        self.tree_state.checked.values().all(|&checked| checked)
    }

    fn toggle_all_resources(&mut self) {
        let new_state = !self.all_checked();
        if let Ok(tree) = self.resource_tree.lock() {
            for (resource_name, files) in tree.iter() {
                self.tree_state.checked.insert(resource_name.clone(), new_state);
                for file in files {
                    let file_id = format!("{}/{}", resource_name, file);
                    self.tree_state.checked.insert(file_id, new_state);
                }
            }
        }
    }

    fn debug_dump_resources(&self) {
        if let Ok(tree) = self.resource_tree.lock() {
            let debug_data = DebugResourceData {
                resources: tree.clone(),
                tree_state: self.tree_state.clone(),
                timestamp: Local::now().to_rfc3339(),
            };

            if let Ok(json) = serde_json::to_string_pretty(&debug_data) {
                let debug_file = format!(
                    "debug_resources_{}.json",
                    Local::now().format("%Y%m%d_%H%M%S")
                );
                std::fs::write(&debug_file, json).unwrap_or_else(|_| {
                    info!("{}", self.translator.t("error_debug_resources"));
                });
                info!("{}", self.translator.t("debug_resources_dumped"));
            }
        }
    }
}

impl App for HotReloadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render(ctx);
    }
}
