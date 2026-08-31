//! Core terrain data structures: `Rect`, `TerrainMap`, and helper methods.
//!
//! This module intentionally has **no egui dependency** — terrain data is a
//! pure simulation concern.  UI layers can convert `[f32; 2]` to `egui::Pos2`
//! at the boundary.

// ---------------------------------------------------------------------------
// Rect — axis-aligned bounding box in metres
// ---------------------------------------------------------------------------

/// An axis-aligned rectangle in the simulation coordinate system (metres).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Rect {
    /// Left edge (x), metres.
    pub x: f32,
    /// Bottom edge (y), metres.
    pub y: f32,
    /// Width (metres).
    pub w: f32,
    /// Height (metres).
    pub h: f32,
}

impl Rect {
    /// Returns `true` if `pos` falls inside (or on the boundary of) this rect.
    pub fn contains(&self, pos: [f32; 2]) -> bool {
        let [px, py] = pos;
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }

    /// Centre of the rect as `[x, y]` in metres.
    pub fn center(&self) -> [f32; 2] {
        [self.x + self.w * 0.5, self.y + self.h * 0.5]
    }
}

// ---------------------------------------------------------------------------
// TerrainMap — the main output of the terrain generator
// ---------------------------------------------------------------------------

/// A fully-generated terrain map for one simulation run.
///
/// All positional data is in **metres** from the bottom-left origin `(0, 0)`.
/// Positions are stored as `[f32; 2]` (x, y) to avoid any UI-crate dependency.
#[derive(Debug, Clone)]
pub struct TerrainMap {
    /// Total map width in metres.
    pub width_m: f32,
    /// Total map height in metres.
    pub height_m: f32,
    /// The terrain classification used during generation.
    pub terrain_type: crate::config::TerrainType,
    /// Y-coordinates (metres) of horizontal streets.
    pub streets_h: Vec<f32>,
    /// X-coordinates (metres) of vertical streets.
    pub streets_v: Vec<f32>,
    /// Building footprints (metres).
    pub buildings: Vec<Rect>,
    /// Base-station positions `[x, y]` in metres.
    pub tower_positions: Vec<[f32; 2]>,
    /// Flat `N×N` grid of path-loss offsets in dB.
    ///
    /// Index `row * path_loss_grid_n + col`, where `row` increases with y and
    /// `col` increases with x.
    pub path_loss_grid: Vec<f32>,
    /// Resolution of `path_loss_grid` along each axis (typically 50).
    pub path_loss_grid_n: usize,
}

impl TerrainMap {
    // -----------------------------------------------------------------------
    // path_loss_at — bilinear interpolation on the coarse NxN grid
    // -----------------------------------------------------------------------

    /// Returns the interpolated path-loss offset in dB at the given position.
    ///
    /// Clamps `(x, y)` to the map boundary before sampling so out-of-bounds
    /// queries are always well-defined.
    pub fn path_loss_at(&self, x: f32, y: f32) -> f32 {
        let n = self.path_loss_grid_n;
        if n == 0 || self.path_loss_grid.is_empty() {
            return 0.0;
        }

        // Normalise (x, y) → [0, n-1] floating-point indices.
        let fx = (x / self.width_m).clamp(0.0, 1.0) * (n as f32 - 1.0);
        let fy = (y / self.height_m).clamp(0.0, 1.0) * (n as f32 - 1.0);

        let col0 = fx.floor() as usize;
        let row0 = fy.floor() as usize;
        let col1 = (col0 + 1).min(n - 1);
        let row1 = (row0 + 1).min(n - 1);

        let tc = fx - col0 as f32; // fractional column
        let tr = fy - row0 as f32; // fractional row

        let v00 = self.path_loss_grid[row0 * n + col0];
        let v10 = self.path_loss_grid[row0 * n + col1];
        let v01 = self.path_loss_grid[row1 * n + col0];
        let v11 = self.path_loss_grid[row1 * n + col1];

        // Bilinear blend
        let top = v00 * (1.0 - tc) + v10 * tc;
        let bot = v01 * (1.0 - tc) + v11 * tc;
        top * (1.0 - tr) + bot * tr
    }

    // -----------------------------------------------------------------------
    // is_on_street — proximity check against H and V street lists
    // -----------------------------------------------------------------------

    /// Returns `true` if `pos` is within `street_width` metres of any street.
    pub fn is_on_street(&self, pos: [f32; 2], street_width: f32) -> bool {
        let [px, py] = pos;
        let half = street_width * 0.5;
        self.streets_h.iter().any(|&sy| (py - sy).abs() <= half)
            || self.streets_v.iter().any(|&sx| (px - sx).abs() <= half)
    }

    // -----------------------------------------------------------------------
    // nearest_intersection — closest (streets_v[i], streets_h[j]) pair
    // -----------------------------------------------------------------------

    /// Returns the nearest street intersection `[x, y]` to `pos`.
    ///
    /// Falls back to the map centre if there are no streets.
    pub fn nearest_intersection(&self, pos: [f32; 2]) -> [f32; 2] {
        let [px, py] = pos;

        let nearest_v = self
            .streets_v
            .iter()
            .copied()
            .min_by(|a, b| (a - px).abs().partial_cmp(&(b - px).abs()).unwrap());
        let nearest_h = self
            .streets_h
            .iter()
            .copied()
            .min_by(|a, b| (a - py).abs().partial_cmp(&(b - py).abs()).unwrap());

        match (nearest_v, nearest_h) {
            (Some(vx), Some(hy)) => [vx, hy],
            (Some(vx), None)     => [vx, py],
            (None,     Some(hy)) => [px, hy],
            (None,     None)     => [self.width_m * 0.5, self.height_m * 0.5],
        }
    }
}
