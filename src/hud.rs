use crate::shaders::ShaderSettings;

const PANEL_WIDTH: usize = 365;
const LINE_HEIGHT: usize = 18;
const FONT_SCALE: usize = 2;

pub fn draw_help(buffer: &mut [u32], width: usize, height: usize, settings: ShaderSettings) {
    if width < PANEL_WIDTH + 8 {
        return;
    }
    let x0 = 8;
    let y0 = 8;
    let lines = help_lines(settings);
    let panel_height = 8 + LINE_HEIGHT * lines.len();
    if height < panel_height + 16 {
        return;
    }
    for y in y0..(y0 + panel_height).min(height) {
        for x in x0..(x0 + PANEL_WIDTH).min(width) {
            buffer[y * width + x] = 0x101a2b;
        }
    }

    for (line_index, line) in lines.iter().enumerate() {
        draw_text(
            buffer,
            width,
            height,
            x0 + 10,
            y0 + 7 + line_index * LINE_HEIGHT,
            line,
            if line_index == 9 || line_index == 10 {
                0x8ee6c0
            } else {
                0xe4edf8
            },
        );
    }
}

fn draw_text(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    text: &str,
    color: u32,
) {
    for (index, character) in text.chars().enumerate() {
        let glyph = glyph(character);
        for (gy, row) in glyph.iter().enumerate() {
            for gx in 0..5 {
                if row & (1 << (4 - gx)) != 0 {
                    for sy in 0..FONT_SCALE {
                        for sx in 0..FONT_SCALE {
                            let px = x + index * 6 * FONT_SCALE + gx * FONT_SCALE + sx;
                            let py = y + gy * FONT_SCALE + sy;
                            if px < width && py < height {
                                buffer[py * width + px] = color;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn help_lines(settings: ShaderSettings) -> Vec<String> {
    vec![
        "AYUDA - ATLAS CELESTE".to_owned(),
        "1 SOL 2 ROCOSO".to_owned(),
        "3 GAS GIGANTE".to_owned(),
        "4 OCEANICO".to_owned(),
        "5 VOLCANICO 6 HELADO".to_owned(),
        "TAB CAMBIA CUERPO".to_owned(),
        "FLECHAS ROTAN".to_owned(),
        "SPACE PAUSA O MOVER".to_owned(),
        "S GUARDA CAPTURA PNG".to_owned(),
        format!("BRILLO Z- X+ {:.2}", settings.brightness),
        format!("DETALLE Q- E+ {:.2}", settings.detail),
        "R REINICIA AJUSTES".to_owned(),
        "H OCULTA AYUDA ESC SALE".to_owned(),
    ]
}

fn glyph(c: char) -> [u8; 7] {
    match c.to_ascii_uppercase() {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 18, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        '+' => [0, 4, 4, 31, 4, 4, 0],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '/' => [1, 2, 2, 4, 8, 8, 16],
        '.' => [0, 0, 0, 0, 0, 12, 12],
        ':' => [0, 12, 12, 0, 12, 12, 0],
        ',' => [0, 0, 0, 0, 4, 4, 8],
        _ => [0; 7],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_draws_scaled_text_inside_panel_and_respects_small_targets() {
        let lines = help_lines(ShaderSettings::default());
        assert!(lines.iter().any(|line| line == "BRILLO Z- X+ 1.00"));
        assert!(lines.iter().any(|line| line == "DETALLE Q- E+ 1.00"));
        let mut pixels = vec![0; 1100 * 760];
        draw_help(&mut pixels, 1100, 760, ShaderSettings::default());
        assert_eq!(pixels[9 * 1100 + 9], 0x101a2b);
        assert!(pixels.contains(&0xe4edf8));
        assert!(pixels.contains(&0x8ee6c0));
        assert_eq!(pixels[220 * 1100 + 400], 0);

        let mut small = vec![0; 200 * 100];
        draw_help(&mut small, 200, 100, ShaderSettings::default());
        assert!(small.iter().all(|&pixel| pixel == 0));
    }
}
