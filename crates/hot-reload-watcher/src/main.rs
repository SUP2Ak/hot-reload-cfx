use hot_reload_watcher::WatcherConfig;
use tracing::error;
use std::path::Path;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    if !Path::new("resources").is_dir() {
        error!("The 'resources' directory does not exist in the current directory!");
        std::process::exit(1);
    }

    let config = WatcherConfig::load_or_create();
    if let Err(e) = hot_reload_watcher::run(config).await {
        error!("Error running watcher: {}", e);
        std::process::exit(1);
    }
}

/*
Il reste à faire :
    - Gérer et sync ce qu'on veut ou non watch depuis l'ui
    - Gérer les fichiers / dossiers ignorés sync depuis l'ui
*/