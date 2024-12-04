mod config;
mod ui;

use crate::utils::{generate_api_key, Translator};
use tokio::runtime::Runtime;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::info;
use chrono::Local;
use config::ServerConfig;
use eframe::egui::ImageSource;
use eframe::{egui, App, Theme};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use hot_reload_common::{AuthRequest, AuthResponse, ChangeType, InitialData, ResourceChange};

#[derive(Clone, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    #[allow(dead_code)]
    Connecting,
    Connected,
    Error(String),
}

pub struct HotReloadApp {
    config: ServerConfig,
    runtime: Arc<Runtime>,
    connection_status: Arc<Mutex<ConnectionStatus>>,
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
}

#[derive(Default, Clone, Serialize, Debug)]
struct ResourceTreeState {
    expanded: HashMap<String, bool>,
    checked: HashMap<String, bool>,
}

pub struct FileIcons {
    lua: ImageSource<'static>,
    javascript: ImageSource<'static>,
    csharp: ImageSource<'static>,
    default: ImageSource<'static>,
}

impl HotReloadApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let runtime = Arc::new(Runtime::new().expect("Erreur lors de la création du runtime!"));
        let connection_status = Arc::new(Mutex::new(ConnectionStatus::Disconnected));
        let resource_tree = Arc::new(Mutex::new(HashMap::new()));
        let resources_path = Arc::new(Mutex::new(None));
        let mut translator = Translator::new();

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

        if let Ok(config_str) = std::fs::read_to_string("server_config.json") {
            let config: ServerConfig = serde_json::from_str(&config_str).unwrap_or_default();
            let _ = translator.set_language(config.language);
        }

        let mut app = Self {
            config: ServerConfig::default(),
            runtime,
            connection_status,
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
        let tr = self.translator.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let connected_message = tr.t("connected_websocket").to_string();

        rt.spawn(async move {
            match connect_async(&ws_url).await {
                Ok((mut ws_stream, _)) => {
                    let is_localhost = ws_url.contains("localhost") || ws_url.contains("127.0.0.1");

                    if !is_localhost {
                        if let Some(key) = api_key {
                            let auth = AuthRequest { api_key: key };
                            if let Ok(auth_msg) = serde_json::to_string(&auth) {
                                if let Err(_) = ws_stream.send(Message::Text(auth_msg)).await {
                                    *status.lock().unwrap() =
                                        ConnectionStatus::Error(tr.t("error_auth"));
                                    return;
                                }

                                match ws_stream.next().await {
                                    Some(Ok(msg)) => {
                                        if let Ok(response) =
                                            serde_json::from_str::<AuthResponse>(&msg.to_string())
                                        {
                                            match response {
                                                AuthResponse::Failed(reason) => {
                                                    *status.lock().unwrap() =
                                                        ConnectionStatus::Error(reason);
                                                    return;
                                                }
                                                AuthResponse::Success => {
                                                    info!("{}", tr.t("success_auth"));
                                                }
                                            }
                                        }
                                    }
                                    _ => {
                                        *status.lock().unwrap() =
                                            ConnectionStatus::Error(tr.t("error_auth_response"));
                                        return;
                                    }
                                }
                            }
                        } else {
                            *status.lock().unwrap() =
                                ConnectionStatus::Error(tr.t("error_auth_api_key"));
                            return;
                        }
                    }

                    *status.lock().unwrap() = ConnectionStatus::Connected;
                    let _ = tx.send(true); // Envoyer l'état connecté
                    info!("{}", connected_message);

                    while let Some(msg) = ws_stream.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(initial_data) = serde_json::from_str::<InitialData>(&text)
                                {
                                    info!("{}", tr.t("init_data_received"));
                                    *resources_path.lock().unwrap() =
                                        Some(initial_data.resources_path);
                                    let mut sorted_resources: Vec<(String, Vec<String>)> =
                                        initial_data.resources.into_iter().collect();
                                    sorted_resources.sort_by(|a, b| {
                                        let a_name =
                                            a.0.trim_start_matches(|c: char| !c.is_alphanumeric());
                                        let b_name =
                                            b.0.trim_start_matches(|c: char| !c.is_alphanumeric());
                                        a_name.to_lowercase().cmp(&b_name.to_lowercase())
                                    });
                                    let sorted_map: HashMap<String, Vec<String>> = sorted_resources
                                        .into_iter()
                                        .map(|(name, mut files)| {
                                            // Trier les fichiers: d'abord les fichiers racine, puis par dossier
                                            files.sort_by(|a, b| {
                                                let a_parts: Vec<&str> =
                                                    a.split(&['/', '\\']).collect();
                                                let b_parts: Vec<&str> =
                                                    b.split(&['/', '\\']).collect();

                                                match (a_parts.len(), b_parts.len()) {
                                                    (1, 1) => {
                                                        a.to_lowercase().cmp(&b.to_lowercase())
                                                    }
                                                    (1, _) => std::cmp::Ordering::Less,
                                                    (_, 1) => std::cmp::Ordering::Greater,
                                                    (_, _) => {
                                                        if a_parts[0] == b_parts[0] {
                                                            a.to_lowercase().cmp(&b.to_lowercase())
                                                        } else {
                                                            a_parts[0]
                                                                .to_lowercase()
                                                                .cmp(&b_parts[0].to_lowercase())
                                                        }
                                                    }
                                                }
                                            });
                                            (name, files)
                                        })
                                        .collect();

                                    *resource_tree.lock().unwrap() = sorted_map;
                                } else if let Ok(change) =
                                    serde_json::from_str::<ResourceChange>(&text)
                                {
                                    info!("{}", tr.t("change_received"));
                                    if let Ok(mut tree) = resource_tree.lock() {
                                        match change.change_type {
                                            ChangeType::FileAdded | ChangeType::FileModified => {
                                                if let Some(files) =
                                                    tree.get_mut(&change.resource_name)
                                                {
                                                    if !files.contains(&change.file_path) {
                                                        files.push(change.file_path);
                                                        files.sort_by(|a, b| {
                                                            let a_parts: Vec<&str> =
                                                                a.split(&['/', '\\']).collect();
                                                            let b_parts: Vec<&str> =
                                                                b.split(&['/', '\\']).collect();

                                                            match (a_parts.len(), b_parts.len()) {
                                                                (1, 1) => a
                                                                    .to_lowercase()
                                                                    .cmp(&b.to_lowercase()),
                                                                (1, _) => std::cmp::Ordering::Less,
                                                                (_, 1) => {
                                                                    std::cmp::Ordering::Greater
                                                                }
                                                                (_, _) => {
                                                                    if a_parts[0] == b_parts[0] {
                                                                        a.to_lowercase()
                                                                            .cmp(&b.to_lowercase())
                                                                    } else {
                                                                        a_parts[0]
                                                                            .to_lowercase()
                                                                            .cmp(
                                                                                &b_parts[0]
                                                                                    .to_lowercase(),
                                                                            )
                                                                    }
                                                                }
                                                            }
                                                        });
                                                    }
                                                }
                                            }
                                            ChangeType::FileRemoved => {
                                                if let Some(files) =
                                                    tree.get_mut(&change.resource_name)
                                                {
                                                    files.retain(|f| f != &change.file_path);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Ok(_) => {}
                            Err(e) => {
                                info!("{}", tr.t("error_websocket"));
                                *status.lock().unwrap() = ConnectionStatus::Error(e.to_string());
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    info!("{}", tr.t("error_websocket_connection"));
                    *status.lock().unwrap() = ConnectionStatus::Error(e.to_string());
                }
            }
        });

        // Mettre à jour l'état de connexion en dehors de la closure async
        if let Ok(is_connected) = rx.blocking_recv() {
            self.set_is_connected(is_connected);
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
            self.tree_state
                .checked
                .insert(subfolder_id.clone(), checked);
            self.propagate_check_state(&subfolder_id, checked);
        }
    }

    #[allow(dead_code)]
    fn update_parent_state(&mut self, parent: &str) {
        if let Ok(tree) = self.resource_tree.lock() {
            if let Some(files) = tree.get(parent) {
                let all_checked = files.iter().all(|file| {
                    let file_id = format!("{}/{}", parent, file);
                    self.tree_state
                        .checked
                        .get(&file_id)
                        .copied()
                        .unwrap_or(true)
                }) && tree.iter().all(|(name, _)| {
                    let subfolder_id = format!("{}/{}", parent, name);
                    self.tree_state
                        .checked
                        .get(&subfolder_id)
                        .copied()
                        .unwrap_or(true)
                });
                self.tree_state
                    .checked
                    .insert(parent.to_string(), all_checked);
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
                self.tree_state
                    .checked
                    .insert(resource_name.clone(), new_state);
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

#[derive(Debug, serde::Serialize)]
struct DebugResourceData {
    resources: HashMap<String, Vec<String>>,
    tree_state: ResourceTreeState,
    timestamp: String,
}

impl App for HotReloadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Menu strip on top
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button(self.translator.t("folder"), |ui| {
                    if ui.button(self.translator.t("reload_config")).clicked() {
                        if let Ok(config_str) = std::fs::read_to_string("server_config.json") {
                            self.config = serde_json::from_str(&config_str).unwrap_or_default();
                            ui.close_menu();
                        }
                    }
                    if ui.button(self.translator.t("save")).clicked() {
                        self.save_config();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(self.translator.t("quit")).clicked() {
                        std::process::exit(0);
                    }
                });

                ui.menu_button(self.translator.t("tools"), |ui| {
                    ui.separator();
                    if ui.button(self.translator.t("generate_api_key")).clicked() {
                        let api_key = generate_api_key();
                        self.new_profile_api_key = api_key;
                        self.show_api_key_popup = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button(self.translator.t("affichage"), |ui| {
                    ui.checkbox(
                        &mut self.show_ignored_files,
                        self.translator.t("ignore_files"),
                    );
                    ui.separator();
                    ui.menu_button(self.translator.t("theme"), |ui| {
                        let current_theme = self.config.theme.clone();
                        
                        if ui.selectable_label(current_theme == "light", self.translator.t("light")).clicked() {
                            ctx.set_visuals(egui::Visuals::light());
                            self.config.theme = "light".to_string();
                            if let Ok(config_json) = serde_json::to_string_pretty(&self.config) {
                                let _ = std::fs::write("server_config.json", config_json);
                            }
                            ui.close_menu();
                        }
                        
                        if ui.selectable_label(current_theme == "dark", self.translator.t("dark")).clicked() {
                            ctx.set_visuals(egui::Visuals::dark());
                            self.config.theme = "dark".to_string();
                            if let Ok(config_json) = serde_json::to_string_pretty(&self.config) {
                                let _ = std::fs::write("server_config.json", config_json);
                            }
                            ui.close_menu();
                        }
                    });
                    ui.separator();
                    ui.menu_button(self.translator.t("language"), |ui| {
                        for language in Translator::available_languages() {
                            let is_selected = self.translator.get_language() == language;
                            if ui.selectable_label(is_selected, format!("{:?}", language)).clicked() {
                                self.translator.set_language(language).unwrap_or_default();
                                self.config.language = language;
                                if let Ok(config_json) = serde_json::to_string_pretty(&self.config) {
                                    let _ = std::fs::write("server_config.json", config_json);
                                }
                                ui.close_menu();
                            }
                        }
                    });
                });

                ui.menu_button(self.translator.t("help"), |ui| {
                    if ui.button(self.translator.t("documentation")).clicked() {
                        // TODO: Ouvrir la doc
                        ui.close_menu();
                    }
                    if ui.button(self.translator.t("report_bug")).clicked() {
                        // TODO: Ouvrir GitHub Issues
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(self.translator.t("about")).clicked() {
                        self.show_about_popup = true;
                        ui.close_menu();
                    }
                });
            });
        });

        // Task bar on top
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(self.translator.t("profile"));
                let current_profile = self.config.current_profile.clone();
                egui::ComboBox::from_label("")
                    .selected_text(
                        self.config
                            .get_current_profile()
                            .map(|p| p.name.clone())
                            .unwrap_or_else(|| self.translator.t("profile_select")),
                    )
                    .show_ui(ui, |ui| {
                        for profile in &self.config.profiles {
                            ui.selectable_value(
                                &mut self.config.current_profile,
                                Some(profile.name.clone()),
                                &profile.name,
                            );
                        }
                    });
                if ui.button("➕").clicked() {
                    self.show_add_profile_popup = true;
                }
                if let Some(current) = &self.config.current_profile {
                    if let Some(profile) = self.config.get_current_profile() {
                        if !profile.is_local {
                            if ui.button("🗑").clicked() {
                                let mut config = self.config.clone();
                                config.remove_profile(current);
                                self.config = config;
                                self.config.current_profile = None;
                                self.save_config();
                            }
                        }
                    }
                }
                ui.separator();
                if let Some(current_name) = &current_profile {
                    if let Some(profile) = self.config.get_current_profile() {
                        let mut ws_url = profile.ws_url.clone();
                        let mut api_key = profile.api_key.clone();
                        let is_local = profile.is_local;

                        ui.label("WebSocket URL:");
                        let text_edit = egui::TextEdit::singleline(&mut ws_url)
                            .hint_text("ws://ip:port")
                            .desired_width(200.0);

                        let ws_changed = ui.add(text_edit).changed();
                        let mut api_changed = false;

                        if !is_local {
                            ui.label("API Key:");
                            let api_key_edit = egui::TextEdit::singleline(&mut api_key)
                                .password(true)
                                .hint_text(self.translator.t("profile_api_key_placeholder"))
                                .desired_width(200.0);

                            api_changed = ui.add(api_key_edit).changed();
                        }

                        if ws_changed || api_changed {
                            if let Some(profile) = self
                                .config
                                .profiles
                                .iter_mut()
                                .find(|p| p.name == *current_name)
                            {
                                if ws_changed {
                                    profile.ws_url = ws_url.clone();
                                }
                                if api_changed {
                                    profile.api_key = api_key.clone();
                                }
                                self.save_config();
                            }
                        }

                        if ui.button(self.translator.t("login")).clicked() {
                            let api_key = if !is_local {
                                Some(api_key.clone())
                            } else {
                                None
                            };
                            self.start_websocket(ws_url.clone(), api_key);
                        }

                        let status = self.connection_status.lock().unwrap().clone();
                        match status {
                            ConnectionStatus::Disconnected => {
                                ui.label(
                                    egui::RichText::new(self.translator.t("disconnected"))
                                        .color(egui::Color32::GRAY),
                                );
                            }
                            ConnectionStatus::Connecting => {
                                ui.label(
                                    egui::RichText::new(self.translator.t("connecting"))
                                        .color(egui::Color32::YELLOW),
                                );
                            }
                            ConnectionStatus::Connected => {
                                ui.label(
                                    egui::RichText::new(self.translator.t("connected"))
                                        .color(egui::Color32::GREEN),
                                );
                            }
                            ConnectionStatus::Error(err) => {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}: {}",
                                        self.translator.t("error_connection"),
                                        err
                                    ))
                                    .color(egui::Color32::RED),
                                );
                            }
                        }
                    }
                }
            });
        });

        if self.show_add_profile_popup {
            egui::Window::new(self.translator.t("profile_new"))
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(self.translator.t("name"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_profile_name)
                                .hint_text(self.translator.t("profile_name_placeholder"))
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("URL:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_profile_url)
                                .hint_text(self.translator.t("profile_url_placeholder"))
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("API Key:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_profile_api_key)
                                .password(true)
                                .hint_text(self.translator.t("profile_api_key_placeholder")),
                        );
                    });
                    ui.horizontal(|ui| {
                        if ui.button(self.translator.t("cancel")).clicked() {
                            self.show_add_profile_popup = false;
                            self.new_profile_name.clear();
                            self.new_profile_url.clear();
                            self.new_profile_api_key.clear();
                        }
                        if ui.button(self.translator.t("add")).clicked() {
                            if !self.new_profile_name.is_empty() && !self.new_profile_url.is_empty()
                            {
                                self.config.add_profile(
                                    self.new_profile_name.clone(),
                                    self.new_profile_url.clone(),
                                    self.new_profile_api_key.clone(),
                                );
                                self.config.current_profile = Some(self.new_profile_name.clone());
                                self.save_config();
                                self.show_add_profile_popup = false;
                                self.new_profile_name.clear();
                                self.new_profile_url.clear();
                                self.new_profile_api_key.clear();
                            }
                        }
                    });
                });
        }

        if self.show_api_key_popup {
            egui::Window::new("API Key")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("API Key :");
                        ui.monospace(&self.new_profile_api_key);
                        if ui.small_button("📋").clicked() {
                            ui.ctx().copy_text(self.new_profile_api_key.clone().into());
                        }
                    });
                    if ui.button(self.translator.t("close")).clicked() {
                        self.show_api_key_popup = false;
                    }
                });
        }

        // Display main content
        self.render_main_content(ctx);
    }
}
