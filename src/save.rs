use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use image::{ExtendedColorType, RgbImage};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Png,
    Webp,
}

impl OutputFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Png => "png",
            OutputFormat::Webp => "webp",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "png" => Some(OutputFormat::Png),
            "webp" => Some(OutputFormat::Webp),
            _ => None,
        }
    }
}

pub fn default_output_path(input: &Path, format: OutputFormat) -> std::path::PathBuf {
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "image".to_string());
    input
        .with_file_name(format!("{stem}_dithered.{}", format.extension()))
}

pub fn save(path: &Path, img: &RgbImage, palette: &[[u8; 3]; 4], format: OutputFormat) -> Result<(), String> {
    match format {
        OutputFormat::Png => write_indexed_png(path, img, palette),
        OutputFormat::Webp => write_webp_lossless(path, img),
    }
}

fn write_indexed_png(path: &Path, img: &RgbImage, palette: &[[u8; 3]; 4]) -> Result<(), String> {
    let file = File::create(path).map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    let writer = BufWriter::new(file);

    let mut encoder = png::Encoder::new(writer, img.width(), img.height());
    encoder.set_color(png::ColorType::Indexed);
    encoder.set_depth(png::BitDepth::Eight);

    let mut flat_palette = Vec::with_capacity(palette.len() * 3);
    for c in palette {
        flat_palette.extend_from_slice(c);
    }
    encoder.set_palette(&flat_palette);

    let mut writer = encoder
        .write_header()
        .map_err(|e| format!("png header failed: {e}"))?;

    let mut indices = Vec::with_capacity((img.width() * img.height()) as usize);
    for p in img.pixels() {
        let idx = palette
            .iter()
            .position(|c| *c == p.0)
            .unwrap_or(0) as u8;
        indices.push(idx);
    }
    writer
        .write_image_data(&indices)
        .map_err(|e| format!("png write failed: {e}"))?;

    Ok(())
}

fn write_webp_lossless(path: &Path, img: &RgbImage) -> Result<(), String> {
    let file = File::create(path).map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    let encoder = image::codecs::webp::WebPEncoder::new_lossless(file);
    encoder
        .encode(img.as_raw(), img.width(), img.height(), ExtendedColorType::Rgb8)
        .map_err(|e| format!("webp encode failed: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_output_path_matches_python() {
        let p = default_output_path(Path::new("/tmp/photo.jpg"), OutputFormat::Png);
        assert_eq!(p, Path::new("/tmp/photo_dithered.png"));
    }

    #[test]
    fn indexed_png_roundtrip() {
        let mut img = RgbImage::new(8, 8);
        for (i, p) in img.pixels_mut().enumerate() {
            *p = image::Rgb([(i % 4) as u8 * 60, (i % 4) as u8 * 60, (i % 4) as u8 * 60]);
        }
        let palette = [[15, 15, 15], [74, 74, 74], [134, 134, 134], [193, 193, 193]];
        let dir = std::env::temp_dir().join("ddn_dither_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_indexed.png");
        save(&path, &img, &palette, OutputFormat::Png).unwrap();

        let decoded = image::open(&path).unwrap().to_rgb8();
        assert_eq!(decoded.dimensions(), (8, 8));
        for p in decoded.pixels() {
            assert!(palette.contains(&p.0), "unexpected color {:?}", p.0);
        }
    }
}
