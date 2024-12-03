use crate::config::WatcherConfig;
use tokio::sync::Mutex as TokioMutex;
use tokio_tungstenite::{accept_async, WebSocketStream, MaybeTlsStream};
use tokio::signal;
use tokio_tungstenite::connect_async;
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, error};
use walkdir::WalkDir;
use futures::{SinkExt, StreamExt};
use notify::{Watcher, RecursiveMode, Event, EventKind};
use std::path::{PathBuf, Path};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::error::Error;
use hot_reload_common::{ResourceChange, ChangeType, FileTree, InitialData, ResourceCategory, AuthRequest, AuthResponse};

async fn scan_resources(resources_path: &str) -> ResourceCategory {
    let mut root_category = ResourceCategory::new();
    
    // Parcourir le dossier resources
    for entry in WalkDir::new(resources_path)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_dir() {
            let dir_name = entry.file_name().to_str().unwrap_or_default();
            
            // Si c'est une catégorie [xxx] (A voir encore ceci j'ai quelque soucis avec des sous dossiers dans les ressources j'ai l'impression)
            if dir_name.starts_with('[') && dir_name.ends_with(']') {
                let category_name = dir_name[1..dir_name.len()-1].to_string();
                let mut category = ResourceCategory::new();
                
                // Scanner les ressources dans cette catégorie
                for resource in WalkDir::new(entry.path())
                    .min_depth(1)
                    .max_depth(1)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if resource.file_type().is_dir() {
                        // Vérifier si c'est une ressource valide (fxmanifest.lua)
                        if resource.path().join("fxmanifest.lua").exists() || 
                           resource.path().join("__resource.lua").exists() {
                            let resource_name = resource.file_name().to_str().unwrap_or_default().to_string();
                            let mut file_tree = FileTree::new();
                            
                            // Scanner les fichiers de la ressource
                            for file in WalkDir::new(resource.path())
                                .into_iter()
                                .filter_map(|e| e.ok())
                            {
                                if file.file_type().is_file() {
                                    if let Some(ext) = file.path().extension() {
                                        if ext == "lua" || ext == "js" {
                                            if let Ok(relative) = file.path().strip_prefix(resource.path()) {
                                                let components: Vec<String> = relative
                                                    .parent()
                                                    .map(|p| p.components()
                                                        .map(|c| c.as_os_str().to_string_lossy().into_owned())
                                                        .collect())
                                                    .unwrap_or_default();
                                                
                                                let mut current = &mut file_tree;
                                                // Créer l'arborescence des dossiers
                                                for component in &components {
                                                    current = current.folders
                                                        .entry(component.clone())
                                                        .or_insert_with(FileTree::new);
                                                }
                                                
                                                if let Some(file_name) = file.path().file_name() {
                                                    if let Some(file_name) = file_name.to_str() {
                                                        current.files.push(file_name.to_string());
                                                        current.files.sort();
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            
                            category.resources.insert(resource_name, file_tree);
                        }
                    }
                }
                
                root_category.categories.insert(category_name, category);
            }
        }
    }
    
    root_category
}

// Ajouter une connexion WebSocket vers le serveur FiveM (Celle ci est en localhost, pour le moment je vois pas d'utilité de le faire en réseau, ca permet juste d'accéder à l'environnement de FiveM via la ressource start dedans qui va manage les ressources, voir plus tard pour une solution sans doute rcon?!)
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

async fn handle_connection(stream: TcpStream, config: &Arc<WatcherConfig>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let addr = stream.peer_addr()?;
    let is_localhost = addr.ip().is_loopback();
    let ws_stream = accept_async(stream).await?;
    let ws_stream = Arc::new(TokioMutex::new(ws_stream));

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
                } else {
                    return Ok(());
                }
            }
            _ => return Ok(()),
        }
    }

    // Connexion au serveur FiveM
    let fivem_stream = connect_to_fivem(config).await?;
    let fivem_stream = Arc::new(TokioMutex::new(fivem_stream));
    let fivem_stream_clone = fivem_stream.clone(); // Clone pour le watcher

    // Scanner les ressources avec la nouvelle structure
    let categories = scan_resources(&config.resources_path).await;
    let initial_data = InitialData {
        resources_path: config.resources_path.clone(),
        categories,
    };

    // Envoyer les données initiales
    let message = serde_json::to_string(&initial_data)?;
    let mut ws = ws_stream.lock().await;
    ws.send(message.into()).await?;
    drop(ws);

    // Configurer le watcher
    let last_events = Arc::new(TokioMutex::new(HashMap::new()));
    let runtime = tokio::runtime::Handle::current();

    let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
        let rt = runtime.clone();
        let last_events = last_events.clone();
        let fivem_stream = fivem_stream_clone.clone(); // Utiliser le clone ici

        if let Ok(event) = res {
            if let Some(path) = event.paths.first() {
                let path_str = path.to_string_lossy().into_owned();

                if let Some(ext) = path.extension() {
                    if ext != "lua" && ext != "js" && ext != "dll" {
                        return;
                    }
                } else {
                    return;
                }

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

                    let path_buf = PathBuf::from(&path_str);
                    if let Some(resource_name) = path_buf.parent()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .map(|n| n.trim_start_matches('[').trim_end_matches(']').to_string())
                    {
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

    info!("👀 Configuration du watcher pour le dossier resources");
    watcher.watch(Path::new(&config.resources_path), RecursiveMode::Recursive)?;
    info!("✅ Watcher configuré avec succès");

    // Juste pour laisser le watcher actif
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
