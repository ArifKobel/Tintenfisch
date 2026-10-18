use crate::error::{Error, Result};
use crate::function::pdf_function::{Bounds, numbers};
use crate::syntax::Dict;

#[derive(Clone, Debug, PartialEq)]
pub struct ExponentialFunction {
    pub bounds: Bounds,
    c0: Vec<f64>,
    c1: Vec<f64>,
    exponent: f64,
}

impl ExponentialFunction {
    pub fn read(dict: &Dict, bounds: Bounds) -> Result<Self> {
        let c0 = numbers(dict.get("C0")).unwrap_or_else(|| vec![0.0]);
        let c1 = numbers(dict.get("C1")).unwrap_or_else(|| vec![1.0]);
        if c0.len() != c1.len() {
            return Err(Error::new("exponential function with mismatched C0 and C1"));
        }
        let exponent =
            dict.get("N").and_then(|n| n.as_number()).ok_or_else(|| Error::new("exponential function without /N"))?;
        Ok(Self { bounds, c0, c1, exponent })
    }

    pub fn evaluate(&self, input: f64) -> Vec<f64> {
        let factor = input.powf(self.exponent);
        self.c0.iter().zip(&self.c1).map(|(low, high)| low + factor * (high - low)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::Parser;

    #[test]
    fn blends_between_c0_and_c1_with_the_exponent() {
        let dict = Parser::new(b"<< /C0 [1 0] /C1 [0 1] /N 2 >>", 0).parse_object().unwrap().as_dict().unwrap().clone();
        let function = ExponentialFunction::read(&dict, Bounds { domain: vec![(0.0, 1.0)], range: None }).unwrap();
        assert_eq!(function.evaluate(0.5), vec![0.75, 0.25]);
    }
}
