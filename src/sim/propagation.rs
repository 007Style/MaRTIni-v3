//! PropagationModel — log-distance path loss, SINR, Shannon capacity, and latency.

use crate::config::RadioConfig;

/// Speed of light in m/s.
const C_MPS: f32 = 3.0e8;

/// Stateless propagation model; all methods are pure functions.
pub struct PropagationModel;

impl PropagationModel {
    // -----------------------------------------------------------------------
    // path_loss_db
    // -----------------------------------------------------------------------

    /// Log-distance path loss in dB.
    ///
    /// `PL = 20·log10(4π·d0/λ) + 10·n·log10(d/d0) + terrain_offset_db`
    ///
    /// Uses reference distance `d0 = 1 m` and the wavelength derived from
    /// `radio.frequency_ghz`.  Distances closer than 1 m are clamped to 1 m.
    pub fn path_loss_db(
        dist_m: f32,
        radio: &RadioConfig,
        terrain_offset_db: f32,
    ) -> f32 {
        let d0: f32 = 1.0;
        let d = dist_m.max(d0);
        let freq_hz = radio.frequency_ghz * 1.0e9;
        let lambda = C_MPS / freq_hz; // metres
        let n = radio.path_loss_exponent();

        // Free-space path loss at d0 (dB).
        let pl_d0 = 20.0 * (4.0 * std::f32::consts::PI * d0 / lambda).log10();

        // Distance-dependent loss.
        let pl_dist = 10.0 * n * (d / d0).log10();

        pl_d0 + pl_dist + terrain_offset_db
    }

    // -----------------------------------------------------------------------
    // received_power_dbm
    // -----------------------------------------------------------------------

    /// Received power in dBm: `tx_power_dbm − path_loss_db`.
    pub fn received_power_dbm(tx_power_dbm: f32, path_loss_db: f32) -> f32 {
        tx_power_dbm - path_loss_db
    }

    // -----------------------------------------------------------------------
    // sinr_db
    // -----------------------------------------------------------------------

    /// SINR in dB.
    ///
    /// Converts all dBm values to milliwatts, computes the linear ratio, then
    /// converts the result back to dB.
    pub fn sinr_db(
        signal_dbm: f32,
        noise_dbm: f32,
        interferer_dbm: &[f32],
    ) -> f32 {
        let dbm_to_mw = |dbm: f32| 10.0_f32.powf(dbm / 10.0);

        let signal_mw = dbm_to_mw(signal_dbm);
        let noise_mw = dbm_to_mw(noise_dbm);
        let interference_mw: f32 = interferer_dbm.iter().copied().map(dbm_to_mw).sum();

        let denominator = noise_mw + interference_mw;
        if denominator <= 0.0 {
            return 0.0;
        }

        let sinr_linear = signal_mw / denominator;
        10.0 * sinr_linear.log10()
    }

    // -----------------------------------------------------------------------
    // shannon_capacity_mbps
    // -----------------------------------------------------------------------

    /// Shannon capacity in Mbps.
    ///
    /// `C = bandwidth_mhz × spectral_efficiency × log2(1 + SINR_linear)`
    pub fn shannon_capacity_mbps(sinr_db: f32, radio: &RadioConfig) -> f32 {
        let sinr_linear = 10.0_f32.powf(sinr_db / 10.0);
        radio.bandwidth_mhz * radio.spectral_efficiency() * (1.0 + sinr_linear).log2()
    }

    // -----------------------------------------------------------------------
    // latency_ms
    // -----------------------------------------------------------------------

    /// Estimated one-way latency in milliseconds.
    ///
    /// `latency = base_rtt_ms + congestion_penalty + handoff_penalty`
    ///
    /// Congestion penalty rises linearly from 0 at 0 % load to `base_rtt_ms`
    /// at 100 % load.  An in-progress handoff adds a flat 20 ms penalty.
    pub fn latency_ms(base_rtt_ms: f32, load_percent: f32, in_handoff: bool) -> f32 {
        let congestion = base_rtt_ms * (load_percent / 100.0).clamp(0.0, 1.0);
        let handoff_penalty = if in_handoff { 20.0 } else { 0.0 };
        base_rtt_ms + congestion + handoff_penalty
    }
}
