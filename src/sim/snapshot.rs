//! SimSnapshot — thread-safe read-only copy of simulation state sent to the UI thread.

use std::collections::VecDeque;
use std::sync::Arc;

use crate::sim::mobile::MobileTerminal;
use crate::sim::base::BaseStation;
use crate::sim::statistics::Statistics;
use crate::terrain::TerrainMap;

// ---------------------------------------------------------------------------
// SimEventType
// ---------------------------------------------------------------------------

/// Classification of a discrete simulation event.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SimEventType {
    /// A new mobile terminal was admitted.
    Arrival,
    /// A mobile terminal left the simulation (normal departure).
    Departure,
    /// A handoff was completed successfully.
    HandoffSuccess,
    /// A handoff attempt failed.
    HandoffFailure,
    /// A mobile's SLA constraint was violated.
    SlaViolation,
    /// A base station was taken offline.
    TowerFailure,
    /// A previously offline base station came back online.
    TowerRestore,
    /// A sudden burst of new mobiles was injected.
    MobileSurge,
}

// ---------------------------------------------------------------------------
// SimEvent
// ---------------------------------------------------------------------------

/// A single discrete event emitted by the simulation engine.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimEvent {
    /// Tick at which this event occurred.
    pub tick: u64,
    /// Mobile terminal involved (0 if not applicable).
    pub mobile_id: u32,
    /// Event classification.
    pub event_type: SimEventType,
    /// Human-readable detail string.
    pub detail: String,
}

// ---------------------------------------------------------------------------
// SimSnapshot
// ---------------------------------------------------------------------------

/// A complete, read-only snapshot of simulation state at a single tick.
///
/// This type is `Clone` and cheaply shareable across threads via the
/// `Arc<TerrainMap>` — the terrain never mutates after generation.
#[derive(Debug, Clone)]
pub struct SimSnapshot {
    /// Tick at which this snapshot was captured.
    pub tick: u64,
    /// All mobile terminals at this tick.
    pub mobiles: Vec<MobileTerminal>,
    /// All base stations at this tick.
    pub bases: Vec<BaseStation>,
    /// Aggregate statistics for this tick.
    pub stats: Statistics,
    /// Ring buffer of the last 500 events.
    pub events: VecDeque<SimEvent>,
    /// Shared reference to the (immutable) terrain map.
    pub terrain: Arc<TerrainMap>,
}
