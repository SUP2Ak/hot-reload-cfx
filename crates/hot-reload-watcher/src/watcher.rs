use crate::config::WatcherConfig;
use futures::{SinkExt, StreamExt};
use hot_reload_common::{ChangeType, InitialData, ResourceChange};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::MAIN_SEPARATOR;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::{TcpListener, TcpStream};
use tokio::signal;
use tokio::sync::mpsc;
use tokio::sync::Mutex as TokioMutex;
use tokio::task::spawn_blocking;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{error, info};
use walkdir::WalkDir;
use crate::fxserver::FxServerConnection;
use crate::types::BoxError;

fn ignored_folders_and_files() -> HashSet<String> {
    return vec![
        "node_modules".to_string(),
        ".git".to_string(),
        ".github".to_string(),
        ".gitignore".to_string(),
        "target".to_string(),
        ".idea".to_string(),
        ".vscode".to_string(),
        "vendor".to_string(),
        "tmp".to_string(),
        "temp".to_string(),
        "logs".to_string(),
        "coverage".to_string(),
        ".next".to_string(),
        ".nuxt".to_string(),
        ".cache".to_string(),
        "package-lock.json".to_string(),
        "yarn.lock".to_string(),
        "pnpm-lock.yaml".to_string(),
        "README.md".to_string(),
        "LICENSE".to_string(),
        "tsconfig.json".to_string(),
        "package.json".to_string(),
        "webpack.config.js".to_string(),
        "tsconfig.json".to_string(),
        "tsconfig.node.json".to_string(),
        "tsconfig.app.json".to_string(),
        "tsconfig.json".to_string(),
    ]
    .into_iter()
    .collect()
}

fn valid_extensions() -> HashSet<String> {
    return vec![
        "lua".to_string(),
        "js".to_string(),
        "dll".to_string(),
    ]
    .into_iter()
    .collect()
}

async fn notify_server_status(
    fx_connection: &FxServerConnection,
    ui_connections: &Arc<TokioMutex<Vec<mpsc::Sender<Message>>>>,
) {
    let status = if fx_connection.is_connected() {
        "online"
    } else {
        "offline"
    };

    let ui_connections = ui_connections.lock().await;
    if !ui_connections.is_empty() {
        let status_message = serde_json::json!({
            "type": "server_status",
            "status": status
        });

        if let Ok(message_str) = serde_json::to_string(&status_message) {
            for client in ui_connections.iter() {
                let _ = client.send(Message::Text(message_str.clone())).await;
            }
        }
    }
}

pub struct ResourceWatcher {
    resources: Arc<TokioMutex<HashMap<String, Vec<String>>>>,
    fx_connection: Arc<FxServerConnection>,
    ui_connections: Arc<TokioMutex<Vec<mpsc::Sender<Message>>>>,
    config: Arc<WatcherConfig>,
}

impl ResourceWatcher {
    pub fn new(config: WatcherConfig) -> Self {
        Self {
            resources: Arc::new(TokioMutex::new(HashMap::new())),
            fx_connection: Arc::new(FxServerConnection::new()),
            ui_connections: Arc::new(TokioMutex::new(Vec::new())),
            config: Arc::new(config),
        }
    }

