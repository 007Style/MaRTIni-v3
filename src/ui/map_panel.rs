//! MapPanel — egui Painter canvas rendering the terrain, towers, and mobile dots.

use egui::{Color32, Pos2, Rect, Stroke, Vec2, pos2};

use crate::config::RadioTechnology;
use crate::config::TrafficProfile;
use crate::sim::SimSnapshot;

// ---------------------------------------------------------------------------
// HeatmapCache
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct HeatmapCache {
    grid_n: usize,
    values: Vec<f32>, // NxN SINR values in dB
}

// ---------------------------------------------------------------------------
// MapPanel
// ---------------------------------------------------------------------------

/// Renders the main simulation map using egui's Painter API.
#[derive(Debug, Default)]
pub struct MapPanel {
    pub show_heatmap: bool,
    pub show_trails: bool,
    pub show_interference: bool,
    pub(crate) heatmap_cache: Option<HeatmapCache>,
}

impl MapPanel {
    pub fn new() -> Self {
        Self {
            show_heatmap: false,
            show_trails: true,
            show_interference: false,
            heatmap_cache: None,
        }
    }

    /// Main render call — call this from app.rs inside a CentralPanel.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &SimSnapshot,
        selected_mobile: &mut Option<u32>,
    ) {
        let rect = ui.available_rect_before_wrap();
        let painter = ui.painter_at(rect);

        let total_m = snapshot.terrain.width_m;

        // ── Coordinate helper ────────────────────────────────────────────────
        let s2s = |sim_pos: [f32; 2]| sim_to_screen(sim_pos, rect, total_m);
        let scale = compute_scale(rect, total_m);

        // ── Layer 1 — Background ─────────────────────────────────────────────
        painter.rect_filled(rect, 0.0, Color32::from_rgb(26, 26, 46));

        // ── Layer 2 — Terrain (streets + buildings) ──────────────────────────
        let terrain = &*snapshot.terrain;

        // Buildings
        for building in &terrain.buildings {
            let tl = s2s([building.x, building.y]);
            let br = s2s([building.x + building.w, building.y + building.h]);
            painter.rect_filled(
                Rect::from_min_max(tl, br),
                0.0,
                Color32::from_rgb(35, 35, 60),
            );
        }

        // Horizontal streets
        let street_color = Color32::from_rgb(58, 58, 90);
        let street_thick = (2.0 * scale).max(1.0);
        for &sy in &terrain.streets_h {
            let left = s2s([0.0, sy]);
            let right = s2s([total_m, sy]);
            painter.line_segment([left, right], Stroke::new(street_thick, street_color));
        }
        // Vertical streets
        for &sx in &terrain.streets_v {
            let top = s2s([sx, 0.0]);
            let bot = s2s([sx, total_m]);
            painter.line_segment([top, bot], Stroke::new(street_thick, street_color));
        }

        // ── Layer 3 — Coverage circles ───────────────────────────────────────
        let n_base = snapshot.bases.len() as f32;
        let isd = if n_base > 0.0 {
            total_m / n_base.sqrt()
        } else {
            total_m
        };
        let cov_radius = isd * 0.45;

        for base in &snapshot.bases {
            let center = s2s(base.position);
            let radius_px = cov_radius * scale;

            if base.failed {
                // Draw a red X — handled in Layer 7
                continue;
            }

            let load = base.load_percent();
            let fill = if load < 50.0 {
                Color32::from_rgba_premultiplied(40, 180, 40, 18)
            } else if load < 80.0 {
                Color32::from_rgba_premultiplied(220, 160, 0, 18)
            } else {
                Color32::from_rgba_premultiplied(200, 40, 40, 18)
            };
            let outline = if load < 50.0 {
                Color32::from_rgba_premultiplied(40, 180, 40, 80)
            } else if load < 80.0 {
                Color32::from_rgba_premultiplied(220, 160, 0, 80)
            } else {
                Color32::from_rgba_premultiplied(200, 40, 40, 80)
            };

            // Filled circle (approximated with a large number of segments via circle)
            painter.circle_filled(center, radius_px, fill);

            // Dashed outline — 24 short arc segments
            let segs = 24usize;
            for i in 0..segs {
                // Draw every other segment for a dashed look
                if i % 2 == 0 {
                    let a0 = (i as f32 / segs as f32) * std::f32::consts::TAU;
                    let a1 = ((i as f32 + 0.8) / segs as f32) * std::f32::consts::TAU;
                    let p0 = pos2(
                        center.x + radius_px * a0.cos(),
                        center.y + radius_px * a0.sin(),
                    );
                    let p1 = pos2(
                        center.x + radius_px * a1.cos(),
                        center.y + radius_px * a1.sin(),
                    );
                    painter.line_segment([p0, p1], Stroke::new(1.0_f32, outline));
                }
            }
        }

        // ── Layer 4 — SINR heatmap (if show_heatmap) ────────────────────────
        if self.show_heatmap {
            let grid_n = 20usize;

            // Rebuild cache only when base count changes (topology change proxy)
            let needs_rebuild = self
                .heatmap_cache
                .as_ref()
                .map(|c| c.grid_n != grid_n)
                .unwrap_or(true);

            if needs_rebuild {
                let radio = &snapshot.bases.first().map(|_| ()).is_some();
                let _ = radio; // just ensuring it exists
                let mut values = vec![0.0f32; grid_n * grid_n];

                let noise_dbm = -114.0f32; // approximate floor

                for row in 0..grid_n {
                    for col in 0..grid_n {
                        let gx = (col as f32 + 0.5) / grid_n as f32 * total_m;
                        let gy = (row as f32 + 0.5) / grid_n as f32 * total_m;

                        // Find best SINR at this grid point
                        let mut best_sinr = f32::NEG_INFINITY;
                        for (bi, base) in snapshot.bases.iter().enumerate() {
                            if base.failed {
                                continue;
                            }
                            let [bx, by] = base.position;
                            let dist = ((gx - bx).powi(2) + (gy - by).powi(2))
                                .sqrt()
                                .max(1.0);
                            // Simplified path loss
                            let pl = 20.0 * (4.0 * std::f32::consts::PI * dist / 0.115).log10();
                            let rx = 43.0 - pl;

                            let interference: Vec<f32> = snapshot
                                .bases
                                .iter()
                                .enumerate()
                                .filter(|&(j, b)| j != bi && !b.failed)
                                .map(|(_, b)| {
                                    let [ibx, iby] = b.position;
                                    let d = ((gx - ibx).powi(2) + (gy - iby).powi(2))
                                        .sqrt()
                                        .max(1.0);
                                    let ipl = 20.0
                                        * (4.0 * std::f32::consts::PI * d / 0.115).log10();
                                    43.0 - ipl
                                })
                                .collect();

                            let dbm_to_mw = |d: f32| 10.0_f32.powf(d / 10.0);
                            let sig_mw = dbm_to_mw(rx);
                            let noise_mw = dbm_to_mw(noise_dbm);
                            let int_mw: f32 = interference.iter().copied().map(dbm_to_mw).sum();
                            let sinr = 10.0 * (sig_mw / (noise_mw + int_mw)).log10();

                            if sinr > best_sinr {
                                best_sinr = sinr;
                            }
                        }
                        values[row * grid_n + col] = best_sinr;
                    }
                }
                self.heatmap_cache = Some(HeatmapCache { grid_n, values });
            }

            if let Some(cache) = &self.heatmap_cache {
                let cell_w = total_m / cache.grid_n as f32 * scale;
                let cell_h = total_m / cache.grid_n as f32 * scale;

                for row in 0..cache.grid_n {
                    for col in 0..cache.grid_n {
                        let sinr = cache.values[row * cache.grid_n + col];
                        let color = sinr_to_color(sinr);
                        let gx = col as f32 / cache.grid_n as f32 * total_m;
                        let gy = row as f32 / cache.grid_n as f32 * total_m;
                        let tl = s2s([gx, gy]);
                        let br = pos2(tl.x + cell_w, tl.y + cell_h);
                        painter.rect_filled(Rect::from_min_max(tl, br), 0.0, color);
                    }
                }
            }
        }

        // ── Layer 5 — Mobile trails ──────────────────────────────────────────
        if self.show_trails {
            for mobile in &snapshot.mobiles {
                if mobile.trail.len() < 2 {
                    continue;
                }
                let [r, g, b] = profile_color(&mobile.profile);
                let n = mobile.trail.len();
                let trail_vec: Vec<_> = mobile.trail.iter().copied().collect();
                for i in 1..n {
                    let alpha = (20 + (80 * i / n)) as u8;
                    let color = Color32::from_rgba_premultiplied(
                        (r as u32 * alpha as u32 / 255) as u8,
                        (g as u32 * alpha as u32 / 255) as u8,
                        (b as u32 * alpha as u32 / 255) as u8,
                        alpha,
                    );
                    let p0 = s2s(trail_vec[i - 1]);
                    let p1 = s2s(trail_vec[i]);
                    painter.line_segment([p0, p1], Stroke::new(1.0_f32, color));
                }
            }
        }

        // ── Layer 6 — Mobile-to-base link lines ──────────────────────────────
        let link_color = Color32::from_rgba_premultiplied(64, 160, 255, 80);
        for mobile in &snapshot.mobiles {
            let mob_pos = s2s(mobile.position);
            if let Some(bid) = mobile.serving_cell {
                if let Some(base) = snapshot.bases.get(bid as usize) {
                    let bp = s2s(base.position);
                    painter.line_segment([mob_pos, bp], Stroke::new(0.8_f32, link_color));
                }
            }

            // Gen3 active set additional lines
            if mobile.active_set.len() > 1 {
                let soft_color = Color32::from_rgba_premultiplied(64, 160, 255, 40);
                for &asid in mobile.active_set.iter().skip(1) {
                    if let Some(base) = snapshot.bases.get(asid as usize) {
                        let bp = s2s(base.position);
                        painter.line_segment([mob_pos, bp], Stroke::new(0.5_f32, soft_color));
                    }
                }
            }

            // Gen5 secondary cell — dashed (approximate with short segments)
            if let Some(scid) = mobile.secondary_cell {
                if let Some(base) = snapshot.bases.get(scid as usize) {
                    let bp = s2s(base.position);
                    let dash_color = Color32::from_rgba_premultiplied(140, 80, 255, 70);
                    draw_dashed_line(&painter, mob_pos, bp, dash_color, 0.6, 6.0);
                }
            }
        }

        // ── Layer 7 — Base station towers ────────────────────────────────────
        let pointer_pos = ui.input(|i| i.pointer.hover_pos());

        for base in &snapshot.bases {
            let center = s2s(base.position);
            let icon_size = (12.0 * scale).clamp(8.0, 18.0);

            let (tower_color, tech_label) = match base.technology {
                RadioTechnology::Gen3Umts => (Color32::from_rgb(100, 150, 255), "3G"),
                RadioTechnology::Gen4Lte => (Color32::from_rgb(80, 220, 80), "4G"),
                RadioTechnology::Gen5NrSub6 => (Color32::from_rgb(255, 160, 0), "5G"),
                RadioTechnology::Gen5NrMmWave => (Color32::from_rgb(220, 80, 220), "mmW"),
            };

            let color = if base.failed {
                Color32::from_rgb(100, 100, 100)
            } else {
                tower_color
            };

            // Triangle pointing up
            let tip = pos2(center.x, center.y - icon_size);
            let bl = pos2(center.x - icon_size * 0.6, center.y + icon_size * 0.3);
            let br = pos2(center.x + icon_size * 0.6, center.y + icon_size * 0.3);
            painter.add(egui::Shape::convex_polygon(
                vec![tip, br, bl],
                color,
                Stroke::new(1.0_f32, color),
            ));

            // Failed: draw red X
            if base.failed {
                let s = icon_size * 0.5;
                painter.line_segment(
                    [pos2(center.x - s, center.y - s), pos2(center.x + s, center.y + s)],
                    Stroke::new(2.0_f32, Color32::RED),
                );
                painter.line_segment(
                    [pos2(center.x + s, center.y - s), pos2(center.x - s, center.y + s)],
                    Stroke::new(2.0_f32, Color32::RED),
                );
            }

            // BS id label beside tower
            painter.text(
                pos2(center.x + icon_size * 0.7, center.y - icon_size * 0.5),
                egui::Align2::LEFT_CENTER,
                format!("{} {}", tech_label, base.id + 1),
                egui::FontId::proportional(10.0),
                Color32::from_rgb(200, 200, 200),
            );

            // Hover tooltip
            if let Some(pp) = pointer_pos {
                if pp.distance(center) < icon_size + 4.0 {
                    let layer_id = ui.layer_id();
                    egui::show_tooltip_at_pointer(
                        ui.ctx(),
                        layer_id,
                        egui::Id::new(("bs_tooltip", base.id)),
                        |ui: &mut egui::Ui| {
                            ui.label(format!("BS {} — {:?}", base.id + 1, base.technology));
                            ui.label(format!(
                                "Load: {:.1}% ({}/{})",
                                base.load_percent(),
                                base.connected_mobiles.len(),
                                base.total_channels
                            ));
                            if base.failed {
                                ui.colored_label(Color32::RED, "⚠ FAILED");
                            }
                        },
                    );
                }
            }
        }

        // ── Layer 8 — Mobile terminals ───────────────────────────────────────
        let dot_radius = (scale * 80.0).clamp(4.0, 10.0);
        let t = ui.ctx().input(|i| i.time) as f32;
        let pulse = 0.5 + 0.5 * (t * 3.0).sin(); // 0..1, 3 Hz

        let mut clicked_mobile: Option<u32> = None;
        let click_happened = ui.input(|i| i.pointer.primary_clicked());

        for mobile in &snapshot.mobiles {
            let mob_pos = s2s(mobile.position);
            let [r, g, b] = profile_color(&mobile.profile);
            let base_color = Color32::from_rgb(r, g, b);

            // Hover check
            let hovered = pointer_pos
                .map(|pp| pp.distance(mob_pos) < 12.0)
                .unwrap_or(false);

            // Click detection
            if click_happened {
                if let Some(pp) = pointer_pos {
                    if pp.distance(mob_pos) < 12.0 {
                        clicked_mobile = Some(mobile.id);
                    }
                }
            }

            // Direction arrow
            let arrow_len = dot_radius * 2.5;
            let (dx, dy) = match mobile.heading {
                crate::sim::Direction::East => (1.0f32, 0.0f32),
                crate::sim::Direction::West => (-1.0, 0.0),
                crate::sim::Direction::North => (0.0, -1.0),
                crate::sim::Direction::South => (0.0, 1.0),
            };
            let arrow_end = pos2(mob_pos.x + dx * arrow_len, mob_pos.y + dy * arrow_len);
            painter.line_segment(
                [mob_pos, arrow_end],
                Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(r, g, b, 160)),
            );

            // SLA ring
            if mobile.sla_violated {
                painter.circle_stroke(
                    mob_pos,
                    dot_radius + 3.0,
                    Stroke::new(1.5_f32, Color32::from_rgb(255, 50, 50)),
                );
            }

            // Selected ring — pulsing white
            if *selected_mobile == Some(mobile.id) {
                let ring_r = dot_radius + 4.0 + pulse * 3.0;
                painter.circle_stroke(
                    mob_pos,
                    ring_r,
                    Stroke::new(2.0_f32, Color32::from_rgba_premultiplied(255, 255, 255, 200)),
                );
            }

            // Dot
            painter.circle_filled(mob_pos, dot_radius, base_color);

            // Hover tooltip
            if hovered {
                let layer_id = ui.layer_id();
                egui::show_tooltip_at_pointer(
                    ui.ctx(),
                    layer_id,
                    egui::Id::new(("mob_tooltip", mobile.id)),
                    |ui: &mut egui::Ui| {
                        ui.label(format!("Mobile #{}", mobile.id));
                        ui.label(format!("Profile: {}", mobile.profile.label()));
                        ui.label(format!("SINR: {:.1} dB", mobile.sinr_db));
                        ui.label(format!("Throughput: {:.2} Mbps", mobile.dl_throughput_mbps));
                        ui.label(format!("Latency: {:.1} ms", mobile.latency_ms));
                        if mobile.sla_violated {
                            ui.colored_label(Color32::RED, "⚠ SLA violated");
                        }
                    },
                );
            }
        }

        if let Some(id) = clicked_mobile {
            *selected_mobile = Some(id);
        }

        // ── Layer 9 — Legend (top-left) ──────────────────────────────────────
        draw_legend(ui, &painter, rect, self.show_heatmap);

        // ── Tick counter (bottom-right) ──────────────────────────────────────
        painter.text(
            pos2(rect.right() - 8.0, rect.bottom() - 8.0),
            egui::Align2::RIGHT_BOTTOM,
            format!("tick {}", snapshot.tick),
            egui::FontId::monospace(11.0),
            Color32::from_rgba_premultiplied(180, 180, 180, 200),
        );
    }
}

