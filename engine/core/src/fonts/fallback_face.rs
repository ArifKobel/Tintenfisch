#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FallbackFamily {
    Sans,
    Serif,
    Mono,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FallbackFace {
    pub family: FallbackFamily,
    pub bold: bool,
    pub italic: bool,
}

const FIXED_PITCH: u32 = 1;
const SERIF: u32 = 1 << 1;
const ITALIC: u32 = 1 << 6;
const FORCE_BOLD: u32 = 1 << 18;

impl FallbackFace {
    pub const ALL: [FallbackFace; 12] = {
        let families = [FallbackFamily::Sans, FallbackFamily::Serif, FallbackFamily::Mono];
        let mut faces = [FallbackFace { family: FallbackFamily::Sans, bold: false, italic: false }; 12];
        let mut index = 0;
        while index < 12 {
            faces[index] = FallbackFace { family: families[index / 4], bold: index % 4 >= 2, italic: index % 2 == 1 };
            index += 1;
        }
        faces
    };

    pub fn for_font(base_font: &str, flags: u32) -> FallbackFace {
        let name = base_font.split_once('+').map_or(base_font, |(_, rest)| rest).to_ascii_lowercase();
        let has = |words: &[&str]| words.iter().any(|word| name.contains(word));
        let family = if has(&["courier", "mono", "consol", "menlo"]) || flags & FIXED_PITCH != 0 {
            FallbackFamily::Mono
        } else if has(&["times", "serif", "georgia", "garamond", "roman", "cambria", "minion", "book"])
            && !has(&["sans"])
        {
            FallbackFamily::Serif
        } else if has(&["arial", "helvetica", "sans", "verdana", "calibri", "segoe"]) {
            FallbackFamily::Sans
        } else if flags & SERIF != 0 {
            FallbackFamily::Serif
        } else {
            FallbackFamily::Sans
        };
        FallbackFace {
            family,
            bold: has(&["bold", "black", "heavy", "semibold", "demi"]) || flags & FORCE_BOLD != 0,
            italic: has(&["italic", "oblique"]) || flags & ITALIC != 0,
        }
    }

    pub fn file_name(&self) -> String {
        let family = match self.family {
            FallbackFamily::Sans => "Sans",
            FallbackFamily::Serif => "Serif",
            FallbackFamily::Mono => "Mono",
        };
        let style = match (self.bold, self.italic) {
            (false, false) => "Regular",
            (true, false) => "Bold",
            (false, true) => "Italic",
            (true, true) => "BoldItalic",
        };
        format!("Liberation{family}-{style}.ttf")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_standard_fonts_and_flags_to_faces() {
        let face = |name: &str, flags: u32| FallbackFace::for_font(name, flags).file_name();
        assert_eq!(face("Helvetica-BoldOblique", 0), "LiberationSans-BoldItalic.ttf");
        assert_eq!(face("Times-Roman", 0), "LiberationSerif-Regular.ttf");
        assert_eq!(face("ABCDEF+Courier", 0), "LiberationMono-Regular.ttf");
        assert_eq!(face("Unknown", SERIF | ITALIC), "LiberationSerif-Italic.ttf");
        assert_eq!(face("ArialMT", 0), "LiberationSans-Regular.ttf");
        assert_eq!(
            FallbackFace::ALL.iter().map(FallbackFace::file_name).collect::<std::collections::HashSet<_>>().len(),
            12
        );
    }
}
