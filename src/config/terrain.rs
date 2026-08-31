//! TerrainConfig — terrain generation parameters.

use serde::{Deserialize, Serialize};

/// Available terrain layout types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TerrainType {
    UrbanGrid,
    Suburban,
    Rural,
    Highway,
}

/// Configuration driving the terrain generator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainConfig {
    /// The terrain layout style to generate.
    pub terrain_type: TerrainType,
    /// Random seed for deterministic terrain generation.
    pub seed: u64,
    /// Fraction of grid cells occupied by buildings, 0.0–1.0.
    pub building_density: f32,
    /// Preferred spacing between cell towers in metres.
    pub tower_spacing_m: f32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            terrain_type: TerrainType::UrbanGrid,
            seed: 42,
            building_density: 0.7,
            tower_spacing_m: 500.0,
        }
    }
}
