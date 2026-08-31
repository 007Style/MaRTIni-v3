//! Top-level application state and eframe::App implementation.

use std::sync::mpsc;

use crate::config::SimSession;
use crate::sim::engine::SimEngine;
use crate::sim::snapshot::SimSnapshot;
use crate::sim::plugin_registry::PluginRegistry;
use crate::sim::replay::ReplayRecorder;
use crate::sim::scenario::Scenario;
use crate::ui::map_panel::MapPanel;
use crate::ui::inspector::InspectorPanel;
use crate::ui::stats_panel::StatsPanel;
use crate::ui::config_panel::ConfigPanel;
use crate::ui::event_log::EventLogPanel;
use crate::ui::ab_panel::AbPanel;
use crate::ui::replay_panel::ReplayPanel;

// ---------------------------------------------------------------------------
// SimState
// ---------------------------------------------------------------------------

/// Simulation lifecycle state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SimState {
    #[default]
    Idle,
    Running,
    Stopped,
}

// ---------------------------------------------------------------------------
// NavPanel
// ---------------------------------------------------------------------------

/// Which main panel is currently displayed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavPanel {
    Map,
    Stats,
    Config,
    EventLog,
    AbTest,
    Replay,
}

// ---------------------------------------------------------------------------
// MaRTIniApp
// ---------------------------------------------------------------------------

/// Root application struct held by eframe.
pub struct MaRTIniApp {
    // ── Sim state ──────────────────────────────────────────────────────────
    pub sim_state: SimState,
    pub snapshot: Option<SimSnapshot>,
    pub sim_rx: Option<mpsc::Receiver<SimSnapshot>>,
    pub stop_tx: Option<mpsc::Sender<()>>,
    pub selected_mobile: Option<u32>,

    // ── Panels ─────────────────────────────────────────────────────────────
    pub map_panel: MapPanel,
    pub event_log: EventLogPanel,
    pub ab_panel: AbPanel,
    pub replay_panel: ReplayPanel,

    // ── Config & session ───────────────────────────────────────────────────
    pub session: SimSession,
    pub config_dirty: bool,

    // ── Protocol & registry ────────────────────────────────────────────────
    pub registry: PluginRegistry,
    pub active_protocol_index: usize,

    // ── Navigation ─────────────────────────────────────────────────────────
    pub active_panel: NavPanel,

    // ── Scenario ───────────────────────────────────────────────────────────
    pub scenario: Option<Scenario>,
    pub scenario_path: String,

    // ── Recording ──────────────────────────────────────────────────────────
    pub recorder: ReplayRecorder,

    // ── About dialog ───────────────────────────────────────────────────────
    pub show_about: bool,
}

impl Default for MaRTIniApp {
    fn default() -> Self {
        let mut registry = PluginRegistry::new();
        // Load any declarative protocols from the plugins/ directory.
        let plugins_dir = std::path::Path::new("plugins");
        registry.load_directory(plugins_dir);

        Self {
            sim_state: SimState::Idle,
            snapshot: None,
            sim_rx: None,
            stop_tx: None,
            selected_mobile: None,
            map_panel: MapPanel::new(),
            event_log: EventLogPanel::new(),
            ab_panel: AbPanel::new(),
            replay_panel: ReplayPanel::new(),
            session: SimSession::default(),
            config_dirty: false,
            registry,
            active_protocol_index: 1, // Default: Gen4 LTE A3 (index 1)
            active_panel: NavPanel::Map,
            scenario: None,
            scenario_path: String::new(),
            recorder: ReplayRecorder::new(),
            show_about: false,
        }
    }
}

// ---------------------------------------------------------------------------
// eframe::App impl
// ---------------------------------------------------------------------------

