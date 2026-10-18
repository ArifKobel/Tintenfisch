use crate::error::{Error, Result};
use crate::shading::Vertex;
use crate::syntax::{Dict, Object};

const PATCH_STEPS: usize = 12;

struct BitReader<'a> {
    data: &'a [u8],
    bit: usize,
}

impl BitReader<'_> {
    fn read(&mut self, bits: u32) -> Option<u64> {
        if self.bit + bits as usize > self.data.len() * 8 {
            return None;
        }
        let mut value = 0u64;
        for _ in 0..bits {
            let byte = self.data[self.bit / 8];
            value = value << 1 | ((byte >> (7 - self.bit % 8)) & 1) as u64;
            self.bit += 1;
        }
        Some(value)
    }

    fn align(&mut self) {
        self.bit = self.bit.div_ceil(8) * 8;
    }
}

pub struct MeshReader<'a> {
    reader: BitReader<'a>,
    bits_per_coordinate: u32,
    bits_per_component: u32,
    bits_per_flag: u32,
    vertices_per_row: usize,
    decode: Vec<(f64, f64)>,
    components: usize,
}

type Point = (f64, f64);

impl<'a> MeshReader<'a> {
    pub fn new(dict: &Dict, data: &'a [u8], components: usize) -> Result<Self> {
        let bits = |key: &str| dict.int(key).filter(|&bits| (1..=32).contains(&bits)).map(|bits| bits as u32);
        let decode: Vec<f64> = dict
            .get("Decode")
            .and_then(Object::as_array)
            .unwrap_or_default()
            .iter()
            .filter_map(Object::as_number)
            .collect();
        if decode.len() < 4 + 2 * components {
            return Err(Error::new("mesh shading with an incomplete /Decode"));
        }
        Ok(Self {
            reader: BitReader { data, bit: 0 },
            bits_per_coordinate: bits("BitsPerCoordinate")
                .ok_or_else(|| Error::new("mesh shading without /BitsPerCoordinate"))?,
            bits_per_component: bits("BitsPerComponent")
                .ok_or_else(|| Error::new("mesh shading without /BitsPerComponent"))?,
            bits_per_flag: bits("BitsPerFlag").unwrap_or(8),
            vertices_per_row: dict.int("VerticesPerRow").unwrap_or(2).max(2) as usize,
            decode: decode.chunks(2).map(|pair| (pair[0], pair[1])).collect(),
            components,
        })
    }

    fn value(&mut self, bits: u32, (low, high): (f64, f64)) -> Option<f64> {
        let raw = self.reader.read(bits)?;
        Some(low + raw as f64 * (high - low) / ((1u64 << bits) - 1) as f64)
    }

    fn point(&mut self) -> Option<Point> {
        Some((
            self.value(self.bits_per_coordinate, self.decode[0])?,
            self.value(self.bits_per_coordinate, self.decode[1])?,
        ))
    }

    fn color(&mut self) -> Option<Vec<f64>> {
        (0..self.components).map(|index| self.value(self.bits_per_component, self.decode[2 + index])).collect()
    }

    fn vertex(&mut self) -> Option<Vertex> {
        let (x, y) = self.point()?;
        Some(Vertex { x, y, color: self.color()? })
    }

    fn following_vertex(&mut self) -> Option<Vertex> {
        self.reader.read(self.bits_per_flag)?;
        let vertex = self.vertex()?;
        self.reader.align();
        Some(vertex)
    }

    fn read_points(&mut self, points: &mut [Point]) -> Option<()> {
        for point in points {
            *point = self.point()?;
        }
        Some(())
    }

    fn read_colors(&mut self, colors: &mut [Vec<f64>]) -> Option<()> {
        for color in colors {
            *color = self.color()?;
        }
        Some(())
    }

    pub fn triangles(mut self, kind: i64) -> Result<Vec<[Vertex; 3]>> {
        Ok(match kind {
            4 => self.free_form(),
            5 => self.lattice(),
            _ => self.patches(kind == 7),
        })
    }

    fn free_form(&mut self) -> Vec<[Vertex; 3]> {
        let mut triangles: Vec<[Vertex; 3]> = Vec::new();
        while let Some(flag) = self.reader.read(self.bits_per_flag) {
            let Some(vertex) = self.vertex() else { break };
            self.reader.align();
            match (flag, triangles.last()) {
                (1, Some([_, b, c])) => triangles.push([b.clone(), c.clone(), vertex]),
                (2, Some([a, _, c])) => triangles.push([a.clone(), c.clone(), vertex]),
                _ => {
                    let (Some(second), Some(third)) = (self.following_vertex(), self.following_vertex()) else {
                        break;
                    };
                    triangles.push([vertex, second, third]);
                }
            }
        }
        triangles
    }

