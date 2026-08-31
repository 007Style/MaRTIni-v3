//! EventLogPanel — scrollable, filterable list of simulation events.

use std::collections::VecDeque;
use crate::sim::snapshot::{SimEvent, SimEventType};

/// Scrollable event log with filter support.
pub struct EventLogPanel {
    pub filter_text: String,
    pub scroll_to_bottom: bool,
}

impl EventLogPanel {
    pub fn new() -> Self {
        Self {
            filter_text: String::new(),
            scroll_to_bottom: true,
        }
    }

    /// Render the event log panel.
    ///
    /// * `events`          – ring buffer from the snapshot.
    /// * `selected_mobile` – set when user clicks a handoff row.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        events: &VecDeque<SimEvent>,
        selected_mobile: &mut Option<u32>,
    ) {
        ui.heading("📋 Event Log");

        // Filter bar + clear button
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.filter_text);
            if ui.button("Clear filter").clicked() {
                self.filter_text.clear();
            }
            let scroll_label = if self.scroll_to_bottom { "Auto-scroll: ON" } else { "Auto-scroll: OFF" };
            if ui.small_button(scroll_label).clicked() {
                self.scroll_to_bottom = !self.scroll_to_bottom;
            }
        });

        ui.separator();

        let filter = self.filter_text.to_lowercase();

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(self.scroll_to_bottom)
            .show(ui, |ui| {
                // Show newest at top — reverse iterate
                for ev in events.iter().rev() {
                    let detail_lower = ev.detail.to_lowercase();
                    let id_str = ev.mobile_id.to_string();
                    if !filter.is_empty() && !detail_lower.contains(&filter) && !id_str.contains(&filter) {
                        continue;
                    }

                    let (color, icon) = event_style(&ev.event_type);
                    let label = format!(
                        "[{:>6}] {icon} M#{} — {}",
                        ev.tick, ev.mobile_id, ev.detail
                    );

                    let is_clickable = matches!(
                        ev.event_type,
                        SimEventType::HandoffSuccess | SimEventType::HandoffFailure
                    );

                    if is_clickable {
                        let response = ui.add(
                            egui::Label::new(
                                egui::RichText::new(&label).color(color).monospace()
                            ).sense(egui::Sense::click())
                        );
                        if response.clicked() {
                            *selected_mobile = Some(ev.mobile_id);
                        }
                        response.on_hover_text("Click to inspect this mobile");
                    } else {
                        ui.colored_label(color, egui::RichText::new(&label).monospace());
                    }
                }
            });
    }
}

impl Default for EventLogPanel {
    fn default() -> Self { Self::new() }
}

// ---------------------------------------------------------------------------
// Colour + icon per event type
// ---------------------------------------------------------------------------

fn event_style(etype: &SimEventType) -> (egui::Color32, &'static str) {
    match etype {
        SimEventType::Arrival        => (egui::Color32::from_rgb(80, 140, 220), "→"),
        SimEventType::Departure      => (egui::Color32::from_gray(140),         "←"),
        SimEventType::HandoffSuccess => (egui::Color32::from_rgb(60, 180, 60),  "⇄"),
        SimEventType::HandoffFailure => (egui::Color32::from_rgb(220, 80, 80),  "✗"),
        SimEventType::SlaViolation   => (egui::Color32::from_rgb(220, 130, 0),  "⚠"),
        SimEventType::TowerFailure   => (egui::Color32::from_rgb(160, 40, 40),  "⚡"),
        SimEventType::TowerRestore   => (egui::Color32::from_rgb(60, 160, 100), "✓"),
        SimEventType::MobileSurge    => (egui::Color32::from_rgb(120, 80, 200), "↑"),
    }
}
