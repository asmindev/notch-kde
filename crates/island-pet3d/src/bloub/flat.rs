use super::decor::{ArcRender, Dot, NotifRender};
use super::face::{EyePose, eye_transform};
use super::math::TAU;
use super::shape::Silhouette;
use super::{BloubFrame, BloubPalette};

const IMAGE_SIZE: u32 = 192;
const SUPERSAMPLE: u32 = 2;

pub struct BloubFlatRenderer {
    size: u32,
    supersample: u32,
}

impl Default for BloubFlatRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl BloubFlatRenderer {
    pub fn new() -> Self {
        Self {
            size: IMAGE_SIZE,
            supersample: SUPERSAMPLE,
        }
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn render(&self, frame: &BloubFrame, palette: BloubPalette) -> Vec<u8> {
        let width = self.size * self.supersample;
        let extent = frame.extent.max(1.08);
        let mut buffer = vec![[0.0f32; 4]; (width * width) as usize];

        draw_decor(&mut buffer, width, extent, frame, palette, true);
        draw_body(&mut buffer, width, extent, frame, palette);
        draw_decor(&mut buffer, width, extent, frame, palette, false);

        downsample(&buffer, width, self.size, self.supersample)
    }
}

fn draw_body(
    buffer: &mut [[f32; 4]],
    width: u32,
    extent: f32,
    frame: &BloubFrame,
    palette: BloubPalette,
) {
    let alpha = palette.body[3] * frame.body_alpha;
    if alpha <= 0.001 {
        return;
    }

    let polygon = body_polygon(&frame.silhouette, width, extent);
    let mask = polygon_mask(&polygon, width);
    let notif = frame.decor.notif;
    let eyes = [
        EyeSample::new(&frame.silhouette, &frame.eyes[0]),
        EyeSample::new(&frame.silhouette, &frame.eyes[1]),
    ];

    for y in 0..width {
        for x in 0..width {
            let index = (y * width + x) as usize;
            if !mask[index] {
                continue;
            }

            let (ox, oy) = pixel_world(x, y, width, extent);
            if let Some(notif) = notif
                && (ox - notif.x).hypot(oy - notif.y) < notif.notch_r
            {
                continue;
            }

            let body_point = [ox, -oy];
            let eye_mask = eyes[0].mask(body_point).max(eyes[1].mask(body_point)) * frame.eye_alpha;
            let color = mix(palette.body, palette.eye, eye_mask);
            blend(&mut buffer[index], color, alpha);
        }
    }
}

struct EyeSample {
    center: [f32; 2],
    inv: [f32; 4],
    half: [f32; 2],
    radius: f32,
    open: f32,
    visible: bool,
}

impl EyeSample {
    fn new(silhouette: &Silhouette, eye: &EyePose) -> Self {
        let (center, inv, hw, hh, open, visible) = eye_transform(silhouette, eye);
        let radius = hw.min(hh) * eye.corner.clamp(0.05, 1.0);
        Self {
            center,
            inv,
            half: [hw - radius, hh - radius],
            radius,
            open,
            visible,
        }
    }

    fn mask(&self, point: [f32; 2]) -> f32 {
        1.0 - smoothstep(-0.012, 0.020, self.sdf(point))
    }

