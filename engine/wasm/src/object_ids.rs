use tintenfisch::syntax::ObjId;
use wasm_bindgen::JsError;

pub fn object_text(id: ObjId) -> String {
    format!("{}-{}", id.num, id.generation)
}

pub fn object_id(text: &str) -> Result<ObjId, JsError> {
    let parsed = text
        .split_once('-')
        .and_then(|(num, generation)| Some(ObjId::new(num.parse().ok()?, generation.parse().ok()?)));
    parsed.ok_or_else(|| JsError::new(&format!("{text} is not an object id")))
}
