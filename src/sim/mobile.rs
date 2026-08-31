//! MobileTerminal — position, heading, radio state, and Manhattan mobility model.

use std::collections::VecDeque;
use rand::Rng;

use crate::config::{TrafficProfile, GridConfig, SpeedConfig};
use crate::terrain::TerrainMap;

// ---------------------------------------------------------------------------
// Direction — cardinal compass heading
// ---------------------------------------------------------------------------

/// Cardinal movement direction for the Manhattan mobility model.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Direction {
    East,
    South,
    West,
    North,
}

impl Direction {
    /// Choose a random direction.
    pub fn random(rng: &mut impl Rng) -> Self {
        match rng.gen_range(0u8..4) {
            0 => Self::East,
            1 => Self::South,
            2 => Self::West,
            _ => Self::North,
        }
    }

    /// Turn 90° left or right at random.
    pub fn turn_random(self, rng: &mut impl Rng) -> Self {
        // Perpendicular directions for each heading.
        let options: [Direction; 2] = match self {
            Self::East  | Self::West  => [Self::North, Self::South],
            Self::North | Self::South => [Self::East,  Self::West],
        };
        if rng.gen::<bool>() { options[0] } else { options[1] }
    }

    /// X-axis delta: East=+1, West=−1, North/South=0.
    pub fn dx(&self) -> f32 {
        match self {
            Self::East  =>  1.0,
            Self::West  => -1.0,
            Self::North | Self::South => 0.0,
        }
    }

