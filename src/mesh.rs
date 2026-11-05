use nalgebra_glm::{vec3, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub triangles: Vec<[Vertex; 3]>,
}

impl Mesh {
    pub fn uv_sphere(latitude_segments: usize, longitude_segments: usize) -> Self {
        assert!(latitude_segments >= 2 && longitude_segments >= 3);
        let at = |lat: usize, lon: usize| {
            let theta = std::f32::consts::PI * lat as f32 / latitude_segments as f32;
            let phi = std::f32::consts::TAU * lon as f32 / longitude_segments as f32;
            let p = vec3(
                theta.sin() * phi.cos(),
                theta.cos(),
                theta.sin() * phi.sin(),
            );
            Vertex {
                position: p,
                normal: p,
            }
        };
        let mut triangles = Vec::with_capacity(latitude_segments * longitude_segments * 2);
        for lat in 0..latitude_segments {
            for lon in 0..longitude_segments {
                let a = at(lat, lon);
                let b = at(lat + 1, lon);
                let c = at(lat + 1, lon + 1);
                let d = at(lat, lon + 1);
                if lat != 0 {
                    triangles.push([a, b, d]);
                }
                if lat + 1 != latitude_segments {
                    triangles.push([d, b, c]);
                }
            }
        }
        Self { triangles }
    }

    pub fn annular_ring(radial_segments: usize, angular_segments: usize) -> Self {
        assert!(radial_segments > 0 && angular_segments >= 3);
        let point = |radius: f32, angle: f32| Vertex {
            position: vec3(radius * angle.cos(), 0.0, radius * angle.sin()),
            normal: vec3(0.0, 1.0, 0.0),
        };
        let mut triangles = Vec::with_capacity(radial_segments * angular_segments * 2);
        for r in 0..radial_segments {
            let inner = 1.35 + 0.8 * r as f32 / radial_segments as f32;
            let outer = 1.35 + 0.8 * (r + 1) as f32 / radial_segments as f32;
            for a in 0..angular_segments {
                let t0 = std::f32::consts::TAU * a as f32 / angular_segments as f32;
                let t1 = std::f32::consts::TAU * (a + 1) as f32 / angular_segments as f32;
                let i0 = point(inner, t0);
                let i1 = point(inner, t1);
                let o0 = point(outer, t0);
                let o1 = point(outer, t1);
                triangles.push([i0, o0, i1]);
                triangles.push([i1, o0, o1]);
            }
        }
        Self { triangles }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sphere_vertices_are_unit_positions_with_radial_normals() {
        let mesh = Mesh::uv_sphere(12, 24);
        assert!(!mesh.triangles.is_empty());
        for tri in mesh.triangles {
            for v in tri {
                assert!((v.position.norm() - 1.0).abs() < 1e-5);
                assert!((v.normal - v.position).norm() < 1e-6);
            }
        }
    }
    #[test]
    fn ring_is_separate_flat_annulus() {
        let mesh = Mesh::annular_ring(2, 24);
        assert_eq!(mesh.triangles.len(), 96);
        for tri in mesh.triangles {
            for v in tri {
                let r = (v.position.x * v.position.x + v.position.z * v.position.z).sqrt();
                assert!((1.35 - 1e-5..=2.15 + 1e-5).contains(&r));
                assert_eq!(v.position.y, 0.0);
            }
        }
    }

    #[test]
    fn generated_sphere_and_annulus_have_no_degenerate_faces() {
        for mesh in [Mesh::uv_sphere(44, 72), Mesh::annular_ring(3, 144)] {
            for triangle in mesh.triangles {
                let area = (triangle[1].position - triangle[0].position)
                    .cross(&(triangle[2].position - triangle[0].position))
                    .norm();
                assert!(area.is_finite() && area > 1e-6);
            }
        }
    }
}
