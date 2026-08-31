//! Integration tests for handoff decision logic and mobile/base model correctness.

use martini::config::{SimSession, TrafficProfile};
use martini::sim::{MobileTerminal, BaseStation};
use martini::terrain::TerrainMap;
use rand::SeedableRng;

fn make_session() -> SimSession {
    SimSession::default()
}

/// A freshly-created MobileTerminal has no serving cell.
#[test]
fn mobile_spawns_with_no_serving_cell() {
    let session = make_session();
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let mobile = MobileTerminal::new(
        0,
        TrafficProfile::VideoStream,
        &session.grid,
        &session.speed,
        &mut rng,
    );
    assert!(
        mobile.serving_cell.is_none(),
        "newly created mobile should have serving_cell = None"
    );
}

/// After one engine tick, every mobile should have a serving cell assigned.
#[test]
fn mobile_gets_serving_cell_after_tick() {
    use martini::sim::SimEngine;
    // Run one tick headless; we expect at least one mobile to have a serving cell.
    let session = make_session();
    let mut engine = SimEngine::new(&session);
    // Run two ticks so radio assignment stabilises.
    engine.tick_for_test();
    engine.tick_for_test();
    let snap = engine.snapshot_for_test();
    let all_assigned = snap.mobiles.iter().all(|m| m.serving_cell.is_some());
    assert!(all_assigned, "all mobiles should have a serving_cell after tick");
}

/// BaseStation::load_percent returns 0.0 when no mobiles are connected.
#[test]
fn base_station_load_percent_zero_when_empty() {
    let session = make_session();
    let base = BaseStation::new(0, [0.0, 0.0], &session.radio);
    assert_eq!(base.load_percent(), 0.0);
}

/// BaseStation::is_at_capacity returns true when connected == total_channels.
#[test]
fn base_station_is_at_capacity_when_full() {
    let session = make_session();
    let mut base = BaseStation::new(0, [0.0, 0.0], &session.radio);
    // Fill all channels.
    for i in 0..base.total_channels {
        base.connected_mobiles.push(i);
    }
    assert!(base.is_at_capacity(), "base should report at-capacity when all channels are filled");
}
