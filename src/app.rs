use std::path::PathBuf;

use eframe::egui;
use egui::{Color32, CursorIcon, Pos2, Rect, Sense, Stroke, TextureOptions, TextureFilter, Vec2};
use image::RgbImage;

use crate::config::{DEFAULT_INTERNAL_SCALE, DEFAULT_OUTPUT_WIDTH, EDGE_TOLERANCE_PX, PALETTES};
use crate::crop::{CropRect, Handle};
use crate::pipeline::{contain_size, process, OutScale, ProcessOptions};
use crate::save::{default_output_path, save, OutputFormat};

struct LoadedImage {
    rgb: RgbImage,
    texture: egui::TextureHandle,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DragAction {
    Idle,
    Create { anchor: (f32, f32) },
    Move { grab_offset: (f32, f32) },
    Resize { handle: Handle },
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum OutMode {
    Integer,
    FixedWidth,
    FixedSize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PreviewKey {
    crop: Option<[u32; 4]>,
    internal_scale: u32,
    out_scale: OutScale,
    palette_index: usize,
}

pub struct DitherApp {
    image: Option<LoadedImage>,
    crop: Option<CropRect>,
    aspect: Option<f64>,
    aspect_preset: usize,
    aspect_custom: String,
    internal_scale: u32,
    out_mode: OutMode,
    out_integer: u32,
    out_width: u32,
    out_height: u32,
    palette_index: usize,
    format: OutputFormat,
    output_path: Option<PathBuf>,
    status: Option<String>,
    preview: Option<egui::TextureHandle>,
    preview_size: Vec2,
    preview_key: Option<PreviewKey>,
    preview_full: Option<RgbImage>,
    action: DragAction,
    last_img_rect: egui::Rect,
    last_scale: f32,
}

const ASPECT_PRESETS: [(&str, Option<f64>); 6] = [
    ("Free", None),
    ("1:1", Some(1.0)),
    ("4:3", Some(4.0 / 3.0)),
    ("16:9", Some(16.0 / 9.0)),
    ("10:9 (GB)", Some(10.0 / 9.0)),
    ("2:1", Some(2.0)),
];

impl Default for DitherApp {
    fn default() -> Self {
        DitherApp {
            image: None,
            crop: None,
            aspect: None,
            aspect_preset: 0,
            aspect_custom: String::new(),
            internal_scale: DEFAULT_INTERNAL_SCALE,
            out_mode: OutMode::FixedWidth,
            out_integer: 2,
            out_width: DEFAULT_OUTPUT_WIDTH,
            out_height: 600,
            palette_index: 0,
            format: OutputFormat::Png,
            output_path: None,
            status: None,
            preview: None,
            preview_size: Vec2::ZERO,
            preview_key: None,
            preview_full: None,
            action: DragAction::Idle,
            last_img_rect: egui::Rect::NOTHING,
            last_scale: 1.0,
        }
    }
}

impl DitherApp {
    fn pick_image_dialog() -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Image", &["png", "jpg", "jpeg", "webp", "gif", "bmp", "tiff"])
            .pick_file()
    }

    fn load_image(&mut self, ctx: &egui::Context, path: PathBuf) {
        match image::open(&path) {
            Ok(dynamic) => {
                let rgb = dynamic.to_rgb8();
                let texture = ctx.load_texture(
                    "source",
                    egui::ColorImage::from_rgb(
                        [rgb.width() as usize, rgb.height() as usize],
                        rgb.as_raw(),
                    ),
                    TextureOptions::default(),
                );
                let bounds = (rgb.width() as f32, rgb.height() as f32);
                self.crop = Some(CropRect::full_frame(bounds, self.aspect));
                self.output_path = Some(default_output_path(&path, self.format));
                self.status = None;
                self.preview_key = None;
                self.image = Some(LoadedImage { rgb, texture });
            }
            Err(e) => {
                self.status = Some(format!("Failed to open image: {e}"));
            }
        }
    }

    fn refresh_preview(&mut self, ctx: &egui::Context) {
        let Some(key) = self.key() else {
            return;
        };
        if self.preview_key.as_ref() == Some(&key) {
            return;
        }
        let Some(loaded) = &self.image else {
            return;
        };

        let opts = ProcessOptions {
            internal_scale: self.internal_scale,
            out_scale: self.out_scale(),
            palette_index: self.palette_index,
        };
        let out = process(&loaded.rgb, self.crop, opts);
        self.preview_size = Vec2::new(out.width() as f32, out.height() as f32);
        let texture = ctx.load_texture(
            "preview",
            egui::ColorImage::from_rgb(
                [out.width() as usize, out.height() as usize],
                out.as_raw(),
            ),
            TextureOptions {
                magnification: TextureFilter::Nearest,
                minification: TextureFilter::Nearest,
                ..Default::default()
            },
        );
        self.preview_full = Some(out);
        self.preview = Some(texture);
        self.preview_key = Some(key);
    }

    fn key(&self) -> Option<PreviewKey> {
        let img = self.image.as_ref()?;
        let crop = self
            .crop
            .map(|c| c.pixel_bounds(img.rgb.width(), img.rgb.height()));
        Some(PreviewKey {
            crop: crop.map(|(x0, y0, x1, y1)| [x0, y0, x1, y1]),
            internal_scale: self.internal_scale,
            out_scale: self.out_scale(),
            palette_index: self.palette_index,
        })
    }

    fn out_scale(&self) -> OutScale {
        match self.out_mode {
            OutMode::Integer => OutScale::Integer(self.out_integer),
            OutMode::FixedWidth => OutScale::Width(self.out_width),
            OutMode::FixedSize => OutScale::Size(self.out_width, self.out_height),
        }
    }

    fn options_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Options");
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Palette:");
            egui::ComboBox::from_id_salt("palette")
                .selected_text(PALETTES[self.palette_index].name)
                .show_ui(ui, |ui| {
                    for (i, p) in PALETTES.iter().enumerate() {
                        ui.selectable_value(&mut self.palette_index, i, p.name);
                    }
                });
        });

