/*
Liste de ce qu'il reste à faire ou à revoir:
    - Revoir la gestion des erreurs
    - Possibilité de désélectionner/sélectionner une ressource dans la liste des ressources
    - Personnaliser la connexion au websocket (url, port, etc...)
    - Revoir la gestion des événements sans doute
    - Compiler pour dev sur windows / linux / mac sur un serveur externe sur linux ou windows
    - Revoir la gestion des logs (afficher les logs dans l'interface)
    - Séparer avec une autre api Rust (une run là où il y a le serveur externe avec les ressources etc afin d'annalyser localement qui communique avec notre interface sur notre pc, d'où le fait de faire une interface graphique compatible mac & windows & unix)
*/

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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ChangeType {
    FileModified,
    FileAdded,
    FileRemoved,
    ManifestChanged,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResourceChange {
    pub resource_name: String,
    pub change_type: ChangeType,
    pub file_path: String,
}

pub struct ResourceWatcher {
    ws_client: Arc<TokioMutex<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
    event_tx: broadcast::Sender<ResourceChange>,
}

// Implémentation de ResourceWatcher
impl ResourceWatcher {
    // Fonction pour créer une instance de ResourceWatcher
    pub async fn new(resources_path: PathBuf, ws_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        println!("Tentative de connexion à {}", ws_url);
        
        let (ws_stream, _) = connect_async(ws_url).await?;
        println!("✅ Connecté au serveur WebSocket!");
        
        let (event_tx, _) = broadcast::channel(100);
        let event_tx_clone = event_tx.clone();
        let last_events = Arc::new(TokioMutex::new(HashMap::new()));
        let last_events_clone = last_events.clone();
        let runtime = tokio::runtime::Handle::current();

        let mut file_watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                let last_events = last_events_clone.clone();
                let event_tx = event_tx_clone.clone();
                let rt = runtime.clone();
                
                rt.spawn(async move {
                    if let Some(change) = Self::process_file_event(event, &last_events).await {
                        let _ = event_tx.send(change);
                    }
                });
            }
        })?;

        file_watcher.watch(&resources_path, RecursiveMode::Recursive)?;

        Ok(Self {
            ws_client: Arc::new(TokioMutex::new(ws_stream)),
            event_tx,
        })
    }

    // Fonction pour traiter les événements de fichier
    async fn process_file_event(
        event: Event, 
        last_events: &Arc<TokioMutex<HashMap<PathBuf, Instant>>>
    ) -> Option<ResourceChange> {
        let path = event.paths.first()?.to_path_buf();
        
        let mut last_events = last_events.lock().await;
        let now = Instant::now();
        
        if let Some(last_time) = last_events.get(&path) {
            if now.duration_since(*last_time) < Duration::from_millis(100) {
                return None;
            }
        }
        
        last_events.insert(path.clone(), now);
        drop(last_events); // Libérer le mutex explicitement (pour éviter les fuites de mémoire)

        // Vérifier l'extension
        if let Some(ext) = path.extension() {
            if ext != "lua" && ext != "js" {
                return None;
            }
        } else {
            return None;
        }

        // Trouver le nom de la ressource
        let resource_name = path.parent()?
            .file_name()?
            .to_str()?
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_string();

        // Déterminer le type de changement
        let change_type = match event.kind {
            EventKind::Create(_) => ChangeType::FileAdded,
            EventKind::Modify(_) => ChangeType::FileModified,
            EventKind::Remove(_) => ChangeType::FileRemoved,
            _ => return None,
        };

        // Retourner le changement sous forme de ResourceChange
        Some(ResourceChange {
            resource_name,
            change_type,
            file_path: path.to_string_lossy().into_owned(),
        })
    }

    // Fonction pour surveiller les événements
    pub async fn watch(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut rx = self.event_tx.subscribe();

        while let Ok(change) = rx.recv().await {
            self.notify_change(change).await?;
        }

        Ok(())
    }

    // Fonction pour envoyer un changement au serveur
    async fn notify_change(&self, change: ResourceChange) -> Result<(), Box<dyn Error + Send + Sync>> {
        println!("📤 Changement détecté: {:?} - {}", change.change_type, change.file_path);
        let message = serde_json::to_string(&change)?;
        let mut ws_client = self.ws_client.lock().await;
        ws_client.send(message.into()).await?;
        Ok(())
    }
} 