//! Integration tests for the terrain generator (Sub-Task 3).

use martini::config::{GridConfig, TerrainConfig, TerrainType};
use martini::terrain::TerrainMap;

// ---------------------------------------------------------------------------
// Helper: build default configs with a given terrain type.
// ---------------------------------------------------------------------------

fn make_terrain(terrain_type: TerrainType, seed: u64) -> TerrainMap {
    let config = TerrainConfig {
        terrain_type,
        seed,
        building_density: 0.7,
        tower_spacing_m: 500.0,
    };
    let grid = GridConfig::default(); // 12 blocks × 400 m = 4800 m
    TerrainMap::generate(&config, &grid)
}

// ---------------------------------------------------------------------------
// 1. Each terrain type generates at least 1 tower position.
// ---------------------------------------------------------------------------

#[test]
fn urban_has_at_least_one_tower() {
    let map = make_terrain(TerrainType::UrbanGrid, 1);
    assert!(!map.tower_positions.is_empty(), "UrbanGrid must have at least 1 tower");
}

#[test]
fn suburban_has_at_least_one_tower() {
    let map = make_terrain(TerrainType::Suburban, 1);
    assert!(!map.tower_positions.is_empty(), "Suburban must have at least 1 tower");
}

#[test]
fn rural_has_at_least_one_tower() {
    let map = make_terrain(TerrainType::Rural, 1);
    assert!(!map.tower_positions.is_empty(), "Rural must have at least 1 tower");
}

#[test]
fn highway_has_at_least_one_tower() {
    let map = make_terrain(TerrainType::Highway, 1);
    assert!(!map.tower_positions.is_empty(), "Highway must have at least 1 tower");
}

// ---------------------------------------------------------------------------
// 2. path_loss_at() returns a finite f32 for any in-bounds point.
// ---------------------------------------------------------------------------

#[test]
fn path_loss_at_is_finite_for_all_terrain_types() {
    let types = [
        TerrainType::UrbanGrid,
        TerrainType::Suburban,
        TerrainType::Rural,
        TerrainType::Highway,
    ];
    let samples = [(0.0f32, 0.0), (100.0, 200.0), (2400.0, 2400.0), (4799.0, 4799.0)];
    for tt in types {
        let map = make_terrain(tt, 42);
        for (x, y) in samples {
            let pl = map.path_loss_at(x, y);
            assert!(pl.is_finite(), "path_loss_at({x},{y}) must be finite, got {pl}");
        }
    }
}

// ---------------------------------------------------------------------------
// 3. UrbanGrid streets_h and streets_v are evenly spaced at block_size intervals.
// ---------------------------------------------------------------------------

#[test]
fn urban_streets_evenly_spaced() {
    let map = make_terrain(TerrainType::UrbanGrid, 1);
    let block = GridConfig::default().block_size as f32;

    // Check vertical streets.
    for window in map.streets_v.windows(2) {
        let diff = window[1] - window[0];
        assert!(
            (diff - block).abs() < 1.0,
            "streets_v gap should be block_size ({block}), got {diff}"
        );
    }
    // Check horizontal streets.
    for window in map.streets_h.windows(2) {
        let diff = window[1] - window[0];
        assert!(
            (diff - block).abs() < 1.0,
            "streets_h gap should be block_size ({block}), got {diff}"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. is_on_street() returns true for a point exactly on a street.
// ---------------------------------------------------------------------------

#[test]
fn is_on_street_true_for_point_on_street() {
    let map = make_terrain(TerrainType::UrbanGrid, 1);
    let street_width = 10.0_f32;

    // Exactly on the first horizontal street.
    let y_street = map.streets_h[0];
    assert!(
        map.is_on_street([100.0, y_street], street_width),
        "Point exactly on streets_h[0] must be on-street"
    );

    // Exactly on the first vertical street.
    let x_street = map.streets_v[0];
    assert!(
        map.is_on_street([x_street, 100.0], street_width),
        "Point exactly on streets_v[0] must be on-street"
    );
}

// ---------------------------------------------------------------------------
// 5. Tower positions are all within [0, width_m] × [0, height_m].
// ---------------------------------------------------------------------------

#[test]
fn all_towers_within_bounds() {
    let types = [
        TerrainType::UrbanGrid,
        TerrainType::Suburban,
        TerrainType::Rural,
        TerrainType::Highway,
    ];
    for tt in types {
        let map = make_terrain(tt, 7);
        for &[x, y] in &map.tower_positions {
            assert!(
                x >= 0.0 && x <= map.width_m,
                "Tower x={x} out of [0, {}]",
                map.width_m
            );
            assert!(
                y >= 0.0 && y <= map.height_m,
                "Tower y={y} out of [0, {}]",
                map.height_m
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 6. Different seeds produce different tower placements (Suburban / Rural).
// ---------------------------------------------------------------------------

#[test]
fn different_seeds_produce_different_towers_suburban() {
    let map_a = make_terrain(TerrainType::Suburban, 1);
    let map_b = make_terrain(TerrainType::Suburban, 9999);
    // Suburban uses Perlin noise for street offsets — different seeds should
    // produce at least one distinct tower coordinate.
    let any_diff = map_a.tower_positions.iter().zip(map_b.tower_positions.iter())
        .any(|(a, b)| (a[0] - b[0]).abs() > 0.01 || (a[1] - b[1]).abs() > 0.01);
    // If they have different lengths they are trivially different.
    let len_diff = map_a.tower_positions.len() != map_b.tower_positions.len();
    assert!(
        any_diff || len_diff,
        "Suburban with seed=1 and seed=9999 should differ in tower placement"
    );
}

#[test]
fn different_seeds_produce_different_towers_rural() {
    let map_a = make_terrain(TerrainType::Rural, 1);
    let map_b = make_terrain(TerrainType::Rural, 9999);
    let any_diff = map_a.tower_positions.iter().zip(map_b.tower_positions.iter())
        .any(|(a, b)| (a[0] - b[0]).abs() > 0.01 || (a[1] - b[1]).abs() > 0.01);
    let len_diff = map_a.tower_positions.len() != map_b.tower_positions.len();
    assert!(
        any_diff || len_diff,
        "Rural with seed=1 and seed=9999 should differ in tower placement"
    );
}