    fn lattice(&mut self) -> Vec<[Vertex; 3]> {
        let mut rows: Vec<Vec<Vertex>> = Vec::new();
        'reading: loop {
            let mut row = Vec::with_capacity(self.vertices_per_row);
            for _ in 0..self.vertices_per_row {
                match self.vertex() {
                    Some(vertex) => row.push(vertex),
                    None => break 'reading,
                }
                self.reader.align();
            }
            rows.push(row);
        }
        let mut triangles = Vec::new();
        for pair in rows.windows(2) {
            for column in 0..self.vertices_per_row - 1 {
                let (a, b, c, d) = (&pair[0][column], &pair[0][column + 1], &pair[1][column], &pair[1][column + 1]);
                triangles.push([a.clone(), b.clone(), c.clone()]);
                triangles.push([b.clone(), d.clone(), c.clone()]);
            }
        }
        triangles
    }

    fn patches(&mut self, tensor: bool) -> Vec<[Vertex; 3]> {
        let mut triangles = Vec::new();
        let mut previous: Option<([Point; 12], [Vec<f64>; 4])> = None;
        while let Some(flag) = self.reader.read(self.bits_per_flag) {
            let reused = match (flag, &previous) {
                (1, Some((points, colors))) => {
                    Some(([points[3], points[4], points[5], points[6]], [colors[1].clone(), colors[2].clone()]))
                }
                (2, Some((points, colors))) => {
                    Some(([points[6], points[7], points[8], points[9]], [colors[2].clone(), colors[3].clone()]))
                }
                (3, Some((points, colors))) => {
                    Some(([points[9], points[10], points[11], points[0]], [colors[3].clone(), colors[0].clone()]))
                }
                _ => None,
            };
            let first_new = if reused.is_some() { 4 } else { 0 };
            let mut boundary = [(0.0, 0.0); 12];
            let mut colors: [Vec<f64>; 4] = Default::default();
            if let Some((edge, edge_colors)) = &reused {
                boundary[..4].copy_from_slice(edge);
                colors[0] = edge_colors[0].clone();
                colors[1] = edge_colors[1].clone();
            }
            let mut inner = [(0.0, 0.0); 4];
            let first_new_color = if reused.is_some() { 2 } else { 0 };
            let complete = self.read_points(&mut boundary[first_new..]).is_some()
                && (!tensor || self.read_points(&mut inner).is_some())
                && self.read_colors(&mut colors[first_new_color..]).is_some();
            if !complete {
                break;
            }
            let interior = tensor.then_some(inner);
            self.reader.align();
            let grid = control_grid(&boundary, interior);
            triangulate(&grid, &colors, &mut triangles);
            previous = Some((boundary, colors));
        }
        triangles
    }
}

fn control_grid(boundary: &[Point; 12], interior: Option<[Point; 4]>) -> [[Point; 4]; 4] {
    let mut grid = [[(0.0, 0.0); 4]; 4];
    let order = [(0, 0), (0, 1), (0, 2), (0, 3), (1, 3), (2, 3), (3, 3), (3, 2), (3, 1), (3, 0), (2, 0), (1, 0)];
    for (point, (u, v)) in boundary.iter().zip(order) {
        grid[u][v] = *point;
    }
    let combine = |terms: &[(f64, Point)]| {
        let (x, y) =
            terms.iter().fold((0.0, 0.0), |(x, y), (weight, point)| (x + weight * point.0, y + weight * point.1));
        (x / 9.0, y / 9.0)
    };
    match interior {
        Some(inner) => {
            grid[1][1] = inner[0];
            grid[1][2] = inner[1];
            grid[2][2] = inner[2];
            grid[2][1] = inner[3];
        }
        None => {
            let g = grid;
            grid[1][1] = combine(&[
                (-4.0, g[0][0]),
                (6.0, g[0][1]),
                (6.0, g[1][0]),
                (-2.0, g[0][3]),
                (-2.0, g[3][0]),
                (3.0, g[3][1]),
                (3.0, g[1][3]),
                (-1.0, g[3][3]),
            ]);
            grid[1][2] = combine(&[
                (-4.0, g[0][3]),
                (6.0, g[0][2]),
                (6.0, g[1][3]),
                (-2.0, g[0][0]),
                (-2.0, g[3][3]),
                (3.0, g[3][2]),
                (3.0, g[1][0]),
                (-1.0, g[3][0]),
            ]);
            grid[2][1] = combine(&[
                (-4.0, g[3][0]),
                (6.0, g[3][1]),
                (6.0, g[2][0]),
                (-2.0, g[3][3]),
                (-2.0, g[0][0]),
                (3.0, g[0][1]),
                (3.0, g[2][3]),
                (-1.0, g[0][3]),
            ]);
            grid[2][2] = combine(&[
                (-4.0, g[3][3]),
                (6.0, g[3][2]),
                (6.0, g[2][3]),
                (-2.0, g[3][0]),
                (-2.0, g[0][3]),
                (3.0, g[2][0]),
                (3.0, g[0][2]),
                (-1.0, g[0][0]),
            ]);
        }
    }
    grid
}