// ---------------------------------------------------------------------------
// Coordinate mapping
// ---------------------------------------------------------------------------

fn compute_scale(rect: Rect, total_m: f32) -> f32 {
    let margin = 8.0;
    ((rect.width() - margin * 2.0) / total_m)
        .min((rect.height() - margin * 2.0) / total_m)
}

fn sim_to_screen(sim_pos: [f32; 2], rect: Rect, total_m: f32) -> Pos2 {
    let margin = 8.0;
    let scale = ((rect.width() - margin * 2.0) / total_m)
        .min((rect.height() - margin * 2.0) / total_m);
    let off_x = rect.left() + (rect.width() - total_m * scale) / 2.0;
    let off_y = rect.top() + (rect.height() - total_m * scale) / 2.0;
    pos2(off_x + sim_pos[0] * scale, off_y + sim_pos[1] * scale)
}

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

fn profile_color(profile: &TrafficProfile) -> [u8; 3] {
    profile.color()
}

fn sinr_to_color(sinr: f32) -> Color32 {
    let alpha = 80u8;
    if sinr < -5.0 {
        // Deep blue
        let t = ((sinr + 20.0) / 15.0).clamp(0.0, 1.0);
        Color32::from_rgba_premultiplied(0, (t * 60.0) as u8, (180.0 + t * 75.0) as u8, alpha)
    } else if sinr < 5.0 {
        // Cyan → green
        let t = (sinr + 5.0) / 10.0;
        Color32::from_rgba_premultiplied(0, (180.0 + t * 75.0) as u8, (255.0 * (1.0 - t)) as u8, alpha)
    } else if sinr < 15.0 {
        // Green → yellow
        let t = (sinr - 5.0) / 10.0;
        Color32::from_rgba_premultiplied((255.0 * t) as u8, 220, 0, alpha)
    } else {
        // Yellow → red
        let t = ((sinr - 15.0) / 10.0).clamp(0.0, 1.0);
        Color32::from_rgba_premultiplied(255, (220.0 * (1.0 - t)) as u8, 0, alpha)
    }
}

