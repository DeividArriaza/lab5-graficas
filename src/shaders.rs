use nalgebra_glm::Vec3;

use crate::color::Color;

/// Los seis cuerpos que pueden dibujarse con estos materiales procedurales.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyKind {
    Sun,
    Rocky,
    GasGiant,
    Ocean,
    Lava,
    Ice,
}

const RING_GAPS: [f32; 3] = [1.55, 1.83, 2.02];

pub const ALL_BODIES: [BodyKind; 6] = [
    BodyKind::Sun,
    BodyKind::Rocky,
    BodyKind::GasGiant,
    BodyKind::Ocean,
    BodyKind::Lava,
    BodyKind::Ice,
];

impl BodyKind {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Sun => "sol",
            Self::Rocky => "rocoso",
            Self::GasGiant => "gaseoso",
            Self::Ocean => "oceanico",
            Self::Lava => "volcanico",
            Self::Ice => "helado",
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Sun => "Sol — Granulación y fuego",
            Self::Rocky => "Rocoso — Estratos minerales",
            Self::GasGiant => "Gigante gaseoso — Tormentas de jade",
            Self::Ocean => "Oceánico — Mares de zafiro",
            Self::Lava => "Volcánico — Ríos de magma",
            Self::Ice => "Helado — Fracturas de cristal",
        }
    }
}

