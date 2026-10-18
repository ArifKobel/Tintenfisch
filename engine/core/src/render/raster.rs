use tiny_skia::Pixmap;

#[derive(Clone, Debug, PartialEq)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Raster {
    pub fn from_pixmap(pixmap: &Pixmap) -> Raster {
        let mut rgba = Vec::with_capacity(pixmap.data().len());
        for pixel in pixmap.pixels() {
            let color = pixel.demultiply();
            rgba.extend_from_slice(&[color.red(), color.green(), color.blue(), color.alpha()]);
        }
        Raster { width: pixmap.width(), height: pixmap.height(), rgba }
    }

    pub fn crop(self, x: u32, y: u32, width: u32, height: u32) -> Raster {
        if (x, y, width, height) == (0, 0, self.width, self.height) {
            return self;
        }
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for row in y..y + height {
            let start = ((row * self.width + x) * 4) as usize;
            rgba.extend_from_slice(&self.rgba[start..start + (width * 4) as usize]);
        }
        Raster { width, height, rgba }
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let start = ((y * self.width + x) * 4) as usize;
        [self.rgba[start], self.rgba[start + 1], self.rgba[start + 2], self.rgba[start + 3]]
    }
}
