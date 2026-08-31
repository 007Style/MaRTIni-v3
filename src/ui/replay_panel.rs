//! ReplayPanel — frame-by-frame playback controls for recorded simulation runs.

use std::time::Instant;
use crate::sim::replay::{ReplayFile, ReplayFrame};
use crate::sim::snapshot::SimSnapshot;
use std::sync::Arc;

/// Panel providing playback controls for a recorded simulation.
pub struct ReplayPanel {
    pub replay_file: Option<ReplayFile>,
    pub current_frame: usize,
    pub playing: bool,
    pub play_speed: f32,
    last_advance: Instant,
    pub path_input: String,
    pub load_error: Option<String>,
}

impl ReplayPanel {
    pub fn new() -> Self {
        Self {
            replay_file: None,
            current_frame: 0,
            playing: false,
            play_speed: 1.0,
            last_advance: Instant::now(),
            path_input: String::new(),
            load_error: None,
        }
    }

    /// Render the replay panel.
    ///
    /// Returns `Some(snapshot)` if a frame is available to display.
    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<crate::sim::SimSnapshot> {
        ui.heading("⏪ Replay");
        ui.separator();

        // ── File loader ──────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label("Path:");
            ui.text_edit_singleline(&mut self.path_input);
        });
        if ui.button("📂 Load").clicked() {
            // Clone to avoid borrow conflict
            let path_str = self.path_input.clone();
            let path = std::path::Path::new(&path_str);
            match self.load(path) {
                Ok(_)  => self.load_error = None,
                Err(e) => self.load_error = Some(e.to_string()),
            }
        }

        if let Some(err) = &self.load_error {
            ui.colored_label(egui::Color32::from_rgb(220, 80, 80), format!("Error: {err}"));
        }

        // Early out if no file loaded — drop borrow before accessing self mutably
        if self.replay_file.is_none() {
            ui.label("No recording loaded.");
            return None;
        }

        let total_frames = self.replay_file.as_ref().map(|r| r.frames.len()).unwrap_or(0);
        if total_frames == 0 {
            ui.label("Recording is empty.");
            return None;
        }

        // ── Info row ─────────────────────────────────────────────────────────
        let tick = self.replay_file.as_ref()
            .and_then(|r| r.frames.get(self.current_frame))
            .map(|f| f.tick)
            .unwrap_or(0);
        let proto_name = self.replay_file.as_ref()
            .map(|r| r.protocol_name.clone())
            .unwrap_or_default();
        ui.label(format!("Protocol: {proto_name}  •  Tick {tick} / frame {}/{total_frames}",
            self.current_frame + 1));

        // ── Playback controls ────────────────────────────────────────────────
        ui.horizontal(|ui| {
            if ui.button("⏮").clicked() { self.current_frame = 0; self.playing = false; }
            if ui.button("⏪").clicked() { self.step_back(); self.playing = false; }

            let play_label = if self.playing { "⏸" } else { "▶" };
            if ui.button(play_label).clicked() { self.playing = !self.playing; }

            if ui.button("⏩").clicked() { self.step_forward(total_frames); self.playing = false; }
            if ui.button("⏭").clicked() { self.current_frame = total_frames.saturating_sub(1); self.playing = false; }
        });

        // Speed selector
        ui.horizontal(|ui| {
            ui.label("Speed:");
            for (label, speed) in &[("0.5×", 0.5_f32), ("1×", 1.0), ("2×", 2.0), ("5×", 5.0)] {
                if ui.selectable_label((self.play_speed - speed).abs() < 0.01, *label).clicked() {
                    self.play_speed = *speed;
                }
            }
        });

        // Scrubber slider
        let mut frame_idx = self.current_frame;
        if ui.add(egui::Slider::new(&mut frame_idx, 0..=(total_frames.saturating_sub(1)))
            .text("Frame")).changed() {
            self.current_frame = frame_idx;
            self.playing = false;
        }

        // Advance frame if playing
        if self.playing {
            let interval_ms = (100.0 / self.play_speed) as u64;
            if self.last_advance.elapsed().as_millis() as u64 >= interval_ms {
                self.step_forward(total_frames);
                self.last_advance = Instant::now();
                ui.ctx().request_repaint();
            } else {
                ui.ctx().request_repaint();
            }
        }

        // Build snapshot from current frame — all borrows resolved above
        self.build_snapshot_for_current_frame()
    }

    /// Load a recording from disk.
    pub fn load(&mut self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let file = crate::sim::replay::ReplayRecorder::load(path)?;
        self.replay_file = Some(file);
        self.current_frame = 0;
        self.playing = false;
        Ok(())
    }

    fn step_forward(&mut self, total: usize) {
        if self.current_frame + 1 < total {
            self.current_frame += 1;
        } else {
            self.playing = false;
        }
    }

    fn step_back(&mut self) {
        self.current_frame = self.current_frame.saturating_sub(1);
    }

    fn build_snapshot_for_current_frame(&self) -> Option<SimSnapshot> {
        let replay = self.replay_file.as_ref()?;
        let frame = replay.frames.get(self.current_frame)?;
        let terrain = Arc::new(crate::terrain::TerrainMap::generate(
            &replay.session.terrain,
            &replay.session.grid,
        ));
        Some(SimSnapshot {
            tick: frame.tick,
            mobiles: frame.mobiles.clone(),
            bases: frame.bases.clone(),
            stats: frame.stats.clone(),
            events: std::collections::VecDeque::new(),
            terrain,
        })
    }
}

impl Default for ReplayPanel {
    fn default() -> Self { Self::new() }
}
