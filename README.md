# MaRTIni v3 — Manhattan Radio and Telecommunications Interactive Network Simulator

**Wireless Network Research Platform**  
WINLab, Rutgers University

---

## Authors

| Name | Affiliation |
|------|-------------|
| **Daneyand Singley** | WINLab, Rutgers University |
| **Roland Wunderlich** | WINLab, Rutgers University |
| **Ramnath Ravindran** | WINLab, Rutgers University |

---

## Project Lineage

| Version | Year | Technology | Notes |
|---------|------|------------|-------|
| **v1** | 1999 | Java AWT | Original Manhattan mobility model research tool |
| **v2** | 2023 | Java 22 Swing | Multi-generation radio with tabbed UI |
| **v3** | 2024 | **Rust + egui** | Full rewrite — real-time, pluggable protocols, research tools |

MaRTIni was created to study handoff algorithms and radio resource management under realistic urban mobility patterns. v3 is the first version with a pluggable protocol SDK, A/B testing infrastructure, and scenario scripting.

---

## Features

### Simulation Core
- **Manhattan Mobility Model** — toroidal grid, Gaussian speed distribution, intersection-aware turning
- **Multi-generation Radio** — 3G UMTS, 4G LTE, 5G NR Sub-6, 5G NR mmWave
- **Propagation** — log-distance path loss with terrain-aware attenuation, SINR computation, Shannon capacity
- **Base Stations** — configurable count and channel capacity, failure/restore events
- **Mobile Terminals** — per-mobile SINR/latency/battery history, SLA tracking
- **Real-time animation** at 10 Hz via background thread + `mpsc` channel

### Pluggable Protocol System
- **Gen3 Soft Handoff (UMTS)** — active-set management within 6 dB window
- **Gen4 LTE A3 Handoff** — 3 dB hysteresis hard handoff
- **Gen5 NR Conditional Handoff (CHO)** — prepare at 1 dB gap, execute at 2 dB gap
- **Declarative TOML Protocols** — define rules like `"sinr_db < -5.0"` → `"handoff_to_best"` with no code
- **Protocol SDK** — implement `HandoffProtocol` trait in Rust and register via `PluginRegistry`
- Live protocol switching from the sidebar or Protocol menu

### Research Tools
- **Scenario Scripting** — TOML files schedule `TowerFailure`, `TowerRestore`, `MobileSurge` at specific ticks
- **Simulation Recording** — record full runs to JSON, replay frame-by-frame
- **KPI Export** — export tick-by-tick CSV: SINR, throughput, latency, block rate, handoff success
- **A/B Test Runner** — headless N-run comparison of two protocols with 95% confidence intervals

### UI
- **Map Panel** — terrain, streets, buildings, tower icons, mobile dots with trails, SINR heatmap
- **Inspector Panel** — per-mobile SINR + latency history charts (`egui_plot`), battery, SLA status
- **Stats Panel** — KPI cards, per-cell load bars, traffic profile mix
- **Config Panel** — live-editable sliders and dropdowns for all session parameters
- **Event Log** — filterable, color-coded, click-to-inspect handoff events
- **A/B Panel** — interactive protocol comparison with highlighted winner column
- **Replay Panel** — play/pause/scrub recorded runs
- Full **menu bar** (File / Simulation / Protocol / View / Help)
- **Keyboard shortcuts**: `S` start · `X` stop · `R` reset · `Esc` deselect mobile

---

## Build & Run

### Prerequisites
- **Rust** 1.75 or later (stable toolchain)
- macOS, Linux, or Windows (egui/eframe supports all platforms)

```bash
# Clone
git clone https://github.com/007Style/MaRTIni-v3.git
cd MaRTIni-v3

# Development build and run
cargo run

# Optimised release build
cargo build --release
./target/release/martini

# Run tests
cargo test
```

### Optional: WASM Plugin Support (advanced)
```bash
cargo build --release --features wasm-plugins
```

---

## Configuration

On first run, default configuration is used. Use **File → Save Config** to write `session.toml`. Edit it and **File → Load Config** to reload, or adjust sliders in the **Config panel** and press ▶ Start to apply.

Key parameters:

| Parameter | Default | Range |
|-----------|---------|-------|
| `no_block` | 12 | 1–30 blocks |
| `block_size` | 400 m | 100–1000 m |
| `no_base` | 12 | 1–30 towers |
| `no_channel` | 30 | 1–100 |
| `bandwidth_mhz` | 20 | 1–100 MHz |
| `target_mobile_count` | 20 | 1–100 |
| `tower_spacing_m` | 500 m | 100–2000 m |

---

## Protocol Authoring Guide

See **[`docs/protocol-sdk.md`](docs/protocol-sdk.md)** for the full guide.

### Quick Start — Declarative Protocol (TOML)

Create a file in the `plugins/` directory, e.g. `plugins/my_protocol.toml`:

```toml
name = "My Aggressive Handoff"
description = "Hand off immediately when SINR drops below -3 dB."

[[rules]]
condition = "sinr_db < -3.0"
action = "handoff_to_best"

[[rules]]
condition = "serving_load >= 80.0"
action = "handoff_to_best"
```

Available **metrics**: `sinr_db`, `rsrp_dbm`, `speed_mps`, `serving_load`  
Available **operators**: `<`, `>`, `<=`, `>=`  
Available **actions**: `handoff_to_best`, `soft_add_best`, `stay`

The protocol will appear automatically in the Protocol dropdown on next launch.

### Quick Start — Native Rust Protocol

```rust
// In your own crate or directly in src/sim/protocol_native.rs:
use martini::sim::protocol::{HandoffProtocol, HandoffDecision};
use martini::sim::{MobileTerminal, BaseStation};
use martini::config::RadioConfig;

pub struct MyProtocol;

impl HandoffProtocol for MyProtocol {
    fn name(&self) -> &str { "My Protocol" }
    fn description(&self) -> &str { "Custom handoff logic." }
    fn decide(&self, mobile: &MobileTerminal, bases: &[BaseStation], config: &RadioConfig) -> HandoffDecision {
        // Your logic here
        HandoffDecision::Stay
    }
}
```

Register it in `PluginRegistry::new()` or pass it via `SimEngine::start(..., Some(Box::new(MyProtocol)), ...)`.

---

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `S` | Start simulation |
| `X` | Stop simulation |
| `R` | Reset simulation |
| `Esc` | Deselect mobile |

---

## Project Structure

```
MaRTIni-v3/
├── src/
│   ├── app.rs                   # Top-level eframe::App
│   ├── config/                  # GridConfig, RadioConfig, SpeedConfig, TerrainConfig, SimSession
│   ├── sim/
│   │   ├── engine.rs            # SimEngine tick loop + background thread
│   │   ├── mobile.rs            # MobileTerminal, Manhattan mobility
│   │   ├── base.rs              # BaseStation
│   │   ├── propagation.rs       # Path loss, SINR, Shannon capacity
│   │   ├── protocol.rs          # HandoffProtocol trait + HandoffDecision
│   │   ├── protocol_native.rs   # Gen3SoftHandoff, Gen4LteA3, Gen5NrCho
│   │   ├── protocol_declarative.rs  # TOML rule engine
│   │   ├── protocol_wasm.rs     # WASM plugin host (optional feature)
│   │   ├── plugin_registry.rs   # PluginRegistry
│   │   ├── scenario.rs          # Scenario scripting
│   │   ├── replay.rs            # ReplayRecorder, ReplayFile
│   │   ├── statistics.rs        # Statistics, KPI CSV export
│   │   ├── ab_test.rs           # A/B test runner
│   │   └── snapshot.rs          # SimSnapshot, SimEvent
│   ├── terrain/                 # TerrainMap generation
│   └── ui/
│       ├── map_panel.rs         # Map canvas with Painter API
│       ├── inspector.rs         # Per-mobile inspector with charts
│       ├── stats_panel.rs       # KPI dashboard
│       ├── config_panel.rs      # Config forms
│       ├── event_log.rs         # Event log with filter
│       ├── ab_panel.rs          # A/B test panel
│       └── replay_panel.rs      # Replay controls
├── docs/
│   └── protocol-sdk.md          # Protocol authoring guide
├── plugins/                     # Drop .toml protocol files here
├── scenarios/                   # Example scenario TOML files
└── tests/                       # Integration tests
```

---

## License

Research use — WINLab, Rutgers University. Contact authors for licensing.
