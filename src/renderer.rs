use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::mesh::{Mesh, Vertex};
use crate::shaders::{shade, shade_moon, shade_ring, BodyKind};
use nalgebra_glm::{vec3, Vec3};

pub const WIDTH: usize = 1100;
pub const HEIGHT: usize = 760;

pub struct Renderer {
    sphere: Mesh,
    ring: Mesh,
    pub framebuffer: Framebuffer,
    pub rotation: Vec3,
    pub translation: Vec3,
}

impl Renderer {
    pub fn new() -> Self {
        let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
        framebuffer.set_background(starfield(WIDTH, HEIGHT));
        Self {
            sphere: Mesh::uv_sphere(44, 72),
            ring: Mesh::annular_ring(3, 144),
            framebuffer,
            rotation: Vec3::zeros(),
            translation: Vec3::zeros(),
        }
    }

    pub fn render(&mut self, kind: BodyKind, time: f32) {
        self.framebuffer.clear();
        let angle = time * 0.18;
        let orientation = rotation_x(self.rotation.x) * rotation_y(self.rotation.y);
        let ring_tilt = orientation * rotation_z(-0.25) * rotation_x(0.55);
        let tilt = ring_tilt * rotation_y(angle);
        let scale = if kind == BodyKind::GasGiant {
            160.0
        } else {
            225.0
        };
        if kind == BodyKind::GasGiant {
            draw_mesh(
                &mut self.framebuffer,
                &self.ring,
                ring_tilt,
                scale,
                self.translation,
                |p, _n| shade_ring(p, time),
            );
        }
        draw_mesh(
            &mut self.framebuffer,
            &self.sphere,
            tilt,
            scale,
            self.translation,
            |p, n| shade(kind, p, n, time),
        );
        if kind == BodyKind::GasGiant {
            let screen_center = self.translation + moon_center(ring_tilt, scale, angle);
            draw_mesh(
                &mut self.framebuffer,
                &self.sphere,
                tilt,
                27.0,
                screen_center,
                |p, n| shade_moon(p, n, time),
            );
        }
    }
}

// Orbital displacement scales with the planet radius, not the moon radius.
fn moon_center(orientation: nalgebra_glm::Mat4, planet_radius: f32, angle: f32) -> Vec3 {
    let phase = angle + 0.35;
    orientation
        .transform_vector(&(vec3(2.85 * phase.cos(), 0.0, 2.85 * phase.sin()) * planet_radius))
}

fn draw_mesh<F: Fn(Vec3, Vec3) -> Color>(
    fb: &mut Framebuffer,
    mesh: &Mesh,
    rotation: nalgebra_glm::Mat4,
    radius: f32,
    center: Vec3,
    shader: F,
) {
    let cx = fb.width as f32 * 0.5;
    let cy = fb.height as f32 * 0.5;
    for tri in &mesh.triangles {
        let mut screen = [(0.0f32, 0.0f32, 0.0f32); 3];
        let mut pos = [vec3(0.0, 0.0, 0.0); 3];
        let mut norm = pos;
        for i in 0..3 {
            let v: &Vertex = &tri[i];
            let transformed = rotation.transform_vector(&v.position);
            pos[i] = v.position;
            norm[i] = rotation.transform_vector(&v.normal).normalize();
            screen[i] = (
                cx + center.x + transformed.x * radius,
                cy - center.y - transformed.y * radius,
                center.z + transformed.z * radius,
            );
        }
        // Cull invisible sphere triangles, but keep both sides of the annulus.
        if tri[0].normal.dot(&tri[0].position) > 0.5 && norm.iter().all(|normal| normal.z <= 0.0) {
            continue;
        }
        raster_triangle(fb, screen, pos, norm, &shader);
    }
}

