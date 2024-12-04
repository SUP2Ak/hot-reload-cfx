use crate::config::WatcherConfig;
use tokio::sync::Mutex as TokioMutex;
use tokio_tungstenite::{accept_async, WebSocketStream, MaybeTlsStream};
use tokio::signal;
use tokio_tungstenite::connect_async;
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, error};
use walkdir::{WalkDir, Error as WalkDirError};
use futures::{SinkExt, StreamExt};
use notify::{Watcher, RecursiveMode, Event, EventKind};
use std::path::Path;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::error::Error;
use hot_reload_common::{ResourceChange, ChangeType, InitialData, AuthRequest, AuthResponse};

type BoxError = Box<dyn Error + Send + Sync>;

async fn scan_resources(_: &str) -> Result<HashMap<String, Vec<String>>, BoxError> {
    info!("📂 Début du scan des ressources");
    
    let mut resources = HashMap::new();
    let path = Path::new("./resources");

    if !path.exists() {
        error!("❌ Le dossier ./resources n'existe pas dans le répertoire courant");
        error!("❌ Assurez-vous de lancer l'exécutable depuis la racine du serveur FiveM");
        return Err("Le dossier resources n'existe pas".into());
    }

    info!("📂 Scan du dossier: {}", path.display());

    // Ignored folders
    let ignored_folders = [
        "node_modules", ".git", "target",
        ".idea", ".vscode", "vendor", "tmp", "temp",
        "logs", "coverage", ".next", ".nuxt", ".cache"
    ];

    // Ignored files
    let ignored_files = [
        "package-lock.json", "yarn.lock", "pnpm-lock.yaml",
        "README.md", "LICENSE", ".gitignore", ".env",
        "tsconfig.json", "package.json", "webpack.config.js"
    ];

    let mut resource_list = Vec::new();
    for entry in WalkDir::new(path)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_str().unwrap_or_default();
            !ignored_folders.contains(&name) && !name.starts_with('.')
        }) {
        let entry = entry.map_err(|e: WalkDirError| -> BoxError { Box::new(e) })?;
        
        if entry.file_type().is_dir() {
            let resource_path = entry.path();
            
            if resource_path.join("fxmanifest.lua").exists() || resource_path.join("__resource.lua").exists() {
                let resource_name = resource_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_string();

                info!("🔍 Ressource trouvée: {}", resource_name);
                let mut resource_files = Vec::new();
                
                for file in WalkDir::new(resource_path)
                    .into_iter()
                    .filter_entry(|e| {
                        let name = e.file_name().to_str().unwrap_or_default();
                        !ignored_folders.contains(&name) && !name.starts_with('.')
                    })
                    .filter_map(|e| e.ok()) {
                    if file.file_type().is_file() {
                        let file_name = file.file_name().to_str().unwrap_or_default();
                        if ignored_files.contains(&file_name) {
                            continue;
                        }

                        if let Some(ext) = file.path().extension() {
                            if ext == "lua" || ext == "js" || ext == "dll" {
                                if let Ok(relative_path) = file.path().strip_prefix(resource_path) {
                                    let file_path = relative_path.to_string_lossy().to_string();
                                    info!("📄 Fichier trouvé dans {}: {}", resource_name, file_path);
                                    resource_files.push(file_path);
                                }
                            }
                        }
                    }
                }

                if !resource_files.is_empty() {
                    resource_files.sort_by_cached_key(|a| a.to_lowercase());
                    resource_list.push((resource_name, resource_files));
                }
            }
        }
    }

    resource_list.sort_by_cached_key(|(name, _)| name.to_lowercase());
    resources = resource_list.into_iter().collect();

    info!("🏁 Scan terminé, {} ressources trouvées", resources.len());
    if resources.is_empty() {
        info!("⚠️ Aucune ressource avec fxmanifest.lua n'a été trouvée");
    }

    Ok(resources)
}

