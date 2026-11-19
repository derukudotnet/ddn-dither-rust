# ddn-dither-rust

Rust rewrite of ddn-dither: convert photos to a Game Boy Camera–style look with
Bayer dithering. Single executable, GUI workflow, Linux.

## Features

- **GUI workflow**: open an image, draw a crop selection (aspect-locked
  rubber-band with anchored resize), see a **live dithered preview**, tweak
  options, save.
- **Faithful pipeline** ported from the Python/original version:
  crop → fit within `128·s × 112·s` (Lanczos, downscale only) → Rec. 601
  luma → autocontrast with `cutoff (0, 10)` → 4×4 Bayer dithering at working
  resolution (thresholds 64/128/192) → 4-tone palette → nearest-neighbor
  upscale.
- **Scaling modes**: default output is a fixed 800 px width (nearest-neighbor,
  matching the Python version), or integer upscaling so dither cells stay
  crisp and uniform. Non-integer widths use nearest-neighbor as well, so the
  output never contains blended/diluted palette tones — cells may vary
  between e.g. 2 and 3 px, which is the accepted trade-off.
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
ddn-dither process photo.jpg -a 16:9 -s 3 -w 800 --palette gray -f png
```

| Option | Description |
|---|---|
| `-s, --internal-scale <N>` | Dither grid scale (default 3) |
| `-w, --width <PX>` | Fixed output width, non-integer resize (default 800) |
| `-k, --out-scale <N>` | Integer output upscale (mutually exclusive with `-w`) |
| `-a, --aspect <W:H\|W/H\|F>` | Center-crop to an aspect ratio |
| `--palette <NAME>` | `Gray`, `DMG`, or `Pocket` |
| `-f, --format <png\|webp>` | Output format (default `png`) |
| `-o, --output <PATH>` | Output file (default `<input>_dithered.<ext>`) |

## Controls (GUI)

- **Drag** on empty area: draw a new selection (rubber-band from the press point; aspect-locked when an aspect ratio is selected)
- **Drag inside** the selection: move it
- **Drag edges/corners**: resize (anchored at the opposite edge/corner; when an aspect ratio is selected, edge drags preserve it and corner drags stay locked to the ratio)
- **Esc**: reset selection to full frame
- Options panel: palette, dither scale (internal), output mode (fixed width in
  px, or integer scale ×N), aspect ratio (presets or custom `W:H`), output
  format. The label under the preview shows the final dimensions and the
  resulting scale factor.

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
