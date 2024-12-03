use notify::{Watcher, RecursiveMode, Event, EventKind};
use tokio::sync::{Mutex as TokioMutex, broadcast};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, WebSocketStream, MaybeTlsStream};
use futures::SinkExt;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    FileModified,
    FileAdded,
    FileRemoved,
    ManifestChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceChange {
    pub resource_name: String,
    pub change_type: ChangeType,
    pub file_path: String,
}

pub struct ResourceWatcher {
    ws_client: Arc<TokioMutex<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
    event_tx: broadcast::Sender<ResourceChange>,
    _watcher: notify::RecommendedWatcher,
    #[allow(unused)]
    last_events: Arc<TokioMutex<HashMap<String, Instant>>>,
}

impl ResourceWatcher {
    pub async fn new(resources_path: PathBuf, ws_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        println!("Tentative de connexion à {}", ws_url);
        
        let (ws_stream, _) = connect_async(ws_url).await?;
        println!("✅ Connecté au serveur WebSocket!");
        
        let (event_tx, _) = broadcast::channel(100);
        let event_tx_clone = event_tx.clone();
        let last_events = Arc::new(TokioMutex::new(HashMap::new()));
        let last_events_clone = last_events.clone();
        
        let runtime = tokio::runtime::Handle::current();
        
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            let rt = runtime.clone();
            
            if let Ok(event) = res {
                if let Some(path) = event.paths.first() {
                    let path_str = path.to_string_lossy().into_owned();
                    
                    if let Some(ext) = path.extension() {
                        if ext != "lua" && ext != "js" {
                            return;
                        }
                    } else {
                        return;
                    }

                    let last_events = last_events_clone.clone();
                    let event_tx = event_tx_clone.clone();
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

                            println!("✨ Envoi du changement: {:?}", change);
                            let _ = event_tx.send(change);
                        }
                    });
                }
            }
        })?;

        println!("👀 Configuration du watcher pour: {:?}", resources_path);
        watcher.watch(&resources_path, RecursiveMode::Recursive)?;
        println!("✅ Watcher configuré avec succès");

        Ok(Self {
            ws_client: Arc::new(TokioMutex::new(ws_stream)),
            event_tx,
            _watcher: watcher,
            last_events,
        })
    }

    pub async fn watch(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        println!("🎯 Démarrage du watcher...");
        let mut rx = self.event_tx.subscribe();
        println!("📡 En attente de changements...");

        while let Ok(change) = rx.recv().await {
            println!("📥 Changement reçu: {:?}", change);
            self.notify_change(change).await?;
        }

        Ok(())
    }

    async fn notify_change(&self, change: ResourceChange) -> Result<(), Box<dyn Error + Send + Sync>> {
        println!("📤 Envoi du changement: {:?}", change);
        let message = serde_json::to_string(&change)?;
        let mut ws_client = self.ws_client.lock().await;
        ws_client.send(message.into()).await?;
        Ok(())
    }
} 