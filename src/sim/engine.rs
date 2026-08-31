//! SimEngine — tick loop and background thread management.

use std::collections::VecDeque;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::config::SimSession;
use crate::terrain::TerrainMap;
use crate::sim::base::{BaseStation, build_base_stations};
use crate::sim::mobile::MobileTerminal;
use crate::sim::propagation::PropagationModel;
use crate::sim::snapshot::{SimEvent, SimEventType, SimSnapshot};
use crate::sim::statistics::Statistics;
use crate::sim::protocol::{HandoffDecision, HandoffProtocol};
use crate::sim::protocol_native::Gen4LteA3;
use crate::sim::scenario::Scenario;

/// Drives the simulation tick-by-tick.
pub struct SimEngine {
    terrain: Arc<TerrainMap>,
    session: SimSession,
    mobiles: Vec<MobileTerminal>,
    bases: Vec<BaseStation>,
    stats: Statistics,
    events: VecDeque<SimEvent>,
    rng: StdRng,
    tick: u64,
    target_count: u32,
    next_mobile_id: u32,
    /// When `true` the engine skips the 100 ms sleep (headless / A-B mode).
    headless: bool,
    /// Active handoff protocol.
    protocol: Box<dyn HandoffProtocol>,
    /// Optional scenario providing scheduled events.
    scenario: Option<Scenario>,
}

impl SimEngine {
    /// Build a new engine from a session configuration.
    pub fn new(session: &SimSession) -> Self {
        let seed = if session.sim_seed == 0 {
            rand::random::<u64>()
        } else {
            session.sim_seed
        };
        let mut rng = StdRng::seed_from_u64(seed);

        let terrain = Arc::new(TerrainMap::generate(&session.terrain, &session.grid));
        let bases = build_base_stations(&terrain, &session.radio);
        let target_count = session.target_mobile_count;

        // Seed initial mobiles.
        let profiles = [
            crate::config::TrafficProfile::VideoStream,
            crate::config::TrafficProfile::CloudGaming,
            crate::config::TrafficProfile::VoiceCall,
            crate::config::TrafficProfile::Idle,
            crate::config::TrafficProfile::WebBrowse,
        ];
        let mut mobiles = Vec::with_capacity(target_count as usize);
        for i in 0..target_count {
            let profile = profiles[(i as usize) % profiles.len()].clone();
            mobiles.push(MobileTerminal::new(i, profile, &session.grid, &session.speed, &mut rng));
        }

        Self {
            terrain,
            session: session.clone(),
            mobiles,
            bases,
            stats: Statistics::default(),
            events: VecDeque::with_capacity(500),
            rng,
            tick: 0,
            target_count,
            next_mobile_id: target_count,
            headless: false,
            protocol: Box::new(Gen4LteA3),
            scenario: None,
        }
    }

    // -----------------------------------------------------------------------
    // start — spawn the background thread
    // -----------------------------------------------------------------------

    /// Start the sim loop on a background thread.
    ///
    /// Returns `(stop_tx, snapshot_rx)`.  Send any value on `stop_tx` to
    /// request a graceful shutdown.  Drop `snapshot_rx` to also trigger a stop.
    ///
    /// `protocol` — override the default Gen4 LTE A3 protocol.
    /// `scenario` — optional scenario providing scheduled events.
    pub fn start(
        session: SimSession,
        headless: bool,
        protocol: Option<Box<dyn HandoffProtocol>>,
        scenario: Option<Scenario>,
    ) -> (mpsc::Sender<()>, mpsc::Receiver<SimSnapshot>) {
        let (snap_tx, snap_rx) = mpsc::channel::<SimSnapshot>();
        let (stop_tx, stop_rx) = mpsc::channel::<()>();

        thread::spawn(move || {
            let mut engine = SimEngine::new(&session);
            engine.headless = headless;
            if let Some(p) = protocol {
                engine.protocol = p;
            }
            engine.scenario = scenario;

            loop {
                let t0 = Instant::now();
                engine.tick();
                let snapshot = engine.build_snapshot();

                if snap_tx.send(snapshot).is_err() {
                    break; // UI receiver disconnected
                }
                if stop_rx.try_recv().is_ok() {
                    break; // explicit stop signal
                }
                if !engine.headless {
                    let elapsed = t0.elapsed();
                    if elapsed < Duration::from_millis(100) {
                        thread::sleep(Duration::from_millis(100) - elapsed);
                    }
                }
            }
        });

        (stop_tx, snap_rx)
    }