        ui.horizontal(|ui| {
            ui.label("Dither scale:");
            ui.add(egui::Slider::new(&mut self.internal_scale, 1..=8).text("internal"));
        });

        ui.horizontal(|ui| {
            ui.label("Output:");
            let mode_text = match self.out_mode {
                OutMode::Integer => "Scale \u{00d7}N",
                OutMode::FixedWidth => "Fixed width",
                OutMode::FixedSize => "Fixed W\u{00d7}H",
            };
            egui::ComboBox::from_id_salt("out_mode")
                .selected_text(mode_text)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.out_mode, OutMode::Integer, "Scale \u{00d7}N");
                    ui.selectable_value(&mut self.out_mode, OutMode::FixedWidth, "Fixed width");
                    ui.selectable_value(&mut self.out_mode, OutMode::FixedSize, "Fixed W\u{00d7}H");
                });
            match self.out_mode {
                OutMode::Integer => {
                    ui.add(egui::Slider::new(&mut self.out_integer, 1..=8).text("\u{00d7}"));
                }
                OutMode::FixedWidth | OutMode::FixedSize => {
                    ui.add(
                        egui::DragValue::new(&mut self.out_width)
                            .range(16..=4096)
                            .suffix(" px"),
                    );
                }
            }
            if self.out_mode == OutMode::FixedSize {
                ui.horizontal(|ui| {
                    ui.label("Height:");
                    ui.add(
                        egui::DragValue::new(&mut self.out_height)
                            .range(16..=4096)
                            .suffix(" px"),
                    );
                    ui.label(
                        egui::RichText::new(
                            "exact size \u{2014} aspect may distort if it differs from the selection",
                        )
                        .weak(),
                    );
                });
            }
        });

        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Aspect:");
            let mut preset = self.aspect_preset;
            egui::ComboBox::from_id_salt("aspect")
                .selected_text(ASPECT_PRESETS[preset].0)
                .show_ui(ui, |ui| {
                    for (i, (name, _)) in ASPECT_PRESETS.iter().enumerate() {
                        ui.selectable_value(&mut preset, i, *name);
                    }
                });
            if preset != self.aspect_preset {
                self.aspect_preset = preset;
                self.on_aspect_preset_changed(ui.ctx());
            }
            if self.aspect_preset == 0 {
                ui.add(
                    egui::TextEdit::singleline(&mut self.aspect_custom)
                        .hint_text("W:H")
                        .desired_width(56.0),
                );
                if ui.button("Apply").clicked() {
                    if let Ok(a) = crate::cli::parse_aspect(&self.aspect_custom) {
                        self.set_aspect(ui.ctx(), Some(a));
                    } else {
                        self.status = Some("Invalid aspect ratio".into());
                    }
                }
            }
        });

        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Format:");
            egui::ComboBox::from_id_salt("format")
                .selected_text(self.format.extension())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.format, OutputFormat::Png, "png");
                    ui.selectable_value(&mut self.format, OutputFormat::Webp, "webp");
                });
        });
        if ui.button("Save As\u{2026}").clicked() {
            self.save_dialog();
        }

        if let Some(s) = &self.status {
            ui.colored_label(Color32::from_rgb(255, 170, 80), s);
        }
    }

    fn set_aspect(&mut self, ctx: &egui::Context, aspect: Option<f64>) {
        self.aspect = aspect;
        if let Some(loaded) = &self.image {
            let bounds = (loaded.rgb.width() as f32, loaded.rgb.height() as f32);
            self.crop = Some(CropRect::full_frame(bounds, aspect));
            self.preview_key = None;
            ctx.request_repaint();
        }
    }

    fn on_aspect_preset_changed(&mut self, ctx: &egui::Context) {
        let aspect = ASPECT_PRESETS[self.aspect_preset].1;
        self.aspect_custom.clear();
        self.set_aspect(ctx, aspect);
    }

    fn save_dialog(&mut self) {
        if self.image.is_none() {
            return;
        }
        let mut dialog = rfd::FileDialog::new()
            .add_filter("Image", &["png", "webp"]);
        if let Some(p) = &self.output_path {
            dialog = dialog.set_file_name(
                p.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
            );
        }
        if let Some(path) = dialog.save_file() {
            self.output_path = Some(path);
            self.save_to_current();
        }
    }

    fn save_to_current(&mut self) {
        let (Some(preview), Some(path)) = (&self.preview_full, &self.output_path) else {
            self.status = Some("Nothing to save yet".into());
            return;
        };
        let palette = PALETTES[self.palette_index.min(PALETTES.len() - 1)].colors;
        match save(path, preview, &palette, self.format) {
            Ok(()) => {
                self.status = Some(format!("Saved to {}", path.display()));
            }
            Err(e) => self.status = Some(e),
        }
    }

    fn crop_view(&mut self, ui: &mut egui::Ui) {
        let (Some(loaded), avail) = (
            self.image.as_ref().map(|l| (&l.rgb, &l.texture)),
            ui.available_size(),
        ) else {
            return;
        };
        let img_size = Vec2::new(loaded.0.width() as f32, loaded.0.height() as f32);
        let scale = (avail.x / img_size.x).min(avail.y / img_size.y).min(1.0).max(0.001);
        let display_size = img_size * scale;

        let (outer, response) = ui.allocate_exact_size(avail, Sense::click_and_drag());
        let img_rect = Rect::from_center_size(outer.center(), display_size);
        self.last_img_rect = img_rect;
        self.last_scale = scale;

        ui.painter().image(
            loaded.1.id(),
            img_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );

        let source_px = |rect: Rect, p: Pos2| -> (f32, f32) {
            (
                (p.x - rect.min.x) / scale,
                (p.y - rect.min.y) / scale,
            )
        };
        let clamp_point = |x: f32, y: f32| -> (f32, f32) {
            (
                x.max(0.0).min(img_size.x),
                y.max(0.0).min(img_size.y),
            )
        };
        let tol = EDGE_TOLERANCE_PX / scale;

        let pointer = response.interact_pointer_pos();
        let press_origin = ui.ctx().input(|i| i.pointer.press_origin());
        let bounds = (img_size.x, img_size.y);
        let down_on_us = response.is_pointer_button_down_on();

        if down_on_us
            && (response.drag_started() || (response.dragged() && self.action == DragAction::Idle))
        {
            if let Some(press) = press_origin {
                let (sx, sy) = source_px(img_rect, press);
                let (sx, sy) = clamp_point(sx, sy);
                let handle = self.crop.and_then(|c| c.hit_test(sx, sy, tol));
                self.action = match handle {
                    None => DragAction::Create { anchor: (sx, sy) },
                    Some(Handle::Inside) => {
                        let c = self.crop.unwrap();
                        DragAction::Move {
                            grab_offset: (sx - c.x0, sy - c.y0),
                        }
                    }
                    Some(h) => DragAction::Resize { handle: h },
                };
            }
        }

        if response.dragged() && self.action != DragAction::Idle {
            if let Some(p) = pointer {
                let (sx, sy) = source_px(img_rect, p);
                let (sx, sy) = clamp_point(sx, sy);
                match self.action {
                    DragAction::Create { anchor } => {
                        self.crop = Some(CropRect::band(
                            anchor.0,
                            anchor.1,
                            sx,
                            sy,
                            self.aspect,
                            bounds,
                        ));
                    }
                    DragAction::Move { grab_offset } => {
                        if let Some(c) = self.crop {
                            self.crop =
                                Some(c.moved_to(sx - grab_offset.0, sy - grab_offset.1, bounds));
                        }
                    }
                    DragAction::Resize { handle } => {
                        if let Some(c) = self.crop {
                            self.crop = Some(c.drag_resize(handle, sx, sy, self.aspect, bounds));
                        }
                    }
                    DragAction::Idle => {}
                }
            }
        }

        if response.drag_stopped() {
            self.action = DragAction::Idle;
            if let Some(c) = self.crop {
                let w = (c.x1 - c.x0).round() as i64;
                let h = (c.y1 - c.y0).round() as i64;
                if w < 4 || h < 4 {
                    self.set_aspect(ui.ctx(), self.aspect);
                    self.status = Some("Selection too small — crop reset to full frame".into());
                }
            }
        }

        if !response.dragged() && self.action == DragAction::Idle {
            if let Some(p) = pointer {
                let (sx, sy) = source_px(img_rect, p);
                if sx >= 0.0 && sy >= 0.0 && sx <= img_size.x && sy <= img_size.y {
                    let cursor = match self.crop.and_then(|c| c.hit_test(sx, sy, tol)) {
                        Some(Handle::Nw) | Some(Handle::Se) => CursorIcon::ResizeNwSe,
                        Some(Handle::Ne) | Some(Handle::Sw) => CursorIcon::ResizeNeSw,
                        Some(Handle::N) | Some(Handle::S) => CursorIcon::ResizeNorth,
                        Some(Handle::E) | Some(Handle::W) => CursorIcon::ResizeEast,
                        Some(Handle::Inside) => CursorIcon::Move,
                        None => CursorIcon::Crosshair,
                    };
                    response.on_hover_cursor(cursor);
                }
            }
        }

        if let Some(c) = self.crop {
            let r = Rect::from_min_max(
                Pos2::new(img_rect.min.x + c.x0 * scale, img_rect.min.y + c.y0 * scale),
                Pos2::new(img_rect.min.x + c.x1 * scale, img_rect.min.y + c.y1 * scale),
            );
            let dim = Color32::from_black_alpha(110);
            let painter = ui.painter();
            painter.rect_filled(Rect::from_min_max(img_rect.min, Pos2::new(img_rect.max.x, r.min.y)), 0.0, dim);
            painter.rect_filled(Rect::from_min_max(Pos2::new(img_rect.min.x, r.max.y), img_rect.max), 0.0, dim);
            painter.rect_filled(Rect::from_min_max(Pos2::new(img_rect.min.x, r.min.y), Pos2::new(r.min.x, r.max.y)), 0.0, dim);
            painter.rect_filled(Rect::from_min_max(Pos2::new(r.max.x, r.min.y), Pos2::new(img_rect.max.x, r.max.y)), 0.0, dim);
            painter.rect_stroke(r, 0.0, Stroke::new(2.0f32, Color32::RED), egui::StrokeKind::Middle);
        }
    }
}

