use std::fmt;
use std::str::FromStr;

use crate::syntax::ObjId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PageId(pub ObjId);

impl fmt::Display for PageId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}-{}", self.0.num, self.0.generation)
    }
}

impl FromStr for PageId {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (num, generation) = text.split_once('-').ok_or(())?;
        Ok(PageId(ObjId::new(num.parse().map_err(|_| ())?, generation.parse().map_err(|_| ())?)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_its_text_form() {
        let id = PageId(ObjId::new(12, 3));
        assert_eq!(id.to_string(), "12-3");
        assert_eq!("12-3".parse::<PageId>(), Ok(id));
        assert!("12".parse::<PageId>().is_err());
    }
}