    fn sdf(&self, point: [f32; 2]) -> f32 {
        if !self.visible {
            return f32::INFINITY;
        }
        let d = [point[0] - self.center[0], point[1] - self.center[1]];
        let squashed = [d[0], d[1] / self.open];
        let q = [
            self.inv[0] * squashed[0] + self.inv[1] * squashed[1],
            self.inv[2] * squashed[0] + self.inv[3] * squashed[1],
        ];
        let edge = [q[0].abs() - self.half[0], q[1].abs() - self.half[1]];
        let outside = edge[0].max(0.0).hypot(edge[1].max(0.0));
        outside + edge[0].max(edge[1]).min(0.0) - self.radius
    }
}

fn body_polygon(silhouette: &Silhouette, width: u32, extent: f32) -> Vec<[f32; 2]> {
    let cr = silhouette.rot.cos();
    let sr = silhouette.rot.sin();
    let flip_cos = silhouette.flip.cos();
    let mut polygon = Vec::with_capacity(silhouette.radii.len());
    for (index, radius) in silhouette.radii.iter().enumerate() {
        let theta = index as f32 / silhouette.radii.len() as f32 * TAU;
        let x0 = radius * theta.cos();
        let y0 = radius * theta.sin();
        let rx = x0 * cr - y0 * sr;
        let ry = x0 * sr + y0 * cr;
        let bx = rx * silhouette.sx + silhouette.cx;
        let by = -(ry * silhouette.sy + silhouette.cy) * flip_cos;
        polygon.push(overlay_to_pixel(bx, -by, width, extent));
    }
    polygon
}

fn polygon_mask(polygon: &[[f32; 2]], width: u32) -> Vec<bool> {
    let mut mask = vec![false; (width * width) as usize];
    let mut intersections = Vec::with_capacity(polygon.len());
    for y in 0..width {
        let sy = y as f32 + 0.5;
        intersections.clear();
        for index in 0..polygon.len() {
            let a = polygon[index];
            let b = polygon[(index + 1) % polygon.len()];
            if (a[1] <= sy && b[1] > sy) || (b[1] <= sy && a[1] > sy) {
                let t = (sy - a[1]) / (b[1] - a[1]);
                intersections.push(a[0] + t * (b[0] - a[0]));
            }
        }
        intersections.sort_by(f32::total_cmp);
        for span in intersections.chunks_exact(2) {
            let start = ((span[0] - 0.5).ceil()).max(0.0) as u32;
            let end = ((span[1] - 0.5).floor()).min(width as f32 - 1.0) as u32;
            if start > end || start >= width {
                continue;
            }
            let row = (y * width) as usize;
            for x in start..=end {
                mask[row + x as usize] = true;
            }
        }
    }
    mask
}

fn draw_decor(
    buffer: &mut [[f32; 4]],
    width: u32,
    extent: f32,
    frame: &BloubFrame,
    palette: BloubPalette,
    behind: bool,
) {
    for arc in frame.decor.arcs.iter().filter(|arc| arc.behind == behind) {
        draw_arc(buffer, width, extent, arc);
    }
    for dot in frame.decor.dots.iter().filter(|dot| dot.behind == behind) {
        draw_dot(buffer, width, extent, dot, palette);
    }

    if !behind && let Some(notif) = frame.decor.notif {
        draw_notification(buffer, width, extent, notif);
    }
}

fn draw_arc(buffer: &mut [[f32; 4]], width: u32, extent: f32, arc: &ArcRender) {
    let scale = width as f32 / (2.0 * extent);
    for pair in arc.points.windows(2) {
        let p0 = overlay_to_pixel(pair[0].x, pair[0].y, width, extent);
        let p1 = overlay_to_pixel(pair[1].x, pair[1].y, width, extent);
        let dx = p1[0] - p0[0];
        let dy = p1[1] - p0[1];
        let length = (dx * dx + dy * dy).sqrt().max(1e-6);
        let nx = -dy / length * arc.width * 0.5 * scale;
        let ny = dx / length * arc.width * 0.5 * scale;
        let a = [p0[0] + nx, p0[1] + ny];
        let b = [p0[0] - nx, p0[1] - ny];
        let c = [p1[0] + nx, p1[1] + ny];
        let d = [p1[0] - nx, p1[1] - ny];
        let color0 = with_alpha(pair[0].color, arc.opacity);
        let color1 = with_alpha(pair[1].color, arc.opacity);
        fill_triangle(buffer, width, a, b, c, color0, color0, color1);
        fill_triangle(buffer, width, b, d, c, color0, color1, color1);
    }
}

fn draw_dot(buffer: &mut [[f32; 4]], width: u32, extent: f32, dot: &Dot, palette: BloubPalette) {
    let color = dot.color.unwrap_or(palette.body);
    let alpha = dot.opacity * dot.depth * color[3];
    let center = overlay_to_pixel(dot.x, dot.y, width, extent);
    let radius = dot.r * width as f32 / (2.0 * extent);
    fill_disc(buffer, width, center, radius, color, alpha);
}

fn draw_notification(buffer: &mut [[f32; 4]], width: u32, extent: f32, notif: NotifRender) {
    let center = overlay_to_pixel(notif.x, notif.y, width, extent);
    let scale = width as f32 / (2.0 * extent);
    let radius = notif.r * scale;
    fill_disc(buffer, width, center, radius, notif.color, 1.0);

    let white = [1.0, 1.0, 1.0, 1.0];
    fill_rect(
        buffer,
        width,
        [center[0], center[1] - radius * 0.10],
        [notif.r * 0.17 * scale, notif.r * 0.62 * scale],
        white,
        0.95,
    );
    fill_disc(
        buffer,
        width,
        [center[0], center[1] + radius * 0.40],
        notif.r * 0.11 * scale,
        white,
        0.95,
    );
}

fn fill_disc(
    buffer: &mut [[f32; 4]],
    width: u32,
    center: [f32; 2],
    radius: f32,
    color: [f32; 4],
    alpha: f32,
) {
    if radius <= 0.0 || alpha <= 0.0 {
        return;
    }
    let min_x = ((center[0] - radius - 0.5).floor()).max(0.0) as u32;
    let max_x = ((center[0] + radius + 0.5).ceil()).min(width as f32 - 1.0) as u32;
    let min_y = ((center[1] - radius - 0.5).floor()).max(0.0) as u32;
    let max_y = ((center[1] + radius + 0.5).ceil()).min(width as f32 - 1.0) as u32;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let distance = (x as f32 + 0.5 - center[0]).hypot(y as f32 + 0.5 - center[1]);
            let coverage = (radius + 0.5 - distance).clamp(0.0, 1.0);
            if coverage > 0.0 {
                blend(
                    &mut buffer[(y * width + x) as usize],
                    color,
                    alpha * coverage,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn fill_rect(
    buffer: &mut [[f32; 4]],
    width: u32,
    center: [f32; 2],
    size: [f32; 2],
    color: [f32; 4],
    alpha: f32,
) {
    let left = center[0] - size[0] * 0.5;
    let right = center[0] + size[0] * 0.5;
    let top = center[1] - size[1] * 0.5;
    let bottom = center[1] + size[1] * 0.5;
    let color = with_alpha(color, alpha);
    fill_triangle(
        buffer,
        width,
        [left, top],
        [right, top],
        [left, bottom],
        color,
        color,
        color,
    );
    fill_triangle(
        buffer,
        width,
        [right, top],
        [right, bottom],
        [left, bottom],
        color,
        color,
        color,
    );
}

#[allow(clippy::too_many_arguments)]
fn fill_triangle(
    buffer: &mut [[f32; 4]],
    width: u32,
    a: [f32; 2],
    b: [f32; 2],
    c: [f32; 2],
    ca: [f32; 4],
    cb: [f32; 4],
    cc: [f32; 4],
) {
    let denominator = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
    if denominator.abs() < 1e-6 {
        return;
    }
    let min_x = (a[0].min(b[0]).min(c[0]) - 0.5).floor().max(0.0) as u32;
    let max_x = (a[0].max(b[0]).max(c[0]) + 0.5)
        .ceil()
        .min(width as f32 - 1.0) as u32;
    let min_y = (a[1].min(b[1]).min(c[1]) - 0.5).floor().max(0.0) as u32;
    let max_y = (a[1].max(b[1]).max(c[1]) + 0.5)
        .ceil()
        .min(width as f32 - 1.0) as u32;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = [x as f32 + 0.5, y as f32 + 0.5];
            let wa = ((b[1] - c[1]) * (p[0] - c[0]) + (c[0] - b[0]) * (p[1] - c[1])) / denominator;
            let wb = ((c[1] - a[1]) * (p[0] - c[0]) + (a[0] - c[0]) * (p[1] - c[1])) / denominator;
            let wc = 1.0 - wa - wb;
            if wa < -1e-4 || wb < -1e-4 || wc < -1e-4 {
                continue;
            }
            let color = [
                ca[0] * wa + cb[0] * wb + cc[0] * wc,
                ca[1] * wa + cb[1] * wb + cc[1] * wc,
                ca[2] * wa + cb[2] * wb + cc[2] * wc,
                ca[3] * wa + cb[3] * wb + cc[3] * wc,
            ];
            blend(&mut buffer[(y * width + x) as usize], color, 1.0);
        }
    }
}

fn downsample(buffer: &[[f32; 4]], width: u32, size: u32, supersample: u32) -> Vec<u8> {
    let mut output = Vec::with_capacity((size * size * 4) as usize);
    let samples = (supersample * supersample) as f32;
    for y in 0..size {
        for x in 0..size {
            let mut red = 0.0;
            let mut green = 0.0;
            let mut blue = 0.0;
            let mut alpha = 0.0;
            for offset_y in 0..supersample {
                for offset_x in 0..supersample {
                    let px = x * supersample + offset_x;
                    let py = y * supersample + offset_y;
                    let pixel = buffer[(py * width + px) as usize];
                    red += pixel[0] * pixel[3];
                    green += pixel[1] * pixel[3];
                    blue += pixel[2] * pixel[3];
                    alpha += pixel[3];
                }
            }
            let (red, green, blue) = if alpha > 1e-6 {
                (red / alpha, green / alpha, blue / alpha)
            } else {
                (0.0, 0.0, 0.0)
            };
            output.extend_from_slice(&[
                to_u8(red),
                to_u8(green),
                to_u8(blue),
                to_u8(alpha / samples),
            ]);
        }
    }
    output
}

fn mix(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3],
    ]
}

