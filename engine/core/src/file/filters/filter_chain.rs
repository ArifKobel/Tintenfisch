use crate::error::{Error, Result};
use crate::file::filters::predictor::Predictor;
use crate::file::filters::{ascii, flate, lzw, run_length};
use crate::syntax::{Dict, Name, Object};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageCodec {
    Dct,
    Jpx,
    CcittFax,
    Jbig2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Filter {
    Flate,
    Lzw,
    Ascii85,
    AsciiHex,
    RunLength,
    Crypt,
    Image(ImageCodec),
}

impl Filter {
    fn from_name(name: &Name) -> Result<Filter> {
        Ok(match name.as_bytes() {
            b"FlateDecode" | b"Fl" => Filter::Flate,
            b"LZWDecode" | b"LZW" => Filter::Lzw,
            b"ASCII85Decode" | b"A85" => Filter::Ascii85,
            b"ASCIIHexDecode" | b"AHx" => Filter::AsciiHex,
            b"RunLengthDecode" | b"RL" => Filter::RunLength,
            b"Crypt" => Filter::Crypt,
            b"DCTDecode" | b"DCT" => Filter::Image(ImageCodec::Dct),
            b"JPXDecode" => Filter::Image(ImageCodec::Jpx),
            b"CCITTFaxDecode" | b"CCF" => Filter::Image(ImageCodec::CcittFax),
            b"JBIG2Decode" => Filter::Image(ImageCodec::Jbig2),
            _ => return Err(Error::new(format!("unsupported filter {name:?}"))),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
struct FilterStep {
    filter: Filter,
    params: Dict,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImageEncoding {
    pub codec: ImageCodec,
    pub params: Dict,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Decoded {
    pub data: Vec<u8>,
    pub image: Option<ImageEncoding>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FilterChain {
    steps: Vec<FilterStep>,
}

impl FilterChain {
    pub fn of(dict: &Dict, resolve: impl Fn(&Object) -> Object) -> Result<FilterChain> {
        let entry = |long: &str, short: &str| dict.get(long).or_else(|| dict.get(short)).map(&resolve);
        let names = match entry("Filter", "F") {
            None | Some(Object::Null) => Vec::new(),
            Some(Object::Name(name)) => vec![name],
            Some(Object::Array(items)) => items
                .iter()
                .map(|item| match resolve(item) {
                    Object::Name(name) => Ok(name),
                    _ => Err(Error::new("filter entry is not a name")),
                })
                .collect::<Result<_>>()?,
            Some(_) => return Err(Error::new("filter is neither a name nor an array")),
        };
        let params = match entry("DecodeParms", "DP") {
            Some(Object::Array(items)) => items.iter().map(|item| params_of(&resolve(item))).collect(),
            Some(single) => vec![params_of(&single)],
            None => Vec::new(),
        };
        let steps = names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                Ok(FilterStep {
                    filter: Filter::from_name(name)?,
                    params: params.get(index).cloned().unwrap_or_default(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        if let Some(position) = steps.iter().position(|step| matches!(step.filter, Filter::Image(_)))
            && position + 1 != steps.len()
        {
            return Err(Error::new("an image codec must be the last filter"));
        }
        Ok(FilterChain { steps })
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn decode(&self, raw: &[u8]) -> Result<Decoded> {
        let mut data = raw.to_vec();
        for step in &self.steps {
            data = match step.filter {
                Filter::Flate => predicted(flate::inflate(&data)?, &step.params)?,
                Filter::Lzw => {
                    let early_change = step.params.int("EarlyChange").unwrap_or(1) != 0;
                    predicted(lzw::decode(&data, early_change)?, &step.params)?
                }
                Filter::Ascii85 => ascii::decode_85(&data)?,
                Filter::AsciiHex => ascii::decode_hex(&data)?,
                Filter::RunLength => run_length::decode(&data),
                Filter::Crypt => data,
                Filter::Image(codec) => {
                    return Ok(Decoded { data, image: Some(ImageEncoding { codec, params: step.params.clone() }) });
                }
            };
        }
        Ok(Decoded { data, image: None })
    }
}

fn params_of(object: &Object) -> Dict {
    object.as_dict().cloned().unwrap_or_default()
}

fn predicted(data: Vec<u8>, params: &Dict) -> Result<Vec<u8>> {
    Ok(match Predictor::from_params(params)? {
        Some(predictor) => predictor.reverse(&data),
        None => data,
    })
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use flate2::Compression;
    use flate2::write::ZlibEncoder;

    use super::*;
    use crate::syntax::Parser;

    fn dict(source: &[u8]) -> Dict {
        Parser::new(source, 0).parse_object().unwrap().as_dict().unwrap().clone()
    }

    fn chain(source: &[u8]) -> Result<FilterChain> {
        FilterChain::of(&dict(source), Object::clone)
    }

    #[test]
    fn decodes_a_chain_in_order() {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"chained").unwrap();
        let hex: String = encoder.finish().unwrap().iter().map(|byte| format!("{byte:02x}")).collect();
        let chain = chain(b"<< /Filter [/AHx /Fl] >>").unwrap();
        assert_eq!(chain.decode(hex.as_bytes()).unwrap().data, b"chained");
    }

    #[test]
    fn applies_decode_parms_per_filter() {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&[2, 1, 2, 2, 1, 1]).unwrap();
        let chain = chain(b"<< /Filter /FlateDecode /DecodeParms << /Predictor 12 /Columns 2 >> >>").unwrap();
        assert_eq!(chain.decode(&encoder.finish().unwrap()).unwrap().data, vec![1, 2, 2, 3]);
    }

    #[test]
    fn stops_before_an_image_codec_and_hands_over_its_params() {
        let chain = chain(b"<< /Filter [/AHx /DCTDecode] /DecodeParms [null << /ColorTransform 0 >>] >>").unwrap();
        let decoded = chain.decode(b"ffd8>").unwrap();
        assert_eq!(decoded.data, vec![0xff, 0xd8]);
        let image = decoded.image.unwrap();
        assert_eq!(image.codec, ImageCodec::Dct);
        assert_eq!(image.params.int("ColorTransform"), Some(0));
    }

    #[test]
    fn resolves_indirect_filter_entries() {
        let source = dict(b"<< /Filter 4 0 R >>");
        let chain = FilterChain::of(&source, |object| match object {
            Object::Ref(_) => Object::Name(Name::from("ASCIIHexDecode")),
            other => other.clone(),
        })
        .unwrap();
        assert_eq!(chain.decode(b"4142").unwrap().data, b"AB");
    }

    #[test]
    fn rejects_unknown_filters_and_codecs_in_the_middle() {
        assert!(chain(b"<< /Filter /Bogus >>").is_err());
        assert!(chain(b"<< /Filter [/DCTDecode /FlateDecode] >>").is_err());
        assert!(chain(b"<< >>").unwrap().is_empty());
    }
}