    // -----------------------------------------------------------------------
    // tick — advance the simulation by one step
    // -----------------------------------------------------------------------

    fn tick(&mut self) {
        self.tick += 1;

        // ── 0. Apply scenario events ─────────────────────────────────────────
        if let Some(scenario) = &self.scenario.clone() {
            for event in scenario.events_at(self.tick) {
                use crate::sim::scenario::ScenarioEventKind::*;
                match event {
                    TowerFailure { base_id } => {
                        let base_id = *base_id;
                        if let Some(b) = self.bases.iter_mut().find(|b| b.id == base_id) {
                            b.failed = true;
                        }
                        self.push_event(SimEvent {
                            tick: self.tick,
                            mobile_id: 0,
                            event_type: SimEventType::TowerFailure,
                            detail: format!("Tower #{base_id} failed (scenario)"),
                        });
                    }
                    TowerRestore { base_id } => {
                        let base_id = *base_id;
                        if let Some(b) = self.bases.iter_mut().find(|b| b.id == base_id) {
                            b.failed = false;
                        }
                        self.push_event(SimEvent {
                            tick: self.tick,
                            mobile_id: 0,
                            event_type: SimEventType::TowerRestore,
                            detail: format!("Tower #{base_id} restored (scenario)"),
                        });
                    }
                    MobileSurge { count } => {
                        let count = *count;
                        let profiles = [
                            crate::config::TrafficProfile::VideoStream,
                            crate::config::TrafficProfile::CloudGaming,
                            crate::config::TrafficProfile::VoiceCall,
                            crate::config::TrafficProfile::Idle,
                            crate::config::TrafficProfile::WebBrowse,
                        ];
                        for _ in 0..count {
                            let id = self.next_mobile_id;
                            self.next_mobile_id += 1;
                            let profile = profiles[(id as usize) % profiles.len()].clone();
                            let m = MobileTerminal::new(id, profile, &self.session.grid, &self.session.speed, &mut self.rng);
                            self.mobiles.push(m);
                            self.stats.total_arrivals += 1;
                        }
                        self.push_event(SimEvent {
                            tick: self.tick,
                            mobile_id: 0,
                            event_type: SimEventType::MobileSurge,
                            detail: format!("Mobile surge: +{count} mobiles (scenario)"),
                        });
                    }
                }
            }
        }

        // ── 1. Move all mobiles ─────────────────────────────────────────────
        for mobile in &mut self.mobiles {
            mobile.step(
                0.1,
                &self.session.grid,
                &self.session.speed,
                &self.terrain,
                &mut self.rng,
            );
        }

        // ── 2 & 3. Compute SINR and assign serving cell ─────────────────────
        let radio = &self.session.radio;
        let noise_dbm = radio.thermal_noise_dbm + radio.noise_figure_db;

        // Pre-compute received power from every base to every mobile.
        // Shape: [mobile_idx][base_idx] → received_power_dbm
        let n_mob = self.mobiles.len();
        let n_bas = self.bases.len();
        let mut rx_power: Vec<Vec<f32>> = vec![vec![f32::NEG_INFINITY; n_bas]; n_mob];

        for (mi, mobile) in self.mobiles.iter().enumerate() {
            let [mx, my] = mobile.position;
            for (bi, base) in self.bases.iter().enumerate() {
                if base.failed {
                    continue;
                }
                let [bx, by] = base.position;
                let dist = ((mx - bx).powi(2) + (my - by).powi(2)).sqrt().max(1.0);
                let terrain_offset = self.terrain.path_loss_at(mx, my);
                let pl = PropagationModel::path_loss_db(dist, radio, terrain_offset);
                rx_power[mi][bi] = PropagationModel::received_power_dbm(radio.max_tx_power_dbm, pl);
            }
        }

        // For each mobile: best SINR base = serving_cell.
        for (mi, mobile) in self.mobiles.iter_mut().enumerate() {
            let mut best_sinr = f32::NEG_INFINITY;
            let mut best_base: Option<u32> = None;

            for (bi, base) in self.bases.iter().enumerate() {
                if base.failed {
                    continue;
                }
                let signal_dbm = rx_power[mi][bi];
                if signal_dbm == f32::NEG_INFINITY {
                    continue;
                }
                // All other active bases are interferers.
                let interferers: Vec<f32> = (0..n_bas)
                    .filter(|&j| j != bi && !self.bases[j].failed)
                    .map(|j| rx_power[mi][j])
                    .filter(|&v| v != f32::NEG_INFINITY)
                    .collect();

                let sinr = PropagationModel::sinr_db(signal_dbm, noise_dbm, &interferers);
                if sinr > best_sinr {
                    best_sinr = sinr;
                    best_base = Some(base.id);
                }
            }

            mobile.sinr_db = best_sinr;
            mobile.rsrp_dbm = best_base
                .map(|bid| rx_power[mi][bid as usize])
                .unwrap_or(f32::NEG_INFINITY);
            mobile.serving_cell = best_base;
            mobile.push_sinr(best_sinr);
        }

        // ── 3b. Apply handoff protocol decisions ────────────────────────────
        let radio_clone = self.session.radio.clone();
        let bases_snapshot = self.bases.clone();

        let mut ho_events: Vec<SimEvent> = Vec::new();

        for mobile in &mut self.mobiles {
            let decision = self.protocol.decide(mobile, &bases_snapshot, &radio_clone);
            match decision {
                HandoffDecision::Stay => {}
                HandoffDecision::HandoffTo { target_id } => {
                    let old_cell = mobile.serving_cell;
                    mobile.serving_cell = Some(target_id);
                    mobile.handoff_count += 1;
                    self.stats.handoff_attempts += 1;
                    self.stats.handoff_successes += 1;
                    ho_events.push(SimEvent {
                        tick: self.tick,
                        mobile_id: mobile.id,
                        event_type: SimEventType::HandoffSuccess,
                        detail: format!(
                            "Mobile #{}: {} → #{}",
                            mobile.id,
                            old_cell.map(|id| format!("#{id}")).unwrap_or_else(|| "none".into()),
                            target_id
                        ),
                    });
                }
                HandoffDecision::SoftAdd { add_id } => {
                    if !mobile.active_set.contains(&add_id) {
                        mobile.active_set.push(add_id);
                        mobile.handoff_count += 1;
                        self.stats.handoff_attempts += 1;
                        self.stats.handoff_successes += 1;
                    }
                }
                HandoffDecision::SoftRemove { remove_id } => {
                    mobile.active_set.retain(|&id| id != remove_id);
                }
                HandoffDecision::ConditionalPrepare { target_id: _ } => {
                    // Prepare phase — no serving-cell change yet.
                }
                HandoffDecision::ConditionalExecute { target_id } => {
                    let old_cell = mobile.serving_cell;
                    mobile.serving_cell = Some(target_id);
                    mobile.handoff_count += 1;
                    self.stats.handoff_attempts += 1;
                    self.stats.handoff_successes += 1;
                    ho_events.push(SimEvent {
                        tick: self.tick,
                        mobile_id: mobile.id,
                        event_type: SimEventType::HandoffSuccess,
                        detail: format!(
                            "Mobile #{}: CHO {} → #{}",
                            mobile.id,
                            old_cell.map(|id| format!("#{id}")).unwrap_or_else(|| "none".into()),
                            target_id
                        ),
                    });
                }
            }
        }
        for ev in ho_events {
            self.push_event(ev);
        }

        // ── 4. Compute throughput and latency ───────────────────────────────
        // Build a snapshot of base load percentages (keyed by base index/id).
        let base_load: Vec<f32> = self.bases.iter().map(|b| b.load_percent()).collect();
        let base_rtt = radio_clone.base_rtt_ms();

        // Collect SLA-violation events to emit after the mutable loop.
        let mut sla_events: Vec<SimEvent> = Vec::new();

        for mobile in &mut self.mobiles {
            let load_pct = mobile
                .serving_cell
                .and_then(|bid| base_load.get(bid as usize).copied())
                .unwrap_or(0.0);

            mobile.dl_throughput_mbps = PropagationModel::shannon_capacity_mbps(mobile.sinr_db, &radio_clone);
            let lat = PropagationModel::latency_ms(base_rtt, load_pct, false);
            mobile.latency_ms = lat;
            mobile.push_latency(lat);

            // ── 5. SLA check ────────────────────────────────────────────────
            let budget = mobile.profile.latency_budget_ms();
            if lat > budget {
                if !mobile.sla_violated {
                    self.stats.sla_violations += 1;
                    sla_events.push(SimEvent {
                        tick: self.tick,
                        mobile_id: mobile.id,
                        event_type: SimEventType::SlaViolation,
                        detail: format!(
                            "latency {:.1}ms > budget {:.1}ms",
                            lat, budget
                        ),
                    });
                }
                mobile.sla_violated = true;
            } else {
                mobile.sla_violated = false;
            }
        }
        for ev in sla_events {
            self.push_event(ev);
        }

        // ── 6. Despawn mobiles ──────────────────────────────────────────────
        let tick_now = self.tick;
        let mut departed_ids: Vec<u32> = Vec::new();
        self.mobiles.retain(|m| {
            if self.rng.gen::<f32>() < 0.003 {
                departed_ids.push(m.id);
                false
            } else {
                true
            }
        });
        for id in departed_ids {
            self.stats.total_finished += 1;
            self.push_event(SimEvent {
                tick: tick_now,
                mobile_id: id,
                event_type: SimEventType::Departure,
                detail: "normal departure".to_string(),
            });
        }

        // ── 7. Spawn new mobiles to reach target count ──────────────────────
        let profiles = [
            crate::config::TrafficProfile::VideoStream,
            crate::config::TrafficProfile::CloudGaming,
            crate::config::TrafficProfile::VoiceCall,
            crate::config::TrafficProfile::Idle,
            crate::config::TrafficProfile::WebBrowse,
        ];
        while self.mobiles.len() < self.target_count as usize {
            let id = self.next_mobile_id;
            self.next_mobile_id += 1;
            let profile = profiles[(id as usize) % profiles.len()].clone();
            let m = MobileTerminal::new(id, profile, &self.session.grid, &self.session.speed, &mut self.rng);
            self.stats.total_arrivals += 1;
            self.push_event(SimEvent {
                tick: tick_now,
                mobile_id: id,
                event_type: SimEventType::Arrival,
                detail: "new mobile spawned".to_string(),
            });
            self.mobiles.push(m);
        }

        // ── 8. Rebuild connected_mobiles lists ──────────────────────────────
        for base in &mut self.bases {
            base.connected_mobiles.clear();
        }
        for mobile in &self.mobiles {
            if let Some(bid) = mobile.serving_cell {
                if let Some(base) = self.bases.get_mut(bid as usize) {
                    base.connected_mobiles.push(mobile.id);
                }
            }
        }

        // ── 9. Rolling averages ─────────────────────────────────────────────
        let n = self.mobiles.len() as f32;
        if n > 0.0 {
            self.stats.avg_sinr_db =
                self.mobiles.iter().map(|m| m.sinr_db).sum::<f32>() / n;
            self.stats.avg_throughput_mbps =
                self.mobiles.iter().map(|m| m.dl_throughput_mbps).sum::<f32>() / n;
            self.stats.avg_latency_ms =
                self.mobiles.iter().map(|m| m.latency_ms).sum::<f32>() / n;
        }

        // ── 10. Update tick counter ─────────────────────────────────────────
        self.stats.tick = self.tick;
        self.stats.active_mobiles = self.mobiles.len() as u32;
    }

    // -----------------------------------------------------------------------
    // build_snapshot — clone current state into a shareable snapshot
    // -----------------------------------------------------------------------

    fn build_snapshot(&self) -> SimSnapshot {
        SimSnapshot {
            tick: self.tick,
            mobiles: self.mobiles.clone(),
            bases: self.bases.clone(),
            stats: self.stats.clone(),
            events: self.events.clone(),
            terrain: Arc::clone(&self.terrain),
        }
    }

    // -----------------------------------------------------------------------
    // push_event — append to ring buffer (max 500)
    // -----------------------------------------------------------------------

    fn push_event(&mut self, event: SimEvent) {
        if self.events.len() == 500 {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    // -----------------------------------------------------------------------
    // Test helpers — only compiled for `#[cfg(test)]` consumers
    // -----------------------------------------------------------------------

    /// Advance the engine by one tick.  Exposed for integration tests only.
    #[doc(hidden)]
    pub fn tick_for_test(&mut self) {
        self.tick();
    }

    /// Return a snapshot of current state.  Exposed for integration tests only.
    #[doc(hidden)]
    pub fn snapshot_for_test(&self) -> SimSnapshot {
        self.build_snapshot()
    }
}
