mod ui;
mod config;

use config::ServerConfig;
use futures_util::{SinkExt, StreamExt};
use tokio::runtime::Runtime;
use hot_reload_common::{ResourceChange, ChangeType, InitialData, ResourceCategory, AuthRequest, AuthResponse};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use eframe::App;
use tracing::info;
use std::sync::Mutex;
use std::sync::Arc;

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
    resource_categories: Arc<Mutex<ResourceCategory>>,
    resources_path: Arc<Mutex<Option<String>>>,
    show_add_profile_popup: bool,
    new_profile_name: String,
    new_profile_url: String,
    new_profile_api_key: String,
}

impl HotReloadApp {
    pub fn new(_: &eframe::CreationContext<'_>) -> Self {
        let runtime = Arc::new(Runtime::new().expect("Erreur lors de la création du runtime!"));
        let connection_status = Arc::new(Mutex::new(ConnectionStatus::Disconnected));
        let resource_categories = Arc::new(Mutex::new(ResourceCategory::new()));
        let resources_path = Arc::new(Mutex::new(None));
        
        let mut app = Self {
            config: ServerConfig::default(),
            runtime,
            connection_status,
            resource_categories,
            resources_path,
            show_add_profile_popup: false,
            new_profile_name: String::new(),
            new_profile_url: String::new(),
            new_profile_api_key: String::new(),
        };

        if let Ok(config_str) = std::fs::read_to_string("server_config.json") {
            app.config = serde_json::from_str(&config_str).unwrap_or_default();
        }

        app
    }

    fn save_config(&self) {
        if let Ok(config_str) = serde_json::to_string_pretty(&self.config) {
            let _ = std::fs::write("server_config.json", config_str);
        }
    }

    fn start_websocket(&self, ws_url: String, api_key: Option<String>) {
        let rt = self.runtime.clone();
        let status = self.connection_status.clone();
        let resource_categories = self.resource_categories.clone();
        let resources_path = self.resources_path.clone();

        rt.spawn(async move {
            match connect_async(&ws_url).await {
                Ok((mut ws_stream, _)) => {
                    // Vérifier si c'est une connexion distante
                    let is_localhost = ws_url.contains("localhost") || ws_url.contains("127.0.0.1");
                    
                    if !is_localhost {
                        // Envoyer l'authentification
                        if let Some(key) = api_key {
                            let auth = AuthRequest { api_key: key };
                            if let Ok(auth_msg) = serde_json::to_string(&auth) {
                                if let Err(_) = ws_stream.send(Message::Text(auth_msg)).await {
                                    *status.lock().unwrap() = ConnectionStatus::Error("Erreur d'authentification".to_string());
                                    return;
                                }
                                
                                // Attendre la réponse
                                match ws_stream.next().await {
                                    Some(Ok(msg)) => {
                                        if let Ok(response) = serde_json::from_str::<AuthResponse>(&msg.to_string()) {
                                            match response {
                                                AuthResponse::Failed(reason) => {
                                                    *status.lock().unwrap() = ConnectionStatus::Error(reason);
                                                    return;
                                                }
                                                AuthResponse::Success => {
                                                    info!("✅ Authentification réussie!");
                                                }
                                            }
                                        }
                                    }
                                    _ => {
                                        *status.lock().unwrap() = ConnectionStatus::Error("Erreur de réponse d'authentification".to_string());
                                        return;
                                    }
                                }
                            }
                        } else {
                            *status.lock().unwrap() = ConnectionStatus::Error("Clé API requise pour les connexions distantes".to_string());
                            return;
                        }
                    }

                    *status.lock().unwrap() = ConnectionStatus::Connected;
                    println!("✅ Connecté au WebSocket!");

                    while let Some(msg) = ws_stream.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(initial_data) = serde_json::from_str::<InitialData>(&text) {
                                    println!("📥 Données initiales reçues");
                                    *resources_path.lock().unwrap() = Some(initial_data.resources_path);
                                    *resource_categories.lock().unwrap() = initial_data.categories;
                                } else if let Ok(change) = serde_json::from_str::<ResourceChange>(&text) {
                                    println!("📥 Changement reçu: {:?}", change);
                                    // self.handle_change(change); // ~l.165
                                }
                            }
                            Ok(_) => {}
                            Err(e) => {
                                println!("❌ Erreur WebSocket: {}", e);
                                *status.lock().unwrap() = ConnectionStatus::Error(e.to_string());
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Erreur de connexion WebSocket: {}", e);
                    *status.lock().unwrap() = ConnectionStatus::Error(e.to_string());
                }
            }
        });
    }

