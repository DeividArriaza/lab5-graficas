/// Color RGB de 8 bits por canal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }

    /// Convierte canales en el rango 0..1 (como los de un MTL) a 8 bits.
    /// Los valores fuera de rango se recortan.
    pub fn from_unit(r: f32, g: f32, b: f32) -> Self {
        let to_byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        Color::new(to_byte(r), to_byte(g), to_byte(b))
    }

    /// Empaqueta el color como 0x00RRGGBB, el formato que usan el framebuffer
    /// y la ventana de minifb.
    pub const fn to_hex(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32
    }
}
