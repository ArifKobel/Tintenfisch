use crate::content::Operation;
use crate::fonts::ShownGlyph;
use crate::geometry::Matrix;
use crate::syntax::Object;
use crate::text::TextState;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShowItem<'a> {
    Text(&'a [u8]),
    Shift(f64),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextMachine {
    text_matrix: Matrix,
    line_matrix: Matrix,
}

impl TextMachine {
    pub fn apply<'o>(&mut self, operation: &'o Operation, state: &mut TextState) -> Vec<ShowItem<'o>> {
        let operands = &operation.operands;
        let number = |index: usize| operands.get(index).and_then(Object::as_number);
        let text =
            |index: usize| operands.get(index).and_then(Object::as_string).map(ShowItem::Text).into_iter().collect();
        match operation.operator.as_str() {
            "BT" => *self = TextMachine::default(),
            "Tc" => state.char_spacing = number(0).unwrap_or(0.0),
            "Tw" => state.word_spacing = number(0).unwrap_or(0.0),
            "Tz" => state.horizontal_scale = number(0).unwrap_or(100.0) / 100.0,
            "TL" => state.leading = number(0).unwrap_or(0.0),
            "Ts" => state.rise = number(0).unwrap_or(0.0),
            "Tr" => state.render_mode = operands.first().and_then(Object::as_int).unwrap_or(0),
            "Tf" => state.size = number(1).unwrap_or(state.size),
            "Td" => self.move_line(number(0).unwrap_or(0.0), number(1).unwrap_or(0.0)),
            "TD" => {
                let ty = number(1).unwrap_or(0.0);
                state.leading = -ty;
                self.move_line(number(0).unwrap_or(0.0), ty);
            }
            "Tm" => {
                let numbers: Vec<f64> = operands.iter().filter_map(Object::as_number).collect();
                if let Some(matrix) = Matrix::from_numbers(&numbers) {
                    self.text_matrix = matrix;
                    self.line_matrix = matrix;
                }
            }
            "T*" => self.move_line(0.0, -state.leading),
            "Tj" => return text(0),
            "'" => {
                self.move_line(0.0, -state.leading);
                return text(0);
            }
            "\"" => {
                state.word_spacing = number(0).unwrap_or(state.word_spacing);
                state.char_spacing = number(1).unwrap_or(state.char_spacing);
                self.move_line(0.0, -state.leading);
                return text(2);
            }
            "TJ" => {
                return operands
                    .first()
                    .and_then(Object::as_array)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|item| match item {
                        Object::String(bytes) => Some(ShowItem::Text(bytes)),
                        other => other.as_number().map(ShowItem::Shift),
                    })
                    .collect();
            }
            _ => {}
        }
        Vec::new()
    }

    fn move_line(&mut self, tx: f64, ty: f64) {
        self.line_matrix = Matrix::translate(tx, ty).then(&self.line_matrix);
        self.text_matrix = self.line_matrix;
    }

    pub fn text_matrix(&self) -> Matrix {
        self.text_matrix
    }

    pub fn line_matrix(&self) -> Matrix {
        self.line_matrix
    }

    pub fn glyph_to_user(&self, state: &TextState) -> Matrix {
        Matrix::new(state.size * state.horizontal_scale, 0.0, 0.0, state.size, 0.0, state.rise).then(&self.text_matrix)
    }

    fn vertical(state: &TextState) -> bool {
        state.font.as_ref().is_some_and(|font| font.is_vertical())
    }

    pub fn shift(&mut self, thousandths: f64, state: &TextState) {
        let amount = -thousandths / 1000.0 * state.size;
        let shift = if Self::vertical(state) {
            Matrix::translate(0.0, amount)
        } else {
            Matrix::translate(amount * state.horizontal_scale, 0.0)
        };
        self.text_matrix = shift.then(&self.text_matrix);
    }

    pub fn advance(&mut self, glyph: &ShownGlyph, state: &TextState) {
        let spacing = state.char_spacing + if glyph.is_word_space { state.word_spacing } else { 0.0 };
        let step = if Self::vertical(state) {
            Matrix::translate(0.0, -(state.size + spacing))
        } else {
            Matrix::translate((glyph.width * state.size + spacing) * state.horizontal_scale, 0.0)
        };
        self.text_matrix = step.then(&self.text_matrix);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::ContentParser;

    fn run(source: &[u8]) -> (TextMachine, TextState, Vec<String>) {
        let mut machine = TextMachine::default();
        let mut state = TextState::default();
        let mut shown = Vec::new();
        for operation in ContentParser::new(source, 0).parse(0) {
            for item in machine.apply(&operation, &mut state) {
                shown.push(match item {
                    ShowItem::Text(bytes) => String::from_utf8_lossy(bytes).into_owned(),
                    ShowItem::Shift(amount) => format!("<{amount}>"),
                });
            }
        }
        (machine, state, shown)
    }

    #[test]
    fn moves_lines_and_keeps_leading() {
        let (machine, state, shown) = run(b"BT /F1 10 Tf 72 700 Td 0 -14 TD (a) Tj T* [(b) 250 (c)] TJ ET");
        assert_eq!(state.leading, 14.0);
        assert_eq!(machine.text_matrix().apply(0.0, 0.0), (72.0, 672.0));
        assert_eq!(shown, ["a", "b", "<250>", "c"]);
    }

    #[test]
    fn sets_spacing_with_the_double_quote_operator() {
        let (_, state, shown) = run(b"BT 12 TL 2 1 (x) \" ET");
        assert_eq!((state.word_spacing, state.char_spacing), (2.0, 1.0));
        assert_eq!(shown, ["x"]);
    }
}