    #[allow(dead_code)] // Todo: Implementer le traitement des changements (catégories, fichiers, dossiers etc...)
    fn handle_change(&mut self, change: ResourceChange) {
        // Exemple de traitement : mettre à jour l'arborescence des fichiers
        if let Ok(mut resource_categories) = self.resource_categories.lock() {
            if let Some(_) = resource_categories.categories.get_mut(&change.resource_name) {
                match change.change_type {
                    ChangeType::FileAdded => {
                        todo!()
                    }
                    _ => {}
                }
            }
        }
    }

}

impl App for HotReloadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Sélection du profil
                ui.label("Profil:");
                let current_profile = self.config.current_profile.clone();
                egui::ComboBox::from_label("")
                    .selected_text(self.config.get_current_profile()
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| "Sélectionner un profil".to_string()))
                    .show_ui(ui, |ui| {
                        for profile in &self.config.profiles {
                            ui.selectable_value(
                                &mut self.config.current_profile,
                                Some(profile.name.clone()),
                                &profile.name
                            );
                        }
                    });

                // Bouton pour ajouter un nouveau profil
                if ui.button("➕").clicked() {
                    self.show_add_profile_popup = true;
                }

                // Bouton pour supprimer le profil actuel (seulement si non local)
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

                // Affichage et édition de l'URL WebSocket et API Key
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
                                .hint_text("Entrez votre clé API")
                                .desired_width(200.0);
                            
                            api_changed = ui.add(api_key_edit).changed();
                        }

                        // Appliquer les changements après les contrôles UI
                        if ws_changed || api_changed {
                            if let Some(profile) = self.config.profiles.iter_mut()
                                .find(|p| p.name == *current_name) {
                                if ws_changed {
                                    profile.ws_url = ws_url.clone();
                                }
                                if api_changed {
                                    profile.api_key = api_key.clone();
                                }
                                self.save_config();
                            }
                        }

                        if ui.button("🔌 Connecter").clicked() {
                            let api_key = if !is_local { Some(api_key.clone()) } else { None };
                            self.start_websocket(ws_url.clone(), api_key);
                        }

                        // Affichage du statut de connexion
                        let status = self.connection_status.lock().unwrap().clone();
                        match status {
                            ConnectionStatus::Disconnected => {
                                ui.label(egui::RichText::new("⚫ Déconnecté").color(egui::Color32::GRAY));
                            },
                            ConnectionStatus::Connecting => {
                                ui.label(egui::RichText::new("🔄 Connexion en cours...").color(egui::Color32::YELLOW));
                            },
                            ConnectionStatus::Connected => {
                                ui.label(egui::RichText::new("🟢 Connecté").color(egui::Color32::GREEN));
                            },
                            ConnectionStatus::Error(err) => {
                                ui.label(egui::RichText::new(format!("🔴 Erreur: {}", err)).color(egui::Color32::RED));
                            },
                        }
                    }
                }
            });
        });

        // Popup pour ajouter un nouveau profil
        if self.show_add_profile_popup {
            egui::Window::new("Nouveau Profil")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Nom:");
                        ui.text_edit_singleline(&mut self.new_profile_name);
                    });
                    ui.horizontal(|ui| {
                        ui.label("URL:");
                        ui.text_edit_singleline(&mut self.new_profile_url);
                    });
                    ui.horizontal(|ui| {
                        ui.label("API Key:");
                        ui.add(egui::TextEdit::singleline(&mut self.new_profile_api_key)
                            .password(true)
                            .hint_text("Entrez votre clé API"));
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Annuler").clicked() {
                            self.show_add_profile_popup = false;
                            self.new_profile_name.clear();
                            self.new_profile_url.clear();
                            self.new_profile_api_key.clear();
                        }
                        if ui.button("Ajouter").clicked() {
                            if !self.new_profile_name.is_empty() && !self.new_profile_url.is_empty() {
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

        // Afficher le contenu principal
        self.render_main_content(ctx);
    }
}