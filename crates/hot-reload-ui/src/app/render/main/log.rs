use super::HotReloadApp;
use eframe::egui;

impl HotReloadApp {
    pub fn render_log(&mut self, ui: &mut egui::Ui) {
        // Not used yet?!
        if let Ok(mut pending) = self.pending_messages.lock() {
            if !pending.is_empty() {
                ui.group(|ui| {
                    ui.heading("Messages en temps réel");
                    for message in pending.drain(..) {
                        // Parser le message pour gérer les batches
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&message) {
                            if json["type"] == "batch" {
                                if let Some(messages) = json["messages"].as_array() {
                                    for msg in messages {
                                        if let Some(fivem_msg) = msg["message"].as_str() {
                                            ui.label(egui::RichText::new(fivem_msg)
                                                .color(egui::Color32::from_rgb(0, 255, 0)));
                                            
                                            // Ajouter aux logs historiques
                                            if let Ok(mut logs) = self.logs.lock() {
                                                if logs.len() >= 100 {
                                                    logs.pop_front();
                                                }
                                                logs.push_back(fivem_msg.to_string());
                                            }
                                        }
                                    }
                                }
                            } else if json["type"] == "fivem_response" {
                                if let Some(message) = json["message"].as_str() {
                                    ui.label(egui::RichText::new(message)
                                        .color(egui::Color32::from_rgb(0, 255, 0)));
                                }
                            }
                        }
                    }
                });
                ui.separator();
            }
        }

        ui.heading("Logs");
        egui::ScrollArea::vertical()
            .max_height(ui.available_height() - 20.0)
            .show(ui, |ui| {
                if let Ok(logs) = self.logs.lock() {
                    for log in logs.iter() {
                        ui.label(log);
                    }
                }
            });
    }
}