fn raster_triangle<F: Fn(Vec3, Vec3) -> Color>(
    fb: &mut Framebuffer,
    s: [(f32, f32, f32); 3],
    p: [Vec3; 3],
    n: [Vec3; 3],
    shader: &F,
) {
    let area = edge(s[0].0, s[0].1, s[1].0, s[1].1, s[2].0, s[2].1);
    if fb.width == 0
        || fb.height == 0
        || !area.is_finite()
        || area.abs() < 0.0001
        || s.iter()
            .any(|v| !v.0.is_finite() || !v.1.is_finite() || !v.2.is_finite())
    {
        return;
    }
    let minx = s
        .iter()
        .map(|v| v.0)
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.0) as usize;
    let maxx = s
        .iter()
        .map(|v| v.0)
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .min((fb.width - 1) as f32) as usize;
    let miny = s
        .iter()
        .map(|v| v.1)
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.0) as usize;
    let maxy = s
        .iter()
        .map(|v| v.1)
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .min((fb.height - 1) as f32) as usize;
    for y in miny..=maxy {
        for x in minx..=maxx {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let w0 = edge(s[1].0, s[1].1, s[2].0, s[2].1, px, py) / area;
            let w1 = edge(s[2].0, s[2].1, s[0].0, s[0].1, px, py) / area;
            let w2 = 1.0 - w0 - w1;
            if w0 >= -0.00001 && w1 >= -0.00001 && w2 >= -0.00001 {
                let position = p[0] * w0 + p[1] * w1 + p[2] * w2;
                let normal = (n[0] * w0 + n[1] * w1 + n[2] * w2)
                    .try_normalize(1e-8)
                    .unwrap_or_else(|| vec3(0.0, 0.0, 1.0));
                fb.set_current_color(shader(position, normal));
                fb.point_with_depth(x, y, s[0].2 * w0 + s[1].2 * w1 + s[2].2 * w2);
            }
        }
    }
}
fn edge(ax: f32, ay: f32, bx: f32, by: f32, x: f32, y: f32) -> f32 {
    (x - ax) * (by - ay) - (y - ay) * (bx - ax)
}
fn rotation_x(a: f32) -> nalgebra_glm::Mat4 {
    nalgebra_glm::rotation(a, &vec3(1.0, 0.0, 0.0))
}
fn rotation_y(a: f32) -> nalgebra_glm::Mat4 {
    nalgebra_glm::rotation(a, &vec3(0.0, 1.0, 0.0))
}
fn rotation_z(a: f32) -> nalgebra_glm::Mat4 {
    nalgebra_glm::rotation(a, &vec3(0.0, 0.0, 1.0))
}
fn starfield(w: usize, h: usize) -> Vec<u32> {
    let mut out = vec![0x050812; w * h];
    let mut seed = 0x91e10da5u32;
    for _ in 0..4300 {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let x = (seed as usize) % w;
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let y = (seed as usize) % h;
        let b = 70 + seed % 146;
        out[y * w + x] = (b << 16) | (b << 8) | b;
    }
    for y in 0..h {
        for x in 0..w {
            let t = (y as f32 / h as f32 * 0.03) as u32;
            out[y * w + x] = out[y * w + x].saturating_add(t << 8);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn degenerate_nonfinite_and_offscreen_triangles_do_not_draw() {
        let mut fb = Framebuffer::new(8, 8);
        let p = [Vec3::zeros(); 3];
        let n = [vec3(0.0, 0.0, 1.0); 3];
        let calls = Cell::new(0);
        let shader = |_, _| {
            calls.set(calls.get() + 1);
            Color::new(255, 0, 0)
        };
        for screen in [
            [(1.0, 1.0, 0.0); 3],
            [(0.0, 0.0, 0.0), (2.0, 2.0, 0.0), (4.0, 4.0, 0.0)],
            [(f32::NAN, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0)],
            [(20.0, 20.0, 0.0), (30.0, 20.0, 0.0), (20.0, 30.0, 0.0)],
        ] {
            raster_triangle(&mut fb, screen, p, n, &shader);
        }
        assert_eq!(calls.get(), 0);
        assert!(fb.buffer.iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn barycentric_local_position_and_normal_are_interpolated() {
        let mut fb = Framebuffer::new(4, 4);
        let screen = [(0.0, 0.0, 0.0), (4.0, 0.0, 0.0), (0.0, 4.0, 0.0)];
        let positions = [
            vec3(0.0, 0.0, 0.0),
            vec3(1.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
        ];
        let normals = [
            vec3(0.0, 0.0, 1.0),
            vec3(1.0, 0.0, 1.0),
            vec3(0.0, 1.0, 1.0),
        ];
        raster_triangle(&mut fb, screen, positions, normals, &|p, n| {
            assert!((n.norm() - 1.0).abs() < 1e-5);
            Color::from_unit(p.x, p.y, n.z)
        });
        let expected = Color::from_unit(0.125, 0.125, vec3(0.125, 0.125, 1.0).normalize().z);
        assert_eq!(fb.buffer[0], expected.to_hex());
        assert_eq!(fb.buffer[15], 0);
    }

    #[test]
    fn interpolated_depth_wins_independent_of_draw_order_and_winding() {
        let p = [Vec3::zeros(); 3];
        let n = [vec3(0.0, 0.0, 1.0); 3];
        let sloped = [(0.0, 0.0, 0.0), (4.0, 0.0, 4.0), (0.0, 4.0, 0.0)];
        let flat = [(0.0, 0.0, 1.0), (0.0, 4.0, 1.0), (4.0, 0.0, 1.0)];
        for reverse in [false, true] {
            let mut fb = Framebuffer::new(4, 4);
            let draw_slope = |fb: &mut Framebuffer| {
                raster_triangle(fb, sloped, p, n, &|_, _| Color::new(255, 0, 0))
            };
            let draw_flat = |fb: &mut Framebuffer| {
                raster_triangle(fb, flat, p, n, &|_, _| Color::new(0, 255, 0))
            };
            if reverse {
                draw_flat(&mut fb);
                draw_slope(&mut fb);
            } else {
                draw_slope(&mut fb);
                draw_flat(&mut fb);
            }
            assert_eq!(fb.buffer[0], 0x00ff00);
            assert_eq!(fb.buffer[2], 0xff0000);
        }
    }

    #[test]
    fn moon_scales_with_planet_and_is_separate_at_default_phase() {
        let orientation = rotation_z(-0.25) * rotation_x(0.55);
        let center = moon_center(orientation, 160.0, 0.0);
        assert!((center.norm() - 2.85 * 160.0).abs() < 0.001);
        assert!((moon_center(orientation, 320.0, 0.0) - center * 2.0).norm() < 0.001);
        assert!(center.xy().norm() > 2.15 * 160.0 + 27.0);
        assert!(center.x.abs() + 27.0 < WIDTH as f32 * 0.5);
        assert!(center.y.abs() + 27.0 < HEIGHT as f32 * 0.5);
    }

    #[test]
    fn model_transform_keeps_local_coordinates_and_rotates_view_normals() {
        let vertex = |p| Vertex {
            position: p,
            normal: vec3(0.0, 0.0, 1.0),
        };
        let mesh = Mesh {
            triangles: vec![[
                vertex(vec3(-0.8, -0.8, 0.0)),
                vertex(vec3(0.8, -0.8, 0.0)),
                vertex(vec3(0.0, 0.8, 0.0)),
            ]],
        };
        let mut fb = Framebuffer::new(32, 32);
        let rotation = rotation_y(0.4);
        let expected = rotation.transform_vector(&vec3(0.0, 0.0, 1.0));
        let calls = Cell::new(0);
        draw_mesh(
            &mut fb,
            &mesh,
            rotation,
            8.0,
            vec3(3.0, 2.0, 1.0),
            |p, n| {
                assert!(p.z.abs() < 1e-6);
                assert!((n - expected).norm() < 1e-5);
                calls.set(calls.get() + 1);
                Color::new(255, 255, 255)
            },
        );
        assert!(calls.get() > 0);
        assert_ne!(fb.buffer[14 * 32 + 19], 0);
    }
}
