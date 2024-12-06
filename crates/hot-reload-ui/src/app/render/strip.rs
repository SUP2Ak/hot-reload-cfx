use super::HotReloadApp;
use crate::app::generate_api_key;
use crate::app::Translator;
use eframe::egui;

impl HotReloadApp {
    pub fn render_menu_strip(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button(self.translator.t("folder"), |ui| {
                    if ui.button(self.translator.t("reload_config")).clicked() {
                        if let Ok(config_str) = std::fs::read_to_string("server_config.json") {
                            self.config = serde_json::from_str(&config_str).unwrap_or_default();
                            ui.close_menu();
                        }
                    }
                    if ui.button(self.translator.t("save")).clicked() {
                        self.save_config();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(self.translator.t("quit")).clicked() {
                        std::process::exit(0);
                    }
                });

                ui.menu_button(self.translator.t("tools"), |ui| {
                    ui.separator();
                    if ui.button(self.translator.t("generate_api_key")).clicked() {
                        let api_key = generate_api_key();
                        self.new_profile_api_key = api_key;
                        self.show_api_key_popup = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button(self.translator.t("affichage"), |ui| {
                    ui.checkbox(
                        &mut self.show_ignored_files,
                        self.translator.t("ignore_files"),
                    );
                    ui.separator();
                    ui.menu_button(self.translator.t("theme"), |ui| {
                        let current_theme = self.config.theme.clone();
                        
                        if ui.selectable_label(current_theme == "light", self.translator.t("light")).clicked() {
                            ctx.set_visuals(egui::Visuals::light());
                            self.config.theme = "light".to_string();
                            if let Ok(config_json) = serde_json::to_string_pretty(&self.config) {
                                let _ = std::fs::write("server_config.json", config_json);
                            }
                            ui.close_menu();
                        }
                        
                        if ui.selectable_label(current_theme == "dark", self.translator.t("dark")).clicked() {
                            ctx.set_visuals(egui::Visuals::dark());
                            self.config.theme = "dark".to_string();
                            if let Ok(config_json) = serde_json::to_string_pretty(&self.config) {
                                let _ = std::fs::write("server_config.json", config_json);
                            }
                            ui.close_menu();
                        }
                    });
                    ui.separator();
                    ui.menu_button(self.translator.t("language"), |ui| {
                        for language in Translator::available_languages() {
                            let is_selected = self.translator.get_language() == language;
                            if ui.selectable_label(is_selected, format!("{:?}", language)).clicked() {
                                self.translator.set_language(language).unwrap_or_default();
                                self.config.language = language;
                                if let Ok(config_json) = serde_json::to_string_pretty(&self.config) {
                                    let _ = std::fs::write("server_config.json", config_json);
                                }
                                ui.close_menu();
                            }
                        }
                    });
                });

                ui.menu_button(self.translator.t("help"), |ui| {
                    if ui.button(self.translator.t("documentation")).clicked() {
                        // TODO: Ouvrir la doc
                        ui.close_menu();
                    }
                    if ui.button(self.translator.t("report_bug")).clicked() {
                        // TODO: Ouvrir GitHub Issues
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(self.translator.t("about")).clicked() {
                        self.show_about_popup = true;
                        ui.close_menu();
                    }
                });
            });
        });
    }
}
