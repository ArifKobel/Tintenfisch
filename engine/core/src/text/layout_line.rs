use crate::geometry::Rect;
use crate::text::GlyphOrigin;

#[derive(Clone, Debug, PartialEq)]
pub struct Cluster {
    pub text: String,
    pub left: f64,
    pub right: f64,
    pub origins: Vec<GlyphOrigin>,
}

impl Cluster {
    pub fn is_space(&self) -> bool {
        self.text.chars().all(char::is_whitespace)
    }

    pub fn is_word(&self) -> bool {
        self.text.chars().any(char::is_alphanumeric)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutLine {
    pub id: String,
    pub paragraph: usize,
    pub angle: f64,
    pub top: f64,
    pub bottom: f64,
    pub baseline: f64,
    pub size: f64,
    pub clusters: Vec<Cluster>,
}

impl LayoutLine {
    pub fn left(&self) -> f64 {
        self.clusters.first().map_or(0.0, |cluster| cluster.left)
    }

    pub fn right(&self) -> f64 {
        self.clusters.last().map_or(0.0, |cluster| cluster.right)
    }

    pub fn height(&self) -> f64 {
        (self.bottom - self.top).max(1.0)
    }

    pub fn bounds(&self) -> Rect {
        Rect::new(self.left(), self.top, self.right(), self.bottom)
    }

    pub fn caret_x(&self, offset: usize) -> f64 {
        match self.clusters.get(offset) {
            Some(cluster) => cluster.left,
            None => self.right(),
        }
    }

    pub fn nearest_offset(&self, x: f64) -> usize {
        (0..=self.clusters.len())
            .min_by(|&a, &b| (self.caret_x(a) - x).abs().total_cmp(&(self.caret_x(b) - x).abs()))
            .unwrap_or(0)
    }

    pub fn text(&self, from: usize, to: usize) -> String {
        self.clusters[from.min(self.clusters.len())..to.min(self.clusters.len())]
            .iter()
            .map(|cluster| cluster.text.as_str())
            .collect()
    }

    pub fn horizontal_distance(&self, x: f64) -> f64 {
        if x < self.left() {
            self.left() - x
        } else if x > self.right() {
            x - self.right()
        } else {
            0.0
        }
    }

    pub fn vertical_distance(&self, y: f64) -> f64 {
        if y < self.top {
            self.top - y
        } else if y > self.bottom {
            y - self.bottom
        } else {
            0.0
        }
    }
}
