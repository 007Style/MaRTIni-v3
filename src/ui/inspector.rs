//! InspectorPanel — per-mobile detail view with SINR/latency charts.

use egui_plot::{Line, Plot, PlotPoints, HLine};

/// Inspector panel shown in the right side panel when a mobile is selected.
pub struct InspectorPanel;

impl InspectorPanel {
    /// Render the inspector for the selected mobile.
    ///
    /// * `selected_id` – id of the mobile to inspect.
    /// * `deselect`    – set to `true` if the user clicks X or presses Escape.
    pub fn show(
        ui: &mut egui::Ui,
        snapshot: &crate::sim::SimSnapshot,
        selected_id: u32,
        deselect: &mut bool,
    ) {
        // ── Find mobile in snapshot ──────────────────────────────────────────
        let mobile = snapshot.mobiles.iter().find(|m| m.id == selected_id);

        if mobile.is_none() {
            ui.vertical(|ui| {
                ui.label(format!("Mobile #{selected_id} no longer active."));
                if ui.button("Dismiss").clicked() {
                    *deselect = true;
                }
            });
            return;
        }
        let m = mobile.unwrap();

        // Escape key deselects
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            *deselect = true;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            // ── 1. Header ────────────────────────────────────────────────────
            ui.horizontal(|ui| {
                ui.heading(format!("📱 Mobile #{}", m.id));
                ui.separator();

                // Profile badge
                let [r, g, b] = m.profile.color();
                let badge_color = egui::Color32::from_rgb(r, g, b);
                ui.colored_label(badge_color, m.profile.label());

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✖").clicked() {
                        *deselect = true;
                    }
                });
            });
            ui.separator();

            // ── 2. Position ──────────────────────────────────────────────────
            let [x, y] = m.position;
            // Estimate grid square assuming equal width/height grid
            let n_streets = snapshot.terrain.streets_v.len().max(1) as f32;
            let block_w = snapshot.terrain.width_m / n_streets;
            let block_x = (x / block_w).floor() as i32;
            let block_y = (y / block_w).floor() as i32;
            ui.label(format!(
                "Position: ({:.1}m, {:.1}m)  •  Grid [{block_x}, {block_y}]",
                x, y
            ));
            ui.add_space(4.0);

            // ── 3. Radio Status ──────────────────────────────────────────────
            egui::CollapsingHeader::new("📶 Radio Status")
                .default_open(true)
                .show(ui, |ui| {
                    // Serving cell
                    if let Some(bid) = m.serving_cell {
                        let tech_label = snapshot.bases.iter()
                            .find(|b| b.id == bid)
                            .map(|b| tech_label(&b.technology))
                            .unwrap_or("?");
                        ui.label(format!("Serving cell: #{bid}  [{tech_label}]"));
                    } else {
                        ui.label("Serving cell: none");
                    }

                    // RSRP
                    let rsrp_color = sinr_color_rsrp(m.rsrp_dbm);
                    ui.colored_label(rsrp_color, format!("RSRP: {:.1} dBm", m.rsrp_dbm));

                    // SINR
                    let sinr_color = sinr_color(m.sinr_db);
                    ui.colored_label(sinr_color, format!("SINR: {:.1} dB", m.sinr_db));

                    // Gen3 active set
                    if !m.active_set.is_empty() {
                        let ids: Vec<String> = m.active_set.iter().map(|id| format!("#{id}")).collect();
                        ui.label(format!("Active set: {}", ids.join(", ")));
                    }

                    // Gen5 secondary cell
                    if let Some(sc) = m.secondary_cell {
                        ui.label(format!("Secondary cell: #{sc}"));
                    }
                });
            ui.add_space(4.0);

            // ── 4. Throughput ────────────────────────────────────────────────
            egui::CollapsingHeader::new("📊 Throughput")
                .default_open(true)
                .show(ui, |ui| {
                    let dl_demand = m.profile.dl_demand_mbps();
                    let ul_demand = m.profile.ul_demand_mbps();

                    let dl_frac = (m.dl_throughput_mbps / dl_demand.max(0.001)).clamp(0.0, 1.0);
                    let ul_frac = (m.ul_throughput_mbps / ul_demand.max(0.001)).clamp(0.0, 1.0);

                    ui.label(format!("DL: {:.2}/{:.2} Mbps", m.dl_throughput_mbps, dl_demand));
                    ui.add(egui::ProgressBar::new(dl_frac).desired_width(200.0));

                    ui.label(format!("UL: {:.2}/{:.2} Mbps", m.ul_throughput_mbps, ul_demand));
                    ui.add(egui::ProgressBar::new(ul_frac).desired_width(200.0));
                });
            ui.add_space(4.0);

            // ── 5. Latency & SLA ─────────────────────────────────────────────
            egui::CollapsingHeader::new("⏱ Latency")
                .default_open(true)
                .show(ui, |ui| {
                    let budget = m.profile.latency_budget_ms();
                    ui.label(format!("Latency: {:.1}ms  /  Budget: {:.1}ms", m.latency_ms, budget));

                    if m.sla_violated {
                        ui.colored_label(egui::Color32::from_rgb(220, 80, 80), "❌ SLA VIOLATED");
                    } else {
                        ui.colored_label(egui::Color32::from_rgb(60, 180, 60), "✅ SLA MET");
                    }
                });
            ui.add_space(4.0);

            // ── 6. Battery ───────────────────────────────────────────────────
            egui::CollapsingHeader::new("🔋 Battery")
                .default_open(true)
                .show(ui, |ui| {
                    let bat = m.battery_percent / 100.0;
                    let bar = egui::ProgressBar::new(bat)
                        .text(format!("{:.1}%", m.battery_percent))
                        .desired_width(200.0);
                    ui.add(bar);
                });
            ui.add_space(4.0);

            // ── 7. SINR chart ────────────────────────────────────────────────
            egui::CollapsingHeader::new("📈 SINR History")
                .default_open(true)
                .show(ui, |ui| {
                    let sinr_vals: Vec<f64> = m.sinr_history.iter().copied().map(|v| v as f64).collect();
                    let n = sinr_vals.len();
                    if n > 0 {
                        let avg = sinr_vals.iter().sum::<f64>() / n as f64;
                        let line_color = if avg > 5.0 {
                            egui::Color32::from_rgb(60, 180, 60)
                        } else if avg >= 0.0 {
                            egui::Color32::from_rgb(210, 160, 0)
                        } else {
                            egui::Color32::from_rgb(220, 80, 80)
                        };
                        let points: PlotPoints = sinr_vals.iter().rev().enumerate()
                            .map(|(i, &v)| [(i as f64), v])
                            .collect();
                        Plot::new(format!("sinr_chart_{}", m.id))
                            .height(90.0)
                            .allow_zoom(false)
                            .allow_drag(false)
                            .show(ui, |plot_ui| {
                                plot_ui.line(Line::new(points).color(line_color).name("SINR dB"));
                            });
                    } else {
                        ui.label("No SINR history yet.");
                    }
                });
            ui.add_space(4.0);

            // ── 8. Latency chart ─────────────────────────────────────────────
            egui::CollapsingHeader::new("📉 Latency History")
                .default_open(true)
                .show(ui, |ui| {
                    let lat_vals: Vec<f64> = m.latency_history.iter().copied().map(|v| v as f64).collect();
                    let budget_y = m.profile.latency_budget_ms() as f64;
                    if !lat_vals.is_empty() {
                        let points: PlotPoints = lat_vals.iter().rev().enumerate()
                            .map(|(i, &v)| [(i as f64), v])
                            .collect();
                        Plot::new(format!("lat_chart_{}", m.id))
                            .height(90.0)
                            .allow_zoom(false)
                            .allow_drag(false)
                            .show(ui, |plot_ui| {
                                plot_ui.line(Line::new(points)
                                    .color(egui::Color32::from_rgb(100, 160, 255))
                                    .name("Latency ms"));
                                plot_ui.hline(HLine::new(budget_y)
                                    .color(egui::Color32::from_rgb(220, 80, 80))
                                    .name("Budget"));
                            });
                    } else {
                        ui.label("No latency history yet.");
                    }
                });
            ui.add_space(4.0);

            // ── 9. Handoff count ─────────────────────────────────────────────
            ui.label(format!("Handoffs: {}", m.handoff_count));
        });
    }
}

// ---------------------------------------------------------------------------
// Colour helpers
// ---------------------------------------------------------------------------

fn sinr_color(sinr: f32) -> egui::Color32 {
    if sinr > 10.0 {
        egui::Color32::from_rgb(60, 180, 60)
    } else if sinr >= 0.0 {
        egui::Color32::from_rgb(210, 160, 0)
    } else {
        egui::Color32::from_rgb(220, 80, 80)
    }
}

fn sinr_color_rsrp(rsrp: f32) -> egui::Color32 {
    if rsrp > -80.0 {
        egui::Color32::from_rgb(60, 180, 60)
    } else if rsrp >= -100.0 {
        egui::Color32::from_rgb(210, 160, 0)
    } else {
        egui::Color32::from_rgb(220, 80, 80)
    }
}

fn tech_label(tech: &crate::config::RadioTechnology) -> &'static str {
    use crate::config::RadioTechnology::*;
    match tech {
        Gen3Umts      => "3G UMTS",
        Gen4Lte       => "4G LTE",
        Gen5NrSub6    => "5G NR Sub-6",
        Gen5NrMmWave  => "5G NR mmWave",
    }
}
