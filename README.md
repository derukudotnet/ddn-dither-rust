# ddn-dither-rust

Rust rewrite of ddn-dither: convert photos to a Game Boy Camera–style look with
Bayer dithering. Single executable, GUI workflow, Linux.

## Features

- **GUI workflow**: open an image, drag a crop selection (aspect-locked,
  expands from the center like the original Tkinter tool), see a **live
  dithered preview**, tweak options, save.
- **Faithful pipeline** ported from the Python/original version:
  crop → fit within `128·s × 112·s` (Lanczos, downscale only) → Rec. 601
  luma → autocontrast with `cutoff (0, 10)` → 4×4 Bayer dithering at working
  resolution (thresholds 64/128/192) → 4-tone palette → nearest-neighbor
  upscale.
- **Integer upscaling**: output is always an integer nearest-neighbor scale of
  the dithered working image, so dither cells stay crisp and uniform.
- **Palettes**: grayscale (faithful to the original), DMG green, GB Pocket.
- **Output**: indexed PNG (4 colors) or lossless WebP.
- **Headless mode**: the same binary can also batch-process from the CLI.

## Build

```sh
cargo build --release
./target/release/ddn-dither
```

The release binary (`target/release/ddn-dither`, ~12 MB) is all you need to
distribute. At runtime it uses only standard desktop libraries (glibc, X11 or
Wayland client libraries, libxkbcommon); file dialogs go through
`xdg-desktop-portal`.

## Usage

Run without arguments for the GUI:

```sh
ddn-dither
```

Headless batch mode:

```sh
ddn-dither process photo.jpg -a 16:9 -s 3 -k 2 --palette gray -f png
```

| Option | Description |
|---|---|
| `-s, --internal-scale <N>` | Dither grid scale (default 3) |
| `-k, --out-scale <N>` | Integer output upscale factor (default: ~800 px width) |
| `-a, --aspect <W:H\|W/H\|F>` | Center-crop to an aspect ratio |
| `--palette <NAME>` | `Gray`, `DMG`, or `Pocket` |
| `-f, --format <png\|webp>` | Output format (default `png`) |
| `-o, --output <PATH>` | Output file (default `<input>_dithered.<ext>`) |

## Controls (GUI)

- **Drag** on empty area: create a selection (expands from the press point)
- **Drag inside** the selection: move it
- **Drag edges/corners**: resize (symmetrically around the center)
- **Esc**: reset selection to full frame
- Options panel: palette, dither scale (internal), output scale, aspect ratio
  (presets or custom `W:H`), output format

## Tests

```sh
cargo test
```

## Mapper overview

| Python module | Rust counterpart |
|---|---|
| `config.py` | `src/config.rs` |
| `dithering.py` | `src/dither.rs` |
| `image_processing.py` | `src/pipeline.rs` |
| `processor.py` | `src/pipeline.rs` (`process`) |
| `ui.py` | `src/app.rs`, `src/crop.rs` |
| `cli.py` | `src/cli.rs` |
