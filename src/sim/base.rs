//! BaseStation — position, channel capacity, load tracking, and factory function.

use crate::config::RadioConfig;
use crate::terrain::TerrainMap;

/// Represents a base station (cell tower or small cell).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BaseStation {
    /// Unique identifier.
    pub id: u32,
    /// 2-D position `[x, y]` in metres.
    pub position: [f32; 2],
    /// Antenna height in metres.
    pub height_m: f32,
    /// Radio access technology for this station.
    pub technology: crate::config::RadioTechnology,
    /// Maximum number of simultaneous channel connections.
    pub total_channels: u32,
    /// Ids of currently connected mobile terminals.
    pub connected_mobiles: Vec<u32>,
    /// Whether this tower has been taken offline by a scenario event.
    pub failed: bool,
}

impl BaseStation {
    /// Construct a new base station from a position and the radio configuration.
    pub fn new(id: u32, pos: [f32; 2], cfg: &RadioConfig) -> Self {
        Self {
            id,
            position: pos,
            height_m: cfg.base_height_m,
            technology: cfg.technology.clone(),
            total_channels: cfg.no_channel,
            connected_mobiles: Vec::new(),
            failed: false,
        }
    }

    /// Percentage of channels currently occupied (0.0–100.0).
    pub fn load_percent(&self) -> f32 {
        if self.total_channels == 0 {
            return 0.0;
        }
        self.connected_mobiles.len() as f32 / self.total_channels as f32 * 100.0
    }

    /// Returns `true` when all channels are occupied.
    pub fn is_at_capacity(&self) -> bool {
        self.connected_mobiles.len() >= self.total_channels as usize
    }
}

/// Build all base stations from the terrain tower positions and the radio config.
pub fn build_base_stations(terrain: &TerrainMap, cfg: &RadioConfig) -> Vec<BaseStation> {
    terrain
        .tower_positions
        .iter()
        .enumerate()
        .map(|(i, &pos)| BaseStation::new(i as u32, pos, cfg))
        .collect()
}