    /// Y-axis delta: South=+1, North=−1, East/West=0.
    pub fn dy(&self) -> f32 {
        match self {
            Self::South =>  1.0,
            Self::North => -1.0,
            Self::East | Self::West => 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// MobileTerminal
// ---------------------------------------------------------------------------

/// A single mobile terminal in the simulation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MobileTerminal {
    /// Unique identifier.
    pub id: u32,
    /// 2-D position `[x, y]` in metres.
    pub position: [f32; 2],
    /// Current cardinal heading.
    pub heading: Direction,
    /// Current speed in metres per second.
    pub speed_mps: f32,
    /// Application traffic profile.
    pub profile: TrafficProfile,
    /// Primary serving base station id (None before first tick).
    pub serving_cell: Option<u32>,
    /// Gen5 dual-connectivity secondary cell id.
    pub secondary_cell: Option<u32>,
    /// Gen3 soft-handoff active set (base station ids).
    pub active_set: Vec<u32>,
    /// Current downlink SINR in dB.
    pub sinr_db: f32,
    /// Current reference signal received power in dBm.
    pub rsrp_dbm: f32,
    /// Estimated downlink throughput in Mbps.
    pub dl_throughput_mbps: f32,
    /// Estimated uplink throughput in Mbps.
    pub ul_throughput_mbps: f32,
    /// Estimated one-way latency in milliseconds.
    pub latency_ms: f32,
    /// Total handoffs performed by this mobile.
    pub handoff_count: u32,
    /// Current transmit power in dBm.
    pub tx_power_dbm: f32,
    /// Remaining battery charge (0.0–100.0).
    pub battery_percent: f32,
    /// Colour slot (id % 8) for rendering.
    pub color_index: u8,
    /// SINR history for the last 60 ticks.
    pub sinr_history: VecDeque<f32>,
    /// Latency history for the last 60 ticks.
    pub latency_history: VecDeque<f32>,
    /// Position trail for the last 10 positions.
    pub trail: VecDeque<[f32; 2]>,
    /// Whether the mobile is currently violating its SLA.
    pub sla_violated: bool,
}

impl MobileTerminal {
    /// Create a new mobile terminal, spawning at a random position.
    pub fn new(
        id: u32,
        profile: TrafficProfile,
        grid: &GridConfig,
        speed_cfg: &SpeedConfig,
        rng: &mut impl Rng,
    ) -> Self {
        let total = grid.total_length() as f32;
        let x = rng.gen_range(0.0..total);
        let y = rng.gen_range(0.0..total);
        let speed_kmh = sample_speed(speed_cfg, rng);
        let speed_mps = speed_kmh / 3.6;
        let tx_power_dbm = 23.0; // typical mobile UE Tx power

        Self {
            id,
            position: [x, y],
            heading: Direction::random(rng),
            speed_mps,
            profile,
            serving_cell: None,
            secondary_cell: None,
            active_set: Vec::new(),
            sinr_db: 0.0,
            rsrp_dbm: -100.0,
            dl_throughput_mbps: 0.0,
            ul_throughput_mbps: 0.0,
            latency_ms: 0.0,
            handoff_count: 0,
            tx_power_dbm,
            battery_percent: 100.0,
            color_index: (id % 8) as u8,
            sinr_history: VecDeque::with_capacity(60),
            latency_history: VecDeque::with_capacity(60),
            trail: VecDeque::with_capacity(10),
            sla_violated: false,
        }
    }

    /// Advance the mobile by one tick using the Manhattan mobility model.
    ///
    /// `dt_s` should be `0.1` for a 100 ms tick.
    pub fn step(
        &mut self,
        dt_s: f32,
        grid: &GridConfig,
        speed_cfg: &SpeedConfig,
        terrain: &TerrainMap,
        rng: &mut impl Rng,
    ) {
        // Save current position to trail before moving.
        self.push_trail();

        let total = grid.total_length() as f32;
        let [x, y] = self.position;
        let step = self.speed_mps * dt_s;

        let new_x = x + self.heading.dx() * step;
        let new_y = y + self.heading.dy() * step;

        // Toroidal boundary wrap.
        let wrapped_x = ((new_x % total) + total) % total;
        let wrapped_y = ((new_y % total) + total) % total;

        // Check proximity to nearest intersection.
        let inter = terrain.nearest_intersection([wrapped_x, wrapped_y]);
        let dx_inter = (wrapped_x - inter[0]).abs();
        let dy_inter = (wrapped_y - inter[1]).abs();
        let snap_threshold = step * 2.0;

        if dx_inter < snap_threshold && dy_inter < snap_threshold {
            // Snap to intersection.
            self.position = inter;

            // Possibly change direction.
            if rng.gen::<f32>() > speed_cfg.prob_ahead {
                self.heading = self.heading.turn_random(rng);
            }

            // Resample speed.
            let new_speed_kmh = sample_speed(speed_cfg, rng);
            self.speed_mps = new_speed_kmh / 3.6;
        } else {
            self.position = [wrapped_x, wrapped_y];
        }

        // Battery drain per tick.
        self.battery_percent = (self.battery_percent - self.tx_power_dbm * 0.00001).clamp(0.0, 100.0);
    }

    /// Push a SINR sample; caps history at 60 entries.
    pub fn push_sinr(&mut self, sinr: f32) {
        if self.sinr_history.len() == 60 {
            self.sinr_history.pop_front();
        }
        self.sinr_history.push_back(sinr);
    }

    /// Push a latency sample; caps history at 60 entries.
    pub fn push_latency(&mut self, latency: f32) {
        if self.latency_history.len() == 60 {
            self.latency_history.pop_front();
        }
        self.latency_history.push_back(latency);
    }

    /// Save the current position to the trail; caps trail at 10 entries.
    pub fn push_trail(&mut self) {
        if self.trail.len() == 10 {
            self.trail.pop_front();
        }
        self.trail.push_back(self.position);
    }
}

// ---------------------------------------------------------------------------
// Internal helper
// ---------------------------------------------------------------------------

/// Sample a speed from Gaussian(mean, sigma), clamped to [min, max], in km/h.
fn sample_speed(cfg: &SpeedConfig, rng: &mut impl Rng) -> f32 {
    // Box-Muller transform for a single normal sample.
    let u1: f32 = rng.gen_range(f32::EPSILON..1.0);
    let u2: f32 = rng.gen_range(0.0..1.0);
    let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos();
    let v = cfg.mean_speed_kmh + cfg.sigma_speed * z;
    v.clamp(cfg.min_speed_kmh, cfg.max_speed_kmh)
}
