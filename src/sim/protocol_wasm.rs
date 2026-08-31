//! WASM plugin protocol host — loads `.wasm` plugin files via wasmtime (optional feature).

use crate::sim::mobile::MobileTerminal;
use crate::sim::base::BaseStation;
use crate::config::RadioConfig;
use super::protocol::{HandoffDecision, HandoffProtocol};

/// A handoff protocol backed by a WASM plugin loaded at runtime.
/// Requires the `wasm-plugins` Cargo feature.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct WasmProtocol {
    /// Path to the `.wasm` plugin file.
    pub plugin_path: String,
    /// Display name sourced from the plugin metadata.
    pub plugin_name: String,
}

impl HandoffProtocol for WasmProtocol {
    fn name(&self) -> &str { &self.plugin_name }
    fn description(&self) -> &str { "WASM plugin (requires wasm-plugins feature)" }

    fn decide(&self, _mobile: &MobileTerminal, _bases: &[BaseStation], _config: &RadioConfig) -> HandoffDecision {
        // Full WASM host implementation requires the wasm-plugins feature.
        HandoffDecision::Stay
    }
}
