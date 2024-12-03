use rand::{thread_rng, Rng};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Config {
    api_key: String,
}

fn generate_api_key() -> String {
    let mut rng = thread_rng();
    let mut bytes = vec![0u8; 32];
    rng.fill(&mut bytes[..]);
    BASE64.encode(&bytes)
}

fn main() {
    println!("🔑 Génération d'une nouvelle clé API...");
    
    let api_key = generate_api_key();
    let config = Config { api_key };
    
    // Créer le fichier config.json
    let config_json = serde_json::to_string_pretty(&config)
        .expect("Erreur lors de la sérialisation de la configuration");
    
    let config_path = Path::new("hot-reload-config.json");
    fs::write(config_path, config_json)
        .expect("Erreur lors de l'écriture du fichier de configuration");
    
    println!("✅ Clé API générée avec succès !");
    println!("📝 Configuration sauvegardée dans: {}", config_path.display());
    println!("\n⚠️  Gardez cette clé en sécurité et utilisez-la dans votre configuration.");
    
    // Attendre une entrée utilisateur avant de fermer sur Windows
    #[cfg(windows)]
    {
        println!("\nAppuyez sur Entrée pour quitter...");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
    }
}

/*

:: Windows
cargo build --release --target x86_64-pc-windows-msvc

:: Linux
cargo build --release --target x86_64-unknown-linux-gnu

:: macOS
cargo build --release --target x86_64-apple-darwin

*/