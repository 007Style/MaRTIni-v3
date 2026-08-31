//! GridConfig — spatial layout parameters for the simulation grid.

use serde::{Deserialize, Serialize};

/// Defines the discrete grid used to position mobiles and base stations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridConfig {
    /// Number of dimensions (always 2 for 2-D simulation).
    pub dimension: u32,
    /// Number of city blocks along one axis.
    pub no_block: u32,
    /// Distance resolution in metres per grid step.
    pub dist_res: u32,
    /// Side length in metres of a single city block.
    pub block_size: u32,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            dimension: 2,
            no_block: 12,
            dist_res: 50,
            block_size: 400,
        }
    }
}

impl GridConfig {
    /// Total side length of the grid in metres (`no_block * block_size`).
    pub fn total_length(&self) -> u32 {
        self.no_block * self.block_size
    }

    /// Largest valid coordinate in metres (`total_length - dist_res`).
    pub fn max_coord(&self) -> u32 {
        self.total_length() - self.dist_res
    }

    /// Number of grid points per block (`block_size / dist_res`).
    pub fn points_per_block(&self) -> u32 {
        self.block_size / self.dist_res
    }

    /// Total number of grid points along one axis (`no_block * points_per_block`).
    pub fn total_points(&self) -> u32 {
        self.no_block * self.points_per_block()
    }

    /// Validates all field constraints.
    ///
    /// Returns `Err` with a human-readable message on the first violation found.
    pub fn validate(&self) -> Result<(), String> {
        if self.no_block < 1 || self.no_block > 50 {
            return Err(format!(
                "no_block must be 1–50, got {}",
                self.no_block
            ));
        }
        if self.dist_res == 0 {
            return Err("dist_res must be > 0".to_string());
        }
        if self.block_size < 100 || self.block_size > 2000 {
            return Err(format!(
                "block_size must be 100–2000, got {}",
                self.block_size
            ));
        }
        Ok(())
    }
}
