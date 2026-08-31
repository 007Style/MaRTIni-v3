//! Integration tests for the configuration model (Sub-Task 2).

use martini::config::{
    GridConfig, RadioConfig, RadioTechnology, SimSession, SpeedConfig, TrafficProfile,
};

// ---------------------------------------------------------------------------
// 1. GridConfig::default() — derived field values
// ---------------------------------------------------------------------------
#[test]
fn grid_config_default_derived_fields() {
    let g = GridConfig::default();
    assert_eq!(g.total_length(), 4800, "total_length = no_block(12) * block_size(400)");
    assert_eq!(g.max_coord(), 4750, "max_coord = total_length(4800) - dist_res(50)");
    assert_eq!(g.points_per_block(), 8, "points_per_block = block_size(400) / dist_res(50)");
    assert_eq!(g.total_points(), 96, "total_points = no_block(12) * points_per_block(8)");
}

// ---------------------------------------------------------------------------
// 2. GridConfig::validate() — constraint violations
// ---------------------------------------------------------------------------
#[test]
fn grid_config_validate_catches_bad_values() {
    // no_block = 0 violates 1–50 range
    let g_zero_blocks = GridConfig {
        no_block: 0,
        ..GridConfig::default()
    };
    assert!(
        g_zero_blocks.validate().is_err(),
        "no_block=0 should fail validation"
    );

    // block_size = 50 violates 100–2000 range
    let g_small_block = GridConfig {
        block_size: 50,
        ..GridConfig::default()
    };
    assert!(
        g_small_block.validate().is_err(),
        "block_size=50 should fail validation"
    );

    // Default should pass
    assert!(GridConfig::default().validate().is_ok());
}

// ---------------------------------------------------------------------------
// 3. TrafficProfile::dl_demand_mbps() — all five profiles
// ---------------------------------------------------------------------------
#[test]
fn traffic_profile_dl_demand() {
    assert!((TrafficProfile::VideoStream.dl_demand_mbps()  - 15.0).abs()    < f32::EPSILON);
    assert!((TrafficProfile::CloudGaming.dl_demand_mbps()  - 50.0).abs()    < f32::EPSILON);
    assert!((TrafficProfile::VoiceCall.dl_demand_mbps()    - 0.1).abs()     < f32::EPSILON);
    assert!((TrafficProfile::Idle.dl_demand_mbps()         - 0.001).abs()   < f32::EPSILON);
    assert!((TrafficProfile::WebBrowse.dl_demand_mbps()    - 5.0).abs()     < f32::EPSILON);
}

// ---------------------------------------------------------------------------
// 4. RadioConfig spectral efficiency — all four generations
// ---------------------------------------------------------------------------
#[test]
fn radio_config_spectral_efficiency() {
    let make = |tech: RadioTechnology| RadioConfig {
        technology: tech,
        ..RadioConfig::default()
    };

    assert!((make(RadioTechnology::Gen3Umts).spectral_efficiency()     - 0.4).abs() < f32::EPSILON);
    assert!((make(RadioTechnology::Gen4Lte).spectral_efficiency()      - 0.6).abs() < f32::EPSILON);
    assert!((make(RadioTechnology::Gen5NrSub6).spectral_efficiency()   - 0.8).abs() < f32::EPSILON);
    assert!((make(RadioTechnology::Gen5NrMmWave).spectral_efficiency() - 0.9).abs() < f32::EPSILON);
}

// ---------------------------------------------------------------------------
// 5. SimSession TOML round-trip
// ---------------------------------------------------------------------------
#[test]
fn sim_session_toml_round_trip() {
    let original = SimSession::default();
    let toml_str = toml::to_string_pretty(&original).expect("serialise to TOML");
    let restored: SimSession = toml::from_str(&toml_str).expect("deserialise from TOML");

    // Compare field-by-field (structs do not derive PartialEq to keep them
    // lightweight; spot-check the key scalars instead).
    assert_eq!(restored.grid.no_block,             original.grid.no_block);
    assert_eq!(restored.grid.block_size,           original.grid.block_size);
    assert_eq!(restored.grid.dist_res,             original.grid.dist_res);
    assert_eq!(restored.speed.mean_speed_kmh,      original.speed.mean_speed_kmh);
    assert_eq!(restored.radio.no_base,             original.radio.no_base);
    assert_eq!(restored.radio.bandwidth_mhz,       original.radio.bandwidth_mhz);
    assert_eq!(restored.terrain.seed,              original.terrain.seed);
    assert_eq!(restored.target_mobile_count,       original.target_mobile_count);
    assert_eq!(restored.sim_seed,                  original.sim_seed);
    assert_eq!(restored.radio.technology,          original.radio.technology);
    assert_eq!(restored.terrain.terrain_type,      original.terrain.terrain_type);
}

// ---------------------------------------------------------------------------
// 6. SpeedConfig::speed_mps() — 90 km/h == 25.0 m/s
// ---------------------------------------------------------------------------
#[test]
fn speed_config_speed_mps() {
    let s = SpeedConfig {
        mean_speed_kmh: 90.0,
        ..SpeedConfig::default()
    };
    let mps = s.speed_mps();
    assert!(
        (mps - 25.0).abs() < 1e-4,
        "90 km/h should be 25.0 m/s, got {mps}"
    );
}
