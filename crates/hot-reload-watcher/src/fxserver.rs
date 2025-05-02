use crate::config::WatcherConfig;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::Mutex as TokioMutex;
use tokio_tungstenite::{WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::connect_async;
use futures::StreamExt;
use futures::SinkExt;
use tracing::info;

pub struct FxServerConnection {
    pub stream: TokioMutex<Option<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
    pub is_connected: AtomicBool,
}

impl FxServerConnection {
    pub fn new() -> Self {
        Self {
            stream: TokioMutex::new(None),
            is_connected: AtomicBool::new(false),
        }
    }

    pub async fn connect(&self, config: &WatcherConfig) -> bool {
        let fxserver_url = format!("ws://localhost:{}", config.fxserver_port);

        match connect_async(&fxserver_url).await {
            Ok((ws_stream, _)) => {
                let mut stream = ws_stream;
                match stream.send(Message::Ping(vec![])).await {
                    Ok(_) => {
                        match tokio::time::timeout(Duration::from_secs(2), stream.next()).await {
                            Ok(Some(Ok(_))) => {
                                let mut locked_stream = self.stream.lock().await;
                                *locked_stream = Some(stream);
                                self.is_connected.store(true, Ordering::SeqCst);
                                info!("✅ Connexion FXServer établie!");
                                true
                            }
                            _ => {
                                self.is_connected.store(false, Ordering::SeqCst);
                                info!("❌ Pas de réponse du serveur FXServer");
                                false
                            }
                        }
                    }
                    Err(_) => {
                        self.is_connected.store(false, Ordering::SeqCst);
                        info!("❌ Échec de l'envoi du ping au serveur FXServer");
                        false
                    }
                }
            }
            Err(e) => {
                self.is_connected.store(false, Ordering::SeqCst);
                info!("❌ Impossible de se connecter au serveur FXServer: {}", e);
                false
            }
        }
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::SeqCst)
    }

    pub async fn send_message(&self, message: &str) -> Option<String> {
        let mut stream_guard = self.stream.lock().await;

        if let Some(stream) = stream_guard.as_mut() {
            match stream.send(Message::Text(message.to_string())).await {
                Ok(_) => match tokio::time::timeout(Duration::from_secs(2), stream.next()).await {
                    Ok(Some(Ok(response))) => {
                        return response.to_text().ok().map(|s| s.to_string());
                    }
                    _ => {
                        self.is_connected.store(false, Ordering::SeqCst);
                        info!("❌ Échec de l'envoi du message au serveur FXServer");
                        None
                    }
                },
                Err(_) => {
                    self.is_connected.store(false, Ordering::SeqCst);
                    info!("❌ Échec de l'envoi du message au serveur FXServer");
                    None
                }
            }
        } else {
            self.is_connected.store(false, Ordering::SeqCst);
            info!("❌ La connexion au serveur FXServer est fermée");
            None
        }
    }
}