    async fn scan_resources(&self) -> Result<HashMap<String, Vec<String>>, BoxError> {
        info!("📂 Démarrage du scan des ressources");

        let path = if cfg!(unix) {
            // Sur Linux/Unix, vérifier d'abord le chemin absolu
            if Path::new(&self.config.resources_path).exists() {
                Path::new(&self.config.resources_path).to_path_buf()
            } else {
                std::env::current_dir()?.join("resources")
            }
        } else if cfg!(windows) {
            // Sur Windows, garder la logique actuelle
            let mut path = std::env::current_dir()?.join("resources");
            if !path.exists() {
                path = Path::new(&self.config.resources_path).to_path_buf();
            }
            path
        } else {
            println!("❌ Os non supporté");
            return Err("Os non supporté".into());
        };

        if !path.exists() {
            error!("❌ Dossier resources introuvable: {}", path.display());
            return Err("Dossier resources introuvable".into());
        }

        info!("📂 Utilisation du dossier: {}", path.display());

        let mut resources = HashMap::new();
        for entry in WalkDir::new(&path).into_iter().filter_entry(|e| {
            let name = e.file_name().to_str().unwrap_or_default();
            !ignored_folders_and_files().contains(name) && !name.starts_with('.')
        }) {
            let entry = entry?;

            if entry.file_type().is_dir() {
                let resource_path = entry.path();

                if resource_path.join("fxmanifest.lua").exists()
                    || resource_path.join("__resource.lua").exists()
                {
                    let resource_name = resource_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_default()
                        .to_string();

                    let mut resource_files = Vec::new();

                    for file in WalkDir::new(resource_path)
                        .into_iter()
                        .filter_entry(|e| {
                            let name = e.file_name().to_str().unwrap_or_default();
                            !ignored_folders_and_files().contains(name) && !name.starts_with('.')
                        })
                        .filter_map(|e| e.ok())
                    {
                        if file.file_type().is_file() {
                            let file_name = file.file_name().to_str().unwrap_or_default();
                            if ignored_folders_and_files().contains(file_name) {
                                continue;
                            }

                            if let Some(ext) = file.path().extension() {
                                if let Some(ext_str) = ext.to_str() {
                                    if valid_extensions().contains(ext_str) {
                                        if let Ok(relative_path) =
                                            file.path().strip_prefix(resource_path)
                                        {
                                            let file_path =
                                                relative_path.to_string_lossy().to_string();
                                            info!(
                                                "📄 Fichier trouvé dans {}: {}",
                                                resource_name, file_path
                                            );
                                            resource_files.push(file_path);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if !resource_files.is_empty() {
                        resource_files.sort_by_cached_key(|a| a.to_lowercase());
                        info!(
                            "📂 Ressource trouvée: {} ({} fichiers)",
                            resource_name,
                            resource_files.len()
                        );
                        resources.insert(resource_name, resource_files);
                    }
                }
            }
        }

        info!("📂 Scan terminé: {} ressources trouvées", resources.len());
        Ok(resources)
    }

    async fn handle_resource_change(&self, change: ResourceChange) {
        info!("✨ Resource change detected: {:?}", change);
        let server_name = Path::new(&change.file_path)
            .components()
            .collect::<Vec<_>>()
            .windows(2)
            .find(|components| {
                components[1].as_os_str() == "resources"
            })
            .and_then(|components| components[0].as_os_str().to_str())
            .unwrap_or("unknown");

        let display_path = if let Some(resources_idx) = change.file_path.find("resources") {
            let relative_path = &change.file_path[resources_idx..];
            if MAIN_SEPARATOR == '\\' {
                relative_path.replace('\\', "/")
            } else {
                relative_path.to_string()
            }
        } else {
            change.file_path.clone()
        };

        let change_message = serde_json::json!({
            "type": "handle_resource_change",
            "action": change.change_type,
            "display_path": format!("{}/{}", server_name, display_path)
        });

        if self.fx_connection.is_connected() {
            if let Ok(message) = serde_json::to_string(&change) {
                info!("📤 Envoi à FiveM: {}", message);
                if let Some(response) = self.fx_connection.send_message(&message).await {
                    info!(
                        "✅ Resource {} restarted: {}",
                        change.resource_name, response
                    );
                } else {
                    error!("❌ Pas de réponse de FiveM pour {}", change.resource_name);
                }
            }
            notify_server_status(&self.fx_connection, &self.ui_connections).await;
        }

        let ui_connections = self.ui_connections.lock().await;
        if !ui_connections.is_empty() {
            for client in ui_connections.iter() {
                if let Ok(message_str) = serde_json::to_string(&change_message) {
                    let _ = client.send(Message::Text(message_str)).await;
                }
            }
        }
    }

    async fn start_file_watcher(self: Arc<Self>) -> Result<(), BoxError> {
        let resources = self.scan_resources().await?;
        *self.resources.lock().await = resources;

        let last_events = Arc::new(TokioMutex::new(HashMap::new()));
        let watcher_self = self.clone();
        
        spawn_blocking(move || {
            let rt = tokio::runtime::Handle::current();

            let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
                let watcher = watcher_self.clone();
                let last_events = last_events.clone();

                if let Ok(event) = res {
                    let should_ignore = event.paths.iter().any(|path| {
                        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                            if ignored_folders_and_files().contains(file_name)
                            {
                                return true;
                            }
                        }

                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            if !valid_extensions().contains(ext) {
                                return true;
                            }
                        } else {
                            return true;
                        }

                        false
                    });

                    if should_ignore {
                        return;
                    }

                    rt.spawn(async move {
                        if let Some(path) = event.paths.first() {
                            let mut current_path = path.to_path_buf();
                            let mut is_valid_resource = false;
                            let mut resource_path = None;

                            while let Some(parent) = current_path.parent().map(|p| p.to_path_buf())
                            {
                                if parent.join("fxmanifest.lua").exists()
                                    || parent.join("__resource.lua").exists()
                                {
                                    is_valid_resource = true;
                                    resource_path = Some(parent);
                                    break;
                                }
                                current_path = parent;
                            }

                            if !is_valid_resource {
                                return;
                            }

                            let path_str = path.to_string_lossy().into_owned();
                            let mut last_events = last_events.lock().await;
                            let now = Instant::now();

                            if let Some(last_time) = last_events.get(&path_str) {
                                if now.duration_since(*last_time) < Duration::from_secs(1) {
                                    return;
                                }
                            }

                            last_events.insert(path_str.clone(), now);
                            drop(last_events);

                            if let Some(resource_path) = resource_path {
                                if let Some(resource_name) = resource_path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .map(String::from)
                                {
                                    let change = ResourceChange {
                                        resource_name,
                                        change_type: match event.kind {
                                            EventKind::Create(_) => ChangeType::FileAdded,
                                            EventKind::Modify(_) => ChangeType::FileModified,
                                            EventKind::Remove(_) => ChangeType::FileRemoved,
                                            _ => return,
                                        },
                                        file_path: path_str,
                                    };

                                    watcher.handle_resource_change(change).await;
                                }
                            }
                        }
                    });
                }
            })?;

            watcher.watch(
                Path::new(&self.config.resources_path),
                RecursiveMode::Recursive,
            )?;

            info!("✅ File watcher started");
            std::thread::park();
            Ok::<(), BoxError>(())
        });

        Ok(())
    }

    async fn handle_ui_connection(&self, stream: TcpStream) -> Result<(), BoxError> {
        let ws_stream = accept_async(stream).await?;
        let (mut ws_write, mut ws_read) = ws_stream.split();
        let (tx, mut rx) = mpsc::channel(100);
        let tx_clone = tx.clone();

        {
            let mut connections = self.ui_connections.lock().await;
            connections.push(tx);
        }

        let initial_data = InitialData {
            resources_path: self.config.resources_path.clone(),
            resources: self.resources.lock().await.clone(),
        };

        ws_write
            .send(Message::Text(serde_json::to_string(&initial_data)?))
            .await?;

        notify_server_status(&self.fx_connection, &self.ui_connections).await;

        loop {
            tokio::select! {
                Some(msg) = ws_read.next() => {
                    match msg {
                        Ok(_) => break,
                        Err(_) => break,
                    }
                }
                Some(msg) = rx.recv() => {
                    if ws_write.send(msg).await.is_err() {
                        break;
                    }
                }
            }
        }

        let mut connections = self.ui_connections.lock().await;
        if let Some(pos) = connections.iter().position(|x| x.same_channel(&tx_clone)) {
            connections.remove(pos);
        }

        Ok(())
    }

    pub async fn run(self: Arc<Self>) -> Result<(), BoxError> {
        // Créer le canal de shutdown
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);
        let shutdown_tx = Arc::new(shutdown_tx);

        // Créer un canal pour gérer le redémarrage du watcher
        let (watcher_tx, mut watcher_rx) = tokio::sync::mpsc::channel::<()>(1);
        let watcher_tx = Arc::new(watcher_tx);

        // Démarrer le watcher de fichiers dans une tâche séparée
        let watcher_self = self.clone();
        let watcher_task = tokio::spawn(async move {
            loop {
                let watcher_clone = watcher_self.clone();
                if let Err(e) = watcher_clone.start_file_watcher().await {
                    error!("❌ Erreur du file watcher: {}", e);
                }
                // Attendre un signal de redémarrage
                if watcher_rx.recv().await.is_none() {
                    break;
                }
                // Petite pause avant de redémarrer
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        // Gérer Ctrl+C
        let shutdown_tx_clone = shutdown_tx.clone();
        let ctrl_c_task = tokio::spawn(async move {
            if let Ok(()) = signal::ctrl_c().await {
                info!("🛑 Arrêt demandé via Ctrl+C");
                let _ = shutdown_tx_clone.send(());
            }
        });

        // Démarrer la surveillance FiveM
        let fx_connection = self.fx_connection.clone();
        let config = self.config.clone();
        let fivem_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                if !fx_connection.is_connected() {
                    fx_connection.connect(&config).await;
                }
            }
        });

        // Démarrer le serveur WebSocket
        let addr = format!("{}:{}", self.config.ws_host, self.config.ws_port);
        let listener = TcpListener::bind(&addr).await?;
        info!("🚀 Watcher démarré sur {}", addr);
        info!("👀 En attente de connexions...");

        // Boucle principale
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("🛑 Arrêt demandé");
                    break;
                }
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            info!("📡 Nouvelle connexion UI depuis: {}", addr);
                            let watcher = self.clone();
                            tokio::spawn(async move {
                                if let Err(e) = watcher.handle_ui_connection(stream).await {
                                    error!("❌ Erreur de connexion UI: {}", e);
                                }
                            });
                        }
                        Err(e) => error!("❌ Erreur d'acceptation de connexion: {}", e),
                    }
                }
            }
        }

        // Attendre que toutes les tâches se terminent proprement
        drop(watcher_tx); // Arrêter la boucle du watcher
        let _ = tokio::join!(watcher_task, ctrl_c_task, fivem_task);

        info!("👋 Serveur arrêté");
        Ok(())
    }
}

// pub async fn run(config: WatcherConfig) -> Result<(), BoxError> {
//     let watcher = Arc::new(ResourceWatcher::new(config));

//     // Créer un canal pour maintenir le programme en vie
//     let (tx, rx) = tokio::sync::oneshot::channel();

//     // Gérer Ctrl+C
//     let tx = Arc::new(tokio::sync::Mutex::new(Some(tx)));
//     let tx_clone = tx.clone();

//     tokio::spawn(async move {
//         if let Ok(()) = signal::ctrl_c().await {
//             info!("🛑 Arrêt demandé via Ctrl+C");
//             if let Some(tx) = tx_clone.lock().await.take() {
//                 let _ = tx.send(());
//             }
//         }
//     });

//     // Démarrer le watcher dans une tâche séparée
//     let watcher_clone = watcher.clone();
//     tokio::spawn(async move {
//         if let Err(e) = watcher_clone.run().await {
//             error!("❌ Erreur du watcher: {}", e);
//         }
//     });

//     // Attendre le signal d'arrêt
//     let _ = rx.await;

//     info!("👋 Programme terminé");
//     Ok(())
// }
