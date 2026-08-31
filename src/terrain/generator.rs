//! Terrain generator — produces a `TerrainMap` from `TerrainConfig` + `GridConfig`.
//!
//! Four terrain types are supported:
//! - **UrbanGrid**  — dense regular block grid, tall buildings, close tower spacing.
//! - **Suburban**   — irregular street spacing via Perlin noise, sparser buildings.
//! - **Rural**      — minimal roads, open field with Simplex noise height variation.
//! - **Highway**    — corridor road, towers evenly spaced along the highway.

use noise::{NoiseFn, Perlin, Simplex};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::config::{GridConfig, TerrainConfig, TerrainType};
use crate::terrain::types::{Rect, TerrainMap};

/// Path-loss grid resolution (N×N cells).
const GRID_N: usize = 50;

impl TerrainMap {
    /// Procedurally generate a `TerrainMap` from the given configuration.
    ///
    /// Generation is deterministic for equal `config.seed` values.
    pub fn generate(config: &TerrainConfig, grid: &GridConfig) -> Self {
        let width_m  = grid.total_length() as f32;
        let height_m = grid.total_length() as f32;
        let block    = grid.block_size     as f32;
        let seed     = config.seed;

        match config.terrain_type {
            TerrainType::UrbanGrid => {
                generate_urban(config, width_m, height_m, block, seed)
            }
            TerrainType::Suburban => {
                generate_suburban(config, width_m, height_m, block, seed)
            }
            TerrainType::Rural => {
                generate_rural(config, width_m, height_m, block, seed)
            }
            TerrainType::Highway => {
                generate_highway(config, width_m, height_m, block, seed)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// UrbanGrid
// ---------------------------------------------------------------------------

fn generate_urban(
    config: &TerrainConfig,
    width_m: f32,
    height_m: f32,
    block: f32,
    seed: u64,
) -> TerrainMap {
    let mut rng = StdRng::seed_from_u64(seed);

    // Regular street grid.
    let streets_h: Vec<f32> = grid_lines(0.0, height_m, block);
    let streets_v: Vec<f32> = grid_lines(0.0, width_m,  block);

    // One building per block, ~70% of block size with a small random offset.
    let density = config.building_density.clamp(0.0, 1.0);
    let mut buildings = Vec::new();
    for &sy in &streets_h {
        for &sx in &streets_v {
            if rng.gen::<f32>() > density {
                continue;
            }
            let bw = block * 0.70;
            let bh = block * 0.70;
            let max_off = block * 0.15;
            let ox: f32 = rng.gen_range(0.0..max_off);
            let oy: f32 = rng.gen_range(0.0..max_off);
            buildings.push(Rect {
                x: sx + ox,
                y: sy + oy,
                w: bw,
                h: bh,
            });
        }
    }

    // Towers: snap to nearest intersection on a tower_spacing_m grid.
    let spacing = config.tower_spacing_m;
    let tower_positions = snap_towers_to_intersections(
        &streets_v, &streets_h, spacing, width_m, height_m,
    );

    // Path-loss grid: building cells +8 dB, street cells +0 dB.
    let path_loss_grid = build_path_loss_grid(&buildings, &[], width_m, height_m, 8.0, 0.0, 0.0);

    TerrainMap {
        width_m,
        height_m,
        terrain_type: TerrainType::UrbanGrid,
        streets_h,
        streets_v,
        buildings,
        tower_positions,
        path_loss_grid,
        path_loss_grid_n: GRID_N,
    }
}

// ---------------------------------------------------------------------------
// Suburban
// ---------------------------------------------------------------------------

fn generate_suburban(
    config: &TerrainConfig,
    width_m: f32,
    height_m: f32,
    block: f32,
    seed: u64,
) -> TerrainMap {
    let mut rng = StdRng::seed_from_u64(seed);
    let perlin  = Perlin::new(seed as u32);

    // Streets: base regular grid + Perlin noise offset.
    let amplitude = block * 0.30;
    let scale     = 1.0 / (width_m * 0.5); // low-frequency variation

    let streets_h: Vec<f32> = grid_lines(0.0, height_m, block)
        .into_iter()
        .map(|y| {
            let offset = perlin.get([y as f64 * scale as f64, 0.0]) as f32 * amplitude;
            (y + offset).clamp(0.0, height_m)
        })
        .collect();

    let streets_v: Vec<f32> = grid_lines(0.0, width_m, block)
        .into_iter()
        .map(|x| {
            let offset = perlin.get([0.0, x as f64 * scale as f64]) as f32 * amplitude;
            (x + offset).clamp(0.0, width_m)
        })
        .collect();

    // Buildings: density * 0.5, larger footprints.
    let density = (config.building_density * 0.5).clamp(0.0, 1.0);
    let mut buildings = Vec::new();
    for &sy in &streets_h {
        for &sx in &streets_v {
            if rng.gen::<f32>() > density {
                continue;
            }
            let bw = block * 0.55 + rng.gen_range(0.0..block * 0.25);
            let bh = block * 0.55 + rng.gen_range(0.0..block * 0.25);
            buildings.push(Rect { x: sx, y: sy, w: bw, h: bh });
        }
    }

    // Towers: wider spacing.
    let spacing = config.tower_spacing_m * 1.5;
    let tower_positions = snap_towers_to_intersections(
        &streets_v, &streets_h, spacing, width_m, height_m,
    );

    // Path-loss grid: moderate building contribution +5 dB.
    let path_loss_grid = build_path_loss_grid(&buildings, &[], width_m, height_m, 5.0, 0.0, 0.0);

    TerrainMap {
        width_m,
        height_m,
        terrain_type: TerrainType::Suburban,
        streets_h,
        streets_v,
        buildings,
        tower_positions,
        path_loss_grid,
        path_loss_grid_n: GRID_N,
    }
}

// ---------------------------------------------------------------------------
// Rural
// ---------------------------------------------------------------------------

fn generate_rural(
    config: &TerrainConfig,
    width_m: f32,
    height_m: f32,
    _block: f32,
    seed: u64,
) -> TerrainMap {
    let mut rng     = StdRng::seed_from_u64(seed);
    let simplex     = Simplex::new(seed as u32);
    let noise_scale = 1.0 / (width_m as f64 * 0.3);

    // 2–3 main horizontal and vertical roads.
    let n_h = rng.gen_range(2usize..=3);
    let n_v = rng.gen_range(2usize..=3);
    let streets_h: Vec<f32> = (1..=n_h)
        .map(|i| height_m * (i as f32) / (n_h as f32 + 1.0))
        .collect();
    let streets_v: Vec<f32> = (1..=n_v)
        .map(|i| width_m * (i as f32) / (n_v as f32 + 1.0))
        .collect();

    // Buildings: very sparse clusters at road intersections only.
    let density = (config.building_density * 0.1).clamp(0.0, 1.0);
    let cluster_radius = width_m * 0.04;
    let mut buildings = Vec::new();
    for &sy in &streets_h {
        for &sx in &streets_v {
            // A few buildings scattered near each intersection.
            let count = rng.gen_range(0usize..=3);
            for _ in 0..count {
                if rng.gen::<f32>() > density * 5.0 {
                    continue;
                }
                let ox: f32 = rng.gen_range(-cluster_radius..cluster_radius);
                let oy: f32 = rng.gen_range(-cluster_radius..cluster_radius);
                let bw = width_m * 0.02 + rng.gen_range(0.0..width_m * 0.02);
                let bh = bw;
                buildings.push(Rect {
                    x: (sx + ox).clamp(0.0, width_m  - bw),
                    y: (sy + oy).clamp(0.0, height_m - bh),
                    w: bw,
                    h: bh,
                });
            }
        }
    }

    // Macro-cell towers: very wide spacing.
    let spacing = config.tower_spacing_m * 4.0;
    let tower_positions = snap_towers_to_intersections(
        &streets_v, &streets_h, spacing, width_m, height_m,
    );

    // Path-loss grid: mostly open (0 dB) with slight terrain noise (±2 dB).
    let path_loss_grid: Vec<f32> = (0..GRID_N * GRID_N)
        .map(|idx| {
            let row = idx / GRID_N;
            let col = idx % GRID_N;
            let nx  = col as f64 * noise_scale * width_m  as f64;
            let ny  = row as f64 * noise_scale * height_m as f64;
            simplex.get([nx, ny]) as f32 * 2.0 // ±2 dB
        })
        .collect();

    TerrainMap {
        width_m,
        height_m,
        terrain_type: TerrainType::Rural,
        streets_h,
        streets_v,
        buildings,
        tower_positions,
        path_loss_grid,
        path_loss_grid_n: GRID_N,
    }
}

// ---------------------------------------------------------------------------
// Highway
// ---------------------------------------------------------------------------

fn generate_highway(
    config: &TerrainConfig,
    width_m: f32,
    height_m: f32,
    block: f32,
    seed: u64,
) -> TerrainMap {
    let lane_width = 20.0_f32; // metres per lane pair

    // Two parallel horizontal roads forming the highway corridor.
    let centre_y = height_m * 0.5;
    let streets_h = vec![centre_y - lane_width, centre_y + lane_width];

    // A few vertical connector roads, evenly spaced.
    let n_connectors = ((width_m / block) as usize).max(2).min(6);
    let streets_v: Vec<f32> = (0..n_connectors)
        .map(|i| width_m * (i as f32 + 1.0) / (n_connectors as f32 + 1.0))
        .collect();

    // Buildings: none on the highway itself; small clusters at junctions.
    let mut rng = StdRng::seed_from_u64(seed);
    let cluster_r = block * 0.25;
    let mut buildings = Vec::new();
    for &sx in &streets_v {
        let count = rng.gen_range(1usize..=4);
        for _ in 0..count {
            let ox: f32 = rng.gen_range(-cluster_r..cluster_r);
            // Place clear of the highway corridor (at least lane_width * 2.5 from centre).
            let sign: f32 = if rng.gen::<bool>() { 1.0 } else { -1.0 };
            let oy: f32 = sign * (lane_width * 2.5 + rng.gen_range(0.0..cluster_r));
            let bw = block * 0.15 + rng.gen_range(0.0..block * 0.10);
            let bh = bw;
            let bx = (sx + ox).clamp(0.0, width_m  - bw);
            let by = (centre_y + oy).clamp(0.0, height_m - bh);
            buildings.push(Rect { x: bx, y: by, w: bw, h: bh });
        }
    }

    // Towers: evenly spaced along the highway corridor at tower_spacing_m.
    let spacing   = config.tower_spacing_m;
    let mut x_pos = spacing * 0.5;
    let mut tower_positions = Vec::new();
    while x_pos < width_m {
        tower_positions.push([x_pos, centre_y]);
        x_pos += spacing;
    }
    // Guarantee at least 1 tower if the map is shorter than the spacing.
    if tower_positions.is_empty() {
        tower_positions.push([width_m * 0.5, centre_y]);
    }

    // Path-loss grid: highway open (0 dB), off-road +3 dB.
    let path_loss_grid: Vec<f32> = (0..GRID_N * GRID_N)
        .map(|idx| {
            let row  = idx / GRID_N;
            let y    = (row as f32 + 0.5) / GRID_N as f32 * height_m;
            let dist = (y - centre_y).abs();
            if dist <= lane_width * 2.0 { 0.0 } else { 3.0 }
        })
        .collect();

    TerrainMap {
        width_m,
        height_m,
        terrain_type: TerrainType::Highway,
        streets_h,
        streets_v,
        buildings,
        tower_positions,
        path_loss_grid,
        path_loss_grid_n: GRID_N,
    }
}

// ---------------------------------------------------------------------------
// Helper utilities
// ---------------------------------------------------------------------------

/// Produces street coordinates from `start` to `end` at `spacing` intervals.
///
/// Streets begin at position `0` (the boundary) and repeat every `spacing`
/// metres, matching the regular urban block layout.
fn grid_lines(start: f32, end: f32, spacing: f32) -> Vec<f32> {
    let mut lines = Vec::new();
    let mut pos = start;
    while pos <= end {
        lines.push(pos);
        pos += spacing;
    }
    lines
}

/// Snap a regular tower grid (spacing × spacing) to the nearest street
/// intersection so towers sit at road crossings rather than in the middle of
/// blocks.
fn snap_towers_to_intersections(
    streets_v: &[f32],
    streets_h: &[f32],
    spacing: f32,
    width_m: f32,
    height_m: f32,
) -> Vec<[f32; 2]> {
    if streets_v.is_empty() || streets_h.is_empty() {
        return vec![[width_m * 0.5, height_m * 0.5]];
    }

    let mut positions: Vec<[f32; 2]> = Vec::new();
    let mut tx = spacing * 0.5;
    while tx < width_m {
        let mut ty = spacing * 0.5;
        while ty < height_m {
            // Snap to nearest intersection.
            let snapped_x = snap_to(streets_v, tx);
            let snapped_y = snap_to(streets_h, ty);
            let candidate = [snapped_x, snapped_y];
            // De-duplicate: skip if already present.
            if !positions.iter().any(|p| p == &candidate) {
                positions.push(candidate);
            }
            ty += spacing;
        }
        tx += spacing;
    }

    // Guarantee at least one tower.
    if positions.is_empty() {
        let cx = snap_to(streets_v, width_m  * 0.5);
        let cy = snap_to(streets_h, height_m * 0.5);
        positions.push([cx, cy]);
    }

    positions
}

/// Return the value in `candidates` closest to `target`.
fn snap_to(candidates: &[f32], target: f32) -> f32 {
    candidates
        .iter()
        .copied()
        .min_by(|a, b| (a - target).abs().partial_cmp(&(b - target).abs()).unwrap())
        .unwrap_or(target)
}

/// Build the N×N path-loss grid.
///
/// Each cell value is the sum of:
/// - `building_db` if the cell centre is inside any building footprint,
/// - `open_db` otherwise,
/// - `base_noise` passed through as a constant baseline (Rural uses its own
///   noise; other types use 0).
fn build_path_loss_grid(
    buildings: &[Rect],
    _extra: &[f32], // reserved for future terrain-height data
    width_m: f32,
    height_m: f32,
    building_db: f32,
    open_db: f32,
    base_noise: f32,
) -> Vec<f32> {
    (0..GRID_N * GRID_N)
        .map(|idx| {
            let row = idx / GRID_N;
            let col = idx % GRID_N;
            let cx  = (col as f32 + 0.5) / GRID_N as f32 * width_m;
            let cy  = (row as f32 + 0.5) / GRID_N as f32 * height_m;
            let in_building = buildings.iter().any(|b| b.contains([cx, cy]));
            base_noise + if in_building { building_db } else { open_db }
        })
        .collect()
}
