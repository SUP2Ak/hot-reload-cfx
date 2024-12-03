use hot_reload_watcher::WatcherConfig;
use tracing::error;
use std::path::Path;

#[tokio::main]
async fn main() {
    // Initialiser le logger
    tracing_subscriber::fmt::init();

    // Vérifier l'existence du dossier resources
    if !Path::new("resources").is_dir() {
        error!("Le dossier 'resources' n'existe pas dans le répertoire courant!");
        std::process::exit(1);
    }

    // Charger ou créer la configuration
    let config = WatcherConfig::load_or_create();

    // Démarrer le watcher
    if let Err(e) = hot_reload_watcher::run(config).await {
        error!("Erreur lors de l'exécution du watcher: {}", e);
        std::process::exit(1);
    }
}