use serde::{Deserialize, Serialize};
use tintenfisch::image::DecodedImage;
use tintenfisch::signing::{SignatureMark, SignatureStatus, SigningDetails, SigningIdentity};
use wasm_bindgen::prelude::*;

use crate::engine::{Engine, page_id};
use crate::engine_forms::{AddedFieldReply, RectRequest};
use crate::object_ids::{object_id, object_text};
use crate::replies::{fail, reply};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InkRequest {
    strokes: Vec<Vec<(f64, f64)>>,
    pen_width: f64,
    color: [f64; 3],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SignatureReply {
    field_id: String,
    field_name: String,
    signer: Option<String>,
    signed_at: String,
    reason: String,
    status: &'static str,
    problem: Option<String>,
    changed_after: bool,
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(js_name = signField)]
    pub fn sign_field(
        &mut self,
        field: &str,
        certificate: Vec<u8>,
        password: &str,
        reason: &str,
        location: &str,
        date: &str,
    ) -> Result<Vec<u8>, JsError> {
        let identity = SigningIdentity::from_pkcs12(&certificate, password).map_err(fail)?;
        let details =
            SigningDetails { reason: reason.to_string(), location: location.to_string(), date: date.to_string() };
        let field = object_id(field)?;
        self.draft = None;
        self.document_mut()?.sign_field(field, &identity, &details).map_err(fail)
    }

    pub fn signatures(&self) -> Result<JsValue, JsError> {
        let reports = self.document()?.signatures().map_err(fail)?;
        let replies: Vec<SignatureReply> = reports
            .into_iter()
            .map(|report| {
                let (status, problem) = match report.status {
                    SignatureStatus::Valid => ("valid", None),
                    SignatureStatus::Broken(problem) => ("broken", Some(problem)),
                    SignatureStatus::Unsupported(problem) => ("unsupported", Some(problem)),
                };
                SignatureReply {
                    field_id: object_text(report.field),
                    field_name: report.field_name,
                    signer: report.signer,
                    signed_at: report.signed_at,
                    reason: report.reason,
                    status,
                    problem,
                    changed_after: report.changed_after,
                }
            })
            .collect();
        reply(&replies)
    }

    #[wasm_bindgen(js_name = addInkSignature)]
    pub fn add_ink_signature(
        &mut self,
        page: &str,
        rect: JsValue,
        name: &str,
        ink: JsValue,
    ) -> Result<JsValue, JsError> {
        let ink: InkRequest = serde_wasm_bindgen::from_value(ink).map_err(|error| JsError::new(&error.to_string()))?;
        let mark = SignatureMark::Ink { strokes: ink.strokes, width: ink.pen_width, color: ink.color };
        self.add_signature(page, rect, name, &mark)
    }

    #[wasm_bindgen(js_name = addImageSignature)]
    pub fn add_image_signature(
        &mut self,
        page: &str,
        rect: JsValue,
        name: &str,
        width: usize,
        height: usize,
        rgba: Vec<u8>,
    ) -> Result<JsValue, JsError> {
        self.add_signature(page, rect, name, &SignatureMark::Image(DecodedImage { width, height, rgba }))
    }

    fn add_signature(
        &mut self,
        page: &str,
        rect: JsValue,
        name: &str,
        mark: &SignatureMark,
    ) -> Result<JsValue, JsError> {
        let rect: RectRequest =
            serde_wasm_bindgen::from_value(rect).map_err(|error| JsError::new(&error.to_string()))?;
        self.draft = None;
        let id = self.document_mut()?.add_signature(page_id(page)?, rect.rect(), name, mark).map_err(fail)?;
        reply(&AddedFieldReply { field_id: object_text(id), dirty_pages: vec![page.to_string()] })
    }
}
