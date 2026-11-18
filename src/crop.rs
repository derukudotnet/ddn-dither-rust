#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CropRect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    Nw,
    Ne,
    Sw,
    Se,
    N,
    S,
    E,
    W,
    Inside,
}

impl CropRect {
    pub fn normalized(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        CropRect {
            x0: x0.min(x1),
            y0: y0.min(y1),
            x1: x0.max(x1),
            y1: y0.max(y1),
        }
    }

    pub fn from_size(x: f32, y: f32, w: f32, h: f32) -> Self {
        CropRect::normalized(x, y, x + w, y + h)
    }

    pub fn width(&self) -> f32 {
        self.x1 - self.x0
    }

    pub fn height(&self) -> f32 {
        self.y1 - self.y0
    }

    pub fn center(&self) -> (f32, f32) {
        ((self.x0 + self.x1) / 2.0, (self.y0 + self.y1) / 2.0)
    }

    pub fn pixel_bounds(&self, img_w: u32, img_h: u32) -> (u32, u32, u32, u32) {
        let clamp_x = |v: f32| v.clamp(0.0, img_w as f32).round() as u32;
        let clamp_y = |v: f32| v.clamp(0.0, img_h as f32).round() as u32;
        (
            clamp_x(self.x0),
            clamp_y(self.y0),
            clamp_x(self.x1).max(clamp_x(self.x0) + 1),
            clamp_y(self.y1).max(clamp_y(self.y0) + 1),
        )
    }

    pub fn hit_test(&self, x: f32, y: f32, tol: f32) -> Option<Handle> {
        let (rx0, ry0, rx1, ry1) = (self.x0, self.y0, self.x1, self.y1);

        if (x - rx0).abs() <= tol && (y - ry0).abs() <= tol {
            return Some(Handle::Nw);
        }
        if (x - rx1).abs() <= tol && (y - ry0).abs() <= tol {
            return Some(Handle::Ne);
        }
        if (x - rx0).abs() <= tol && (y - ry1).abs() <= tol {
            return Some(Handle::Sw);
        }
        if (x - rx1).abs() <= tol && (y - ry1).abs() <= tol {
            return Some(Handle::Se);
        }

        if (x - rx0).abs() <= tol && ry0 + tol < y && y < ry1 - tol {
            return Some(Handle::W);
        }
        if (x - rx1).abs() <= tol && ry0 + tol < y && y < ry1 - tol {
            return Some(Handle::E);
        }
        if (y - ry0).abs() <= tol && rx0 + tol < x && x < rx1 - tol {
            return Some(Handle::N);
        }
        if (y - ry1).abs() <= tol && rx0 + tol < x && x < rx1 - tol {
            return Some(Handle::S);
        }

        if rx0 + tol < x && x < rx1 - tol && ry0 + tol < y && y < ry1 - tol {
            return Some(Handle::Inside);
        }

        None
    }

    pub fn rect_from_center(
        cx: f32,
        cy: f32,
        mx: f32,
        my: f32,
        aspect: Option<f64>,
        bounds: (f32, f32),
    ) -> Self {
        let mut dx = (mx - cx).abs();
        let mut dy = (my - cy).abs();

        if dx < 1.0 && dy < 1.0 {
            dx = 1.0;
            dy = 1.0;
        }

        let (mut hw, mut hh) = match aspect {
            Some(a) => {
                let a = a as f32;
                if dy == 0.0 || dx / dy > a {
                    let hw = dx;
                    (hw, hw / a)
                } else {
                    let hh = dy;
                    (hh * a, hh)
                }
            }
            None => (dx, dy),
        };

        let hw_max = (cx.min(bounds.0 - cx)).max(0.0);
        let hh_max = (cy.min(bounds.1 - cy)).max(0.0);

        if hw <= 0.0 || hh <= 0.0 {
            return CropRect::from_size(cx - 1.0, cy - 1.0, 2.0, 2.0);
        }

        let scale_limit = 1.0f32.min(hw_max / hw).min(hh_max / hh);
        hw = (hw * scale_limit).max(0.5);
        hh = (hh * scale_limit).max(0.5);

        CropRect::normalized(cx - hw, cy - hh, cx + hw, cy + hh)
            .clamped(bounds.0, bounds.1)
    }

    pub fn moved(&self, dx: f32, dy: f32, bounds: (f32, f32)) -> Self {
        let w = self.width();
        let h = self.height();
        let max_x0 = (bounds.0 - w).max(0.0);
        let max_y0 = (bounds.1 - h).max(0.0);
        let nx0 = (self.x0 + dx).max(0.0).min(max_x0);
        let ny0 = (self.y0 + dy).max(0.0).min(max_y0);
        CropRect::from_size(nx0, ny0, w, h)
    }

    pub fn clamped(&self, max_w: f32, max_h: f32) -> Self {
        CropRect {
            x0: self.x0.max(0.0).min(max_w),
            y0: self.y0.max(0.0).min(max_h),
            x1: self.x1.max(0.0).min(max_w),
            y1: self.y1.max(0.0).min(max_h),
        }
    }

    pub fn full_frame(bounds: (f32, f32), aspect: Option<f64>) -> Self {
        match aspect {
            None => CropRect::from_size(0.0, 0.0, bounds.0, bounds.1),
            Some(a) => {
                let cx = bounds.0 / 2.0;
                let cy = bounds.1 / 2.0;
                CropRect::rect_from_center(cx, cy, bounds.0, bounds.1, Some(a), bounds)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_from_center_respects_aspect() {
        let r = CropRect::rect_from_center(
            100.0,
            100.0,
            160.0,
            100.0,
            Some(2.0),
            (400.0, 300.0),
        );
        assert!((r.width() / r.height() - 2.0).abs() < 0.01);
    }

    #[test]
    fn rect_from_center_clamps_to_bounds() {
        let r = CropRect::rect_from_center(
            10.0,
            10.0,
            500.0,
            500.0,
            Some(1.0),
            (400.0, 300.0),
        );
        assert!(r.x0 >= 0.0 && r.x1 <= 400.0);
        assert!(r.y0 >= 0.0 && r.y1 <= 300.0);
        assert!((r.width() / r.height() - 1.0).abs() < 0.01);
    }

    #[test]
    fn move_clamps_within_bounds() {
        let r = CropRect::from_size(50.0, 50.0, 100.0, 50.0);
        let m = r.moved(1000.0, 1000.0, (200.0, 200.0));
        assert_eq!(m.x1, 200.0);
        assert_eq!(m.y1, 200.0);
    }

    #[test]
    fn hit_test_corners_and_inside() {
        let r = CropRect::from_size(10.0, 10.0, 100.0, 50.0);
        assert_eq!(r.hit_test(10.0, 10.0, 8.0), Some(Handle::Nw));
        assert_eq!(r.hit_test(110.0, 60.0, 8.0), Some(Handle::Se));
        assert_eq!(r.hit_test(60.0, 35.0, 8.0), Some(Handle::Inside));
        assert_eq!(r.hit_test(150.0, 150.0, 8.0), None);
    }

    #[test]
    fn full_frame_cover_shapes_max_aspect_rect() {
        let r = CropRect::full_frame((400.0, 300.0), Some(2.0));
        assert!((r.width() / r.height() - 2.0).abs() < 0.01);
        assert_eq!(r.x0, 0.0);
        assert_eq!(r.x1, 400.0);
    }
}
