//! Native Rust handoff protocol implementations — Gen3, Gen4, Gen5.

use crate::sim::mobile::MobileTerminal;
use crate::sim::base::BaseStation;
use crate::config::RadioConfig;
use super::protocol::{HandoffDecision, HandoffProtocol};

// ---------------------------------------------------------------------------
// Helper: compute SINR from rx_power arrays (simple linear approximation)
// ---------------------------------------------------------------------------

/// Returns the received power for a mobile at `pos` from a base at `bpos`,
/// using a simplified free-space path-loss formula for decision comparison.
fn rx_power_dbm(mobile_pos: [f32; 2], base_pos: [f32; 2], tx_dbm: f32, pl_exp: f32) -> f32 {
    let [mx, my] = mobile_pos;
    let [bx, by] = base_pos;
    let dist = ((mx - bx).powi(2) + (my - by).powi(2)).sqrt().max(1.0);
    // Simple log-distance: rx = tx - 10*n*log10(d/1m)
    tx_dbm - 10.0 * pl_exp * dist.log10()
}

// ---------------------------------------------------------------------------
// Gen3SoftHandoff
// ---------------------------------------------------------------------------

/// 3G UMTS soft handoff: maintain an active set of up to 3 cells within 6 dB
/// of the best SINR.
pub struct Gen3SoftHandoff;

impl HandoffProtocol for Gen3SoftHandoff {
    fn name(&self) -> &str { "Gen3 Soft Handoff (UMTS)" }
    fn description(&self) -> &str {
        "UMTS active-set management: add/remove cells within 6 dB of best SINR."
    }

    fn decide(&self, mobile: &MobileTerminal, bases: &[BaseStation], config: &RadioConfig) -> HandoffDecision {
        let pl_exp = config.path_loss_exponent();
        let tx = config.max_tx_power_dbm;

        // Compute rx power for every non-failed base.
        let powers: Vec<(u32, f32)> = bases.iter()
            .filter(|b| !b.failed)
            .map(|b| (b.id, rx_power_dbm(mobile.position, b.position, tx, pl_exp)))
            .collect();

        if powers.is_empty() {
            return HandoffDecision::Stay;
        }

        let best_power = powers.iter().map(|(_, p)| *p).fold(f32::NEG_INFINITY, f32::max);

        // Candidates: all bases within 6 dB of best.
        let candidates: Vec<u32> = powers.iter()
            .filter(|(_, p)| *p >= best_power - 6.0)
            .map(|(id, _)| *id)
            .collect();

        // Best candidate not in active set → SoftAdd
        if let Some(&best_id) = candidates.first() {
            if !mobile.active_set.contains(&best_id) && mobile.active_set.len() < 3 {
                return HandoffDecision::SoftAdd { add_id: best_id };
            }
        }

        // Any cell in active set that is >6 dB worse than best → SoftRemove
        for &set_id in &mobile.active_set {
            let set_power = powers.iter().find(|(id, _)| *id == set_id).map(|(_, p)| *p);
            if let Some(sp) = set_power {
                if best_power - sp > 6.0 {
                    return HandoffDecision::SoftRemove { remove_id: set_id };
                }
            }
        }

        HandoffDecision::Stay
    }
}

// ---------------------------------------------------------------------------
// Gen4LteA3
// ---------------------------------------------------------------------------

/// 4G LTE A3 event handoff: hard handoff when best neighbour exceeds serving
/// cell by more than 3 dB (hysteresis).
pub struct Gen4LteA3;

impl HandoffProtocol for Gen4LteA3 {
    fn name(&self) -> &str { "Gen4 LTE A3 Handoff" }
    fn description(&self) -> &str {
        "LTE A3 event: handoff when best neighbour SINR > serving SINR + 3 dB."
    }

