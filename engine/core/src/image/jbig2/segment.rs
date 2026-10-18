use crate::error::{Error, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment<'d> {
    pub number: u32,
    pub kind: u8,
    pub referred: Vec<u32>,
    pub data: &'d [u8],
}

pub struct SegmentReader<'d> {
    data: &'d [u8],
    position: usize,
}

impl<'d> SegmentReader<'d> {
    pub fn new(data: &'d [u8]) -> Self {
        Self { data, position: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'d [u8]> {
        let bytes = self
            .data
            .get(self.position..self.position + count)
            .ok_or_else(|| Error::new("a JBIG2 segment is cut short"))?;
        self.position += count;
        Ok(bytes)
    }

    fn number(&mut self, size: usize) -> Result<u32> {
        Ok(self.take(size)?.iter().fold(0, |value, byte| value << 8 | *byte as u32))
    }

    fn segment(&mut self) -> Result<Segment<'d>> {
        let number = self.number(4)?;
        let flags = self.take(1)?[0];
        let first = self.take(1)?[0];
        let count = if first >> 5 == 7 {
            self.position -= 1;
            let count = self.number(4)? & 0x1fff_ffff;
            self.take((count as usize + 1).div_ceil(8))?;
            count
        } else {
            (first >> 5) as u32
        };
        let size = match number {
            0..=256 => 1,
            257..=65536 => 2,
            _ => 4,
        };
        let referred = (0..count).map(|_| self.number(size)).collect::<Result<Vec<_>>>()?;
        self.take(if flags & 0x40 != 0 { 4 } else { 1 })?;
        let length = self.number(4)?;
        if length == u32::MAX {
            return Err(Error::unsupported("JBIG2 segments of unknown length are not supported yet"));
        }
        let data = self.take(length as usize)?;
        Ok(Segment { number, kind: flags & 0x3f, referred, data })
    }
}

impl<'d> Iterator for SegmentReader<'d> {
    type Item = Result<Segment<'d>>;

    fn next(&mut self) -> Option<Result<Segment<'d>>> {
        (self.position < self.data.len()).then(|| self.segment())
    }
}
