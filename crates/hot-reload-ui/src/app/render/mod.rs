mod header;
mod strip;
mod tree;
mod content;
mod main;

use super::HotReloadApp;
use eframe::egui;

impl HotReloadApp {
    pub fn render(&mut self, ctx: &egui::Context) {
        self.render_menu_strip(ctx);
        self.render_header(ctx);
        self.render_tree(ctx);
        self.render_content(ctx);
    }
}