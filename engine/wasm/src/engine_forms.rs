use serde::{Deserialize, Serialize};
use tintenfisch::document::FieldInput;
use tintenfisch::forms::{Field, FieldKind, FieldProperties, FieldValue, NewField, NewFieldKind};
use tintenfisch::geometry::Rect;
use tintenfisch::render::DisplaySpace;
use wasm_bindgen::prelude::*;

use crate::engine::{Engine, page_id};
use crate::object_ids::{object_id, object_text};
use crate::replies::{ChangeReply, RectReply, fail, reply};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FieldReply {
    id: String,
    name: String,
    partial_name: String,
    kind: &'static str,
    multiline: bool,
    password: bool,
    max_length: Option<usize>,
    editable: bool,
    read_only: bool,
    required: bool,
    text: String,
    on: Option<String>,
    chosen: Vec<String>,
    options: Vec<OptionReply>,
    widgets: Vec<WidgetReply>,
}

#[derive(Serialize)]
struct OptionReply {
    value: String,
    label: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WidgetReply {
    id: String,
    page_id: String,
    rect: RectReply,
    on_state: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum InputRequest {
    Text { text: String },
    Checked { checked: bool },
    Radio { widget: String },
    Choices { choices: Vec<String> },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewFieldRequest {
    kind: String,
    name: String,
    rect: RectRequest,
    options: Vec<String>,
}

#[derive(Deserialize)]
pub(crate) struct RectRequest {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl RectRequest {
    pub(crate) fn rect(&self) -> Rect {
        Rect::new(self.x, self.y, self.x + self.width, self.y + self.height)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PropertiesRequest {
    name: String,
    required: bool,
    read_only: bool,
    multiline: bool,
    options: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddedFieldReply {
    pub(crate) field_id: String,
    pub(crate) dirty_pages: Vec<String>,
}

#[wasm_bindgen]
impl Engine {
    pub fn fields(&self) -> Result<JsValue, JsError> {
        let document = self.document()?;
        let pages = document.view(&self.library).pages().map_err(fail)?;
        let replies: Vec<FieldReply> =
            document.fields().map_err(fail)?.into_iter().map(|field| field_reply(field, &pages)).collect();
        reply(&replies)
    }

    #[wasm_bindgen(js_name = setField)]
    pub fn set_field(&mut self, field: &str, input: JsValue) -> Result<JsValue, JsError> {
        let input: InputRequest =
            serde_wasm_bindgen::from_value(input).map_err(|error| JsError::new(&error.to_string()))?;
        let input = match input {
            InputRequest::Text { text } => FieldInput::Text(text),
            InputRequest::Checked { checked } => FieldInput::Checked(checked),
            InputRequest::Radio { widget } => FieldInput::Radio(object_id(&widget)?),
            InputRequest::Choices { choices } => FieldInput::Choices(choices),
        };
        let field = object_id(field)?;
        self.draft = None;
        let library = &self.library;
        let document = self.document.as_mut().ok_or_else(|| JsError::new("no document is open"))?;
        let pages = document.set_field(field, input, library).map_err(fail)?;
        reply(&ChangeReply { dirty_pages: pages.iter().map(ToString::to_string).collect(), ui_state: None })
    }

    #[wasm_bindgen(js_name = addField)]
    pub fn add_field(&mut self, page: &str, field: JsValue) -> Result<JsValue, JsError> {
        let request: NewFieldRequest =
            serde_wasm_bindgen::from_value(field).map_err(|error| JsError::new(&error.to_string()))?;
        let kind = match request.kind.as_str() {
            "text" => NewFieldKind::Text { multiline: false },
            "multiline" => NewFieldKind::Text { multiline: true },
            "checkbox" => NewFieldKind::Checkbox,
            "radio" => NewFieldKind::Radio,
            "combo" => NewFieldKind::Combo { options: request.options },
            "list" => NewFieldKind::List { options: request.options },
            other => return Err(JsError::new(&format!("{other} is not a kind of field"))),
        };
        let new = NewField { kind, name: request.name, rect: request.rect.rect() };
        self.draft = None;
        let library = &self.library;
        let document = self.document.as_mut().ok_or_else(|| JsError::new("no document is open"))?;
        let id = document.add_field(page_id(page)?, &new, library).map_err(fail)?;
        reply(&AddedFieldReply { field_id: object_text(id), dirty_pages: vec![page.to_string()] })
    }

    #[wasm_bindgen(js_name = setFieldProperties)]
    pub fn set_field_properties(&mut self, field: &str, properties: JsValue) -> Result<JsValue, JsError> {
        let request: PropertiesRequest =
            serde_wasm_bindgen::from_value(properties).map_err(|error| JsError::new(&error.to_string()))?;
        let properties = FieldProperties {
            name: request.name,
            required: request.required,
            read_only: request.read_only,
            multiline: request.multiline,
            options: request.options,
        };
        let field = object_id(field)?;
        let library = &self.library;
        let document = self.document.as_mut().ok_or_else(|| JsError::new("no document is open"))?;
        let pages = document.set_field_properties(field, &properties, library).map_err(fail)?;
        reply(&ChangeReply { dirty_pages: pages.iter().map(ToString::to_string).collect(), ui_state: None })
    }

    #[wasm_bindgen(js_name = moveWidget)]
    pub fn move_widget(
        &mut self,
        page: &str,
        widget: &str,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<JsValue, JsError> {
        let widget = object_id(widget)?;
        let library = &self.library;
        let document = self.document.as_mut().ok_or_else(|| JsError::new("no document is open"))?;
        document.move_widget(page_id(page)?, widget, Rect::new(x, y, x + width, y + height), library).map_err(fail)?;
        reply(&ChangeReply { dirty_pages: vec![page.to_string()], ui_state: None })
    }

    #[wasm_bindgen(js_name = deleteField)]
    pub fn delete_field(&mut self, field: &str) -> Result<JsValue, JsError> {
        let pages = self.document_mut()?.delete_field(object_id(field)?).map_err(fail)?;
        reply(&ChangeReply { dirty_pages: pages.iter().map(ToString::to_string).collect(), ui_state: None })
    }
}

fn field_reply(field: Field, pages: &[tintenfisch::pages::Page]) -> FieldReply {
    let (kind, multiline, password, max_length, editable) = match &field.kind {
        FieldKind::Text { multiline, password, comb } => ("text", *multiline, *password, *comb, false),
        FieldKind::Checkbox => ("checkbox", false, false, None, false),
        FieldKind::Radio => ("radio", false, false, None, false),
        FieldKind::PushButton => ("button", false, false, None, false),
        FieldKind::Choice { combo: true, editable, .. } => ("combo", false, false, None, *editable),
        FieldKind::Choice { combo: false, .. } => ("list", false, false, None, false),
        FieldKind::Signature => ("signature", false, false, None, false),
    };
    let options = match &field.kind {
        FieldKind::Choice { options, .. } => options
            .iter()
            .map(|option| OptionReply { value: option.value.clone(), label: option.label.clone() })
            .collect(),
        _ => Vec::new(),
    };
    let (text, on, chosen) = match &field.value {
        FieldValue::Text(text) => (text.clone(), None, Vec::new()),
        FieldValue::State(state) => (String::new(), state.as_ref().map(name_text), Vec::new()),
        FieldValue::Chosen(chosen) => (String::new(), None, chosen.clone()),
    };
    let widgets = field
        .widgets
        .iter()
        .filter_map(|widget| {
            let page = pages.iter().find(|page| Some(page.id) == widget.page)?;
            let rect = DisplaySpace::of(page).ok()?.rect_to_display(widget.rect);
            Some(WidgetReply {
                id: object_text(widget.id),
                page_id: page.id.to_string(),
                rect: RectReply::from(rect),
                on_state: widget.on_state.as_ref().map(name_text),
            })
        })
        .collect();
    FieldReply {
        id: object_text(field.id),
        partial_name: field.name.rsplit('.').next().unwrap_or_default().to_string(),
        name: field.name,
        kind,
        multiline,
        password,
        max_length,
        editable,
        read_only: field.read_only,
        required: field.required,
        text,
        on,
        chosen,
        options,
        widgets,
    }
}

fn name_text(name: &tintenfisch::syntax::Name) -> String {
    String::from_utf8_lossy(name.as_bytes()).into_owned()
}
