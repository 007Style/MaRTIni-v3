//! SimSession — top-level container for all simulation configuration.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    grid::GridConfig,
    radio::RadioConfig,
    speed::SpeedConfig,
    terrain::TerrainConfig,
};

/// Aggregates every configuration group for one simulation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimSession {
    /// Spatial grid configuration.
    pub grid: GridConfig,
    /// Mobile speed distribution.
    pub speed: SpeedConfig,
    /// Radio technology and channel parameters.
    pub radio: RadioConfig,
    /// Terrain generation parameters.
    pub terrain: TerrainConfig,
    /// Total number of mobile terminals to spawn.
    pub target_mobile_count: u32,
    /// Random seed for the simulation (0 = pick randomly at runtime).
    pub sim_seed: u64,
}

impl Default for SimSession {
    fn default() -> Self {
        Self {
            grid: GridConfig::default(),
            speed: SpeedConfig::default(),
            radio: RadioConfig::default(),
            terrain: TerrainConfig::default(),
            target_mobile_count: 20,
            sim_seed: 0,
        }
    }
}

impl SimSession {
    /// Serialises this session to a TOML file at `path`.
    pub fn save_toml(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let toml_str = toml::to_string_pretty(self)?;
        std::fs::write(path, toml_str)?;
        Ok(())
    }

    /// Deserialises a `SimSession` from a TOML file at `path`.
    pub fn load_toml(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let toml_str = std::fs::read_to_string(path)?;
        let session: Self = toml::from_str(&toml_str)?;
        Ok(session)
    }
}
