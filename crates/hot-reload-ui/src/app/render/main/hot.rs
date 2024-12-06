use super::HotReloadApp;
use eframe::egui;

impl HotReloadApp {
    pub fn render_hot(&mut self, ui: &mut egui::Ui) {
        ui.heading("Hot Reload");

        if let Some(res_path) = self.resources_path.lock().unwrap().as_ref() {
            ui.horizontal(|ui| {
                ui.label("📁");
                ui.label(format!(
                    "{}: {}",
                    self.translator.t("resources_path"),
                    res_path
                ));
            });
        }
    }
}