fn bernstein(t: f64) -> [f64; 4] {
    let s = 1.0 - t;
    [s * s * s, 3.0 * t * s * s, 3.0 * t * t * s, t * t * t]
}

fn surface(grid: &[[Point; 4]; 4], u: f64, v: f64) -> Point {
    let (bu, bv) = (bernstein(u), bernstein(v));
    let mut point = (0.0, 0.0);
    for i in 0..4 {
        for j in 0..4 {
            point.0 += bu[i] * bv[j] * grid[i][j].0;
            point.1 += bu[i] * bv[j] * grid[i][j].1;
        }
    }
    point
}

fn blend(colors: &[Vec<f64>; 4], u: f64, v: f64) -> Vec<f64> {
    (0..colors[0].len())
        .map(|index| {
            let bottom = colors[0][index] * (1.0 - u) + colors[3][index] * u;
            let top = colors[1][index] * (1.0 - u) + colors[2][index] * u;
            bottom * (1.0 - v) + top * v
        })
        .collect()
}

fn triangulate(grid: &[[Point; 4]; 4], colors: &[Vec<f64>; 4], into: &mut Vec<[Vertex; 3]>) {
    let vertex = |i: usize, j: usize| {
        let (u, v) = (i as f64 / PATCH_STEPS as f64, j as f64 / PATCH_STEPS as f64);
        let (x, y) = surface(grid, u, v);
        Vertex { x, y, color: blend(colors, u, v) }
    };
    for i in 0..PATCH_STEPS {
        for j in 0..PATCH_STEPS {
            let (a, b, c, d) = (vertex(i, j), vertex(i + 1, j), vertex(i, j + 1), vertex(i + 1, j + 1));
            into.push([a, b.clone(), c.clone()]);
            into.push([b, d, c]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::Parser;

    fn dict(source: &[u8]) -> Dict {
        Parser::new(source, 0).parse_object().unwrap().as_dict().unwrap().clone()
    }

    #[test]
    fn reads_free_form_triangles_with_shared_edges() {
        let dict = dict(b"<< /BitsPerCoordinate 8 /BitsPerComponent 8 /BitsPerFlag 8 /Decode [0 255 0 255 0 1] >>");
        let data = [0, 0, 0, 0, 0, 100, 0, 255, 0, 0, 100, 128, 1, 100, 100, 255];
        let triangles = MeshReader::new(&dict, &data, 1).unwrap().triangles(4).unwrap();
        assert_eq!(triangles.len(), 2);
        assert_eq!((triangles[0][1].x, triangles[0][1].color[0]), (100.0, 1.0));
        assert_eq!((triangles[1][2].x, triangles[1][2].y), (100.0, 100.0));
    }

    #[test]
    fn a_flat_coons_patch_spans_its_corners() {
        let boundary = [
            (0.0, 0.0),
            (0.0, 1.0),
            (0.0, 2.0),
            (0.0, 3.0),
            (1.0, 3.0),
            (2.0, 3.0),
            (3.0, 3.0),
            (3.0, 2.0),
            (3.0, 1.0),
            (3.0, 0.0),
            (2.0, 0.0),
            (1.0, 0.0),
        ];
        let grid = control_grid(&boundary, None);
        assert!((grid[1][1].0 - 1.0).abs() < 1e-9 && (grid[1][1].1 - 1.0).abs() < 1e-9);
        let center = surface(&grid, 0.5, 0.5);
        assert!((center.0 - 1.5).abs() < 1e-9 && (center.1 - 1.5).abs() < 1e-9);
        let colors = [vec![0.0], vec![1.0], vec![1.0], vec![0.0]];
        assert_eq!(blend(&colors, 0.25, 0.5), vec![0.5]);
    }
}
