use crate::geometry::Quad;
use crate::syntax::ObjId;

#[derive(Clone, Debug, PartialEq)]
pub struct PathDraw {
    pub scope: Option<ObjId>,
    pub ordinals: Vec<usize>,
    pub points: Vec<(f64, f64)>,
    pub reach: f64,
}

impl PathDraw {
    pub fn lies_within(&self, area: &Quad) -> bool {
        let reach = self.reach;
        !self.points.is_empty()
            && self.points.iter().all(|&(x, y)| {
                [(-reach, -reach), (reach, -reach), (reach, reach), (-reach, reach)]
                    .iter()
                    .all(|(dx, dy)| area.contains((x + dx, y + dy)))
            })
    }
}
