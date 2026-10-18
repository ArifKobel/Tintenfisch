const PRINT: i32 = 1 << 2;
const MODIFY: i32 = 1 << 3;
const COPY: i32 = 1 << 4;
const ANNOTATE: i32 = 1 << 5;
const FILL_FORMS: i32 = 1 << 8;
const ACCESSIBILITY: i32 = 1 << 9;
const ASSEMBLE: i32 = 1 << 10;
const PRINT_FAITHFUL: i32 = 1 << 11;
const RESERVED: i32 = 0xFFFF_F0C0_u32 as i32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Permissions {
    pub print: bool,
    pub print_faithful: bool,
    pub modify: bool,
    pub copy: bool,
    pub annotate: bool,
    pub fill_forms: bool,
    pub assemble: bool,
}

impl Permissions {
    pub const ALL: Permissions = Permissions {
        print: true,
        print_faithful: true,
        modify: true,
        copy: true,
        annotate: true,
        fill_forms: true,
        assemble: true,
    };

    pub fn from_bits(bits: i32) -> Permissions {
        let granted = |flag: i32| bits & flag != 0;
        Permissions {
            print: granted(PRINT),
            print_faithful: granted(PRINT_FAITHFUL),
            modify: granted(MODIFY),
            copy: granted(COPY),
            annotate: granted(ANNOTATE),
            fill_forms: granted(FILL_FORMS),
            assemble: granted(ASSEMBLE),
        }
    }

    pub fn bits(&self) -> i32 {
        [
            (self.print, PRINT),
            (self.print_faithful, PRINT_FAITHFUL),
            (self.modify, MODIFY),
            (self.copy, COPY),
            (self.annotate, ANNOTATE),
            (self.fill_forms, FILL_FORMS),
            (self.assemble, ASSEMBLE),
        ]
        .into_iter()
        .filter(|(granted, _)| *granted)
        .fold(RESERVED | ACCESSIBILITY, |bits, (_, flag)| bits | flag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_the_bits_the_specification_reserves_and_reads_back_what_it_grants() {
        assert_eq!(Permissions::ALL.bits(), -4);
        let none = Permissions::from_bits(RESERVED);
        assert_eq!(none.bits(), 0xFFFF_F2C0_u32 as i32);
        let printing = Permissions { print: true, copy: true, ..none };
        assert_eq!(Permissions::from_bits(printing.bits()), printing);
    }
}