impl eframe::App for DitherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
    }
}

impl DitherApp {
    fn ui(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            let aspect = self.aspect;
            self.set_aspect(ctx, aspect);
        }

        egui::TopBottomPanel::top("topbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open Image…").clicked() {
                    if let Some(path) = Self::pick_image_dialog() {
                        self.load_image(ctx, path);
                    }
                }
                ui.separator();
                ui.label(
                    egui::RichText::new(
                        "Drag to draw a selection · drag inside to move · drag edges/corners to resize · Esc resets",
                    )
                    .weak(),
                );
            });
        });

        egui::SidePanel::right("controls")
            .default_width(380.0)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.heading("Preview");
                if let Some(tex) = &self.preview {
                    ui.add(
                        egui::Image::new(egui::load::SizedTexture::new(tex.id(), self.preview_size))
                            .max_width(ui.available_width()),
                    );
                    let (cw, ch) = match (self.crop, self.image.as_ref()) {
                        (Some(c), Some(loaded)) => {
                            let (x0, y0, x1, y1) =
                                c.pixel_bounds(loaded.rgb.width(), loaded.rgb.height());
                            (x1 - x0, y1 - y0)
                        }
                        (_, Some(loaded)) => (loaded.rgb.width(), loaded.rgb.height()),
                        _ => (0, 0),
                    };
                    let (work_w, _work_h) = contain_size(
                        cw,
                        ch,
                        128 * self.internal_scale.max(1),
                        112 * self.internal_scale.max(1),
                    );
                    let factor = self.preview_size.x / work_w as f32;
                    ui.label(format!(
                        "Output: {} × {} (×{:.2})",
                        self.preview_size.x as u32,
                        self.preview_size.y as u32,
                        factor
                    ));
                } else {
                    ui.label("No preview yet.");
                }
                ui.add_space(8.0);
                self.options_panel(ui);
                ui.add_space(8.0);
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(8.0))
            .show(ctx, |ui| {
            if self.image.is_none() {
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                    ui.add_space(80.0);
                    ui.heading("DDN Dither");
                    ui.add_space(12.0);
                    if ui
                        .add(egui::Button::new(egui::RichText::new("Open Image…").size(20.0)))
                        .clicked()
                    {
                        if let Some(path) = Self::pick_image_dialog() {
                            self.load_image(ctx, path);
                        }
                    }
                });
            } else {
                self.crop_view(ui);
            }
        });

        self.refresh_preview(ctx);
    }
}

