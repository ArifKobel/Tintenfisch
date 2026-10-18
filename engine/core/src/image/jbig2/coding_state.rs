use crate::image::jbig2::generic_region::GenericRegion;
use crate::image::jbig2::integer_decoder::{IntegerDecoder, SymbolIdDecoder};
use crate::image::jbig2::refinement_region::RefinementRegion;
use crate::image::mq_decoder::MqContext;

pub struct CodingState {
    pub height_delta: IntegerDecoder,
    pub width_delta: IntegerDecoder,
    pub export_run: IntegerDecoder,
    pub instance_count: IntegerDecoder,
    pub strip_delta: IntegerDecoder,
    pub first_s: IntegerDecoder,
    pub s_delta: IntegerDecoder,
    pub instance_t: IntegerDecoder,
    pub refined: IntegerDecoder,
    pub refinement_width: IntegerDecoder,
    pub refinement_height: IntegerDecoder,
    pub refinement_x: IntegerDecoder,
    pub refinement_y: IntegerDecoder,
    pub symbol_id: SymbolIdDecoder,
    pub generic: Vec<MqContext>,
    pub refinement: Vec<MqContext>,
}

impl CodingState {
    pub fn new(symbol_code_length: u32, generic_template: u8, refinement_template: u8) -> Self {
        Self {
            height_delta: IntegerDecoder::new(),
            width_delta: IntegerDecoder::new(),
            export_run: IntegerDecoder::new(),
            instance_count: IntegerDecoder::new(),
            strip_delta: IntegerDecoder::new(),
            first_s: IntegerDecoder::new(),
            s_delta: IntegerDecoder::new(),
            instance_t: IntegerDecoder::new(),
            refined: IntegerDecoder::new(),
            refinement_width: IntegerDecoder::new(),
            refinement_height: IntegerDecoder::new(),
            refinement_x: IntegerDecoder::new(),
            refinement_y: IntegerDecoder::new(),
            symbol_id: SymbolIdDecoder::new(symbol_code_length),
            generic: GenericRegion::contexts(generic_template),
            refinement: RefinementRegion::contexts(refinement_template),
        }
    }
}
