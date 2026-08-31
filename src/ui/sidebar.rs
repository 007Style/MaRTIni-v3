//! Sidebar — left navigation panel with simulation controls and configuration shortcuts.

/// Left sidebar panel with start/stop controls and navigation.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct Sidebar;

impl Sidebar {
    /// Draw the sidebar into the given egui Ui.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        // Stub — full controls in Sub-Task 8.
        ui.label("Sidebar (stub)");
    }
}
