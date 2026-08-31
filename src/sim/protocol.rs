//! HandoffProtocol trait — the pluggable handoff decision interface.

use crate::sim::mobile::MobileTerminal;
use crate::sim::base::BaseStation;
use crate::config::RadioConfig;

// ---------------------------------------------------------------------------
// HandoffDecision
// ---------------------------------------------------------------------------

/// The result of a protocol's handoff decision for one mobile.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum HandoffDecision {
    /// Remain on the current serving cell.
    Stay,
    /// Hard handoff to a different base station.
    HandoffTo { target_id: u32 },
    /// Gen3 soft-handoff: add a new cell to the active set.
    SoftAdd { add_id: u32 },
    /// Gen3 soft-handoff: remove a weak cell from the active set.
    SoftRemove { remove_id: u32 },
    /// Gen5 Conditional Handoff: begin preparation (T304 timer not started).
    ConditionalPrepare { target_id: u32 },
    /// Gen5 Conditional Handoff: execute the prepared handoff.
    ConditionalExecute { target_id: u32 },
}

// ---------------------------------------------------------------------------
// HandoffProtocol trait
// ---------------------------------------------------------------------------

/// Core pluggable handoff-protocol interface.
///
/// All native, declarative, and WASM protocol implementations satisfy this trait.
pub trait HandoffProtocol: Send + Sync {
    /// Short human-readable name shown in the UI protocol selector.
    fn name(&self) -> &str;

    /// One-sentence description shown in the UI.
    fn description(&self) -> &str;

    /// Decide whether `mobile` should hand off given the current set of
    /// `bases` and `config`.  Called once per mobile per tick.
    fn decide(
        &self,
        mobile: &MobileTerminal,
        bases: &[BaseStation],
        config: &RadioConfig,
    ) -> HandoffDecision;
}