fn with_alpha(color: [f32; 4], alpha: f32) -> [f32; 4] {
    [color[0], color[1], color[2], color[3] * alpha]
}

fn blend(destination: &mut [f32; 4], source: [f32; 4], alpha: f32) {
    let source_alpha = (source[3] * alpha).clamp(0.0, 1.0);
    if source_alpha <= 0.0 {
        return;
    }
    let destination_alpha = destination[3];
    let output_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);
    if output_alpha <= 1e-6 {
        return;
    }
    for channel in 0..3 {
        destination[channel] = (source[channel] * source_alpha
            + destination[channel] * destination_alpha * (1.0 - source_alpha))
            / output_alpha;
    }
    destination[3] = output_alpha;
}

fn overlay_to_pixel(x: f32, y: f32, width: u32, extent: f32) -> [f32; 2] {
    [
        (x / extent * 0.5 + 0.5) * width as f32,
        (y / extent * 0.5 + 0.5) * width as f32,
    ]
}

fn pixel_world(x: u32, y: u32, width: u32, extent: f32) -> (f32, f32) {
    let scale = width as f32;
    (
        ((x as f32 + 0.5) / scale * 2.0 - 1.0) * extent,
        ((y as f32 + 0.5) / scale * 2.0 - 1.0) * extent,
    )
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bloub::{BloubEngine, BloubPalette};

    fn frame() -> BloubFrame {
        let mut engine = BloubEngine::new();
        engine.sample(0.0)
    }

    #[test]
    fn renders_body_and_eyes() {
        let renderer = BloubFlatRenderer::new();
        let size = renderer.size();
        let rgba = renderer.render(&frame(), BloubPalette::DARK);
        assert_eq!(rgba.len(), (size * size * 4) as usize);

        let opaque = rgba.chunks_exact(4).filter(|pixel| pixel[3] > 0).count();
        assert!(opaque > (size * size / 20) as usize);

        let light = rgba
            .chunks_exact(4)
            .filter(|pixel| pixel[0] > 150 && pixel[1] > 150 && pixel[2] > 150)
            .count();
        assert!(light > 10);

        for (x, y) in [(0, 0), (size - 1, 0), (0, size - 1), (size - 1, size - 1)] {
            let index = ((y * size + x) * 4) as usize;
            assert_eq!(rgba[index + 3], 0);
        }
    }

    #[test]
    fn render_is_deterministic() {
        let renderer = BloubFlatRenderer::new();
        let frame = frame();
        assert_eq!(
            renderer.render(&frame, BloubPalette::LIGHT),
            renderer.render(&frame, BloubPalette::LIGHT)
        );
    }
}
