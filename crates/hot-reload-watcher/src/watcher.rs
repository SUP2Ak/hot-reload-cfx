use crate::config::WatcherConfig;
use tokio::sync::Mutex as TokioMutex;
use tokio_tungstenite::{accept_async, WebSocketStream, MaybeTlsStream, tungstenite::Message};
use tokio::signal;
use tokio_tungstenite::connect_async;
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, error};
use walkdir::{WalkDir, Error as WalkDirError};
use futures::{SinkExt, StreamExt};
use futures::stream::SplitSink;
use notify::{Watcher, RecursiveMode, Event, EventKind};
use std::path::Path;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::error::Error;
use hot_reload_common::{ResourceChange, ChangeType, InitialData, AuthRequest, AuthResponse};
use tokio::sync::mpsc;
use std::collections::HashSet;

type BoxError = Box<dyn Error + Send + Sync>;

async fn scan_resources(_: &str) -> Result<HashMap<String, Vec<String>>, BoxError> {
    info!("📂 Start scanning resources");
    let path = Path::new("./resources");
    if !path.exists() {
        error!("❌ Folder ./resources doesn't exist in current directory");
        error!("❌ Ensure you run the executable from the root of the FiveM server");
        return Err("Folder resources doesn't exist".into());
    }

    info!("📂 Scan folder: {}", path.display());
    let valid_extensions: HashSet<&str> = ["lua", "js", "dll"].into_iter().collect();

    // TODO : Manage ignored folders & files from UI and sync it with all clients
    let ignored_folders: HashSet<&str> = [
        "node_modules", ".git", "target",
        ".idea", ".vscode", "vendor", "tmp", "temp",
        "logs", "coverage", ".next", ".nuxt", ".cache"
    ].into_iter().collect();
    let ignored_files: HashSet<&str> = [
        "package-lock.json", "yarn.lock", "pnpm-lock.yaml",
        "README.md", "LICENSE", ".gitignore", ".env",
        "tsconfig.json", "package.json", "webpack.config.js"
    ].into_iter().collect();

    let mut resource_list = Vec::new();
    for entry in WalkDir::new(path)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_str().unwrap_or_default();
            !ignored_folders.contains(name) && !name.starts_with('.')
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

                //info!("🔍 Ressource trouvée: {}", resource_name);
                let mut resource_files = Vec::new();
                
                for file in WalkDir::new(resource_path)
                    .into_iter()
                    .filter_entry(|e| {
                        let name = e.file_name().to_str().unwrap_or_default();
                        !ignored_folders.contains(name) && !name.starts_with('.')
                    })
                    .filter_map(|e| e.ok()) {
                    if file.file_type().is_file() {
                        let file_name = file.file_name().to_str().unwrap_or_default();
                        if ignored_files.contains(file_name) {
                            continue;
                        }

                        if let Some(ext) = file.path().extension() {
                            if let Some(ext_str) = ext.to_str() {
                                if valid_extensions.contains(ext_str) {
                                    if let Ok(relative_path) = file.path().strip_prefix(resource_path) {
                                        let file_path = relative_path.to_string_lossy().to_string();
                                        info!("📄 Fichier trouvé dans {}: {}", resource_name, file_path);
                                        resource_files.push(file_path);
                                    }
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
    let resources: HashMap<String, Vec<String>> = resource_list.into_iter().collect();

    info!("🏁 Scan finished, {} resources found", resources.len());
    if resources.is_empty() {
        info!("⚠️ No resource with fxmanifest.lua found");
    }

    Ok(resources)
}

async fn handle_connection(stream: TcpStream, config: &Arc<WatcherConfig>) -> Result<(), BoxError> {
    let addr = stream.peer_addr()?;
    let is_localhost = addr.ip().is_loopback();
    let ws_stream = accept_async(stream).await?;
    let (mut ws_write, mut ws_read) = ws_stream.split();
    let (tx, mut rx) = mpsc::channel::<Message>(1000);
    let tx = Arc::new(tx);

    if !is_localhost {
        if let Some(Ok(msg)) = ws_read.next().await {
            if let Ok(auth) = serde_json::from_str::<AuthRequest>(&msg.to_string()) {
                if auth.api_key != config.api_key {
                    let response = AuthResponse::Failed("Clé API invalide".to_string());
                    tx.send(Message::Text(serde_json::to_string(&response)?)).await?;
                    return Ok(());
                }
                let response = AuthResponse::Success;
                tx.send(Message::Text(serde_json::to_string(&response)?)).await?;
            }
        }
    }

    info!("📤 Sending initial data to client");
    let resources = scan_resources(&config.resources_path).await?;
    let initial_data = InitialData {
        resources_path: config.resources_path.clone(),
        resources: resources.clone(),
    };

    let initial_data_str = serde_json::to_string(&initial_data)?;
    // info!("📦 Initial data prepared: {}", initial_data_str);
    ws_write.send(Message::Text(initial_data_str)).await?;
    info!("✅ Initial data sent");

    let fx_stream = connect_to_fxserver(config).await?;
    let fx_stream = Arc::new(TokioMutex::new(fx_stream));
    let fx_stream_clone = fx_stream.clone();
    let runtime = tokio::runtime::Handle::current();
    let last_events = Arc::new(TokioMutex::new(HashMap::new()));
    let last_events_clone = last_events.clone();
    
    let mut watcher = {
        let tx = tx.clone();
        notify::recommended_watcher(move |res: Result<Event, _>| {
            let tx = tx.clone();
            let rt = runtime.clone();
            let last_events = last_events_clone.clone();
            let fx_stream = fx_stream_clone.clone();

            if let Ok(event) = res {
                let should_process = event.paths.iter().any(|path| {
                    path.extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| matches!(ext, "lua" | "js" | "dll"))
                        .unwrap_or(false)
                });

                if !should_process {
                    return ();
                }

                if let Some(path) = event.paths.first() {
                    let path = path.to_path_buf();
                    let path_str = path.to_string_lossy().into_owned();
                    let event_kind = event.kind;

                    let _ = rt.spawn(async move {
                        let mut last_events = last_events.lock().await;
                        let now = Instant::now();

                        if let Some(last_time) = last_events.get(&path_str) {
                            if now.duration_since(*last_time) < Duration::from_secs(1) {
                                return Ok::<(), BoxError>(());
                            }
                        }

                        last_events.insert(path_str.clone(), now);
                        drop(last_events);

                        if let Some(resource_name) = path.parent()
                            .and_then(|p| p.file_name())
                            .and_then(|n| n.to_str())
                            .map(String::from) {
                            
                            let change = ResourceChange {
                                resource_name: resource_name.clone(),
                                change_type: match event_kind {
                                    EventKind::Create(_) => ChangeType::FileAdded,
                                    EventKind::Modify(_) => ChangeType::FileModified,
                                    EventKind::Remove(_) => ChangeType::FileRemoved,
                                    _ => return Ok(()),
                                },
                                file_path: path_str,
                            };

                            info!("✨ Change detected: {:?}", change);
                            let fx_response = {
                                let mut fx = fx_stream.lock().await;
                                if let Ok(message) = serde_json::to_string(&change) {
                                    if let Ok(_) = fx.send(Message::Text(message)).await {
                                        info!("✅ Message sent to FXserver");
                                        
                                        if let Some(Ok(response)) = fx.next().await {
                                            if let Ok(text) = response.to_text() {
                                                Some(text.to_string())
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            };

                            if let Some(response_text) = fx_response {
                                info!("FXserver response: {}", response_text);
                                
                                let message = serde_json::json!({
                                    "type": "fivem_response",
                                    "message": response_text
                                });

                                if let Ok(message_str) = serde_json::to_string(&message) {
                                    info!("🔄 Sending via handler...");
                                    
                                    if let Err(e) = tx.send(Message::Text(message_str)).await {
                                        error!("❌ Error sending to handler: {}", e);
                                    } else {
                                        info!("✅ Message sent to handler");
                                    }
                                }
                            }

                            Ok::<(), BoxError>(())
                        } else {
                            Ok::<(), BoxError>(())
                        }
                    });
                }
            }()
        })?
    };

    watcher.watch(Path::new(&config.resources_path), RecursiveMode::Recursive)?;
    info!("✅ Monitoring resources enabled");
    let mut pending_messages = Vec::with_capacity(100);
    let mut last_batch_time = std::time::Instant::now();

    loop {
        tokio::select! {
            Some(ws_msg) = ws_read.next() => {
                match ws_msg {
                    Ok(msg) => {
                        if let Ok(text) = msg.to_text() {
                            info!("📨 Message received from client: {}", text);
                        }
                    }
                    Err(e) => {
                        error!("❌ WebSocket error: {}", e);
                        break;
                    }
                }
            }
            
            Some(message) = rx.recv() => {
                if let Ok(text) = message.to_text() {
                    info!("📨 Message received for batch: {}", text);
                    pending_messages.push(text.to_string());
                    
                    if pending_messages.len() >= 10 || last_batch_time.elapsed() > Duration::from_millis(100) {
                        if !pending_messages.is_empty() {
                            info!("🔄 Processing batch of {} messages", pending_messages.len());
                            if let Err(e) = process_message_batch(&pending_messages, &mut ws_write).await {
                                error!("❌ Error sending batch: {}", e);
                            }
                            pending_messages.clear();
                            last_batch_time = Instant::now();
                        }
                    }
                }
            }

            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                if !pending_messages.is_empty() {
                    info!("🔄 Processing batch by timeout");
                    if let Err(e) = process_message_batch(&pending_messages, &mut ws_write).await {
                        error!("❌ Error sending batch: {}", e);
                    }
                    pending_messages.clear();
                    last_batch_time = Instant::now();
                }
            }
        }
    }

    Ok(())
}

async fn connect_to_fxserver(config: &WatcherConfig) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, BoxError> {
    let fivem_url = format!("ws://localhost:{}", config.fivem_port);
    info!("🔌 Trying to connect to FXserver on {}", fivem_url);
    let (ws_stream, _) = connect_async(&fivem_url).await?;
    info!("✅ FXserver connection established!");
    
    Ok(ws_stream)
}

async fn process_message_batch(
    messages: &[String],
    ws_write: &mut SplitSink<WebSocketStream<TcpStream>, Message>
) -> Result<(), BoxError> {
    let batch = serde_json::json!({
        "type": "batch",
        "messages": messages.iter().map(|msg| {
            serde_json::json!({
                "type": "fivem_response",
                "message": msg
            })
        }).collect::<Vec<_>>()
    });

    let batch_str = serde_json::to_string(&batch)?;
    info!("📦 Sending batch: {}", batch_str);
    ws_write.send(Message::Text(batch_str)).await?;
    info!("✅ Batch sent successfully");
    Ok(())
}


pub async fn run(config: WatcherConfig) -> Result<(), Box<dyn Error + Send + Sync>> {
    let addr = format!("{}:{}", config.ws_host, config.ws_port);
    let listener = TcpListener::bind(&addr).await?;
    info!("🚀 WebSocket server started on {}", addr);
    info!("👀 Waiting for connections...");

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
                info!("🛑 Server shutdown requested");
                break;
            }
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, addr)) => {
                        info!("📡 New connection from: {}", addr);
                        let _ = shutdown_tx.clone();
                        let config = config.clone();
                        
                        tokio::spawn(async move {
                            match handle_connection(stream, &config).await {
                                Ok(_) => info!("✅ Connection closed: {}", addr),
                                Err(e) => error!("❌ Connection error: {}", e),
                            }
                        });
                    }
                    Err(e) => {
                        error!("❌ Connection accept error: {}", e);
                    }
                }
            }
        }
    }

    info!("👋 Server shutdown");
    Ok(())
}