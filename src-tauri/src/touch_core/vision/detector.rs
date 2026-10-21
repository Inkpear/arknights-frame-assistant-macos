//! White-diamond detector.
//!
//! The selection diamond around a deployed operator has four white edges. The
//! reference implementation detects a pair of opposite edges, reconstructs the
//! four vertices from their endpoints, and uses the midpoints of the other two
//! edges as the retreat (NW) and skill (SE) buttons.
//!
//! This implementation uses a restricted-angle directional histogram instead
//! of OpenCV's HoughLinesP. It intentionally does not use the mouse position:
//! after clicking an operator the game camera tilts, so the original click
//! point no longer points at the operator. The diamond geometry is the only
//! anchor.

use image::{Rgba, RgbaImage};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq)]
pub struct Pt {
    pub x: f32,
    pub y: f32,
}

impl Pt {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn dist(self, other: Pt) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    pub fn mid(self, other: Pt) -> Pt {
        Pt::new((self.x + other.x) * 0.5, (self.y + other.y) * 0.5)
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

impl Rect {
    pub fn new(x: usize, y: usize, w: usize, h: usize) -> Self {
        Self { x, y, w, h }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct LineCand {
    pub angle_deg: f32,
    /// Perpendicular offset in working pixels.
    pub v: f32,
    /// Segment extents along the line direction, in working pixels.
    pub u0: f32,
    pub u1: f32,
    pub len: f32,
    pub fill: f32,
    pub thick_neighbors: usize,
    pub p0: Pt,
    pub p1: Pt,
}

#[derive(Clone, Debug, Serialize)]
pub struct Detection {
    pub top: Pt,
    pub right: Pt,
    pub bottom: Pt,
    pub left: Pt,
    pub retreat: Pt,
    pub skill: Pt,
    pub confidence: f32,
    pub line_a: LineCand,
    pub line_b: LineCand,
}

#[derive(Clone, Debug)]
pub struct Outcome {
    pub detection: Option<Detection>,
    pub candidates: Vec<LineCand>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Params {
    /// Width the content is downscaled to before detection.
    pub target_width: usize,
    /// A pixel is white when min(R, G, B) >= this value.
    pub white_threshold: u8,
    pub angle_min: f32,
    pub angle_max: f32,
    pub angle_step: f32,
    /// Allowed gap between consecutive occupied u bins.
    pub max_gap: u32,
    /// Minimum segment length as a fraction of the working width.
    pub min_len_ratio: f32,
    /// Minimum occupied fraction along the segment.
    pub min_fill: f32,
    /// Maximum number of adjacent thick rows before a candidate is rejected.
    pub max_thick_neighbors: usize,
    /// Opposite diamond edges are slightly non-parallel on screen because of
    /// the camera tilt.
    pub max_angle_diff: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            target_width: 720,
            white_threshold: 240,
            angle_min: 24.0,
            angle_max: 48.0,
            angle_step: 2.0,
            max_gap: 3,
            min_len_ratio: 0.10,
            min_fill: 0.45,
            max_thick_neighbors: 4,
            max_angle_diff: 12.0,
        }
    }
}

pub struct WorkImage {
    pub w: usize,
    pub h: usize,
    pub block: usize,
    pub white: Vec<bool>,
}

impl WorkImage {
    pub fn is_white(&self, x: usize, y: usize) -> bool {
        x < self.w && y < self.h && self.white[y * self.w + x]
    }
}

/// Downscale the content with block-max. Taking the maximum preserves the thin
/// white diamond outline, which nearest-neighbour or averaging can drop.
pub fn build_work(frame: &RgbaImage, roi: Rect, params: &Params) -> WorkImage {
    let block = ((roi.w as f32 / params.target_width as f32).ceil() as usize).max(1);
    let w = (roi.w / block).max(1);
    let h = (roi.h / block).max(1);
    let mut white = vec![false; w * h];

    for wy in 0..h {
        for wx in 0..w {
            let x0 = roi.x + wx * block;
            let y0 = roi.y + wy * block;
            let x1 = (x0 + block).min(roi.x + roi.w).min(frame.width() as usize);
            let y1 = (y0 + block).min(roi.y + roi.h).min(frame.height() as usize);
            let mut max_min_rgb = 0u8;
            for y in y0..y1 {
                for x in x0..x1 {
                    let px = frame.get_pixel(x as u32, y as u32);
                    let min_rgb = px[0].min(px[1]).min(px[2]);
                    if min_rgb > max_min_rgb {
                        max_min_rgb = min_rgb;
                    }
                }
            }
            white[wy * w + wx] = max_min_rgb >= params.white_threshold;
        }
    }

    WorkImage { w, h, block, white }
}

pub fn detect(work: &WorkImage, params: &Params) -> Outcome {
    if work.w < 32 || work.h < 32 {
        return Outcome {
            detection: None,
            candidates: Vec::new(),
            reason: Some("content area is too small after downscaling".to_string()),
        };
    }

    let mut candidates = Vec::new();
    // Detect both opposite-edge pairs. The reference only uses the
    // positive-slope pair; looking at the negative-slope pair as well makes
    // the detector survive when one pair is partially occluded.
    for (angle_min, angle_max) in [
        (params.angle_min, params.angle_max),
        (180.0 - params.angle_max, 180.0 - params.angle_min),
    ] {
        let mut angle = angle_min;
        while angle <= angle_max + 0.01 {
            collect_candidates(work, angle, params, &mut candidates);
            angle += params.angle_step;
        }
    }
    let candidates = merge_candidates(candidates);
    let selected = nms(candidates);
    let min_len = work.w as f32 * params.min_len_ratio;

    let mut best: Option<Detection> = None;
    for i in 0..selected.len() {
        for j in (i + 1)..selected.len() {
            if let Some(detection) =
                evaluate_pair(&selected[i], &selected[j], work, min_len, params)
            {
                let better = best
                    .as_ref()
                    .map(|best| detection.confidence > best.confidence)
                    .unwrap_or(true);
                if better {
                    best = Some(detection);
                }
            }
        }
    }

    let reason = if best.is_none() {
        Some(format!(
            "no valid parallel pair ({} line candidates)",
            selected.len()
        ))
    } else {
        None
    };
    Outcome {
        detection: best,
        candidates: selected,
        reason,
    }
}

fn collect_candidates(work: &WorkImage, angle_deg: f32, params: &Params, out: &mut Vec<LineCand>) {
    let rad = angle_deg.to_radians();
    let (cos, sin) = (rad.cos(), rad.sin());

    let v_min = -(work.w as f32) - 2.0;
    let v_max = work.h as f32 + 2.0;
    let v_bins = ((v_max - v_min).ceil() as usize) + 1;
    let mut rows: Vec<Vec<u32>> = vec![Vec::new(); v_bins];

    // For angles past 90 degrees cos() is negative and u can be negative;
    // shift u into a non-negative range and undo the shift when emitting
    // endpoints.
    let u_min = (work.w as f32 * cos).min(0.0) + (work.h as f32 * sin).min(0.0);

    for y in 0..work.h {
        for x in 0..work.w {
            if !work.is_white(x, y) {
                continue;
            }
            let xf = x as f32;
            let yf = y as f32;
            let u = xf * cos + yf * sin;
            let v = -xf * sin + yf * cos;
            let vb = ((v - v_min).round() as i32).clamp(0, v_bins as i32 - 1) as usize;
            rows[vb].push((u - u_min).round().max(0.0) as u32);
        }
    }

    let min_len = (work.w as f32 * params.min_len_ratio).round() as u32;
    let mut runs: Vec<Option<(u32, u32, u32)>> = Vec::with_capacity(rows.len());
    for us in rows.iter_mut() {
        us.sort_unstable();
        us.dedup();
        runs.push(longest_run(us, params.max_gap));
    }

    for (vb, run) in runs.iter().enumerate() {
        let Some((u0, u1, occupied)) = *run else {
            continue;
        };
        let len = u1 - u0;
        if len < min_len {
            continue;
        }
        let fill = occupied as f32 / (len as f32 + 1.0);
        if fill < params.min_fill {
            continue;
        }

        let mut thick_neighbors = 0usize;
        for delta in [-3i32, -2, -1, 1, 2, 3] {
            let nb = vb as i32 + delta;
            if nb < 0 || nb >= rows.len() as i32 {
                continue;
            }
            let neighbor = &rows[nb as usize];
            if neighbor.is_empty() {
                continue;
            }
            let count = neighbor.iter().filter(|&&u| u >= u0 && u <= u1).count();
            if count as f32 > 0.5 * len as f32 {
                thick_neighbors += 1;
            }
        }
        if thick_neighbors > params.max_thick_neighbors {
            continue;
        }

        let v = v_min + vb as f32;
        let u0 = u_min + u0 as f32;
        let u1 = u_min + u1 as f32;
        let (p0, p1) = endpoints(angle_deg, v, u0, u1);
        out.push(LineCand {
            angle_deg,
            v,
            u0,
            u1,
            len: len as f32,
            fill,
            thick_neighbors,
            p0,
            p1,
        });
    }
}

fn longest_run(sorted: &[u32], max_gap: u32) -> Option<(u32, u32, u32)> {
    let first = *sorted.first()?;
    let mut best = (first, first, 1u32);
    let mut start = first;
    let mut prev = first;
    let mut count = 1u32;

    for &u in &sorted[1..] {
        if u == prev {
            continue;
        }
        if u - prev <= max_gap + 1 {
            prev = u;
            count += 1;
        } else {
            if prev - start > best.1 - best.0 {
                best = (start, prev, count);
            }
            start = u;
            prev = u;
            count = 1;
        }
    }
    if prev - start > best.1 - best.0 {
        best = (start, prev, count);
    }
    Some(best)
}

fn endpoints(angle_deg: f32, v: f32, u0: f32, u1: f32) -> (Pt, Pt) {
    let rad = angle_deg.to_radians();
    let (cos, sin) = (rad.cos(), rad.sin());
    let at = |u: f32| Pt::new(u * cos - v * sin, u * sin + v * cos);
    (at(u0), at(u1))
}

/// Merge collinear segments separated by a gap. An operator sprite or a
/// floating UI element can split one diamond edge into two histogram runs;
/// merging them restores the full edge before pairing.
fn merge_candidates(mut candidates: Vec<LineCand>) -> Vec<LineCand> {
    loop {
        let mut merged_any = false;
        'outer: for i in 0..candidates.len() {
            for j in (i + 1)..candidates.len() {
                let mut a = candidates[i].clone();
                let mut b = candidates[j].clone();
                if a.u0 > b.u0 {
                    std::mem::swap(&mut a, &mut b);
                }
                let angle_ok = (a.angle_deg - b.angle_deg).abs() <= 2.0;
                let v_ok = (a.v - b.v).abs() <= 3.0;
                let gap = b.u0 - a.u1;
                let max_gap = 0.35 * a.len.max(b.len) + 8.0;
                if !(angle_ok && v_ok && gap <= max_gap && gap >= -4.0) {
                    continue;
                }

                let u0 = a.u0.min(b.u0);
                let u1 = a.u1.max(b.u1);
                let len = u1 - u0;
                let angle_deg = (a.angle_deg + b.angle_deg) * 0.5;
                let v = (a.v + b.v) * 0.5;
                let occupied = a.fill * (a.len + 1.0) + b.fill * (b.len + 1.0);
                let (p0, p1) = endpoints(angle_deg, v, u0, u1);
                candidates[i] = LineCand {
                    angle_deg,
                    v,
                    u0,
                    u1,
                    len,
                    fill: occupied / (len + 1.0),
                    thick_neighbors: a.thick_neighbors.max(b.thick_neighbors),
                    p0,
                    p1,
                };
                candidates.remove(j);
                merged_any = true;
                break 'outer;
            }
        }
        if !merged_any {
            break;
        }
    }
    candidates
}

fn nms(mut candidates: Vec<LineCand>) -> Vec<LineCand> {
    candidates.sort_by(|a, b| b.len.total_cmp(&a.len));
    let mut kept: Vec<LineCand> = Vec::new();
    for candidate in candidates {
        let duplicate = kept.iter().any(|line| {
            (line.angle_deg - candidate.angle_deg).abs() <= 2.0
                && (line.v - candidate.v).abs() <= 3.0
        });
        if !duplicate {
            kept.push(candidate);
        }
        if kept.len() >= 32 {
            break;
        }
    }
    kept
}

fn evaluate_pair(
    a: &LineCand,
    b: &LineCand,
    work: &WorkImage,
    min_len: f32,
    params: &Params,
) -> Option<Detection> {
    let dangle = (a.angle_deg - b.angle_deg).abs();
    if dangle > params.max_angle_diff {
        return None;
    }
    let separation = (a.v - b.v).abs();
    if separation < 0.08 * work.h as f32 || separation > 0.95 * work.h as f32 {
        return None;
    }
    let max_len = a.len.max(b.len);
    let min_pair_len = a.len.min(b.len);
    if min_pair_len / max_len < 0.55 {
        return None;
    }
    if min_pair_len < min_len {
        return None;
    }

    let points = [a.p0, a.p1, b.p0, b.p1];
    let top = *points.iter().min_by(|p, q| p.y.total_cmp(&q.y)).unwrap();
    let bottom = *points.iter().max_by(|p, q| p.y.total_cmp(&q.y)).unwrap();
    let left = *points.iter().min_by(|p, q| p.x.total_cmp(&q.x)).unwrap();
    let right = *points.iter().max_by(|p, q| p.x.total_cmp(&q.x)).unwrap();
    if !(distinct(top, right)
        && distinct(right, bottom)
        && distinct(bottom, left)
        && distinct(left, top))
    {
        return None;
    }

    let vertices = [top, right, bottom, left];
    if !is_convex(&vertices) {
        return None;
    }

    let width = right.x - left.x;
    let height = bottom.y - top.y;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    if width < 0.15 * work.w as f32 || width > 1.1 * work.w as f32 {
        return None;
    }
    if height < 0.12 * work.h as f32 || height > 1.1 * work.h as f32 {
        return None;
    }
    if !extrema_ok(&vertices, width.max(height) * 0.08) {
        return None;
    }

    let retreat = left.mid(top);
    let skill = bottom.mid(right);
    if !in_bounds(retreat, work) || !in_bounds(skill, work) {
        return None;
    }

    let len_score = (min_pair_len / (0.22 * work.w as f32)).min(1.0);
    let parallel_score = (1.0 - dangle / 18.0).max(0.0);
    let symmetry_score = min_pair_len / max_len;
    let confidence = 0.40 * len_score + 0.35 * parallel_score + 0.25 * symmetry_score;

    Some(Detection {
        top,
        right,
        bottom,
        left,
        retreat,
        skill,
        confidence,
        line_a: a.clone(),
        line_b: b.clone(),
    })
}

fn distinct(a: Pt, b: Pt) -> bool {
    a.dist(b) > 2.0
}

fn is_convex(points: &[Pt; 4]) -> bool {
    let mut sign = 0.0f32;
    for i in 0..4 {
        let a = points[i];
        let b = points[(i + 1) % 4];
        let c = points[(i + 2) % 4];
        let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
        if cross.abs() < 1.0 {
            continue;
        }
        let current = cross.signum();
        if sign == 0.0 {
            sign = current;
        } else if current != sign {
            return false;
        }
    }
    sign != 0.0
}

fn extrema_ok(points: &[Pt; 4], tol: f32) -> bool {
    let [top, right, bottom, left] = *points;
    top.y <= left.y + tol
        && top.y <= right.y + tol
        && bottom.y >= left.y - tol
        && bottom.y >= right.y - tol
        && left.x <= top.x + tol
        && left.x <= bottom.x + tol
        && right.x >= top.x - tol
        && right.x >= bottom.x - tol
}

fn in_bounds(point: Pt, work: &WorkImage) -> bool {
    point.x >= 0.0
        && point.y >= 0.0
        && point.x <= work.w as f32 - 1.0
        && point.y <= work.h as f32 - 1.0
}

/// Convert a working-image point back to image pixels.
pub fn work_to_image(point: Pt, roi: Rect, block: usize) -> Pt {
    Pt::new(
        roi.x as f32 + point.x * block as f32 + block as f32 * 0.5,
        roi.y as f32 + point.y * block as f32 + block as f32 * 0.5,
    )
}

pub fn draw_line(img: &mut RgbaImage, a: Pt, b: Pt, color: Rgba<u8>, width: i32) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let steps = dx.abs().max(dy.abs()).max(1.0);
    let half = (width.max(1) - 1) / 2;
    let mut i = 0i32;
    while (i as f32) <= steps {
        let t = i as f32 / steps;
        let x = a.x + dx * t;
        let y = a.y + dy * t;
        for oy in -half..=half {
            for ox in -half..=half {
                let px = (x.round() as i32 + ox).clamp(0, img.width() as i32 - 1) as u32;
                let py = (y.round() as i32 + oy).clamp(0, img.height() as i32 - 1) as u32;
                img.put_pixel(px, py, color);
            }
        }
        i += 1;
    }
}

pub fn draw_circle(img: &mut RgbaImage, center: Pt, radius: i32, color: Rgba<u8>) {
    let r2 = radius * radius;
    for oy in -radius..=radius {
        for ox in -radius..=radius {
            if ox * ox + oy * oy > r2 {
                continue;
            }
            let x = (center.x.round() as i32 + ox).clamp(0, img.width() as i32 - 1) as u32;
            let y = (center.y.round() as i32 + oy).clamp(0, img.height() as i32 - 1) as u32;
            img.put_pixel(x, y, color);
        }
    }
}

pub fn draw_cross(img: &mut RgbaImage, center: Pt, size: i32, color: Rgba<u8>) {
    draw_line(
        img,
        Pt::new(center.x - size as f32, center.y),
        Pt::new(center.x + size as f32, center.y),
        color,
        2,
    );
    draw_line(
        img,
        Pt::new(center.x, center.y - size as f32),
        Pt::new(center.x, center.y + size as f32),
        color,
        2,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);

    struct Case {
        center: Pt,
        diamond_w: f32,
        diamond_h: f32,
        gap: bool,
        noise: bool,
    }

    fn frame(width: u32, height: u32, case: &Case) -> RgbaImage {
        let mut img = RgbaImage::from_pixel(width, height, Rgba([16, 18, 22, 255]));
        let top = Pt::new(case.center.x, case.center.y - case.diamond_h * 0.5);
        let right = Pt::new(case.center.x + case.diamond_w * 0.5, case.center.y);
        let bottom = Pt::new(case.center.x, case.center.y + case.diamond_h * 0.5);
        let left = Pt::new(case.center.x - case.diamond_w * 0.5, case.center.y);

        if case.gap {
            let a = lerp(top, right, 0.32);
            let b = lerp(top, right, 0.58);
            draw_line(&mut img, top, a, WHITE, 5);
            draw_line(&mut img, b, right, WHITE, 5);
        } else {
            draw_line(&mut img, top, right, WHITE, 5);
        }
        draw_line(&mut img, right, bottom, WHITE, 5);
        draw_line(&mut img, bottom, left, WHITE, 5);
        draw_line(&mut img, left, top, WHITE, 5);

        if case.noise {
            draw_line(
                &mut img,
                Pt::new(60.0, 130.0),
                Pt::new(2140.0, 130.0),
                WHITE,
                3,
            );
            draw_line(
                &mut img,
                Pt::new(90.0, 80.0),
                Pt::new(90.0, 1700.0),
                WHITE,
                3,
            );
            draw_line(
                &mut img,
                Pt::new(1650.0, 80.0),
                Pt::new(1760.0, 140.0),
                WHITE,
                3,
            );
            draw_line(
                &mut img,
                Pt::new(150.0, 1600.0),
                Pt::new(420.0, 1710.0),
                WHITE,
                3,
            );
            draw_line(
                &mut img,
                Pt::new(1700.0, 1500.0),
                Pt::new(1740.0, 1700.0),
                WHITE,
                3,
            );
            for y in 1450..1570 {
                for x in 160..300 {
                    img.put_pixel(x, y, WHITE);
                }
            }
            for i in 0..24 {
                let x = 420.0 + i as f32 * 45.0;
                draw_line(
                    &mut img,
                    Pt::new(x, 1600.0),
                    Pt::new(x + 18.0, 1582.0),
                    WHITE,
                    2,
                );
            }
        }
        img
    }

    fn lerp(a: Pt, b: Pt, t: f32) -> Pt {
        Pt::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
    }

    fn run_case(case: &Case) -> (Detection, Pt, Pt) {
        let img = frame(2200, 1800, case);
        let roi = Rect::new(0, 0, img.width() as usize, img.height() as usize);
        let params = Params::default();
        let work = build_work(&img, roi, &params);
        let outcome = detect(&work, &params);
        let det = outcome.detection.expect("diamond detected");
        let top = Pt::new(case.center.x, case.center.y - case.diamond_h * 0.5);
        let right = Pt::new(case.center.x + case.diamond_w * 0.5, case.center.y);
        let bottom = Pt::new(case.center.x, case.center.y + case.diamond_h * 0.5);
        let left = Pt::new(case.center.x - case.diamond_w * 0.5, case.center.y);
        let retreat = work_to_image(det.retreat, roi, work.block);
        let skill = work_to_image(det.skill, roi, work.block);
        (det, retreat, skill)
    }

    #[test]
    fn detects_centered_diamond() {
        let case = Case {
            center: Pt::new(1100.0, 900.0),
            diamond_w: 1500.0,
            diamond_h: 1000.0,
            gap: false,
            noise: false,
        };
        let (det, retreat, skill) = run_case(&case);
        assert!(det.confidence > 0.7);
        assert!(retreat.dist(Pt::new(725.0, 650.0)) < 40.0);
        assert!(skill.dist(Pt::new(1475.0, 1150.0)) < 40.0);
    }

    #[test]
    fn detects_offset_diamond_with_ui_noise() {
        let case = Case {
            center: Pt::new(1250.0, 950.0),
            diamond_w: 1400.0,
            diamond_h: 950.0,
            gap: false,
            noise: true,
        };
        let (det, retreat, skill) = run_case(&case);
        assert!(det.confidence > 0.7);
        assert!(retreat.dist(Pt::new(900.0, 712.5)) < 40.0);
        assert!(skill.dist(Pt::new(1600.0, 1187.5)) < 40.0);
    }

    #[test]
    fn detects_diamond_with_gap_and_noise() {
        let case = Case {
            center: Pt::new(1100.0, 900.0),
            diamond_w: 1500.0,
            diamond_h: 1000.0,
            gap: true,
            noise: true,
        };
        let (det, retreat, skill) = run_case(&case);
        assert!(det.confidence > 0.6);
        assert!(retreat.dist(Pt::new(725.0, 650.0)) < 40.0);
        assert!(skill.dist(Pt::new(1475.0, 1150.0)) < 40.0);
    }

    #[test]
    fn work_to_image_uses_block_centers() {
        let roi = Rect::new(100, 200, 400, 300);
        let point = work_to_image(Pt::new(2.0, 3.0), roi, 4);
        assert!((point.x - 110.0).abs() < 1e-6);
        assert!((point.y - 214.0).abs() < 1e-6);
    }
}
