const TOLERANCE: f64 = 0.01;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Piece {
    Glyph(f64),
    Space(f64),
    Break,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineBreak {
    pub start: usize,
    pub end: usize,
    pub next: usize,
}

impl LineBreak {
    pub fn ended_by_break(&self) -> bool {
        self.next > self.end
    }
}

pub fn break_line(pieces: &[Piece], start: usize, available: f64) -> LineBreak {
    let mut width = 0.0;
    let mut after_space = None;
    for (index, piece) in pieces.iter().enumerate().skip(start) {
        match *piece {
            Piece::Break => return LineBreak { start, end: index, next: index + 1 },
            Piece::Space(space) => {
                width += space;
                after_space = Some(index + 1);
            }
            Piece::Glyph(glyph) => {
                if width + glyph > available + TOLERANCE && index > start {
                    let end = after_space.unwrap_or(index);
                    return LineBreak { start, end, next: end };
                }
                width += glyph;
            }
        }
    }
    LineBreak { start, end: pieces.len(), next: pieces.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pieces(text: &str) -> Vec<Piece> {
        text.chars()
            .map(|character| match character {
                '\n' => Piece::Break,
                ' ' => Piece::Space(1.0),
                _ => Piece::Glyph(1.0),
            })
            .collect()
    }

    fn lines(text: &str, available: f64) -> Vec<String> {
        let pieces = pieces(text);
        let characters: Vec<char> = text.chars().collect();
        let mut out = Vec::new();
        let mut start = 0;
        loop {
            let line = break_line(&pieces, start, available);
            out.push(characters[line.start..line.end].iter().collect());
            if line.next >= pieces.len() {
                if line.ended_by_break() {
                    out.push(String::new());
                }
                return out;
            }
            start = line.next;
        }
    }

    #[test]
    fn whole_words_wrap_and_keep_their_trailing_space() {
        assert_eq!(lines("one two three", 8.0), ["one two ", "three"]);
        assert_eq!(lines("one two three", 7.0), ["one two ", "three"]);
        assert_eq!(lines("one two three", 6.0), ["one ", "two ", "three"]);
    }

    #[test]
    fn hard_breaks_end_lines_and_may_leave_them_empty() {
        assert_eq!(lines("a\n\nb", 9.0), ["a", "", "b"]);
        assert_eq!(lines("ab\n", 9.0), ["ab", ""]);
    }

    #[test]
    fn a_word_longer_than_the_line_breaks_between_clusters() {
        assert_eq!(lines("abcdefg", 3.0), ["abc", "def", "g"]);
    }

    #[test]
    fn a_line_that_fits_exactly_stays_whole() {
        assert_eq!(lines("abc def", 7.0), ["abc def"]);
    }
}
