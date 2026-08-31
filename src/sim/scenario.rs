//! Scenario scripting — TOML-driven events injected at specific ticks.

// ---------------------------------------------------------------------------
// ScenarioEventKind
// ---------------------------------------------------------------------------

/// The kind of event that a scenario can inject.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ScenarioEventKind {
    /// Take a base station offline.
    TowerFailure { base_id: u32 },
    /// Restore a previously offline base station.
    TowerRestore { base_id: u32 },
    /// Inject a burst of new mobile terminals.
    MobileSurge { count: u32 },
}

// ---------------------------------------------------------------------------
// ScheduledEvent
// ---------------------------------------------------------------------------

/// A single event scheduled to fire at a specific tick.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScheduledEvent {
    /// Tick at which this event fires.
    pub tick: u64,
    /// What to do when the tick arrives.
    pub kind: ScenarioEventKind,
}

// ---------------------------------------------------------------------------
// Scenario
// ---------------------------------------------------------------------------

/// A collection of scheduled events that make up a simulation scenario.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Scenario {
    /// Human-readable scenario name.
    pub name: String,
    /// Optional description of what this scenario tests.
    pub description: String,
    /// Ordered list of events (sorted by tick at load time).
    pub events: Vec<ScheduledEvent>,
}

impl Scenario {
    /// Load a scenario from a TOML file.
    pub fn from_toml(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let text = std::fs::read_to_string(path)?;
        let mut s: Self = toml::from_str(&text)?;
        s.events.sort_by_key(|e| e.tick);
        Ok(s)
    }

    /// Returns the events scheduled at exactly `tick`.
    pub fn events_at(&self, tick: u64) -> Vec<&ScenarioEventKind> {
        self.events.iter()
            .filter(|e| e.tick == tick)
            .map(|e| &e.kind)
            .collect()
    }
}
