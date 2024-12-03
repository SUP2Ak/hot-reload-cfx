mod watcher;
use crate::watcher::ResourceWatcher;
use eframe::egui;
use image;
use tokio::runtime::Runtime;
use serde::{Serialize, Deserialize};
use walkdir::WalkDir;
use std::sync::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone)]
struct ServerConfig {
    resources_path: Option<PathBuf>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            resources_path: None,
        }
    }
}

#[derive(Clone)]
enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

#[derive(Default)]
struct FileTree {
    folders: HashMap<String, FileTree>,
    files: Vec<String>,
}

impl FileTree {
    fn insert(&mut self, path: &[String], file: String) {
        if path.is_empty() {
            self.files.push(file);
            self.files.sort();
            return;
        }

        let folder = &path[0];
        let entry = self.folders.entry(folder.clone()).or_default();
        entry.insert(&path[1..], file);
    }

    fn from_path(base_path: &Path) -> Self {
        let mut tree = FileTree::default();
        
        let walker = WalkDir::new(base_path)
            .min_depth(1)
            .into_iter()
            .filter_entry(|e| {
                !e.file_name()
                    .to_str()
                    .map(|s| s.starts_with('.'))
                    .unwrap_or(false)
            });

        for entry in walker.filter_map(Result::ok) {
            if let Ok(relative) = entry.path().strip_prefix(base_path) {
                let path_components: Vec<String> = relative
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect();

                if entry.file_type().is_file() {
                    if let Some((file_name, path)) = path_components.split_last() {
                        if file_name.ends_with(".lua") || file_name.ends_with(".js") {
                            tree.insert(&path.to_vec(), file_name.to_string());
                        }
                    }
                }
            }
        }
        
        tree
    }
}

#[derive(Default)]
#[allow(dead_code)]
struct ResourceInfo {
    name: String,
    path: PathBuf,
    file_tree: FileTree,
}

struct HotReloadApp {
    config: ServerConfig,
    resources: Vec<String>,
    runtime: Arc<Runtime>,
    connection_status: Arc<Mutex<ConnectionStatus>>,
    resource_trees: HashMap<String, ResourceInfo>,
}

// Implémentation de l'application (Interface graphique et gestionnaire de ressources)
impl HotReloadApp {
    // Fonction pour créer une instance de l'application
    fn new(_: &eframe::CreationContext<'_>) -> Self {
        let runtime = Arc::new(Runtime::new().expect("Erreur lors de la création du runtime!"));
        let connection_status = Arc::new(Mutex::new(ConnectionStatus::Disconnected));
        
        let mut app = Self {
            config: ServerConfig::default(),
            resources: Vec::new(),
            runtime,
            connection_status,
            resource_trees: HashMap::new(),
        };

        // Charger la configuration initiale
        if let Ok(config_str) = std::fs::read_to_string("server_config.json") {
            app.config = serde_json::from_str(&config_str).unwrap_or_default();
            app.scan_resources();
        }

        // Démarrer le watcher si un chemin est configuré
        if let Some(resources_path) = app.config.resources_path.clone() {
            let rt = app.runtime.clone();
            let status = app.connection_status.clone();
            
            rt.spawn(async move {
                *status.lock().unwrap() = ConnectionStatus::Connecting;
                
                match ResourceWatcher::new(resources_path, "ws://localhost:3090").await {
                    Ok(watcher) => {
                        println!("✅ Watcher créé avec succès");
                        *status.lock().unwrap() = ConnectionStatus::Connected;
                        
                        // Garder le watcher en vie
                        if let Err(e) = watcher.watch().await {
                            println!("❌ Erreur du watcher: {}", e);
                            *status.lock().unwrap() = ConnectionStatus::Error(e.to_string());
                        }
                    },
                    Err(e) => {
                        println!("❌ Erreur lors de la création du watcher: {}", e);
                        *status.lock().unwrap() = ConnectionStatus::Error(e.to_string());
                    }
                }
            });
        }

        app
    }

