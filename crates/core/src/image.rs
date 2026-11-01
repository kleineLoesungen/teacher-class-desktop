//! Bilder für den PDF-Ausdruck (Logo): JPEG und PNG.
//!
//! JPEG geht unverändert ins PDF (das Format kennt jedes PDF-Programm). PNG
//! wird gelesen und als Bildpunkte neu verpackt; ein Alphakanal wird zur
//! Maske, damit ein Logo mit durchsichtigem Hintergrund durchsichtig bleibt.

use std::io::Write as _;
use std::path::Path;

use flate2::Compression;
use flate2::write::ZlibEncoder;

/// So groß darf ein Bild höchstens sein; ein Logo braucht nicht mehr.
const MAX_BYTES: u64 = 20 * 1024 * 1024;

pub struct Image {
    pub width: u32,
    pub height: u32,
    pub data: Data,
}

pub enum Data {
    /// Eine JPEG-Datei, so wie sie ist.
    Jpeg { bytes: Vec<u8>, color: &'static str },
    /// Bildpunkte mit 8 Bit je Farbe, zlib-gepackt; dazu optional die Maske.
    Raw {
        color: &'static str,
        packed: Vec<u8>,
        mask: Option<Vec<u8>>,
    },
}

/// Liest das Bild. Der Fehler ist eine fertige Meldung (ohne Dateinamen).
pub fn load(path: &Path) -> Result<Image, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("lässt sich nicht lesen ({e})"))?;
    if meta.len() > MAX_BYTES {
        return Err("ist größer als 20 MB".into());
    }
    let bytes = std::fs::read(path).map_err(|e| format!("lässt sich nicht lesen ({e})"))?;
    if bytes.starts_with(&[0xFF, 0xD8]) {
        jpeg(bytes)
    } else if bytes.starts_with(b"\x89PNG") {
        png(&bytes)
    } else {
        Err("ist weder PNG noch JPEG".into())
    }
}

fn jpeg(bytes: Vec<u8>) -> Result<Image, String> {
    let mut i = 2;
    while i + 9 < bytes.len() {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = bytes[i + 1];
        if marker == 0xFF {
            i += 1;
            continue;
        }
        if (0xD0..=0xD9).contains(&marker) || marker == 0x01 {
            i += 2;
            continue;
        }
        let len = usize::from(bytes[i + 2]) << 8 | usize::from(bytes[i + 3]);
        // SOF0 bis SOF15 ohne DHT (C4), JPG (C8) und DAC (CC).
        if (0xC0..=0xCF).contains(&marker) && ![0xC4, 0xC8, 0xCC].contains(&marker) {
            let height = u32::from(bytes[i + 5]) << 8 | u32::from(bytes[i + 6]);
            let width = u32::from(bytes[i + 7]) << 8 | u32::from(bytes[i + 8]);
            let color = match bytes[i + 9] {
                1 => "DeviceGray",
                3 => "DeviceRGB",
                _ => return Err("ist ein JPEG mit CMYK-Farben — als RGB speichern".into()),
            };
            if width == 0 || height == 0 {
                return Err("hat keine Größe".into());
            }
            return Ok(Image {
                width,
                height,
                data: Data::Jpeg { bytes, color },
            });
        }
        i += 2 + len;
    }
    Err("ist ein beschädigtes JPEG".into())
}

fn png(bytes: &[u8]) -> Result<Image, String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("ist ein beschädigtes PNG ({e})"))?;
    let size = reader.output_buffer_size().ok_or("ist zu groß")?;
    if size as u64 > 4 * MAX_BYTES {
        return Err("ist zu groß".into());
    }
    let mut buf = vec![0; size];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("ist ein beschädigtes PNG ({e})"))?;
    let raw = &buf[..info.buffer_size()];

    let (color, channels, alpha) = match info.color_type {
        png::ColorType::Grayscale => ("DeviceGray", 1, false),
        png::ColorType::GrayscaleAlpha => ("DeviceGray", 1, true),
        png::ColorType::Rgb => ("DeviceRGB", 3, false),
        png::ColorType::Rgba => ("DeviceRGB", 3, true),
        png::ColorType::Indexed => {
            return Err("hat eine Farbtabelle, die nicht aufgelöst wurde".into());
        }
    };
    let stride = channels + usize::from(alpha);
    let (picture, mask) = if alpha {
        let mut picture = Vec::with_capacity(raw.len());
        let mut mask = Vec::with_capacity(raw.len() / stride);
        for px in raw.chunks_exact(stride) {
            picture.extend_from_slice(&px[..channels]);
            mask.push(px[channels]);
        }
        (picture, Some(mask))
    } else {
        (raw.to_vec(), None)
    };
    Ok(Image {
        width: info.width,
        height: info.height,
        data: Data::Raw {
            color,
            packed: zlib(&picture)?,
            mask: mask.map(|m| zlib(&m)).transpose()?,
        },
    })
}

fn zlib(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(data)
        .and_then(|()| enc.finish())
        .map_err(|e| format!("lässt sich nicht verpacken ({e})"))
}
