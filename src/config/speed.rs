//! SpeedConfig — mobile terminal speed distribution parameters.

use serde::{Deserialize, Serialize};

/// Gaussian speed model parameters (km/h internally; use `speed_mps()` for m/s).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedConfig {
    /// Minimum allowed speed (km/h).
    pub min_speed_kmh: f32,
    /// Maximum allowed speed (km/h).
    pub max_speed_kmh: f32,
    /// Mean of the speed distribution (km/h).
    pub mean_speed_kmh: f32,
    /// Standard deviation of the speed distribution.
    pub sigma_speed: f32,
    /// Probability that a mobile continues straight ahead (vs turning), range 0.0–1.0.
    pub prob_ahead: f32,
}

impl Default for SpeedConfig {
    fn default() -> Self {
        Self {
            min_speed_kmh: 25.0,
            max_speed_kmh: 90.0,
            mean_speed_kmh: 25.0,
            sigma_speed: 4.17,
            prob_ahead: 0.6,
        }
    }
}

impl SpeedConfig {
    /// Converts `mean_speed_kmh` to metres per second.
    pub fn speed_mps(&self) -> f32 {
        self.mean_speed_kmh / 3.6
    }
}
