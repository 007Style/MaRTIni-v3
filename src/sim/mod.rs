//! Simulation module — engine, mobiles, base stations, protocols, propagation, replay, and more.

pub mod ab_test;
pub mod base;
pub mod engine;
pub mod mobile;
pub mod plugin_registry;
pub mod propagation;
pub mod protocol;
pub mod protocol_declarative;
pub mod protocol_native;
pub mod protocol_wasm;
pub mod replay;
pub mod scenario;
pub mod snapshot;
pub mod statistics;

// Public re-exports for convenience.
pub use engine::SimEngine;
pub use snapshot::{SimSnapshot, SimEvent, SimEventType};
pub use mobile::{MobileTerminal, Direction};
pub use base::BaseStation;
pub use statistics::Statistics;
pub use propagation::PropagationModel;
pub use protocol::{HandoffProtocol, HandoffDecision};
pub use plugin_registry::PluginRegistry;
pub use scenario::Scenario;
pub use replay::ReplayRecorder;
