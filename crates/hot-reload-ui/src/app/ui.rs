use super::HotReloadApp;
use eframe::egui;
use hot_reload_common::{FileTree, ResourceCategory};

impl HotReloadApp {
    fn render_file_tree(&self, ui: &mut egui::Ui, tree: &FileTree) {
        // Afficher les dossiers
        for (folder, subtree) in &tree.folders {
            ui.collapsing(
                egui::RichText::new(format!("📁 {}", folder))
                    .color(egui::Color32::from_rgb(255, 208, 0)),
                |ui| {
                    ui.indent(folder, |ui| {
                        self.render_file_tree(ui, subtree);
                    });
                }
            );
        }

        // Afficher les fichiers
        for file in &tree.files {
            ui.horizontal(|ui| {
                let icon = if file.ends_with(".lua") {
                    "🌙"
                } else if file.ends_with(".js") {
                    "📜"
                } else if file.ends_with(".dll") {
                    "🔧"
                } else {
                    "📄"
                };
                
                ui.label(format!("{} {}", icon, file));
            });
        }
    }

    fn render_category(&self, ui: &mut egui::Ui, category: &ResourceCategory, category_name: &str) {
        ui.collapsing(
            egui::RichText::new(format!("📂 [{}]", category_name))
                .strong()
                .color(egui::Color32::from_rgb(255, 165, 0)), // Orange pour les catégories
            |ui| {
                // Afficher les sous-catégories
                for (sub_name, sub_category) in &category.categories {
                    self.render_category(ui, sub_category, sub_name);
                }

                // Afficher les ressources de cette catégorie
                for (resource_name, file_tree) in &category.resources {
                    ui.collapsing(
                        egui::RichText::new(format!("📦 {}", resource_name))
                            .strong()
                            .color(egui::Color32::LIGHT_BLUE),
                        |ui| {
                            self.render_file_tree(ui, file_tree);
                        }
                    );
                }
            }
        );
    }

    pub fn render_main_content(&self, ctx: &egui::Context) {
        egui::SidePanel::left("resources_panel")
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.heading("📦 Resources");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    if let Ok(categories) = self.resource_categories.lock() {
                        // Afficher les catégories principales
                        for (category_name, category) in &categories.categories {
                            self.render_category(ui, category, category_name);
                        }

                        // Afficher les ressources sans catégorie (si il y en a)
                        if !categories.resources.is_empty() {
                            ui.collapsing(
                                egui::RichText::new("📂 [autres]")
                                    .strong()
                                    .color(egui::Color32::GRAY),
                                |ui| {
                                    for (resource_name, file_tree) in &categories.resources {
                                        ui.collapsing(
                                            egui::RichText::new(format!("📦 {}", resource_name))
                                                .strong()
                                                .color(egui::Color32::LIGHT_BLUE),
                                            |ui| {
                                                self.render_file_tree(ui, file_tree);
                                            }
                                        );
                                    }
                                }
                            );
                        }
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hot Reload");
            
            if let Some(res_path) = self.resources_path.lock().unwrap().as_ref() {
                ui.horizontal(|ui| {
                    ui.label("📁");
                    ui.label(format!("Dossier Resources: {}", res_path));
                });
            }
        });
    }
}
