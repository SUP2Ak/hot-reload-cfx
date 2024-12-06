mod hot;
mod log;

use super::HotReloadApp;
use eframe::egui;

impl HotReloadApp {
    pub fn render_main(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| { 
            self.render_hot(ui);
            ui.separator();
            self.render_log(ui);
        });
    }
}
