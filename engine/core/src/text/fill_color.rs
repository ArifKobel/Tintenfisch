use crate::syntax::Object;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FillColor {
    pub operations: Vec<(String, Vec<Object>)>,
}

impl FillColor {
    pub fn apply(&mut self, operator: &str, operands: &[Object]) {
        match operator {
            "g" | "rg" | "k" | "cs" => self.operations = vec![(operator.to_string(), operands.to_vec())],
            "sc" | "scn" => {
                self.operations.retain(|(operator, _)| operator == "cs");
                self.operations.push((operator.to_string(), operands.to_vec()));
            }
            _ => {}
        }
    }

    pub fn rgb(&self) -> Option<[f64; 3]> {
        let Some((operator, operands)) = self.operations.last() else { return Some([0.0; 3]) };
        let values: Vec<f64> = operands.iter().filter_map(Object::as_number).collect();
        match (operator.as_str(), &values[..]) {
            ("g", &[gray]) => Some([gray; 3]),
            ("rg", &[red, green, blue]) => Some([red, green, blue]),
            ("k", &[cyan, magenta, yellow, black]) => {
                Some([cyan, magenta, yellow].map(|ink| (1.0 - ink) * (1.0 - black)))
            }
            _ => None,
        }
    }
}
