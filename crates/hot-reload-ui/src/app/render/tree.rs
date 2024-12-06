use super::HotReloadApp;
use eframe::egui::ImageSource;
use eframe::egui;
use std::collections::HashMap;

impl HotReloadApp {
    fn get_file_icon(&self, file: &str) -> &ImageSource<'static> {
        let icons = self.icons.as_ref().unwrap();
        if let Some(ext) = file.split('.').last() {
            match ext.to_lowercase().as_str() {
                "lua" => &icons.lua,
                "js" => &icons.javascript,
                "dll" => &icons.csharp,
                _ => &icons.default,
            }
        } else {
            &icons.default
        }
    }

    pub fn render_tree(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("resources_panel")
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.heading(self.translator.t("resources"));
                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .button(if self.all_checked() {
                            self.translator.t("deselect_all")
                        } else {
                            self.translator.t("select_all")
                        })
                        .clicked()
                    {
                        self.toggle_all_resources();
                    }
                    if ui.button(self.translator.t("debug")).clicked() {
                        self.debug_dump_resources();
                    }
                });
                egui::ScrollArea::vertical()
                    .stick_to_right(true)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        let resources_data = if let Ok(tree) = self.resource_tree.lock() {
                            tree.clone()
                        } else {
                            HashMap::new()
                        };

                        let mut resources: Vec<_> = resources_data.into_iter().collect();
                        resources.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

                        for (resource_name, files) in resources {
                            let mut is_expanded = *self
                                .tree_state
                                .expanded
                                .entry(resource_name.clone())
                                .or_insert(false);
                            let mut is_checked = *self
                                .tree_state
                                .checked
                                .entry(resource_name.clone())
                                .or_insert(true);

                            ui.horizontal(|ui| {
                                let folder_icon = if is_expanded { "📂" } else { "📁" };
                                if ui
                                    .selectable_label(false, folder_icon)
                                    .on_hover_text(self.translator.t("expend_tree"))
                                    .clicked()
                                {
                                    is_expanded = !is_expanded;
                                    self.tree_state
                                        .expanded
                                        .insert(resource_name.clone(), is_expanded);
                                }
                                ui.checkbox(
                                    &mut is_checked,
                                    egui::RichText::new(&resource_name)
                                        .color(egui::Color32::from_rgb(255, 208, 0)),
                                );
                                self.tree_state
                                    .checked
                                    .insert(resource_name.clone(), is_checked);
                            });

                            if is_expanded {
                                ui.indent(resource_name.clone(), |ui| {
                                    for file in files {
                                        let file_id = format!("{}/{}", resource_name, file);
                                        let mut is_file_checked = *self
                                            .tree_state
                                            .checked
                                            .entry(file_id.clone())
                                            .or_insert(true);

                                        ui.horizontal(|ui| {
                                            let icon = self.get_file_icon(&file);
                                            ui.image(icon.clone());

                                            ui.checkbox(
                                                &mut is_file_checked,
                                                egui::RichText::new(&file)
                                                    .color(egui::Color32::from_rgb(255, 208, 0)),
                                            );

                                            self.tree_state
                                                .checked
                                                .insert(file_id.clone(), is_file_checked);
                                        });
                                    }
                                });
                            }
                        }
                    });
            });
    }
}
