use hot_reload_watcher::WatcherConfig;
use tracing::error;
use std::path::Path;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    if !Path::new("resources").is_dir() {
        error!("Le dossier 'resources' n'existe pas dans le répertoire courant!");
        std::process::exit(1);
    }

    let config = WatcherConfig::load_or_create();
    if let Err(e) = hot_reload_watcher::run(config).await {
        error!("Erreur lors de l'exécution du watcher: {}", e);
        std::process::exit(1);
    }
}