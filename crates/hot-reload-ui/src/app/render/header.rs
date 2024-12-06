use super::HotReloadApp;
use crate::app::ConnectionStatus;
use eframe::egui;

impl HotReloadApp {
    pub fn render_header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(self.translator.t("profile"));
                let current_profile = self.config.current_profile.clone();
                egui::ComboBox::from_label("")
                    .selected_text(
                        self.config
                            .get_current_profile()
                            .map(|p| p.name.clone())
                            .unwrap_or_else(|| self.translator.t("profile_select")),
                    )
                    .show_ui(ui, |ui| {
                        for profile in &self.config.profiles {
                            ui.selectable_value(
                                &mut self.config.current_profile,
                                Some(profile.name.clone()),
                                &profile.name,
                            );
                        }
                    });
                if ui.button("➕").clicked() {
                    self.show_add_profile_popup = true;
                }
                if let Some(current) = &self.config.current_profile {
                    if let Some(profile) = self.config.get_current_profile() {
                        if !profile.is_local {
                            if ui.button("🗑").clicked() {
                                let mut config = self.config.clone();
                                config.remove_profile(current);
                                self.config = config;
                                self.config.current_profile = None;
                                self.save_config();
                            }
                        }
                    }
                }
                ui.separator();
                if let Some(current_name) = &current_profile {
                    if let Some(profile) = self.config.get_current_profile() {
                        let mut ws_url = profile.ws_url.clone();
                        let mut api_key = profile.api_key.clone();
                        let is_local = profile.is_local;

                        ui.label("WebSocket URL:");
                        let text_edit = egui::TextEdit::singleline(&mut ws_url)
                            .hint_text("ws://ip:port")
                            .desired_width(200.0);

                        let ws_changed = ui.add(text_edit).changed();
                        let mut api_changed = false;

                        if !is_local {
                            ui.label("API Key:");
                            let api_key_edit = egui::TextEdit::singleline(&mut api_key)
                                .password(true)
                                .hint_text(self.translator.t("profile_api_key_placeholder"))
                                .desired_width(200.0);

                            api_changed = ui.add(api_key_edit).changed();
                        }

                        if ws_changed || api_changed {
                            if let Some(profile) = self
                                .config
                                .profiles
                                .iter_mut()
                                .find(|p| p.name == *current_name)
                            {
                                if ws_changed {
                                    profile.ws_url = ws_url.clone();
                                }
                                if api_changed {
                                    profile.api_key = api_key.clone();
                                }
                                self.save_config();
                            }
                        }

                        if ui.button(self.translator.t("login")).clicked() {
                            let api_key = if !is_local {
                                Some(api_key.clone())
                            } else {
                                None
                            };
                            self.start_websocket(ws_url.clone(), api_key);
                        }

                        let status = self.connection_status.lock().unwrap().clone();
                        match status {
                            ConnectionStatus::Disconnected => {
                                ui.label(
                                    egui::RichText::new(self.translator.t("disconnected"))
                                        .color(egui::Color32::GRAY),
                                );
                            }
                            ConnectionStatus::Connecting => {
                                ui.label(
                                    egui::RichText::new(self.translator.t("connecting"))
                                        .color(egui::Color32::YELLOW),
                                );
                            }
                            ConnectionStatus::Connected => {
                                ui.label(
                                    egui::RichText::new(self.translator.t("connected"))
                                        .color(egui::Color32::GREEN),
                                );
                            }
                            ConnectionStatus::Error(err) => {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}: {}",
                                        self.translator.t("error_connection"),
                                        err
                                    ))
                                    .color(egui::Color32::RED),
                                );
                            }
                        }
                    }
                }
            });
        });

        if self.show_add_profile_popup {
            egui::Window::new(self.translator.t("profile_new"))
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(self.translator.t("name"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_profile_name)
                                .hint_text(self.translator.t("profile_name_placeholder"))
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("URL:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_profile_url)
                                .hint_text(self.translator.t("profile_url_placeholder"))
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("API Key:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_profile_api_key)
                                .password(true)
                                .hint_text(self.translator.t("profile_api_key_placeholder")),
                        );
                    });
                    ui.horizontal(|ui| {
                        if ui.button(self.translator.t("cancel")).clicked() {
                            self.show_add_profile_popup = false;
                            self.new_profile_name.clear();
                            self.new_profile_url.clear();
                            self.new_profile_api_key.clear();
                        }
                        if ui.button(self.translator.t("add")).clicked() {
                            if !self.new_profile_name.is_empty() && !self.new_profile_url.is_empty()
                            {
                                self.config.add_profile(
                                    self.new_profile_name.clone(),
                                    self.new_profile_url.clone(),
                                    self.new_profile_api_key.clone(),
                                );
                                self.config.current_profile = Some(self.new_profile_name.clone());
                                self.save_config();
                                self.show_add_profile_popup = false;
                                self.new_profile_name.clear();
                                self.new_profile_url.clear();
                                self.new_profile_api_key.clear();
                            }
                        }
                    });
                });
        }

        if self.show_api_key_popup {
            egui::Window::new("API Key")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("API Key :");
                        ui.monospace(&self.new_profile_api_key);
                        if ui.small_button("📋").clicked() {
                            ui.ctx().copy_text(self.new_profile_api_key.clone().into());
                        }
                    });
                    if ui.button(self.translator.t("close")).clicked() {
                        self.show_api_key_popup = false;
                    }
                });
        }
    }
}