// ---------------------------------------------------------------------------
// Dashed line helper
// ---------------------------------------------------------------------------

fn draw_dashed_line(
    painter: &egui::Painter,
    from: Pos2,
    to: Pos2,
    color: Color32,
    thickness: f32,
    dash_len: f32,
) {
    let delta = to - from;
    let total_len = delta.length();
    if total_len < 0.001 {
        return;
    }
    let dir = delta / total_len;
    let mut traveled = 0.0f32;
    let mut draw = true;
    while traveled < total_len {
        let seg_end = (traveled + dash_len).min(total_len);
        if draw {
            let p0 = from + dir * traveled;
            let p1 = from + dir * seg_end;
            painter.line_segment([p0, p1], Stroke::new(thickness, color));
        }
        traveled += dash_len;
        draw = !draw;
    }
}

// ---------------------------------------------------------------------------
// Legend
// ---------------------------------------------------------------------------

fn draw_legend(_ui: &mut egui::Ui, painter: &egui::Painter, rect: Rect, heatmap_on: bool) {
    let x = rect.left() + 8.0;
    let mut y = rect.top() + 8.0;
    let row_h = 14.0;
    let swatch = 10.0;

    // Background
    let legend_w = 130.0;
    let legend_h = row_h * 14.0 + 8.0;
    painter.rect_filled(
        Rect::from_min_size(pos2(x - 4.0, y - 4.0), Vec2::new(legend_w, legend_h)),
        4.0,
        Color32::from_rgba_premultiplied(0, 0, 0, 160),
    );

    // Technology colors
    let techs: &[(&str, Color32)] = &[
        ("Gen3 UMTS", Color32::from_rgb(100, 150, 255)),
        ("Gen4 LTE", Color32::from_rgb(80, 220, 80)),
        ("Gen5 NR Sub6", Color32::from_rgb(255, 160, 0)),
        ("Gen5 mmWave", Color32::from_rgb(220, 80, 220)),
    ];
    painter.text(
        pos2(x, y),
        egui::Align2::LEFT_TOP,
        "Technology",
        egui::FontId::proportional(10.0),
        Color32::from_rgb(200, 200, 200),
    );
    y += row_h;
    for (label, color) in techs {
        painter.rect_filled(
            Rect::from_min_size(pos2(x, y + 1.0), Vec2::splat(swatch)),
            1.0,
            *color,
        );
        painter.text(
            pos2(x + swatch + 4.0, y),
            egui::Align2::LEFT_TOP,
            *label,
            egui::FontId::proportional(9.0),
            Color32::from_rgb(180, 180, 180),
        );
        y += row_h;
    }

    // Profile colors
    let profiles: &[(&str, TrafficProfile)] = &[
        ("Video Stream", TrafficProfile::VideoStream),
        ("Cloud Gaming", TrafficProfile::CloudGaming),
        ("Voice Call", TrafficProfile::VoiceCall),
        ("Idle", TrafficProfile::Idle),
        ("Web Browse", TrafficProfile::WebBrowse),
    ];
    painter.text(
        pos2(x, y),
        egui::Align2::LEFT_TOP,
        "Traffic Profile",
        egui::FontId::proportional(10.0),
        Color32::from_rgb(200, 200, 200),
    );
    y += row_h;
    for (label, profile) in profiles {
        let [r, g, b] = profile.color();
        painter.circle_filled(
            pos2(x + swatch * 0.5, y + swatch * 0.5),
            swatch * 0.5,
            Color32::from_rgb(r, g, b),
        );
        painter.text(
            pos2(x + swatch + 4.0, y),
            egui::Align2::LEFT_TOP,
            *label,
            egui::FontId::proportional(9.0),
            Color32::from_rgb(180, 180, 180),
        );
        y += row_h;
    }

    // Heatmap indicator
    let hm_label = if heatmap_on {
        "SINR Heatmap ON"
    } else {
        "SINR Heatmap OFF"
    };
    let hm_color = if heatmap_on {
        Color32::from_rgb(100, 220, 100)
    } else {
        Color32::from_rgb(150, 150, 150)
    };
    painter.text(
        pos2(x, y),
        egui::Align2::LEFT_TOP,
        hm_label,
        egui::FontId::proportional(9.0),
        hm_color,
    );
}