    // Fonction pour sauvegarder la configuration (Création du fichier server_config.json si il n'existe pas)
    fn save_config(&self) {
        if let Ok(config_str) = serde_json::to_string_pretty(&self.config) {
            let _ = std::fs::write("server_config.json", config_str);
        }
    }

    // Fonction pour scanner les ressources
    fn scan_resources(&mut self) {
        if let Some(base_path) = &self.config.resources_path {
            self.resources.clear();
            self.resource_trees.clear();
            
            if let Ok(entries) = std::fs::read_dir(base_path) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        if let Some(resource_name) = entry.file_name().to_str() {
                            let clean_name = resource_name.trim_start_matches('[')
                                                        .trim_end_matches(']')
                                                        .to_string();
                            
                            let file_tree = FileTree::from_path(&entry.path());
                            self.resources.push(clean_name.clone());
                            
                            self.resource_trees.insert(
                                clean_name.clone(),
                                ResourceInfo {
                                    name: clean_name.clone(),
                                    path: entry.path(),
                                    file_tree,
                                }
                            );
                        }
                    }
                }
            }
            self.resources.sort();
        }
    }

    // Afficher l'arbre des fichiers
    fn render_file_tree(&self, ui: &mut egui::Ui, tree: &FileTree, resource_name: &str) {
        // Afficher les dossiers
        for (folder, subtree) in &tree.folders {
            ui.collapsing(
                egui::RichText::new(format!("📁 {}", folder))
                    .color(egui::Color32::from_rgb(255, 208, 0)),
                |ui| {
                    ui.indent(folder, |ui| {
                        self.render_file_tree(ui, subtree, resource_name);
                    });
                }
            );
        }

        // Afficher les fichiers
        for file in &tree.files {
            ui.horizontal(|ui| {
                let icon = if file.ends_with(".lua") {
                    "🌙"
                } else if file.ends_with(".js") {
                    "📜"
                } else {
                    "📄"
                };
                
                ui.label(format!("{} {}", icon, file));
            });
        }
    }
}

impl eframe::App for HotReloadApp {
    // Fonction appelée à chaque frame
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Sélection du dossier resources
                if ui.button("📂 Sélectionner Resources").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Sélectionner le dossier resources")
                        .pick_folder() 
                    {
                        self.config.resources_path = Some(path);
                        self.scan_resources();
                        self.save_config();
                    }
                }

                // Afficher le chemin configuré
                if let Some(res_path) = &self.config.resources_path {
                    ui.label(format!("📁 Resources: {}", res_path.display()));
                }

                // Afficher le statut de connexion
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
            });
        });

        // Panel des resources
        egui::SidePanel::left("resources_panel")
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.heading("📦 Resources");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for resource_name in &self.resources {
                        if let Some(resource_info) = self.resource_trees.get(resource_name) {
                            ui.collapsing(
                                egui::RichText::new(format!("📦 {}", resource_name))
                                    .strong()
                                    .color(egui::Color32::LIGHT_BLUE),
                                |ui| {
                                    self.render_file_tree(ui, &resource_info.file_tree, resource_name);
                                }
                            );
                        }
                    }
                });
            });

        // Panel central
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hot Reload");
        });
    }
}

// Fonction principale pour lancer l'application
fn main() -> Result<(), eframe::Error> {
    // Charger l'icône
    let icon = image::load_from_memory(include_bytes!("../assets/supv.ico"))
        .expect("Erreur lors du chargement de l'icône!")
        .into_rgba8();
    let (width, height) = icon.dimensions();
    
    // Définir les options de l'application
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("FiveM Hot Reload")
            .with_icon(egui::IconData {
                rgba: icon.into_raw(),
                width,
                height,
            }),
        ..Default::default()
    };

    // Lancer l'application
    eframe::run_native(
        "FiveM Hot Reload",
        options,
        Box::new(|cc| Box::new(HotReloadApp::new(cc))),
    )
}