async fn handle_connection(stream: TcpStream, config: &Arc<WatcherConfig>) -> Result<(), BoxError> {
    let addr = stream.peer_addr()?;
    let is_localhost = addr.ip().is_loopback();
    let ws_stream = accept_async(stream).await?;
    let ws_stream = Arc::new(TokioMutex::new(ws_stream));
    //let ws_stream_watcher = ws_stream.clone();

    if !is_localhost {
        let mut ws = ws_stream.lock().await;
        match ws.next().await {
            Some(Ok(msg)) => {
                if let Ok(auth) = serde_json::from_str::<AuthRequest>(&msg.to_string()) {
                    if auth.api_key != config.api_key {
                        let response = AuthResponse::Failed("Clé API invalide".to_string());
                        ws.send(serde_json::to_string(&response)?.into()).await?;
                        return Ok(());
                    }
                    let response = AuthResponse::Success;
                    ws.send(serde_json::to_string(&response)?.into()).await?;
                }
            }
            _ => return Ok(()),
        }
    }

    let fivem_stream = connect_to_fivem(config).await?;
    let fivem_stream = Arc::new(TokioMutex::new(fivem_stream));
    let fivem_stream_clone = fivem_stream.clone(); // Clone pour le watcher
    let resources = scan_resources(&config.resources_path).await?;
    let initial_data = InitialData {
        resources_path: config.resources_path.clone(),
        resources,
    };
    let mut ws = ws_stream.lock().await;
    ws.send(serde_json::to_string(&initial_data)?.into()).await?;
    drop(ws);
    let last_events = Arc::new(TokioMutex::new(HashMap::new()));
    let runtime = tokio::runtime::Handle::current();

    let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
        let rt = runtime.clone();
        let last_events = last_events.clone();
        //let ws_stream = ws_stream_watcher.clone();
        let fivem_stream = fivem_stream_clone.clone();

        if let Ok(event) = res {
            if let Some(path) = event.paths.first() {
                let path = path.to_path_buf();
                
                if let Some(ext) = path.extension() {
                    if ext != "lua" && ext != "js" && ext != "dll" {
                        return;
                    }
                } else {
                    return;
                }

                let path_str = path.to_string_lossy().into_owned();
                let event_kind = event.kind;

                let _ = rt.spawn(async move {
                    let mut last_events = last_events.lock().await;
                    let now = Instant::now();

                    if let Some(last_time) = last_events.get(&path_str) {
                        if now.duration_since(*last_time) < Duration::from_secs(1) {
                            return;
                        }
                    }

                    last_events.insert(path_str.clone(), now);
                    drop(last_events);

                    if let Some(resource_name) = path.parent()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .map(String::from) {
                        
                        let change = ResourceChange {
                            resource_name,
                            change_type: match event_kind {
                                EventKind::Create(_) => ChangeType::FileAdded,
                                EventKind::Modify(_) => ChangeType::FileModified,
                                EventKind::Remove(_) => ChangeType::FileRemoved,
                                _ => return,
                            },
                            file_path: path_str,
                        };

                        info!("✨ Changement détecté: {:?}", change);
                        if let Ok(message) = serde_json::to_string(&change) {
                            let mut fivem = fivem_stream.lock().await;
                            if let Err(e) = fivem.send(message.into()).await {
                                error!("❌ Erreur d'envoi vers FiveM: {}", e);
                            }
                        }
                    }
                });
            }
        }
    })?;

    watcher.watch(Path::new(&config.resources_path), RecursiveMode::Recursive)?;
    info!("✅ Surveillance des ressources activée");

    loop {
        let mut ws = ws_stream.lock().await;
        match ws.next().await {
            Some(Ok(_)) => {
                continue;
            }
            Some(Err(e)) => {
                error!("❌ Erreur WebSocket: {}", e);
                break;
            }
            None => {
                info!("👋 Client déconnecté");
                break;
            }
        }
    }

    info!("👋 Connexion terminée: {}", addr);
    Ok(())
}

async fn connect_to_fivem(config: &WatcherConfig) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, Box<dyn Error + Send + Sync>> {
    let fivem_url = format!("ws://localhost:{}", config.fivem_port);
    info!("🔌 Connexion au serveur FiveM sur {}", fivem_url);
    
    let (ws_stream, _) = connect_async(&fivem_url).await?;
    info!("✅ Connecté au serveur FiveM!");
    
    Ok(ws_stream)
}

pub async fn run(config: WatcherConfig) -> Result<(), Box<dyn Error + Send + Sync>> {
    let addr = format!("{}:{}", config.ws_host, config.ws_port);
    let listener = TcpListener::bind(&addr).await?;
    info!("🚀 Serveur WebSocket démarré sur {}", addr);
    info!("👀 En attente de connexions...");

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel(1);
    let shutdown_tx = Arc::new(shutdown_tx);
    let config = Arc::new(config);

    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        if let Ok(()) = signal::ctrl_c().await {
            info!("Arrêt du serveur...");
            let _ = shutdown_tx_clone.send(());
        }
    });

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("🛑 Arrêt du serveur demandé");
                break;
            }
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, addr)) => {
                        info!("📡 Nouvelle connexion depuis: {}", addr);
                        let _ = shutdown_tx.clone();
                        let config = config.clone();
                        
                        tokio::spawn(async move {
                            match handle_connection(stream, &config).await {
                                Ok(_) => info!("✅ Connexion terminée: {}", addr),
                                Err(e) => error!("❌ Erreur de connexion {}: {}", addr, e),
                            }
                        });
                    }
                    Err(e) => {
                        error!("❌ Erreur d'acceptation de connexion: {}", e);
                    }
                }
            }
        }
    }

    info!("👋 Serveur arrêté");
    Ok(())
}
