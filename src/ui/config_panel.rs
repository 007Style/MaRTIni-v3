//! ConfigPanel — editable configuration forms for all SimSession fields.

use crate::config::{RadioTechnology, TerrainType};

/// Panel providing egui form widgets for every SimSession configuration field.
pub struct ConfigPanel;

impl ConfigPanel {
    /// Render the configuration panel.
    ///
    /// * `session` – mutable session; edits take effect on next sim restart.
    /// * `dirty`   – set to `true` whenever any value changes.
    pub fn show(
        ui: &mut egui::Ui,
        session: &mut crate::config::SimSession,
        dirty: &mut bool,
    ) {
        ui.heading("⚙ Configuration");
        if *dirty {
            ui.colored_label(
                egui::Color32::from_rgb(220, 140, 0),
                "⚠ Config changed — restart simulation to apply",
            );
        }
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // ── Grid ─────────────────────────────────────────────────────────
            egui::CollapsingHeader::new("🗺 Grid")
                .default_open(true)
                .show(ui, |ui| {
                    let mut no_block = session.grid.no_block as f32;
                    if ui.add(egui::Slider::new(&mut no_block, 1.0..=30.0)
                        .step_by(1.0)
                        .text("City blocks")).changed() {
                        session.grid.no_block = no_block as u32;
                        *dirty = true;
                    }

                    let mut block_size = session.grid.block_size as f32;
                    if ui.add(egui::Slider::new(&mut block_size, 100.0..=1000.0)
                        .step_by(50.0)
                        .text("Block size (m)")).changed() {
                        session.grid.block_size = block_size as u32;
                        *dirty = true;
                    }

                    let mut dist_res = session.grid.dist_res as f32;
                    if ui.add(egui::Slider::new(&mut dist_res, 10.0..=200.0)
                        .step_by(10.0)
                        .text("Dist resolution (m)")).changed() {
                        session.grid.dist_res = dist_res as u32;
                        *dirty = true;
                    }
                });

            ui.add_space(4.0);

            // ── Speed ─────────────────────────────────────────────────────────
            egui::CollapsingHeader::new("🚗 Speed")
                .default_open(true)
                .show(ui, |ui| {
                    if ui.add(egui::Slider::new(&mut session.speed.min_speed_kmh, 1.0..=50.0)
                        .text("Min speed (km/h)")).changed() { *dirty = true; }
                    if ui.add(egui::Slider::new(&mut session.speed.max_speed_kmh, 10.0..=200.0)
                        .text("Max speed (km/h)")).changed() { *dirty = true; }
                    if ui.add(egui::Slider::new(&mut session.speed.mean_speed_kmh, 1.0..=120.0)
                        .text("Mean speed (km/h)")).changed() { *dirty = true; }
                    if ui.add(egui::Slider::new(&mut session.speed.sigma_speed, 0.1..=30.0)
                        .text("Sigma (km/h)")).changed() { *dirty = true; }
                    if ui.add(egui::Slider::new(&mut session.speed.prob_ahead, 0.0..=1.0)
                        .text("Prob ahead")).changed() { *dirty = true; }
                });

            ui.add_space(4.0);

            // ── Radio ─────────────────────────────────────────────────────────
            egui::CollapsingHeader::new("📡 Radio")
                .default_open(true)
                .show(ui, |ui| {
                    // Technology dropdown
                    let techs = [
                        RadioTechnology::Gen3Umts,
                        RadioTechnology::Gen4Lte,
                        RadioTechnology::Gen5NrSub6,
                        RadioTechnology::Gen5NrMmWave,
                    ];
                    let tech_labels = ["3G UMTS", "4G LTE", "5G NR Sub-6", "5G NR mmWave"];
                    let current = techs.iter().position(|t| t == &session.radio.technology).unwrap_or(1);
                    egui::ComboBox::from_label("Technology")
                        .selected_text(tech_labels[current])
                        .show_ui(ui, |ui| {
                            for (i, label) in tech_labels.iter().enumerate() {
                                if ui.selectable_label(current == i, *label).clicked() {
                                    session.radio.technology = techs[i].clone();
                                    *dirty = true;
                                }
                            }
                        });

                    let mut no_base = session.radio.no_base as f32;
                    if ui.add(egui::Slider::new(&mut no_base, 1.0..=30.0)
                        .step_by(1.0)
                        .text("Base stations")).changed() {
                        session.radio.no_base = no_base as u32;
                        *dirty = true;
                    }

                    let mut no_channel = session.radio.no_channel as f32;
                    if ui.add(egui::Slider::new(&mut no_channel, 1.0..=100.0)
                        .step_by(1.0)
                        .text("Channels / base")).changed() {
                        session.radio.no_channel = no_channel as u32;
                        *dirty = true;
                    }

                    if ui.add(egui::Slider::new(&mut session.radio.bandwidth_mhz, 1.0..=100.0)
                        .text("Bandwidth (MHz)")).changed() { *dirty = true; }
                });

            ui.add_space(4.0);

            // ── Terrain ───────────────────────────────────────────────────────
            egui::CollapsingHeader::new("🏙 Terrain")
                .default_open(true)
                .show(ui, |ui| {
                    let terrain_types = [
                        TerrainType::UrbanGrid,
                        TerrainType::Suburban,
                        TerrainType::Rural,
                        TerrainType::Highway,
                    ];
                    let terrain_labels = ["Urban Grid", "Suburban", "Rural", "Highway"];
                    let current = terrain_types.iter()
                        .position(|t| t == &session.terrain.terrain_type)
                        .unwrap_or(0);
                    egui::ComboBox::from_label("Terrain type")
                        .selected_text(terrain_labels[current])
                        .show_ui(ui, |ui| {
                            for (i, label) in terrain_labels.iter().enumerate() {
                                if ui.selectable_label(current == i, *label).clicked() {
                                    session.terrain.terrain_type = terrain_types[i].clone();
                                    *dirty = true;
                                }
                            }
                        });

                    if ui.add(egui::Slider::new(&mut session.terrain.tower_spacing_m, 100.0..=2000.0)
                        .text("Tower spacing (m)")).changed() { *dirty = true; }

                    let mut seed_str = session.terrain.seed.to_string();
                    ui.horizontal(|ui| {
                        ui.label("Seed:");
                        if ui.text_edit_singleline(&mut seed_str).changed() {
                            if let Ok(v) = seed_str.parse::<u64>() {
                                session.terrain.seed = v;
                                *dirty = true;
                            }
                        }
                    });
                });

            ui.add_space(4.0);

            // ── Simulation ────────────────────────────────────────────────────
            egui::CollapsingHeader::new("🎮 Simulation")
                .default_open(true)
                .show(ui, |ui| {
                    let mut count = session.target_mobile_count as f32;
                    if ui.add(egui::Slider::new(&mut count, 1.0..=100.0)
                        .step_by(1.0)
                        .text("Target mobiles")).changed() {
                        session.target_mobile_count = count as u32;
                        *dirty = true;
                    }
                });
        });
    }
}
