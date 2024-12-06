use super::HotReloadApp;
use eframe::egui;

impl HotReloadApp {
    pub fn render_content(&mut self, ctx: &egui::Context) {
        self.render_main(ctx);
    }
}