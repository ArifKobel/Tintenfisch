use crate::error::{Error, Result};
use crate::function::exponential::ExponentialFunction;
use crate::function::postscript::PostScriptFunction;
use crate::function::sampled::SampledFunction;
use crate::function::stitching::StitchingFunction;
use crate::store::ObjectStore;
use crate::syntax::{Dict, Object};

#[derive(Clone, Debug, PartialEq)]
pub enum PdfFunction {
    Sampled(SampledFunction),
    Exponential(ExponentialFunction),
    Stitching(StitchingFunction),
    PostScript(PostScriptFunction),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bounds {
    pub domain: Vec<(f64, f64)>,
    pub range: Option<Vec<(f64, f64)>>,
}

impl Bounds {
    fn read(dict: &Dict) -> Result<Bounds> {
        let domain = pairs(dict.get("Domain")).ok_or_else(|| Error::new("function without /Domain"))?;
        Ok(Bounds { domain, range: pairs(dict.get("Range")) })
    }

    fn clip_inputs(&self, inputs: &[f64]) -> Vec<f64> {
        inputs
            .iter()
            .enumerate()
            .map(|(index, &value)| match self.domain.get(index) {
                Some(&(low, high)) => value.clamp(low.min(high), high.max(low)),
                None => value,
            })
            .collect()
    }

    fn clip_outputs(&self, mut outputs: Vec<f64>) -> Vec<f64> {
        if let Some(range) = &self.range {
            for (value, &(low, high)) in outputs.iter_mut().zip(range) {
                *value = value.clamp(low.min(high), high.max(low));
            }
        }
        outputs
    }
}

pub fn pairs(object: Option<&Object>) -> Option<Vec<(f64, f64)>> {
    let numbers: Vec<f64> = object?.as_array()?.iter().filter_map(Object::as_number).collect();
    (numbers.len() >= 2 && numbers.len().is_multiple_of(2))
        .then(|| numbers.chunks(2).map(|pair| (pair[0], pair[1])).collect())
}

pub fn numbers(object: Option<&Object>) -> Option<Vec<f64>> {
    Some(object?.as_array()?.iter().filter_map(Object::as_number).collect())
}

impl PdfFunction {
    pub fn read(object: &Object, store: &ObjectStore) -> Result<PdfFunction> {
        let object = store.resolve(object)?;
        let dict = object.as_dict().ok_or_else(|| Error::new("function is not a dictionary or stream"))?;
        let bounds = Bounds::read(dict)?;
        match dict.int("FunctionType") {
            Some(0) => {
                let stream = object.as_stream().ok_or_else(|| Error::new("sampled function is not a stream"))?;
                let samples = store.decode_stream(stream)?.data;
                Ok(PdfFunction::Sampled(SampledFunction::read(dict, bounds, samples)?))
            }
            Some(2) => Ok(PdfFunction::Exponential(ExponentialFunction::read(dict, bounds)?)),
            Some(3) => Ok(PdfFunction::Stitching(StitchingFunction::read(dict, bounds, store)?)),
            Some(4) => {
                let stream = object.as_stream().ok_or_else(|| Error::new("PostScript function is not a stream"))?;
                let program = store.decode_stream(stream)?.data;
                Ok(PdfFunction::PostScript(PostScriptFunction::read(bounds, &program)?))
            }
            other => Err(Error::new(format!("unsupported function type {other:?}"))),
        }
    }

    fn bounds(&self) -> &Bounds {
        match self {
            PdfFunction::Sampled(function) => &function.bounds,
            PdfFunction::Exponential(function) => &function.bounds,
            PdfFunction::Stitching(function) => &function.bounds,
            PdfFunction::PostScript(function) => &function.bounds,
        }
    }

    pub fn evaluate(&self, inputs: &[f64]) -> Vec<f64> {
        let bounds = self.bounds();
        let inputs = bounds.clip_inputs(inputs);
        let outputs = match self {
            PdfFunction::Sampled(function) => function.evaluate(&inputs),
            PdfFunction::Exponential(function) => function.evaluate(inputs[0]),
            PdfFunction::Stitching(function) => function.evaluate(inputs[0]),
            PdfFunction::PostScript(function) => function.evaluate(&inputs),
        };
        bounds.clip_outputs(outputs)
    }
}
