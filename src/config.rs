pub const PHOTO_WIDTH: u32 = 128;
pub const PHOTO_HEIGHT: u32 = 112;
pub const DEFAULT_INTERNAL_SCALE: u32 = 3;
pub const DEFAULT_OUTPUT_WIDTH: u32 = 800;
pub const EDGE_TOLERANCE_PX: f32 = 8.0;

pub const PALETTE_LEVELS: [u16; 3] = [64, 128, 192];

pub const BAYER_4X4: [[u16; 4]; 4] = [
    [0, 8, 2, 10],
    [12, 4, 14, 6],
    [3, 11, 1, 9],
    [15, 7, 13, 5],
];

pub struct Palette {
    pub name: &'static str,
    pub colors: [[u8; 3]; 4],
}

pub const PALETTES: [Palette; 3] = [
    Palette {
        name: "Grayscale (faithful)",
        colors: [
            [15, 15, 15],
            [74, 74, 74],
            [134, 134, 134],
            [193, 193, 193],
        ],
    },
    Palette {
        name: "DMG green",
        colors: [
            [8, 24, 32],
            [52, 104, 86],
            [134, 192, 108],
            [224, 248, 208],
        ],
    },
    Palette {
        name: "GB Pocket",
        colors: [
            [31, 31, 31],
            [77, 83, 60],
            [139, 149, 109],
            [197, 207, 161],
        ],
    },
];
