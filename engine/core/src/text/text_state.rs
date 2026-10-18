use std::rc::Rc;

use crate::fonts::Font;

#[derive(Clone)]
pub struct TextState {
    pub font: Option<Rc<Font>>,
    pub size: f64,
    pub char_spacing: f64,
    pub word_spacing: f64,
    pub horizontal_scale: f64,
    pub leading: f64,
    pub rise: f64,
    pub render_mode: i64,
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            font: None,
            size: 0.0,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scale: 1.0,
            leading: 0.0,
            rise: 0.0,
            render_mode: 0,
        }
    }
}