fn fract(v: f32) -> f32 {
    v - v.floor()
}
fn hash(x: f32, y: f32, z: f32) -> f32 {
    fract((x * 127.1 + y * 311.7 + z * 74.7).sin() * 43_758.547) * 2.0 - 1.0
}
fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Continuous trilinear value noise; deterministic and bounded to [-1, 1].
fn noise(p: Vec3) -> f32 {
    let (x, y, z) = (p.x.floor(), p.y.floor(), p.z.floor());
    let (u, v, w) = (smooth(fract(p.x)), smooth(fract(p.y)), smooth(fract(p.z)));
    let a = hash(x, y, z) * (1.0 - u) + hash(x + 1.0, y, z) * u;
    let b = hash(x, y + 1.0, z) * (1.0 - u) + hash(x + 1.0, y + 1.0, z) * u;
    let c = hash(x, y, z + 1.0) * (1.0 - u) + hash(x + 1.0, y, z + 1.0) * u;
    let d = hash(x, y + 1.0, z + 1.0) * (1.0 - u) + hash(x + 1.0, y + 1.0, z + 1.0) * u;
    (a * (1.0 - v) + b * v) * (1.0 - w) + (c * (1.0 - v) + d * v) * w
}
fn fbm(mut p: Vec3, octaves: usize) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.5;
    let mut norm = 0.0;
    for _ in 0..octaves {
        sum += noise(p) * amp;
        norm += amp;
        p *= 2.03;
        amp *= 0.5;
    }
    sum / norm
}
#[cfg(test)]
thread_local! {
    static GLOBAL_FBM_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
fn warp(p: Vec3, time: f32) -> Vec3 {
    p + Vec3::new(
        fbm(p * 1.7 + Vec3::new(time * 0.12, 2.1, 0.0), 3),
        fbm(p * 1.7 + Vec3::new(4.2, time * 0.09, 1.3), 3),
        fbm(p * 1.7 + Vec3::new(0.7, 3.8, time * 0.1), 3),
    ) * 0.22
}
fn mix(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a * (1.0 - t) + b * t
}
fn ramp(a: Vec3, b: Vec3, x: f32, lo: f32, hi: f32) -> Vec3 {
    mix(a, b, smooth(((x - lo) / (hi - lo)).clamp(0.0, 1.0)))
}
fn clamp_color(c: Vec3) -> Color {
    Color::from_unit(c.x, c.y, c.z)
}
fn lighting(base: Vec3, position: Vec3, normal: Vec3, atmosphere: Vec3) -> Vec3 {
    let n = normal.normalize();
    let light = Vec3::new(-0.4, 0.6, 1.0).normalize();
    let diffuse = n.dot(&light).max(0.0);
    let rim = (1.0 - n.z.abs()).powf(2.0) * 0.16;
    base * (0.40 + 0.60 * diffuse)
        + atmosphere * (rim + 0.035)
        + Vec3::new(1.0, 0.72, 0.42) * 0.018 * position.z.abs()
}

pub fn shade(kind: BodyKind, position: Vec3, normal: Vec3, time: f32) -> Color {
    let p = warp(position * 3.1, time);
    let n = if kind == BodyKind::Rocky {
        #[cfg(test)]
        GLOBAL_FBM_CALLS.with(|calls| calls.set(calls.get() + 1));
        fbm(p * 2.3, 4)
    } else {
        0.0
    };
    let base = match kind {
        BodyKind::Sun => {
            // Granulación fotosférica, plasma convectivo, manchas oscuras y filamentos calientes.
            let gran = fbm(p * 8.0, 3); // granular cell structure
            let plasma = fbm(warp(p * 2.2, time * 1.4) * 2.0, 4); // animated plasma
            let spots = fbm(p * 3.0 + Vec3::new(3.2, 1.7, 0.4), 3); // magnetic sunspots
            let filaments = (fbm(p * 13.0 + Vec3::new(time * 0.3, 0.0, 0.0), 3) * 8.0)
                .sin()
                .abs(); // bright filaments
            let gold = ramp(
                Vec3::new(0.35, 0.045, 0.008),
                Vec3::new(1.0, 0.49, 0.055),
                plasma,
                -0.55,
                0.55,
            );
            let granulated = gold * (0.78 + 0.22 * gran);
            let spotted = mix(
                granulated,
                Vec3::new(0.19, 0.025, 0.006),
                ((0.1 - spots) * 3.0).clamp(0.0, 0.72),
            );
            spotted + Vec3::new(1.0, 0.52, 0.12) * (filaments * 0.11)
        }
        BodyKind::Rocky => {
            // Sedimentary terracotta strata, impact basins, exposed minerals, cyan veins.
            let strata = (position.y * 27.0 + n * 3.0).sin(); // layered strata
            let crater = fbm(p * 5.4, 3); // cratered regolith
            let minerals = fbm(p * 12.0 + Vec3::new(2.0, 0.0, 3.0), 3); // mineral inclusions
            let vein = (fbm(p * 9.0, 3) * 7.0).sin().abs(); // branching cyan mineral veins
            let earth = ramp(
                Vec3::new(0.18, 0.055, 0.035),
                Vec3::new(0.76, 0.30, 0.13),
                strata,
                -0.7,
                0.7,
            );
            let weathered = mix(
                earth,
                Vec3::new(0.34, 0.21, 0.13),
                ((0.0 - crater) * 1.4).clamp(0.0, 0.58),
            );
            mix(
                weathered,
                Vec3::new(0.12, 0.52, 0.55),
                ((0.12 - vein) * 3.0).clamp(0.0, 0.55),
            ) * (0.82 + 0.18 * minerals)
        }
        BodyKind::GasGiant => {
            // Turquoise/teal/cream/ochre cloud belts and a persistent rotating storm.
            let belts = (position.y * 24.0 + fbm(p * 1.2, 3) * 3.8).sin(); // cloud bands
            let turbulence = fbm(warp(p * 2.0, time * 0.8), 4); // turbulent swirls
            let storm_uv = Vec3::new(position.x * 4.0, position.y * 3.0, position.z * 2.0);
            let storm = ((storm_uv.x * storm_uv.x * 1.2 + storm_uv.y * storm_uv.y * 2.0).sqrt()
                - 0.68
                + fbm(storm_uv * 3.0 + Vec3::new(time * 0.16, 0.0, 0.0), 3) * 0.18)
                .abs();
            let cloud = ramp(
                Vec3::new(0.015, 0.22, 0.25),
                Vec3::new(0.28, 0.77, 0.66),
                belts,
                -0.8,
                0.8,
            );
            let cream = mix(
                cloud,
                Vec3::new(0.92, 0.77, 0.54),
                ((belts - 0.42) * 1.7).clamp(0.0, 0.62),
            );
            let ochre = mix(
                cream,
                Vec3::new(0.68, 0.30, 0.11),
                ((turbulence - 0.35) * 1.6).clamp(0.0, 0.38),
            );
            mix(
                ochre,
                Vec3::new(0.78, 0.20, 0.095),
                ((0.12 - storm) * 3.0).clamp(0.0, 0.88),
            )
        }
        BodyKind::Ocean => {
            // Navy abyss, turquoise shelves, jade continents, high cloud and polar ice.
            let continents = fbm(p * 2.0, 4); // landmass silhouettes
            let shelf = fbm(p * 5.0, 3); // shallow continental shelves
            let clouds = fbm(warp(p * 4.5, time * 0.15), 3); // drifting cloud cover
            let ice = position.y.abs() + fbm(p * 3.0, 3) * 0.18; // polar frost caps
            let sea = ramp(
                Vec3::new(0.008, 0.025, 0.14),
                Vec3::new(0.0, 0.48, 0.66),
                shelf,
                -0.2,
                0.65,
            );
            let land = mix(
                sea,
                Vec3::new(0.10, 0.43, 0.22),
                ((continents - 0.03) * 5.0).clamp(0.0, 0.96),
            );
            let cloudy = mix(
                land,
                Vec3::new(0.82, 0.91, 0.88),
                ((clouds - 0.40) * 2.8).clamp(0.0, 0.56),
            );
            mix(
                cloudy,
                Vec3::new(0.78, 0.92, 1.0),
                ((ice - 0.72) * 4.0).clamp(0.0, 0.88),
            )
        }
        BodyKind::Lava => {
            // Purple basalt, branching fissures, incandescent gold cores and drifting ash.
            let basalt = fbm(p * 1.8, 3); // broad fractured basalt plates
            let channel_field = fbm(p * 3.0 + Vec3::new(0.0, time * 0.025, 0.0), 3);
            let fissure = (channel_field * 5.0).sin().abs(); // branching magma rivers
            let core = fbm(p * 6.0 + Vec3::new(time * 0.07, 0.0, 0.0), 2); // gold-hot cores
            let ash = fbm(p * 5.0, 2); // soot and volcanic ash
            let dark = ramp(
                Vec3::new(0.055, 0.025, 0.11),
                Vec3::new(0.24, 0.10, 0.28),
                basalt,
                -0.7,
                0.7,
            ) * (0.9 + ash.abs() * 0.1);
            let lava = ramp(
                Vec3::new(0.8, 0.08, 0.008),
                Vec3::new(1.0, 0.72, 0.08),
                core,
                -0.7,
                0.7,
            );
            let river = ((0.28 - fissure) * 5.0).clamp(0.0, 0.90);
            let core_glow = Vec3::new(1.0, 0.48, 0.06) * river * (0.18 + core.max(0.0) * 0.22);
            mix(dark, lava, river * (0.78 + core.max(0.0) * 0.16)) + core_glow
        }
        BodyKind::Ice => {
            // Blue-white ridges, cyan crevasses, violet mineral deposits and polar frost.
            let ridges = fbm(p * 5.0, 4); // wind-carved ice ridges
            let crack = (fbm(p * 10.0, 3) * 9.0).sin().abs(); // cyan fracture network
            let mineral = fbm(p * 3.0 + Vec3::new(1.0, 4.0, 0.0), 3); // violet mineral strata
            let frost = fbm(p * 13.0, 3); // crystalline surface frost
            let blue = ramp(
                Vec3::new(0.12, 0.34, 0.67),
                Vec3::new(0.88, 0.96, 1.0),
                ridges,
                -0.65,
                0.68,
            );
            let violet = mix(
                blue,
                Vec3::new(0.44, 0.31, 0.70),
                ((mineral - 0.36) * 1.5).clamp(0.0, 0.40),
            );
            let fractured = mix(
                violet,
                Vec3::new(0.04, 0.75, 0.82),
                ((0.12 - crack) * 3.5).clamp(0.0, 0.50),
            );
            mix(
                fractured,
                Vec3::new(0.94, 0.99, 1.0),
                ((frost - 0.35) * 0.8).clamp(0.0, 0.16) + position.y.abs().powf(3.0) * 0.14,
            )
        }
    };
    if kind == BodyKind::Sun {
        return clamp_color(base * 1.65);
    }
    let atmosphere = match kind {
        BodyKind::Rocky => Vec3::new(0.20, 0.10, 0.07),
        BodyKind::GasGiant => Vec3::new(0.10, 0.45, 0.43),
        BodyKind::Ocean => Vec3::new(0.04, 0.35, 0.75),
        BodyKind::Lava => Vec3::new(0.42, 0.08, 0.20),
        BodyKind::Ice => Vec3::new(0.30, 0.66, 1.0),
        _ => Vec3::zeros(),
    };
    clamp_color(lighting(base, position, normal, atmosphere))
}

/// An annulus with several dusty bands and narrow Cassini-like dark divisions.
pub fn shade_ring(position: Vec3, time: f32) -> Color {
    let r = (position.x * position.x + position.z * position.z).sqrt();
    let radial = fbm(
        Vec3::new(r * 17.0 + time * 0.015, position.y * 11.0, r * 3.0),
        3,
    );
    let bands = (r * 57.0 + radial * 2.8).sin();
    let gap = RING_GAPS
        .iter()
        .map(|g| (1.0 - ((r - *g).abs() / 0.018).clamp(0.0, 1.0)).max(0.0))
        .fold(0.0_f32, f32::max);
    let annulus = ((r - 1.35) / 0.12).clamp(0.0, 1.0) * ((2.15 - r) / 0.12).clamp(0.0, 1.0);
    let dust = 0.30 + 0.20 * bands + 0.12 * radial;
    let c = Vec3::new(0.64, 0.48, 0.29) * (dust.clamp(0.08, 0.75) * annulus * (1.0 - gap * 0.94));
    clamp_color(c + Vec3::new(0.025, 0.018, 0.012))
}

/// A moon surface with noisy impact basins, rims and bright ejecta.
pub fn shade_moon(position: Vec3, normal: Vec3, time: f32) -> Color {
    let p = warp(position * 4.0, time * 0.03);
    let terrain = fbm(p * 3.0, 4);
    let craters = fbm(p * 8.0, 3);
    let rim = (craters.abs() * 2.0).clamp(0.0, 1.0);
    let ejecta = fbm(p * 15.0, 3);
    let stone = ramp(
        Vec3::new(0.18, 0.17, 0.16),
        Vec3::new(0.57, 0.54, 0.49),
        terrain,
        -0.6,
        0.6,
    );
    let basin = mix(stone, Vec3::new(0.095, 0.085, 0.08), (1.0 - rim) * 0.34);
    let detailed = mix(
        basin,
        Vec3::new(0.72, 0.68, 0.59),
        ((ejecta - 0.25) * 0.30).clamp(0.0, 0.12),
    );
    clamp_color(lighting(
        detailed,
        position,
        normal,
        Vec3::new(0.12, 0.14, 0.18),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra_glm::vec3;
    use std::collections::HashSet;

    #[test]
    fn global_fbm_is_evaluated_only_for_rocky() {
        let p = vec3(0.2, -0.3, 0.8);
        let normal = vec3(0.0, 0.0, 1.0);
        for kind in ALL_BODIES {
            GLOBAL_FBM_CALLS.with(|calls| calls.set(0));
            let _ = shade(kind, p, normal, 0.75);
            let calls = GLOBAL_FBM_CALLS.with(std::cell::Cell::get);
            assert_eq!(calls, usize::from(kind == BodyKind::Rocky), "{kind:?}");
        }
    }

    #[test]
    fn six_named_distinct_deterministic_spatial_shaders() {
        let p = vec3(0.2, -0.3, 0.8);
        let n = vec3(0.0, 0.0, 1.0);
        let mut slugs = HashSet::new();
        let mut colors = HashSet::new();
        assert_eq!(ALL_BODIES.len(), 6);
        for kind in ALL_BODIES {
            assert!(!kind.name().is_empty());
            assert!(slugs.insert(kind.slug()));
            let color = shade(kind, p, n, 0.75);
            assert_eq!(color, shade(kind, p, n, 0.75));
            assert_ne!(
                color,
                shade(kind, vec3(-0.61, 0.44, 0.31), n, 0.75),
                "{kind:?}"
            );
            assert!(colors.insert(color.to_hex()), "{kind:?}");
        }
    }

    #[test]
    fn animated_sun_and_gas_change_with_time() {
        let p = vec3(0.2, -0.3, 0.8);
        let n = vec3(0.0, 0.0, 1.0);
        for (kind, time) in [(BodyKind::Sun, 1.0), (BodyKind::GasGiant, 3.0)] {
            assert_ne!(shade(kind, p, n, 0.0), shade(kind, p, n, time));
        }
    }

    #[test]
    fn lava_has_visible_warm_channels_and_dark_basalt() {
        let (mut warm, mut dark, mut visible) = (0, 0, 0);
        for yi in -40..=40 {
            for xi in -40..=40 {
                let (x, y) = (xi as f32 / 50.0, yi as f32 / 50.0);
                let r2 = x * x + y * y;
                if r2 <= 1.0 {
                    let p = vec3(x, y, (1.0 - r2).sqrt());
                    let c = shade(BodyKind::Lava, p, p, 0.0);
                    visible += 1;
                    if c.r > 150 && c.g > 45 && u16::from(c.r) > u16::from(c.g) * 3 / 2 {
                        warm += 1;
                    }
                    if c.r < 120 && c.g < 70 && c.b < 150 {
                        dark += 1;
                    }
                }
            }
        }
        let fraction = warm as f32 / visible as f32;
        assert!(
            (0.05..=0.25).contains(&fraction),
            "warm fraction: {fraction:.3}"
        );
        assert!(
            dark * 2 > visible,
            "dark basalt fraction: {:.3}",
            dark as f32 / visible as f32
        );
    }

    #[test]
    fn noise_is_bounded_continuous_and_deterministic() {
        for i in -50..=50 {
            let p = vec3(i as f32 * 0.13, -0.37, 1.23);
            let value = noise(p);
            assert!((-1.0..=1.0).contains(&value));
            assert_eq!(value, noise(p));
            assert!((value - noise(p + vec3(0.0001, 0.0, 0.0))).abs() < 0.01);
            assert!((-1.0..=1.0).contains(&fbm(p, 4)));
        }
    }

    #[test]
    fn ring_gaps_and_moon_surface_have_detail() {
        let brightness = |c: Color| u32::from(c.r) + u32::from(c.g) + u32::from(c.b);
        let gap = shade_ring(vec3(1.55, 0.0, 0.0), 0.0);
        let band = shade_ring(vec3(1.7, 0.0, 0.0), 0.0);
        assert!(brightness(gap) < brightness(band));
        assert_eq!(band, shade_ring(vec3(1.7, 0.0, 0.0), 0.0));
        let normal = vec3(0.0, 0.0, 1.0);
        let p = vec3(0.2, -0.3, 0.8);
        assert_ne!(
            shade_moon(p, normal, 0.0),
            shade_moon(vec3(-0.61, 0.44, 0.31), normal, 0.0)
        );
        assert_eq!(shade_moon(p, normal, 0.0), shade_moon(p, normal, 0.0));
    }
}
