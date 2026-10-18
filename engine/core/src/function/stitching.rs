use crate::error::{Error, Result};
use crate::function::PdfFunction;
use crate::function::pdf_function::{Bounds, numbers};
use crate::store::ObjectStore;
use crate::syntax::{Dict, Object};

#[derive(Clone, Debug, PartialEq)]
pub struct StitchingFunction {
    pub bounds: Bounds,
    functions: Vec<PdfFunction>,
    limits: Vec<f64>,
    encode: Vec<f64>,
}

impl StitchingFunction {
    pub fn read(dict: &Dict, bounds: Bounds, store: &ObjectStore) -> Result<Self> {
        let parts = store.resolve(dict.get("Functions").unwrap_or(&Object::Null))?;
        let functions = parts
            .as_array()
            .ok_or_else(|| Error::new("stitching function without /Functions"))?
            .iter()
            .map(|part| PdfFunction::read(part, store))
            .collect::<Result<Vec<_>>>()?;
        let limits = numbers(dict.get("Bounds")).unwrap_or_default();
        let encode = numbers(dict.get("Encode")).unwrap_or_default();
        if functions.is_empty() || limits.len() + 1 != functions.len() || encode.len() != functions.len() * 2 {
            return Err(Error::new("stitching function with inconsistent /Bounds or /Encode"));
        }
        Ok(Self { bounds, functions, limits, encode })
    }

    pub fn evaluate(&self, input: f64) -> Vec<f64> {
        let (low, high) = self.bounds.domain[0];
        let index = self.limits.iter().position(|&limit| input < limit).unwrap_or(self.limits.len());
        let start = if index == 0 { low } else { self.limits[index - 1] };
        let end = if index == self.limits.len() { high } else { self.limits[index] };
        let (encode_low, encode_high) = (self.encode[index * 2], self.encode[index * 2 + 1]);
        let mapped = if end > start {
            encode_low + (input - start) * (encode_high - encode_low) / (end - start)
        } else {
            encode_low
        };
        self.functions[index].evaluate(&[mapped])
    }
}
