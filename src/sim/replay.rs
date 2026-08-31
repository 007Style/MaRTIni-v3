//! Replay — record and replay simulation runs frame-by-frame.

use crate::config::SimSession;
use crate::sim::mobile::MobileTerminal;
use crate::sim::base::BaseStation;
use crate::sim::statistics::Statistics;
use crate::sim::snapshot::SimSnapshot;

// ---------------------------------------------------------------------------
// ReplayFrame
// ---------------------------------------------------------------------------

/// A single recorded frame of simulation state.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ReplayFrame {
    /// Simulation tick for this frame.
    pub tick: u64,
    /// Mobile terminal states.
    pub mobiles: Vec<MobileTerminal>,
    /// Base station states.
    pub bases: Vec<BaseStation>,
    /// Aggregate statistics.
    pub stats: Statistics,
}

// ---------------------------------------------------------------------------
// ReplayFile
// ---------------------------------------------------------------------------

/// The on-disk format for a saved simulation recording.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ReplayFile {
    /// Session configuration used for this run.
    pub session: SimSession,
    /// Name of the protocol used.
    pub protocol_name: String,
    /// Ordered frames.
    pub frames: Vec<ReplayFrame>,
}

// ---------------------------------------------------------------------------
// ReplayRecorder
// ---------------------------------------------------------------------------

/// Records simulation snapshots and can save/load them to disk.
pub struct ReplayRecorder {
    pub frames: Vec<ReplayFrame>,
    pub recording: bool,
}

impl ReplayRecorder {
    /// Create a new idle recorder.
    pub fn new() -> Self {
        Self { frames: Vec::new(), recording: false }
    }

    /// Begin recording; clears any previously recorded frames.
    pub fn start(&mut self) {
        self.frames.clear();
        self.recording = true;
    }

    /// Stop recording.
    pub fn stop(&mut self) {
        self.recording = false;
    }

    /// Append a frame from the given snapshot (no-op when not recording).
    pub fn record(&mut self, snapshot: &SimSnapshot) {
        if !self.recording {
            return;
        }
        self.frames.push(ReplayFrame {
            tick: snapshot.tick,
            mobiles: snapshot.mobiles.clone(),
            bases: snapshot.bases.clone(),
            stats: snapshot.stats.clone(),
        });
    }

    /// Serialise all recorded frames to a JSON file at `path`.
    pub fn save(
        &self,
        path: &std::path::Path,
        session: &SimSession,
        protocol_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file = ReplayFile {
            session: session.clone(),
            protocol_name: protocol_name.to_string(),
            frames: self.frames.iter().map(|f| ReplayFrame {
                tick: f.tick,
                mobiles: f.mobiles.clone(),
                bases: f.bases.clone(),
                stats: f.stats.clone(),
            }).collect(),
        };
        let json = serde_json::to_string_pretty(&file)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load a recording from a JSON file at `path`.
    pub fn load(path: &std::path::Path) -> Result<ReplayFile, Box<dyn std::error::Error>> {
        let text = std::fs::read_to_string(path)?;
        let file: ReplayFile = serde_json::from_str(&text)?;
        Ok(file)
    }
}

impl Default for ReplayRecorder {
    fn default() -> Self { Self::new() }
}
