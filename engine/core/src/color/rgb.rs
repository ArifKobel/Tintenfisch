#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Rgb {
    pub const BLACK: Rgb = Rgb { r: 0.0, g: 0.0, b: 0.0 };

    pub fn new(r: f64, g: f64, b: f64) -> Self {
        Self { r: r.clamp(0.0, 1.0) as f32, g: g.clamp(0.0, 1.0) as f32, b: b.clamp(0.0, 1.0) as f32 }
    }

    pub fn gray(level: f64) -> Self {
        Self::new(level, level, level)
    }

    pub fn from_cmyk(c: f64, m: f64, y: f64, k: f64) -> Self {
        Self::new((1.0 - c) * (1.0 - k), (1.0 - m) * (1.0 - k), (1.0 - y) * (1.0 - k))
    }

    pub fn from_lab(l: f64, a: f64, b: f64, white: [f64; 3]) -> Self {
        let fy = (l + 16.0) / 116.0;
        let inverse =
            |t: f64| if t > 6.0 / 29.0 { t.powi(3) } else { 3.0 * (6.0f64 / 29.0).powi(2) * (t - 4.0 / 29.0) };
        let x = white[0] * inverse(fy + a / 500.0);
        let y = white[1] * inverse(fy);
        let z = white[2] * inverse(fy - b / 200.0);
        let linear = [
            3.2406 * x - 1.5372 * y - 0.4986 * z,
            -0.9689 * x + 1.8758 * y + 0.0415 * z,
            0.0557 * x - 0.2040 * y + 1.0570 * z,
        ];
        let gamma = |c: f64| if c <= 0.0031308 { 12.92 * c } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 };
        Self::new(gamma(linear[0]), gamma(linear[1]), gamma(linear[2]))
    }

    pub fn to_skia(self, alpha: f32) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba(self.r, self.g, self.b, alpha.clamp(0.0, 1.0)).unwrap_or(tiny_skia::Color::BLACK)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_cmyk_and_lab() {
        assert_eq!(Rgb::from_cmyk(0.0, 0.0, 0.0, 1.0), Rgb::BLACK);
        assert_eq!(Rgb::from_cmyk(1.0, 0.0, 0.0, 0.0), Rgb::new(0.0, 1.0, 1.0));
        let white = Rgb::from_lab(100.0, 0.0, 0.0, [0.9505, 1.0, 1.089]);
        assert!(white.r > 0.99 && white.g > 0.99 && white.b > 0.99);
    }
}
