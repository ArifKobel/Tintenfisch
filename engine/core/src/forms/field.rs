use crate::geometry::Rect;
use crate::pages::PageId;
use crate::syntax::{Dict, Name, ObjId};

#[derive(Clone, Debug, PartialEq)]
pub enum FieldKind {
    Text { multiline: bool, password: bool, comb: Option<usize> },
    Checkbox,
    Radio,
    PushButton,
    Choice { combo: bool, editable: bool, multiple: bool, options: Vec<ChoiceOption> },
    Signature,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceOption {
    pub value: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue {
    Text(String),
    State(Option<Name>),
    Chosen(Vec<String>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Widget {
    pub id: ObjId,
    pub page: Option<PageId>,
    pub rect: Rect,
    pub on_state: Option<Name>,
    pub dict: Dict,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub id: ObjId,
    pub name: String,
    pub kind: FieldKind,
    pub value: FieldValue,
    pub read_only: bool,
    pub required: bool,
    pub default_appearance: Option<Vec<u8>>,
    pub quadding: i64,
    pub widgets: Vec<Widget>,
}

impl Field {
    pub fn shown_choice(&self, chosen: &[String]) -> String {
        let Some(value) = chosen.first() else { return String::new() };
        match &self.kind {
            FieldKind::Choice { options, .. } => options
                .iter()
                .find(|option| &option.value == value)
                .map_or(value.clone(), |option| option.label.clone()),
            _ => value.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FieldFlags(pub i64);

impl FieldFlags {
    pub const READ_ONLY: i64 = 1;
    pub const REQUIRED: i64 = 1 << 1;
    pub const MULTILINE: i64 = 1 << 12;
    pub const PASSWORD: i64 = 1 << 13;
    pub const NO_TOGGLE_TO_OFF: i64 = 1 << 14;
    pub const RADIO: i64 = 1 << 15;
    pub const PUSH_BUTTON: i64 = 1 << 16;
    pub const COMBO: i64 = 1 << 17;
    pub const EDITABLE: i64 = 1 << 18;
    pub const MULTI_SELECT: i64 = 1 << 21;
    pub const COMB: i64 = 1 << 24;

    pub fn read_only(self) -> bool {
        self.0 & Self::READ_ONLY != 0
    }

    pub fn required(self) -> bool {
        self.0 & Self::REQUIRED != 0
    }

    pub fn with(self, bit: i64, on: bool) -> FieldFlags {
        FieldFlags(if on { self.0 | bit } else { self.0 & !bit })
    }
}

impl FieldKind {
    pub fn of(
        field_type: Option<&Name>,
        FieldFlags(flags): FieldFlags,
        options: Vec<ChoiceOption>,
        max_length: Option<i64>,
    ) -> Option<FieldKind> {
        let field_type = field_type?;
        Some(if field_type.is("Tx") {
            FieldKind::Text {
                multiline: flags & FieldFlags::MULTILINE != 0,
                password: flags & FieldFlags::PASSWORD != 0,
                comb: max_length
                    .filter(|_| flags & FieldFlags::COMB != 0)
                    .and_then(|length| usize::try_from(length).ok()),
            }
        } else if field_type.is("Btn") {
            if flags & FieldFlags::PUSH_BUTTON != 0 {
                FieldKind::PushButton
            } else if flags & FieldFlags::RADIO != 0 {
                FieldKind::Radio
            } else {
                FieldKind::Checkbox
            }
        } else if field_type.is("Ch") {
            FieldKind::Choice {
                combo: flags & FieldFlags::COMBO != 0,
                editable: flags & FieldFlags::EDITABLE != 0,
                multiple: flags & FieldFlags::MULTI_SELECT != 0,
                options,
            }
        } else if field_type.is("Sig") {
            FieldKind::Signature
        } else {
            return None;
        })
    }

    pub fn is_button(&self) -> bool {
        matches!(self, FieldKind::Checkbox | FieldKind::Radio | FieldKind::PushButton)
    }
}
