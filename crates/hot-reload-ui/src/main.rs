#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod utils;

use app::HotReloadApp;
use eframe::egui;
use tokio::runtime::Runtime;
use std::sync::Arc;
use tracing::{info, debug};

fn main() -> Result<(), eframe::Error> {
    if std::env::var("RUST_LOG").map(|v| v == "debug").unwrap_or(false) {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_target(false)
            .with_thread_ids(false)
            .with_file(true)
            .with_line_number(true)
            .init();
        
        debug!("🔧 Application démarrée en mode DEBUG");
    }

    let runtime = Arc::new(Runtime::new().expect("Failed to create Tokio runtime"));
    let icon = image::load_from_memory(include_bytes!("../../../assets/supv.ico"))
        .expect("Erreur lors du chargement de l'icône!")
        .into_rgba8();
    let (width, height) = icon.dimensions();

    info!("🚀 Démarrage de l'interface...");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("FiveM Hot Reload Client")
            .with_icon(egui::IconData {
                rgba: icon.into_raw(),
                width,
                height,
            }),
        ..Default::default()
    };

    let runtime_for_block = runtime.clone();
    let runtime_for_app = runtime.clone();

    eframe::run_native(
        "FiveM Hot Reload Client",
        options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::new(runtime_for_block.block_on(async {
                HotReloadApp::new(cc, runtime_for_app).await
            }))
        }),
    )
}
