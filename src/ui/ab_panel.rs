//! AbPanel — A/B protocol comparison test runner.

use crate::sim::ab_test::{AbTestConfig, AbResult, run_ab_test};
use crate::sim::plugin_registry::PluginRegistry;
use crate::config::SimSession;

/// Panel for running and displaying A/B protocol comparisons.
pub struct AbPanel {
    pub config: AbTestConfig,
    pub result_a: Option<AbResult>,
    pub result_b: Option<AbResult>,
    pub running: bool,
}

impl AbPanel {
    pub fn new() -> Self {
        Self {
            config: AbTestConfig::default(),
            result_a: None,
            result_b: None,
            running: false,
        }
    }

    /// Render the A/B test panel.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        registry: &PluginRegistry,
        session: &SimSession,
    ) {
        ui.heading("⚖ A/B Protocol Test");
        ui.separator();

        let names = registry.names();
        let n = names.len();

        if n == 0 {
            ui.label("No protocols registered.");
            return;
        }

        // Protocol A selector
        ui.horizontal(|ui| {
            ui.label("Protocol A:");
            let cur_a = self.config.protocol_a_index.min(n - 1);
            egui::ComboBox::from_id_source("proto_a")
                .selected_text(names.get(cur_a).copied().unwrap_or("?"))
                .show_ui(ui, |ui| {
                    for (i, name) in names.iter().enumerate() {
                        if ui.selectable_label(cur_a == i, *name).clicked() {
                            self.config.protocol_a_index = i;
                        }
                    }
                });
        });

        // Protocol B selector
        ui.horizontal(|ui| {
            ui.label("Protocol B:");
            let cur_b = self.config.protocol_b_index.min(n - 1);
            egui::ComboBox::from_id_source("proto_b")
                .selected_text(names.get(cur_b).copied().unwrap_or("?"))
                .show_ui(ui, |ui| {
                    for (i, name) in names.iter().enumerate() {
                        if ui.selectable_label(cur_b == i, *name).clicked() {
                            self.config.protocol_b_index = i;
                        }
                    }
                });
        });

        // N runs slider
        let mut n_runs = self.config.n_runs as f32;
        if ui.add(egui::Slider::new(&mut n_runs, 1.0..=50.0).step_by(1.0).text("N runs")).changed() {
            self.config.n_runs = n_runs as u32;
        }

        // Ticks per run
        let mut tpr = self.config.ticks_per_run as f32;
        if ui.add(egui::Slider::new(&mut tpr, 100.0..=5000.0).step_by(100.0).text("Ticks / run")).changed() {
            self.config.ticks_per_run = tpr as u64;
        }

        ui.checkbox(&mut self.config.same_seed, "Same seed (reproducible)");

        ui.separator();

        if !self.running {
            if ui.add(
                egui::Button::new(
                    egui::RichText::new("▶ Run A/B Test").color(egui::Color32::from_rgb(80, 220, 80))
                )
            ).clicked() {
                self.running = true;
                let (ra, rb) = run_ab_test(&self.config, session, registry);
                self.result_a = Some(ra);
                self.result_b = Some(rb);
                self.running = false;
            }
        } else {
            ui.label("Running…");
        }

        // Results table
        if let (Some(ra), Some(rb)) = (&self.result_a, &self.result_b) {
            ui.separator();
            ui.label("Results:");

            egui::Grid::new("ab_results")
                .num_columns(3)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("KPI");
                    ui.label(&ra.protocol_name);
                    ui.label(&rb.protocol_name);
                    ui.end_row();

                    kpi_row(ui, "Block Rate", ra.mean_block_rate, rb.mean_block_rate, true, |v| format!("{:.2}%", v * 100.0));
                    kpi_row(ui, "HO Success Rate", ra.mean_ho_success_rate, rb.mean_ho_success_rate, false, |v| format!("{:.2}%", v * 100.0));
                    kpi_row(ui, "Avg SINR (dB)", ra.mean_avg_sinr_db, rb.mean_avg_sinr_db, false, |v| format!("{:.2}", v));
                    kpi_row(ui, "Avg Throughput", ra.mean_avg_throughput_mbps, rb.mean_avg_throughput_mbps, false, |v| format!("{:.2} Mbps", v));
                    kpi_row(ui, "Avg Latency", ra.mean_avg_latency_ms, rb.mean_avg_latency_ms, true, |v| format!("{:.1} ms", v));
                    kpi_row(ui, "95% CI (block)", ra.confidence_interval_95, rb.confidence_interval_95, true, |v| format!("±{:.4}", v));
                });

            // Export button
            if ui.button("📥 Export Results CSV").clicked() {
                if let Ok(path) = std::env::current_dir() {
                    let csv_path = path.join("ab_results.csv");
                    let _ = export_ab_csv(ra, rb, &csv_path);
                }
            }
        }
    }
}

impl Default for AbPanel {
    fn default() -> Self { Self::new() }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn kpi_row(
    ui: &mut egui::Ui,
    label: &str,
    va: f32,
    vb: f32,
    lower_is_better: bool,
    fmt: impl Fn(f32) -> String,
) {
    ui.label(label);

    let a_wins = if lower_is_better { va <= vb } else { va >= vb };
    let b_wins = !a_wins || (va == vb);

    let win_color = egui::Color32::from_rgb(60, 180, 60);
    let norm_color = ui.style().visuals.text_color();

    ui.colored_label(if a_wins { win_color } else { norm_color }, fmt(va));
    ui.colored_label(if b_wins && !a_wins { win_color } else { norm_color }, fmt(vb));
    ui.end_row();
}

fn export_ab_csv(ra: &AbResult, rb: &AbResult, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    writeln!(f, "protocol,mean_block_rate,stddev_block_rate,mean_ho_success_rate,mean_avg_sinr_db,mean_avg_throughput_mbps,mean_avg_latency_ms,ci_95,n_runs")?;
    for r in &[ra, rb] {
        writeln!(f, "{},{},{},{},{},{},{},{},{}", r.protocol_name, r.mean_block_rate, r.stddev_block_rate, r.mean_ho_success_rate, r.mean_avg_sinr_db, r.mean_avg_throughput_mbps, r.mean_avg_latency_ms, r.confidence_interval_95, r.n_runs)?;
    }
    Ok(())
}