impl eframe::App for MaRTIniApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Poll latest snapshot from sim thread ─────────────────────────────
        if let Some(rx) = &self.sim_rx {
            while let Ok(snap) = rx.try_recv() {
                // Feed the recorder if active
                self.recorder.record(&snap);
                self.snapshot = Some(snap);
            }
            if self.sim_state == SimState::Running {
                ctx.request_repaint();
            }
        }

        // ── About dialog ─────────────────────────────────────────────────────
        if self.show_about {
            egui::Window::new("About MaRTIni v3")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("MaRTIni v3 — Wireless Network Research Platform");
                    ui.label("WINLab, Rutgers University");
                    ui.separator();
                    ui.label("Authors:");
                    ui.label("  • Daneyand Singley");
                    ui.label("  • Roland Wunderlich");
                    ui.label("  • Ramnath Ravindran");
                    ui.separator();
                    ui.label("Project lineage:");
                    ui.label("  v1 (1999) – Java AWT");
                    ui.label("  v2 (2023) – Java 22 Swing");
                    ui.label("  v3 (2024) – Rust + egui");
                    ui.separator();
                    ui.label("Keyboard shortcuts:");
                    ui.label("  Escape — deselect mobile");
                    ui.label("  S       — Start simulation");
                    ui.label("  X       — Stop simulation");
                    ui.label("  R       — Reset simulation");
                    if ui.button("Close").clicked() {
                        self.show_about = false;
                    }
                });
        }

        // ── Menu bar ─────────────────────────────────────────────────────────
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // File menu
                ui.menu_button("File", |ui| {
                    if ui.button("New Session").clicked() {
                        self.session = SimSession::default();
                        self.config_dirty = false;
                        ui.close_menu();
                    }
                    if ui.button("Save Config…").clicked() {
                        if let Ok(dir) = std::env::current_dir() {
                            let p = dir.join("session.toml");
                            let _ = self.session.save_toml(&p);
                        }
                        ui.close_menu();
                    }
                    if ui.button("Load Config…").clicked() {
                        if let Ok(dir) = std::env::current_dir() {
                            let p = dir.join("session.toml");
                            if let Ok(s) = SimSession::load_toml(&p) {
                                self.session = s;
                                self.config_dirty = false;
                            }
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Open Scenario…").clicked() {
                        let p = std::path::Path::new(&self.scenario_path);
                        if let Ok(s) = Scenario::from_toml(p) {
                            self.scenario = Some(s);
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Export KPIs…").clicked() {
                        if let Ok(dir) = std::env::current_dir() {
                            let path = dir.join("kpis.csv");
                            let _ = crate::sim::statistics::export_kpi_csv(
                                &self.recorder.frames, &path
                            );
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                // Simulation menu
                ui.menu_button("Simulation", |ui| {
                    if ui.add_enabled(self.sim_state != SimState::Running, egui::Button::new("▶ Start")).clicked() {
                        self.start_sim();
                        ui.close_menu();
                    }
                    if ui.add_enabled(self.sim_state == SimState::Running, egui::Button::new("■ Stop")).clicked() {
                        self.stop_sim();
                        self.sim_state = SimState::Stopped;
                        ui.close_menu();
                    }
                    if ui.button("↺ Reset").clicked() {
                        self.stop_sim();
                        self.snapshot = None;
                        self.selected_mobile = None;
                        self.sim_state = SimState::Idle;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.add_enabled(!self.recorder.recording, egui::Button::new("⏺ Start Recording")).clicked() {
                        self.recorder.start();
                        ui.close_menu();
                    }
                    if ui.add_enabled(self.recorder.recording, egui::Button::new("⏹ Stop Recording")).clicked() {
                        self.recorder.stop();
                        ui.close_menu();
                    }
                    if ui.button("💾 Save Recording…").clicked() {
                        if let Ok(dir) = std::env::current_dir() {
                            let path = dir.join("recording.json");
                            let proto_name = self.registry.get(self.active_protocol_index)
                                .map(|p| p.name().to_string())
                                .unwrap_or_default();
                            let _ = self.recorder.save(&path, &self.session, &proto_name);
                        }
                        ui.close_menu();
                    }
                    if ui.button("📂 Open Recording…").clicked() {
                        self.active_panel = NavPanel::Replay;
                        ui.close_menu();
                    }
                });

                // Protocol menu
                ui.menu_button("Protocol", |ui| {
                    let names = self.registry.names();
                    for (i, name) in names.iter().enumerate() {
                        if ui.radio(self.active_protocol_index == i, *name).clicked() {
                            self.active_protocol_index = i;
                        }
                    }
                });

                // View menu
                ui.menu_button("View", |ui| {
                    if ui.button(if self.map_panel.show_heatmap { "✓ Heatmap" } else { "  Heatmap" }).clicked() {
                        self.map_panel.show_heatmap = !self.map_panel.show_heatmap;
                        self.map_panel.heatmap_cache = None;
                        ui.close_menu();
                    }
                    if ui.button(if self.map_panel.show_trails { "✓ Trails" } else { "  Trails" }).clicked() {
                        self.map_panel.show_trails = !self.map_panel.show_trails;
                        ui.close_menu();
                    }
                    if ui.button(if self.map_panel.show_interference { "✓ Interference" } else { "  Interference" }).clicked() {
                        self.map_panel.show_interference = !self.map_panel.show_interference;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Event Log").clicked() {
                        self.active_panel = NavPanel::EventLog;
                        ui.close_menu();
                    }
                });

                // Help menu
                ui.menu_button("Help", |ui| {
                    if ui.button("About MaRTIni v3…").clicked() {
                        self.show_about = true;
                        ui.close_menu();
                    }
                });
            });
        });

        // ── Left sidebar (180px) ──────────────────────────────────────────────
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(190.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.heading("MaRTIni v3");
                    ui.small("WINLab, Rutgers");
                    ui.separator();

                    // Navigation buttons
                    for (label, panel) in &[
                        ("🗺  Map",       NavPanel::Map),
                        ("📊 Stats",      NavPanel::Stats),
                        ("⚙  Config",    NavPanel::Config),
                        ("📋 Event Log",  NavPanel::EventLog),
                        ("⚖  A/B Test", NavPanel::AbTest),
                        ("⏪ Replay",     NavPanel::Replay),
                    ] {
                        let active = &self.active_panel == panel;
                        if ui.add(egui::SelectableLabel::new(active, *label)).clicked() {
                            self.active_panel = panel.clone();
                        }
                    }

                    ui.separator();

                    // Sim controls
                    ui.horizontal(|ui| {
                        if self.sim_state != SimState::Running {
                            if ui.add(egui::Button::new(
                                egui::RichText::new("▶").color(egui::Color32::from_rgb(80, 220, 80))
                            )).on_hover_text("Start (S)").clicked() {
                                self.start_sim();
                            }
                        }
                        if self.sim_state == SimState::Running {
                            if ui.add(egui::Button::new(
                                egui::RichText::new("■").color(egui::Color32::from_rgb(220, 80, 80))
                            )).on_hover_text("Stop (X)").clicked() {
                                self.stop_sim();
                                self.sim_state = SimState::Stopped;
                            }
                        }
                        if ui.add(egui::Button::new(
                            egui::RichText::new("↺").color(egui::Color32::from_rgb(100, 160, 255))
                        )).on_hover_text("Reset (R)").clicked() {
                            self.stop_sim();
                            self.snapshot = None;
                            self.selected_mobile = None;
                            self.sim_state = SimState::Idle;
                        }
                    });

                    ui.separator();

                    // Terrain type shortcut
                    ui.label("Terrain:");
                    let terrain_labels = ["Urban", "Suburban", "Rural", "Highway"];
                    use crate::config::TerrainType::*;
                    let terrain_types = [UrbanGrid, Suburban, Rural, Highway];
                    let cur = terrain_types.iter()
                        .position(|t| t == &self.session.terrain.terrain_type)
                        .unwrap_or(0);
                    egui::ComboBox::from_id_source("sidebar_terrain")
                        .selected_text(terrain_labels[cur])
                        .show_ui(ui, |ui| {
                            for (i, lbl) in terrain_labels.iter().enumerate() {
                                if ui.selectable_label(cur == i, *lbl).clicked() {
                                    self.session.terrain.terrain_type = terrain_types[i].clone();
                                    self.config_dirty = true;
                                }
                            }
                        });

                    // Protocol shortcut
                    ui.label("Protocol:");
                    let names = self.registry.names();
                    let n = names.len();
                    let cur_p = self.active_protocol_index.min(n.saturating_sub(1));
                    egui::ComboBox::from_id_source("sidebar_protocol")
                        .selected_text(names.get(cur_p).copied().unwrap_or("?"))
                        .show_ui(ui, |ui| {
                            for (i, name) in names.iter().enumerate() {
                                if ui.selectable_label(cur_p == i, *name).clicked() {
                                    self.active_protocol_index = i;
                                }
                            }
                        });

                    ui.separator();

                    // State indicator
                    let (state_label, state_color) = match self.sim_state {
                        SimState::Idle    => ("● IDLE",    egui::Color32::from_rgb(150, 150, 150)),
                        SimState::Running => ("● RUNNING", egui::Color32::from_rgb(80, 220, 80)),
                        SimState::Stopped => ("● STOPPED", egui::Color32::from_rgb(220, 80, 80)),
                    };
                    ui.colored_label(state_color, state_label);

                    if self.recorder.recording {
                        ui.colored_label(egui::Color32::from_rgb(220, 80, 80), "⏺ REC");
                    }

                    if let Some(snap) = &self.snapshot {
                        ui.label(format!("Tick: {}", snap.tick));
                        ui.label(format!("Mobiles: {}", snap.stats.active_mobiles));
                    }
                });
            });

        // ── Keyboard shortcuts ────────────────────────────────────────────────
        ctx.input(|i| {
            if i.key_pressed(egui::Key::S) && self.sim_state != SimState::Running {
                self.start_sim();
            }
            if i.key_pressed(egui::Key::X) && self.sim_state == SimState::Running {
                self.stop_sim();
                self.sim_state = SimState::Stopped;
            }
            if i.key_pressed(egui::Key::R) {
                self.stop_sim();
                self.snapshot = None;
                self.selected_mobile = None;
                self.sim_state = SimState::Idle;
            }
        });

        // ── Central panel ────────────────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.active_panel {
                NavPanel::Map => {
                    if let Some(snapshot) = &self.snapshot.clone() {
                        // If a mobile is selected, split into map (left) + inspector (right)
                        if self.selected_mobile.is_some() {
                            let sel_id = self.selected_mobile.unwrap();
                            let available = ui.available_width();
                            let map_width = available * 0.65;
                            ui.horizontal(|ui| {
                                ui.set_width(available);
                                // Map pane
                                ui.allocate_ui(
                                    egui::vec2(map_width, ui.available_height()),
                                    |ui| {
                                        self.map_panel.show(ui, snapshot, &mut self.selected_mobile);
                                    },
                                );
                                // Inspector pane
                                egui::Frame::side_top_panel(ui.style()).show(ui, |ui| {
                                    let mut deselect = false;
                                    InspectorPanel::show(ui, snapshot, sel_id, &mut deselect);
                                    if deselect {
                                        self.selected_mobile = None;
                                    }
                                });
                            });
                        } else {
                            self.map_panel.show(ui, snapshot, &mut self.selected_mobile);
                        }
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label("Simulation not started. Press ▶ Start or S to begin.");
                        });
                    }
                }

                NavPanel::Stats => {
                    if let Some(snapshot) = &self.snapshot.clone() {
                        StatsPanel::show(ui, snapshot);
                    } else {
                        ui.label("No simulation data yet.");
                    }
                }

                NavPanel::Config => {
                    ConfigPanel::show(ui, &mut self.session, &mut self.config_dirty);
                }

                NavPanel::EventLog => {
                    if let Some(snapshot) = &self.snapshot.clone() {
                        self.event_log.show(ui, &snapshot.events, &mut self.selected_mobile);
                        // Selecting a mobile in event log switches to Map view
                        if self.selected_mobile.is_some() {
                            self.active_panel = NavPanel::Map;
                        }
                    } else {
                        ui.label("No events yet. Start the simulation first.");
                    }
                }

                NavPanel::AbTest => {
                    let session = self.session.clone();
                    self.ab_panel.show(ui, &self.registry, &session);
                }

                NavPanel::Replay => {
                    let replay_snap = self.replay_panel.show(ui);
                    // Replay snapshot could feed the map panel if desired.
                    // For now just show stats from the frame in the panel itself.
                    let _ = replay_snap;
                }
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

impl MaRTIniApp {
    fn start_sim(&mut self) {
        self.stop_sim();

        // Build protocol from registry
        // Since we can't send a `&dyn HandoffProtocol` across threads easily,
        // we pass None and let the engine use its built-in Gen4 default,
        // which maps to active_protocol_index == 1. For indices 0 and 2 we
        // pass a boxed native implementation.
        use crate::sim::protocol_native::{Gen3SoftHandoff, Gen4LteA3, Gen5NrCho};
        let protocol: Option<Box<dyn crate::sim::HandoffProtocol>> = match self.active_protocol_index {
            0 => Some(Box::new(Gen3SoftHandoff)),
            1 => Some(Box::new(Gen4LteA3)),
            2 => Some(Box::new(Gen5NrCho)),
            _ => None, // fallback to engine default
        };

        let scenario = self.scenario.clone();

        let (stop_tx, sim_rx) = SimEngine::start(
            self.session.clone(),
            false,
            protocol,
            scenario,
        );
        self.stop_tx = Some(stop_tx);
        self.sim_rx = Some(sim_rx);
        self.sim_state = SimState::Running;
    }

    fn stop_sim(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        self.sim_rx = None;
    }
}
