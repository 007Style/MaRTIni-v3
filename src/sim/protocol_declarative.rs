//! Declarative TOML rule-based handoff protocol — no Rust code required.

use crate::sim::mobile::MobileTerminal;
use crate::sim::base::BaseStation;
use crate::config::RadioConfig;
use super::protocol::{HandoffDecision, HandoffProtocol};

// ---------------------------------------------------------------------------
// DeclarativeRule
// ---------------------------------------------------------------------------

/// A single rule in a declarative protocol definition.
///
/// # Condition syntax
/// `"{metric} {op} {value}"`, e.g. `"sinr_db < -5.0"`
///
/// Metrics: `sinr_db`, `rsrp_dbm`, `serving_load`, `speed_mps`
/// Ops: `<`, `>`, `<=`, `>=`
///
/// # Action values
/// `"handoff_to_best"`, `"soft_add_best"`, `"stay"`
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeclarativeRule {
    /// Condition string, e.g. `"sinr_db < -5.0"`.
    pub condition: String,
    /// Action string, e.g. `"handoff_to_best"`.
    pub action: String,
}

// ---------------------------------------------------------------------------
// DeclarativeProtocol
// ---------------------------------------------------------------------------

/// A handoff protocol fully defined by an ordered list of TOML rules.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeclarativeProtocol {
    /// Human-readable name.
    pub name: String,
    /// One-sentence description.
    pub description: String,
    /// Ordered rules — first matching rule wins.
    pub rules: Vec<DeclarativeRule>,
}

impl DeclarativeProtocol {
    /// Load a `DeclarativeProtocol` from a TOML file.
    pub fn from_toml(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let text = std::fs::read_to_string(path)?;
        let proto: Self = toml::from_str(&text)?;
        Ok(proto)
    }
}

impl HandoffProtocol for DeclarativeProtocol {
    fn name(&self) -> &str { &self.name }
    fn description(&self) -> &str { &self.description }

    fn decide(&self, mobile: &MobileTerminal, bases: &[BaseStation], config: &RadioConfig) -> HandoffDecision {
        for rule in &self.rules {
            if eval_condition(&rule.condition, mobile, bases, config) {
                return exec_action(&rule.action, mobile, bases, config);
            }
        }
        HandoffDecision::Stay
    }
}

// ---------------------------------------------------------------------------
// Condition evaluator
// ---------------------------------------------------------------------------

fn eval_condition(cond: &str, mobile: &MobileTerminal, bases: &[BaseStation], _config: &RadioConfig) -> bool {
    let parts: Vec<&str> = cond.split_whitespace().collect();
    if parts.len() != 3 {
        return false;
    }
    let metric = parts[0];
    let op = parts[1];
    let threshold: f32 = match parts[2].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };

    let value: f32 = match metric {
        "sinr_db"      => mobile.sinr_db,
        "rsrp_dbm"     => mobile.rsrp_dbm,
        "speed_mps"    => mobile.speed_mps,
        "serving_load" => {
            mobile.serving_cell
                .and_then(|bid| bases.iter().find(|b| b.id == bid))
                .map(|b| b.load_percent())
                .unwrap_or(0.0)
        }
        _ => return false,
    };

    match op {
        "<"  => value <  threshold,
        ">"  => value >  threshold,
        "<=" => value <= threshold,
        ">=" => value >= threshold,
        _    => false,
    }
}

// ---------------------------------------------------------------------------
// Action executor
// ---------------------------------------------------------------------------

fn exec_action(action: &str, mobile: &MobileTerminal, bases: &[BaseStation], config: &RadioConfig) -> HandoffDecision {
    match action {
        "stay" => HandoffDecision::Stay,
        "handoff_to_best" => {
            let serving_id = mobile.serving_cell.unwrap_or(u32::MAX);
            let pl_exp = config.path_loss_exponent();
            let tx = config.max_tx_power_dbm;
            let best = bases.iter()
                .filter(|b| b.id != serving_id && !b.failed)
                .map(|b| {
                    let [mx, my] = mobile.position;
                    let [bx, by] = b.position;
                    let d = ((mx - bx).powi(2) + (my - by).powi(2)).sqrt().max(1.0);
                    let p = tx - 10.0 * pl_exp * d.log10();
                    (b.id, p)
                })
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            match best {
                Some((id, _)) => HandoffDecision::HandoffTo { target_id: id },
                None => HandoffDecision::Stay,
            }
        }
        "soft_add_best" => {
            let serving_id = mobile.serving_cell.unwrap_or(u32::MAX);
            let pl_exp = config.path_loss_exponent();
            let tx = config.max_tx_power_dbm;
            let best = bases.iter()
                .filter(|b| b.id != serving_id && !b.failed && !mobile.active_set.contains(&b.id))
                .map(|b| {
                    let [mx, my] = mobile.position;
                    let [bx, by] = b.position;
                    let d = ((mx - bx).powi(2) + (my - by).powi(2)).sqrt().max(1.0);
                    let p = tx - 10.0 * pl_exp * d.log10();
                    (b.id, p)
                })
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            match best {
                Some((id, _)) => HandoffDecision::SoftAdd { add_id: id },
                None => HandoffDecision::Stay,
            }
        }
        _ => HandoffDecision::Stay,
    }
}
