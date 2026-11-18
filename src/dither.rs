use crate::config::{BAYER_4X4, PALETTE_LEVELS};

pub fn bayer_dither_4tone(gray: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut out = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            let value = gray[y * width + x] as u16;
            let bayer = BAYER_4X4[y % 4][x % 4] * 64 / 16;

            let level = if value > PALETTE_LEVELS[2] + bayer {
                3u8
            } else if value > PALETTE_LEVELS[1] + bayer {
                2u8
            } else if value > PALETTE_LEVELS[0] + bayer {
                1u8
            } else {
                0u8
            };
            out[y * width + x] = level;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_dark_gives_level_zero() {
        let gray = vec![10u8; 8 * 8];
        let out = bayer_dither_4tone(&gray, 8, 8);
        assert!(out.iter().all(|&l| l == 0));
    }

    #[test]
    fn uniform_bright_gives_level_three() {
        let gray = vec![255u8; 8 * 8];
        let out = bayer_dither_4tone(&gray, 8, 8);
        assert!(out.iter().all(|&l| l == 3));
    }

    #[test]
    fn mid_gray_halftones() {
        let gray = vec![160u8; 8 * 8];
        let out = bayer_dither_4tone(&gray, 8, 8);
        let zeros = out.iter().filter(|&&l| l == 0).count();
        let threes = out.iter().filter(|&&l| l == 3).count();
        let ones = out.iter().filter(|&&l| l == 1).count();
        let twos = out.iter().filter(|&&l| l == 2).count();
        assert!(ones > 0);
        assert!(twos > 0);
        assert_eq!(zeros + threes, 0);
        assert_eq!(ones + twos, 64);
    }

    #[test]
    fn first_pixel_threshold_is_64() {
        let mut gray = vec![0u8; 8 * 8];
        gray[0] = 65;
        let out = bayer_dither_4tone(&gray, 8, 8);
        assert_eq!(out[0], 1);
        gray[0] = 64;
        let out = bayer_dither_4tone(&gray, 8, 8);
        assert_eq!(out[0], 0);
    }
}