    fn decide(&self, mobile: &MobileTerminal, bases: &[BaseStation], config: &RadioConfig) -> HandoffDecision {
        let serving_id = match mobile.serving_cell {
            Some(id) => id,
            None => return HandoffDecision::Stay,
        };

        let pl_exp = config.path_loss_exponent();
        let tx = config.max_tx_power_dbm;

        // Find serving power.
        let serving_power = bases.iter()
            .find(|b| b.id == serving_id && !b.failed)
            .map(|b| rx_power_dbm(mobile.position, b.position, tx, pl_exp))
            .unwrap_or(f32::NEG_INFINITY);

        // Find best non-serving base.
        let best_neighbor = bases.iter()
            .filter(|b| b.id != serving_id && !b.failed)
            .map(|b| (b.id, rx_power_dbm(mobile.position, b.position, tx, pl_exp)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        if let Some((best_id, best_power)) = best_neighbor {
            // A3 condition: neighbor_power > serving_power + hysteresis(3dB)
            if best_power > serving_power + 3.0 {
                return HandoffDecision::HandoffTo { target_id: best_id };
            }
        }

        HandoffDecision::Stay
    }
}

// ---------------------------------------------------------------------------
// Gen5NrCho
// ---------------------------------------------------------------------------

/// 5G NR Conditional Handoff (CHO): prepare at 1 dB gap, execute at 2 dB gap.
pub struct Gen5NrCho;

impl HandoffProtocol for Gen5NrCho {
    fn name(&self) -> &str { "Gen5 NR Conditional Handoff" }
    fn description(&self) -> &str {
        "5G NR CHO: prepare handoff at 1 dB gap, execute at 2 dB gap (tight hysteresis)."
    }

    fn decide(&self, mobile: &MobileTerminal, bases: &[BaseStation], config: &RadioConfig) -> HandoffDecision {
        let serving_id = match mobile.serving_cell {
            Some(id) => id,
            None => return HandoffDecision::Stay,
        };

        let pl_exp = config.path_loss_exponent();
        let tx = config.max_tx_power_dbm;

        let serving_power = bases.iter()
            .find(|b| b.id == serving_id && !b.failed)
            .map(|b| rx_power_dbm(mobile.position, b.position, tx, pl_exp))
            .unwrap_or(f32::NEG_INFINITY);

        let best_neighbor = bases.iter()
            .filter(|b| b.id != serving_id && !b.failed)
            .map(|b| (b.id, rx_power_dbm(mobile.position, b.position, tx, pl_exp)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        if let Some((best_id, best_power)) = best_neighbor {
            let delta = best_power - serving_power;
            if delta > 2.0 {
                return HandoffDecision::ConditionalExecute { target_id: best_id };
            } else if delta > 1.0 {
                return HandoffDecision::ConditionalPrepare { target_id: best_id };
            }
        }

        HandoffDecision::Stay
    }
}

// ---------------------------------------------------------------------------
// Legacy stubs (kept to avoid breaking any existing references)
// ---------------------------------------------------------------------------

/// Strongest-Signal-First handoff — kept for backward compatibility.
pub struct StrongestFirstProtocol;

impl HandoffProtocol for StrongestFirstProtocol {
    fn name(&self) -> &str { "Strongest-First (Legacy)" }
    fn description(&self) -> &str { "Always hand off to the base with the highest received power." }

    fn decide(&self, mobile: &MobileTerminal, bases: &[BaseStation], config: &RadioConfig) -> HandoffDecision {
        // Delegate to Gen4 A3 with 0 dB hysteresis
        let serving_id = match mobile.serving_cell {
            Some(id) => id,
            None => return HandoffDecision::Stay,
        };
        let pl_exp = config.path_loss_exponent();
        let tx = config.max_tx_power_dbm;
        let best = bases.iter()
            .filter(|b| b.id != serving_id && !b.failed)
            .map(|b| (b.id, rx_power_dbm(mobile.position, b.position, tx, pl_exp)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let serving_p = bases.iter()
            .find(|b| b.id == serving_id && !b.failed)
            .map(|b| rx_power_dbm(mobile.position, b.position, tx, pl_exp))
            .unwrap_or(f32::NEG_INFINITY);
        if let Some((id, p)) = best {
            if p > serving_p {
                return HandoffDecision::HandoffTo { target_id: id };
            }
        }
        HandoffDecision::Stay
    }
}

/// Hysteresis-based handoff — kept for backward compatibility.
pub struct HysteresisProtocol {
    pub threshold_db: f32,
}
impl Default for HysteresisProtocol {
    fn default() -> Self { Self { threshold_db: 3.0 } }
}
impl HandoffProtocol for HysteresisProtocol {
    fn name(&self) -> &str { "Hysteresis (Legacy)" }
    fn description(&self) -> &str { "Hand off when SINR improvement exceeds threshold_db." }
    fn decide(&self, mobile: &MobileTerminal, bases: &[BaseStation], config: &RadioConfig) -> HandoffDecision {
        let serving_id = match mobile.serving_cell { Some(id) => id, None => return HandoffDecision::Stay };
        let pl_exp = config.path_loss_exponent();
        let tx = config.max_tx_power_dbm;
        let sp = bases.iter().find(|b| b.id == serving_id && !b.failed)
            .map(|b| rx_power_dbm(mobile.position, b.position, tx, pl_exp))
            .unwrap_or(f32::NEG_INFINITY);
        if let Some((id, p)) = bases.iter().filter(|b| b.id != serving_id && !b.failed)
            .map(|b| (b.id, rx_power_dbm(mobile.position, b.position, tx, pl_exp)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()) {
            if p > sp + self.threshold_db {
                return HandoffDecision::HandoffTo { target_id: id };
            }
        }
        HandoffDecision::Stay
    }
}
