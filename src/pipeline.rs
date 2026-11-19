use image::imageops::FilterType;
use image::{Rgb, RgbImage};

use crate::config::{PHOTO_WIDTH, PHOTO_HEIGHT, PALETTES};
use crate::crop::CropRect;
use crate::dither::bayer_dither_4tone;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutScale {
    Integer(u32),
    Width(u32),
}

impl Default for OutScale {
    fn default() -> Self {
        OutScale::Width(crate::config::DEFAULT_OUTPUT_WIDTH)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProcessOptions {
    pub internal_scale: u32,
    pub out_scale: OutScale,
    pub palette_index: usize,
}

impl Default for ProcessOptions {
    fn default() -> Self {
        ProcessOptions {
            internal_scale: crate::config::DEFAULT_INTERNAL_SCALE,
            out_scale: OutScale::default(),
            palette_index: 0,
        }
    }
}

pub fn to_luma(rgb: &[u8]) -> Vec<u8> {
    rgb.chunks_exact(3)
        .map(|px| {
            let (r, g, b) = (px[0] as u32, px[1] as u32, px[2] as u32);
            (((299 * r + 587 * g + 114 * b) + 500) / 1000) as u8
        })
        .collect()
}

pub fn autocontrast(gray: &[u8], cutoff_lo_pct: f64, cutoff_hi_pct: f64) -> Vec<u8> {
    let total = gray.len();
    if total == 0 {
        return gray.to_vec();
    }

    let mut hist = [0u64; 256];
    for &v in gray {
        hist[v as usize] += 1;
    }

    let target_lo = (total as f64 * cutoff_lo_pct / 100.0).floor() as u64;
    let target_hi = (total as f64 * cutoff_hi_pct / 100.0).floor() as u64;

    let mut lo = 0usize;
    let mut acc = 0u64;
    for (i, &c) in hist.iter().enumerate() {
        acc += c;
        if acc > target_lo {
            lo = i;
            break;
        }
    }

    let mut hi = 255usize;
    let mut acc = 0u64;
    for i in (0..256).rev() {
        acc += hist[i];
        if acc > target_hi {
            hi = i;
            break;
        }
    }

    if hi <= lo {
        return gray.to_vec();
    }

    let range = (hi - lo) as f64;
    gray.iter()
        .map(|&v| {
            let stretched = ((v as f64 - lo as f64) * 255.0 / range).round();
            stretched.clamp(0.0, 255.0) as u8
        })
        .collect()
}

pub fn contain_size(w: u32, h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    if w <= max_w && h <= max_h {
        return (w, h);
    }
    let ratio = (max_w as f64 / w as f64).min(max_h as f64 / h as f64);
    (
        ((w as f64 * ratio).floor() as u32).max(1),
        ((h as f64 * ratio).floor() as u32).max(1),
    )
}

pub fn contain_downscale(img: &RgbImage, max_w: u32, max_h: u32) -> RgbImage {
    if img.width() <= max_w && img.height() <= max_h {
        return img.clone();
    }
    let (new_w, new_h) = contain_size(img.width(), img.height(), max_w, max_h);
    image::imageops::resize(img, new_w, new_h, FilterType::Lanczos3)
}

pub fn integer_upscale(img: &RgbImage, factor: u32) -> RgbImage {
    if factor <= 1 {
        return img.clone();
    }
    image::imageops::resize(
        img,
        img.width() * factor,
        img.height() * factor,
        FilterType::Nearest,
    )
}

pub fn resize_output(img: &RgbImage, mode: OutScale) -> RgbImage {
    match mode {
        OutScale::Integer(factor) => integer_upscale(img, factor.max(1)),
        OutScale::Width(width) => {
            let width = width.max(1);
            if img.width() == width {
                return img.clone();
            }
            let factor = width as f64 / img.width() as f64;
            let new_h = ((img.height() as f64 * factor).round() as u32).max(1);
            image::imageops::resize(img, width, new_h, FilterType::Nearest)
        }
    }
}

pub fn process(
    source: &RgbImage,
    crop: Option<CropRect>,
    opts: ProcessOptions,
) -> RgbImage {
    let internal_scale = opts.internal_scale.max(1);
    let cropped: RgbImage = match crop {
        Some(c) => {
            let (x0, y0, x1, y1) = c.pixel_bounds(source.width(), source.height());
            image::imageops::crop_imm(source, x0, y0, x1 - x0, y1 - y0).to_image()
        }
        None => source.clone(),
    };

    let working = contain_downscale(
        &cropped,
        PHOTO_WIDTH * internal_scale,
        PHOTO_HEIGHT * internal_scale,
    );

    let mut luma = to_luma(working.as_raw());
    luma = autocontrast(&luma, 0.0, 10.0);

    let levels = bayer_dither_4tone(&luma, working.width() as usize, working.height() as usize);

    let palette_colors = PALETTES[opts.palette_index.min(PALETTES.len() - 1)].colors;
    let mut palette_img = RgbImage::new(working.width(), working.height());
    for (i, &level) in levels.iter().enumerate() {
        let x = (i % working.width() as usize) as u32;
        let y = (i / working.width() as usize) as u32;
        palette_img.put_pixel(x, y, Rgb(palette_colors[level.min(3) as usize]));
    }

    resize_output(&palette_img, opts.out_scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luma_matches_rec601() {
        assert_eq!(to_luma(&[255, 255, 255]), vec![255]);
        assert_eq!(to_luma(&[0, 0, 0]), vec![0]);
        assert_eq!(to_luma(&[255, 0, 0]), vec![76]);
        assert_eq!(to_luma(&[0, 255, 0]), vec![150]);
        assert_eq!(to_luma(&[0, 0, 255]), vec![29]);
    }

    #[test]
    fn autocontrast_no_cutoff_is_identity_when_full_range() {
        let gray: Vec<u8> = (0..=255).collect();
        assert_eq!(autocontrast(&gray, 0.0, 0.0), gray);
    }

    #[test]
    fn autocontrast_high_cutoff_only_clips_top() {
        let gray: Vec<u8> = (0..=255).collect();
        let out = autocontrast(&gray, 0.0, 10.0);
        assert_eq!(out[0], 0);
        assert_eq!(out[255], 255);
        assert_eq!(out[230], 255);
        assert_eq!(out[24], 27);
        assert!(out[100] > out[50]);
    }

    #[test]
    fn contain_downscale_never_upscales() {
        let img = RgbImage::new(100, 100);
        let out = contain_downscale(&img, 384, 336);
        assert_eq!(out.dimensions(), (100, 100));
    }

    #[test]
    fn contain_downscale_fits_within_bounds() {
        let img = RgbImage::new(1000, 800);
        let out = contain_downscale(&img, 384, 336);
        assert!(out.width() <= 384 && out.height() <= 336);
        let ratio = out.width() as f64 / out.height() as f64;
        assert!((ratio - 1.25).abs() < 0.05);
    }

    #[test]
    fn integer_upscale_scales_exactly() {
        let img = RgbImage::new(10, 5);
        let out = integer_upscale(&img, 3);
        assert_eq!(out.dimensions(), (30, 15));
        let out = integer_upscale(&img, 1);
        assert_eq!(out.dimensions(), (10, 5));
    }

    #[test]
    fn fixed_width_hits_exact_width_and_rounds_height() {
        let img = RgbImage::new(384, 336);
        let out = resize_output(&img, OutScale::Width(800));
        assert_eq!(out.dimensions(), (800, 700));

        let img = RgbImage::new(100, 75);
        let out = resize_output(&img, OutScale::Width(800));
        assert_eq!(out.dimensions(), (800, 600));

        let img = RgbImage::new(300, 200);
        let out = resize_output(&img, OutScale::Width(800));
        assert_eq!(out.width(), 800);
        assert_eq!(out.height(), 533);
    }

    #[test]
    fn fixed_width_can_downscale() {
        let img = RgbImage::new(1600, 900);
        let out = resize_output(&img, OutScale::Width(800));
        assert_eq!(out.dimensions(), (800, 450));
        let out = resize_output(&img, OutScale::Width(1600));
        assert_eq!(out.dimensions(), (1600, 900));
    }

    #[test]
    fn process_output_stays_in_palette() {
        let mut src = RgbImage::new(400, 300);
        for (x, y, p) in src.enumerate_pixels_mut() {
            let v = ((x + y) % 256) as u8;
            *p = Rgb([v, v, v]);
        }
        let out = process(
            &src,
            None,
            ProcessOptions {
                internal_scale: 3,
                out_scale: OutScale::Integer(2),
                palette_index: 0,
            },
        );
        assert_eq!(out.width(), 384 * 2);
        assert_eq!(out.height(), 288 * 2);
        for p in out.pixels() {
            assert!(
                crate::config::PALETTES[0].colors.contains(&p.0),
                "unexpected color {:?}",
                p.0
            );
        }
    }

    #[test]
    fn process_fixed_width_800_stays_in_palette() {
        let mut src = RgbImage::new(400, 300);
        for (x, y, p) in src.enumerate_pixels_mut() {
            let v = ((x + y) % 256) as u8;
            *p = Rgb([v, v, v]);
        }
        let out = process(
            &src,
            None,
            ProcessOptions {
                internal_scale: 3,
                out_scale: OutScale::Width(800),
                palette_index: 0,
            },
        );
        assert_eq!(out.width(), 800);
        for p in out.pixels() {
            assert!(
                crate::config::PALETTES[0].colors.contains(&p.0),
                "unexpected color {:?}",
                p.0
            );
        }
    }

    #[test]
    fn process_fixed_width_downscale_stays_in_palette() {
        let src = RgbImage::new(4000, 3000);
        let out = process(
            &src,
            None,
            ProcessOptions {
                internal_scale: 3,
                out_scale: OutScale::Width(200),
                palette_index: 0,
            },
        );
        assert_eq!(out.width(), 200);
        for p in out.pixels() {
            assert!(
                crate::config::PALETTES[0].colors.contains(&p.0),
                "unexpected color {:?}",
                p.0
            );
        }
    }
}