pub fn run() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("DDN Dither"),
        ..Default::default()
    };
    eframe::run_native(
        "DDN Dither",
        options,
        Box::new(|_cc| Ok(Box::<DitherApp>::default())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Harness {
        ctx: egui::Context,
        app: DitherApp,
    }

    impl Harness {
        fn new(img_w: u32, img_h: u32) -> Self {
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(1.0);
            let mut img = RgbImage::new(img_w, img_h);
            for (x, y, p) in img.enumerate_pixels_mut() {
                *p = image::Rgb([(x % 256) as u8, (y % 256) as u8, 128]);
            }
            let dir = std::env::temp_dir().join("ddn_ui_test");
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.join(format!("img_{img_w}x{img_h}.png"));
            img.save(&path).unwrap();

            let mut app = DitherApp::default();
            app.load_image(&ctx, path);
            let mut h = Harness { ctx, app };
            h.frame(vec![]);
            h
        }

        fn frame(&mut self, events: Vec<egui::Event>) {
            let Harness { ctx, app } = self;
            let raw = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    Pos2::ZERO,
                    Vec2::new(1200.0, 800.0),
                )),
                events,
                ..Default::default()
            };
            let _ = ctx.run(raw, |ctx| app.ui(ctx));
        }

        fn crop_screen_rect(&self) -> egui::Rect {
            let c = self.app.crop.unwrap();
            let r = self.app.last_img_rect;
            let s = self.app.last_scale;
            egui::Rect::from_min_max(
                Pos2::new(r.min.x + c.x0 * s, r.min.y + c.y0 * s),
                Pos2::new(r.min.x + c.x1 * s, r.min.y + c.y1 * s),
            )
        }
    }

    fn press(p: Pos2) -> egui::Event {
        egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        }
    }

    fn release(p: Pos2) -> egui::Event {
        egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        }
    }

    #[test]
    fn corner_resize_tracks_cursor_one_to_one_downscaled_image() {
        let mut h = Harness::new(2000, 1500);
        assert!(h.app.last_scale < 1.0, "test image must be downscaled");

        h.app.crop = Some(CropRect::from_size(200.0, 200.0, 800.0, 700.0));
        let before = h.crop_screen_rect();
        let start = before.max - Vec2::new(1.0, 1.0);
        h.frame(vec![press(start)]);
        h.frame(vec![egui::Event::PointerMoved(start + Vec2::new(6.0, 4.0))]);
        h.frame(vec![egui::Event::PointerMoved(start + Vec2::new(12.0, 8.0))]);

        let after = h.crop_screen_rect();
        assert_eq!(after.min, before.min, "anchor must stay fixed");
        assert!(
            (after.max.x - (start.x + 12.0)).abs() < 1.0,
            "right edge at {:?}, cursor expected near {:?}",
            after.max.x,
            start.x + 12.0
        );
        assert!(
            (after.max.y - (start.y + 8.0)).abs() < 1.0,
            "bottom edge at {:?}, cursor expected near {:?}",
            after.max.y,
            start.y + 8.0
        );
        assert_eq!(after.min, before.min, "anchor must stay fixed");

        h.frame(vec![release(start + Vec2::new(12.0, 8.0))]);
    }

    #[test]
    fn move_drag_tracks_cursor_one_to_one() {
        let mut h = Harness::new(800, 600);
        h.app.crop = Some(CropRect::from_size(100.0, 100.0, 200.0, 150.0));
        let r0 = h.crop_screen_rect();
        let start = r0.center();
        h.frame(vec![press(start)]);
        h.frame(vec![egui::Event::PointerMoved(start + Vec2::new(6.0, 4.0))]);
        h.frame(vec![egui::Event::PointerMoved(start + Vec2::new(20.0, 10.0))]);

        let r1 = h.crop_screen_rect();
        assert!((r1.min.x - (r0.min.x + 20.0)).abs() < 1.0, "moved by {:?}", r1.min - r0.min);
        assert!((r1.min.y - (r0.min.y + 10.0)).abs() < 1.0);

        h.frame(vec![release(start + Vec2::new(20.0, 10.0))]);
    }

    #[test]
    fn create_rubber_band_tracks_cursor() {
        let mut h = Harness::new(800, 600);
        h.app.crop = None;
        let start = h.app.last_img_rect.min + Vec2::new(50.0, 50.0);
        h.frame(vec![press(start)]);
        h.frame(vec![egui::Event::PointerMoved(start + Vec2::new(6.0, 6.0))]);
        h.frame(vec![egui::Event::PointerMoved(start + Vec2::new(100.0, 80.0))]);

        let r = h.crop_screen_rect();
        assert!((r.min.x - start.x).abs() < 1.0);
        assert!((r.max.x - (start.x + 100.0)).abs() < 1.0);
        assert!((r.max.y - (start.y + 80.0)).abs() < 1.0);

        h.frame(vec![release(start + Vec2::new(100.0, 80.0))]);
    }
}
