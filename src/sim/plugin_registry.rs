//! PluginRegistry — owns all available protocol implementations.

use super::protocol::HandoffProtocol;
use super::protocol_native::{Gen3SoftHandoff, Gen4LteA3, Gen5NrCho};
use super::protocol_declarative::DeclarativeProtocol;

/// Holds all available handoff protocol implementations.
pub struct PluginRegistry {
    pub protocols: Vec<Box<dyn HandoffProtocol>>,
}

impl PluginRegistry {
    /// Create the registry pre-loaded with the three built-in protocols.
    pub fn new() -> Self {
        let mut reg = Self { protocols: Vec::new() };
        reg.protocols.push(Box::new(Gen3SoftHandoff));
        reg.protocols.push(Box::new(Gen4LteA3));
        reg.protocols.push(Box::new(Gen5NrCho));
        reg
    }

    /// Scan `dir` for `*.toml` files and load each as a `DeclarativeProtocol`.
    ///
    /// Invalid or unreadable files are silently skipped.
    pub fn load_directory(&mut self, dir: &std::path::Path) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                match DeclarativeProtocol::from_toml(&path) {
                    Ok(proto) => self.protocols.push(Box::new(proto)),
                    Err(e) => {
                        log::warn!("Failed to load declarative protocol {:?}: {}", path, e);
                    }
                }
            }
        }
    }

    /// Returns the display names of all registered protocols.
    pub fn names(&self) -> Vec<&str> {
        self.protocols.iter().map(|p| p.name()).collect()
    }

    /// Get a protocol by index.
    pub fn get(&self, index: usize) -> Option<&dyn HandoffProtocol> {
        self.protocols.get(index).map(|p| p.as_ref())
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
