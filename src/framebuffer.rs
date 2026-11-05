#[cfg(test)]
use crate::color::Color;
use rayon::prelude::*;

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub buffer: Vec<u32>,
    // Profundidad del píxel más cercano dibujado hasta ahora. Valores mayores
    // están más cerca de la cámara.
    pub(crate) zbuffer: Vec<f32>,
    // Imagen de fondo que clear() copia en cada cuadro.
    background: Vec<u32>,
    #[cfg(test)]
    current_color: u32,
}

impl Framebuffer {
    pub(crate) fn for_each_band_mut<F>(&mut self, rows_per_band: usize, f: F)
    where
        F: Fn(usize, &mut [u32], &mut [f32]) + Send + Sync,
    {
        if self.width == 0 || self.height == 0 {
            return;
        }
        let band_len = self.width * rows_per_band;
        self.buffer
            .par_chunks_mut(band_len)
            .zip(self.zbuffer.par_chunks_mut(band_len))
            .enumerate()
            .for_each(|(index, (pixels, depths))| f(index * rows_per_band, pixels, depths));
    }

    pub fn new(width: usize, height: usize) -> Self {
        Framebuffer {
            width,
            height,
            buffer: vec![0; width * height],
            zbuffer: vec![f32::NEG_INFINITY; width * height],
            background: vec![0; width * height],
            #[cfg(test)]
            current_color: 0xFFFFFF,
        }
    }

    pub fn clear(&mut self) {
        self.buffer.copy_from_slice(&self.background);
        for depth in self.zbuffer.iter_mut() {
            *depth = f32::NEG_INFINITY;
        }
    }

    // Pinta el píxel solo si está más cerca que lo que ya hay en el z-buffer.
    #[cfg(test)]
    pub fn point_with_depth(&mut self, x: usize, y: usize, depth: f32) {
        if x < self.width && y < self.height {
            let index = y * self.width + x;
            if depth > self.zbuffer[index] {
                self.zbuffer[index] = depth;
                self.buffer[index] = self.current_color;
            }
        }
    }

    /// Usa una imagen (0x00RRGGBB, del tamaño del framebuffer) como fondo.
    /// Si el tamaño no coincide, se conserva el fondo anterior.
    pub fn set_background(&mut self, image: Vec<u32>) {
        if image.len() == self.background.len() {
            self.background = image;
        }
    }

    #[cfg(test)]
    pub fn set_current_color(&mut self, color: Color) {
        self.current_color = color.to_hex();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearer_fragment_wins_regardless_of_draw_order() {
        let mut fb = Framebuffer::new(2, 2);
        fb.set_current_color(Color::new(255, 0, 0));
        fb.point_with_depth(0, 0, 2.0);
        fb.set_current_color(Color::new(0, 255, 0));
        fb.point_with_depth(0, 0, 1.0);
        assert_eq!(fb.buffer[0], 0xff0000);
        fb.point_with_depth(0, 0, 3.0);
        assert_eq!(fb.buffer[0], 0x00ff00);
    }

    #[test]
    fn clear_restores_background_and_resets_depth() {
        let mut fb = Framebuffer::new(2, 2);
        fb.set_background(vec![0x112233; 4]);
        fb.point_with_depth(0, 0, 100.0);
        fb.clear();
        assert_eq!(fb.buffer, vec![0x112233; 4]);
        fb.set_current_color(Color::new(1, 2, 3));
        fb.point_with_depth(0, 0, -100.0);
        assert_eq!(fb.buffer[0], 0x010203);
    }

    #[test]
    fn out_of_bounds_and_invalid_background_do_not_damage_buffer() {
        let mut fb = Framebuffer::new(2, 2);
        fb.set_background(vec![1]);
        fb.point_with_depth(usize::MAX, 0, 0.0);
        fb.point_with_depth(0, usize::MAX, 0.0);
        fb.clear();
        assert_eq!(fb.buffer, vec![0; 4]);
    }
}
