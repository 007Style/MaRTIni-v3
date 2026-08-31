//! StatsPanel — live KPI dashboard.

/// Panel showing live aggregate simulation statistics.
pub struct StatsPanel;

impl StatsPanel {
    /// Render the statistics panel.
    pub fn show(ui: &mut egui::Ui, snapshot: &crate::sim::SimSnapshot) {
        let stats = &snapshot.stats;

        ui.heading("📊 Statistics");
        ui.separator();

        // ── 1. Summary cards (4 columns) ─────────────────────────────────────
        ui.columns(4, |cols| {
            kpi_card(&mut cols[0], "Active Mobiles", &format!("{}", stats.active_mobiles));
            kpi_card(&mut cols[1], "Block Rate", &format!("{:.1}%", stats.block_rate() * 100.0));
            kpi_card(&mut cols[2], "Drop Rate", &format!("{:.1}%", stats.drop_rate() * 100.0));
            kpi_card(&mut cols[3], "HO Success", &format!("{:.1}%", stats.handoff_success_rate() * 100.0));
        });

        ui.add_space(8.0);
        ui.separator();

        // ── 2. KPI table ──────────────────────────────────────────────────────
        ui.label("Average KPIs:");
        egui::Grid::new("kpi_table")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                ui.label("Avg SINR");
                ui.label(format!("{:.2} dB", stats.avg_sinr_db));
                ui.end_row();

                ui.label("Avg Throughput");
                ui.label(format!("{:.2} Mbps", stats.avg_throughput_mbps));
                ui.end_row();

                ui.label("Avg Latency");
                ui.label(format!("{:.1} ms", stats.avg_latency_ms));
                ui.end_row();

                ui.label("SLA Violations");
                ui.label(format!("{}", stats.sla_violations));
                ui.end_row();

                ui.label("Handoff Attempts");
                ui.label(format!("{}", stats.handoff_attempts));
                ui.end_row();

                ui.label("Handoff Successes");
                ui.label(format!("{}", stats.handoff_successes));
                ui.end_row();
            });

        ui.add_space(8.0);
        ui.separator();

        // ── 3. Per-cell load bars ─────────────────────────────────────────────
        ui.label("Base Station Load:");
        for base in &snapshot.bases {
            let load = base.load_percent() / 100.0;
            let label = if base.failed {
                format!("Cell #{} [FAILED]", base.id)
            } else {
                format!("Cell #{}", base.id)
            };
            ui.horizontal(|ui| {
                ui.label(&label);
                let bar = egui::ProgressBar::new(load.clamp(0.0, 1.0))
                    .text(format!("{:.0}%", base.load_percent()))
                    .desired_width(160.0);
                ui.add(bar);
            });
        }

        ui.add_space(8.0);
        ui.separator();

        // ── 4. Traffic profile mix ────────────────────────────────────────────
        ui.label("Traffic Profile Mix:");
        use crate::config::TrafficProfile::*;
        let profiles = [VideoStream, CloudGaming, VoiceCall, Idle, WebBrowse];
        for profile in &profiles {
            let count = snapshot.mobiles.iter().filter(|m| &m.profile == profile).count();
            if count > 0 {
                let [r, g, b] = profile.color();
                ui.horizontal(|ui| {
                    ui.colored_label(
                        egui::Color32::from_rgb(r, g, b),
                        format!("{}: {}", profile.label(), count),
                    );
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn kpi_card(ui: &mut egui::Ui, title: &str, value: &str) {
    egui::Frame::none()
        .inner_margin(egui::Margin::same(6.0))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
        .rounding(egui::Rounding::same(4.0))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(value).strong().size(18.0));
                ui.label(egui::RichText::new(title).size(11.0).color(egui::Color32::from_gray(150)));
            });
        });
}
