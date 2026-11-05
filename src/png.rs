use std::fs::File;
use std::io::{BufWriter, Write};

use crate::framebuffer::Framebuffer;

/// Escritor de PNG (RGB de 8 bits) implementado desde cero, sin dependencias.
///
/// La compresión zlib se hace con bloques "stored" (sin comprimir), que son
/// deflate válido, de modo que el archivo lo abre cualquier visor de imágenes.
pub fn write_png_file(
    file_path: &str,
    buffer: &[u32],
    width: usize,
    height: usize,
) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(file_path)?);

    // Firma PNG.
    file.write_all(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A])?;

    // --- IHDR ---
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&(width as u32).to_be_bytes());
    ihdr.extend_from_slice(&(height as u32).to_be_bytes());
    ihdr.push(8); // profundidad de bits
    ihdr.push(2); // tipo de color: 2 = RGB (truecolor)
    ihdr.push(0); // compresión
    ihdr.push(0); // filtro
    ihdr.push(0); // sin entrelazado
    write_chunk(&mut file, b"IHDR", &ihdr)?;

    // --- IDAT ---
    // Datos crudos: cada fila lleva un byte de filtro (0 = None) seguido de RGB.
    // El PNG almacena las filas de arriba hacia abajo (buffer[0] = arriba).
    let mut raw = Vec::with_capacity(height * (1 + width * 3));
    for y in 0..height {
        raw.push(0); // filtro None
        for x in 0..width {
            let pixel = buffer[y * width + x];
            raw.push((pixel >> 16) as u8); // R
            raw.push((pixel >> 8) as u8); // G
            raw.push(pixel as u8); // B
        }
    }

    let zlib = zlib_store(&raw);
    write_chunk(&mut file, b"IDAT", &zlib)?;

    // --- IEND ---
    write_chunk(&mut file, b"IEND", &[])?;

    Ok(())
}

/// Escribe un chunk PNG: longitud (BE) + tipo + datos + CRC32(tipo+datos) (BE).
fn write_chunk(
    file: &mut BufWriter<File>,
    chunk_type: &[u8; 4],
    data: &[u8],
) -> std::io::Result<()> {
    file.write_all(&(data.len() as u32).to_be_bytes())?;
    file.write_all(chunk_type)?;
    file.write_all(data)?;

    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(chunk_type);
    crc_input.extend_from_slice(data);
    file.write_all(&crc32(&crc_input).to_be_bytes())?;

    Ok(())
}

/// Envuelve `data` en un flujo zlib usando bloques deflate "stored".
fn zlib_store(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();

    // Cabecera zlib: CMF=0x78, FLG=0x01 (comprobación (0x78*256+0x01) % 31 == 0).
    out.push(0x78);
    out.push(0x01);

    // Bloques stored: cada uno lleva hasta 65535 bytes.
    let max = 65535usize;
    let mut offset = 0;
    while offset < data.len() {
        let end = (offset + max).min(data.len());
        let chunk = &data[offset..end];
        let is_last = end == data.len();

        out.push(if is_last { 1 } else { 0 }); // BFINAL, BTYPE=00 (stored)
        let len = chunk.len() as u16;
        out.extend_from_slice(&len.to_le_bytes()); // LEN
        out.extend_from_slice(&(!len).to_le_bytes()); // NLEN (complemento)
        out.extend_from_slice(chunk);

        offset = end;
    }

    // Caso borde: datos vacíos -> un bloque final vacío.
    if data.is_empty() {
        out.push(1);
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&(!0u16).to_le_bytes());
    }

    // Checksum adler32 del flujo sin comprimir (BE).
    out.extend_from_slice(&adler32(data).to_be_bytes());

    out
}

/// CRC-32 (polinomio IEEE 802.3, reflejado) usado por los chunks PNG.
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    crc ^ 0xFFFF_FFFF
}

/// Adler-32 usado por el flujo zlib.
fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

pub trait WritePng {
    fn render_png(&self, file_path: &str) -> std::io::Result<()>;
}

impl WritePng for Framebuffer {
    fn render_png(&self, file_path: &str) -> std::io::Result<()> {
        write_png_file(file_path, &self.buffer, self.width, self.height)
    }
}
