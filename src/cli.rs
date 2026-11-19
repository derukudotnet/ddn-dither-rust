use std::path::PathBuf;

use image::open;

use crate::config::PALETTES;
use crate::crop::CropRect;
use crate::pipeline::{contain_size, process, OutScale, ProcessOptions};
use crate::save::{default_output_path, save, OutputFormat};

pub fn parse_aspect(s: &str) -> Result<f64, String> {
    let s = s.trim();
    if let Some((w, h)) = s.split_once(':').or_else(|| s.split_once('/')) {
        let w: f64 = w.parse().map_err(|_| format!("invalid aspect: {s}"))?;
        let h: f64 = h.parse().map_err(|_| format!("invalid aspect: {s}"))?;
        if h == 0.0 {
            return Err(format!("aspect height must be nonzero: {s}"));
        }
        return Ok(w / h);
    }
    s.parse::<f64>()
        .map_err(|_| format!("aspect must be W:H, W/H, or a number, got: {s}"))
}

pub fn run(args: &[String]) -> Result<(), String> {
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        print_help();
        return Ok(());
    }

    let mut input: Option<String> = None;
    let mut internal_scale = crate::config::DEFAULT_INTERNAL_SCALE;
    let mut out_scale_arg: Option<u32> = None;
    let mut out_width_arg: Option<u32> = None;
    let mut aspect: Option<f64> = None;
    let mut palette_index = 0usize;
    let mut format = OutputFormat::Png;
    let mut output: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-s" | "--internal-scale" => {
                i += 1;
                internal_scale = args.get(i).map(|s| s.parse().ok()).flatten().unwrap_or(0);
                if internal_scale < 1 {
                    return Err("--internal-scale must be >= 1".into());
                }
            }
            "-k" | "--out-scale" => {
                i += 1;
                let v: u32 = args
                    .get(i)
                    .map(|s| s.parse().ok())
                    .flatten()
                    .unwrap_or(0);
                if v < 1 {
                    return Err("--out-scale must be >= 1".into());
                }
                out_scale_arg = Some(v);
            }
            "-w" | "--width" => {
                i += 1;
                let v: u32 = args
                    .get(i)
                    .map(|s| s.parse().ok())
                    .flatten()
                    .unwrap_or(0);
                if v < 1 {
                    return Err("--width must be >= 1".into());
                }
                out_width_arg = Some(v);
            }
            "-a" | "--aspect" => {
                i += 1;
                let v = args.get(i).ok_or("--aspect requires a value")?;
                aspect = Some(parse_aspect(v)?);
            }
            "--palette" => {
                i += 1;
                let v = args.get(i).ok_or("--palette requires a value")?;
                palette_index = PALETTES
                    .iter()
                    .position(|p| {
                        p.name.eq_ignore_ascii_case(v)
                            || v.eq_ignore_ascii_case(p.name.split(' ').next().unwrap_or(""))
                    })
                    .unwrap_or(0);
            }
            "-f" | "--format" => {
                i += 1;
                let v = args.get(i).ok_or("--format requires a value")?;
                format = OutputFormat::from_str(v).ok_or("format must be png or webp")?;
            }
            "-o" | "--output" => {
                i += 1;
                output = Some(args.get(i).ok_or("--output requires a value")?.clone());
            }
            other => {
                if input.is_none() {
                    input = Some(other.to_string());
                } else {
                    return Err(format!("unexpected argument: {other}"));
                }
            }
        }
        i += 1;
    }

    let input = input.ok_or("input image path required (see --help)")?;
    let input_path = PathBuf::from(&input);
    if !input_path.is_file() {
        return Err(format!("input file not found: {input}"));
    }

    let source = open(&input_path)
        .map_err(|e| format!("cannot open image: {e}"))?
        .to_rgb8();

    let crop = aspect
        .map(|a| CropRect::full_frame((source.width() as f32, source.height() as f32), Some(a)));

    let out_scale = match (out_scale_arg, out_width_arg) {
        (Some(_), Some(_)) => {
            return Err("--out-scale and --width are mutually exclusive".into());
        }
        (Some(k), None) => OutScale::Integer(k),
        (None, Some(w)) => OutScale::Width(w),
        (None, None) => OutScale::default(),
    };

    let opts = ProcessOptions {
        internal_scale,
        out_scale,
        palette_index,
    };
    let out_img = process(&source, crop, opts);

    let out_path = output
        .map(PathBuf::from)
        .unwrap_or_else(|| default_output_path(&input_path, format));

    save(&out_path, &out_img, &PALETTES[palette_index].colors, format)?;

    let (work_w, work_h) = working_size(&source, crop, internal_scale);
    eprintln!(
        "Saved {} (working {}x{}, output {}x{})",
        out_path.display(),
        work_w,
        work_h,
        out_img.width(),
        out_img.height()
    );
    Ok(())
}

fn working_size(source: &image::RgbImage, crop: Option<CropRect>, internal_scale: u32) -> (u32, u32) {
    let (cw, ch) = match crop {
        Some(c) => {
            let (x0, y0, x1, y1) = c.pixel_bounds(source.width(), source.height());
            (x1 - x0, y1 - y0)
        }
        None => (source.width(), source.height()),
    };
    contain_size(
        cw,
        ch,
        crate::config::PHOTO_WIDTH * internal_scale,
        crate::config::PHOTO_HEIGHT * internal_scale,
    )
}

fn print_help() {
    println!(
        "ddn-dither - Game Boy Camera-style dithering\n\
         \n\
         Usage:\n\
           ddn-dither                Open the interactive GUI.\n\
           ddn-dither process <IMG> [options]\n\
         \n\
         Options:\n\
           -s, --internal-scale <N>  Dither grid scale (default 3)\n\
           -k, --out-scale <N>       Integer output upscale
           -w, --width <PX>          Fixed output width, non-integer resize (default 800)
                                     -k and -w are mutually exclusive\n\
           -a, --aspect <W:H|W/H|F>  Center-crop to aspect ratio\n\
           --palette <NAME>          Gray | DMG | Pocket\n\
           -f, --format <png|webp>   Output format (default png)\n\
           -o, --output <PATH>       Output file (default: <input>_dithered.<ext>)"
    );
}
