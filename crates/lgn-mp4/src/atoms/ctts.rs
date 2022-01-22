use std::io::{Read, Seek, Write};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use serde::Serialize;

use super::{
    box_start, read_atom_header_ext, skip_bytes_to, write_atom_header_ext, Atom, AtomHeader,
    ReadAtom, WriteAtom, HEADER_EXT_SIZE, HEADER_SIZE,
};
use crate::{FourCC, Result};

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct CttsAtom {
    pub version: u8,
    pub flags: u32,

    #[serde(skip_serializing)]
    pub entries: Vec<CttsEntry>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct CttsEntry {
    pub sample_count: u32,
    pub sample_offset: i32,
}

impl Atom for CttsAtom {
    const FOUR_CC: FourCC = FourCC::new(b"ctts");

    fn size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 4 + (8 * self.entries.len() as u64)
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("entries_count={}", self.entries.len());
        Ok(s)
    }
}

impl<R: Read + Seek> ReadAtom<&mut R> for CttsAtom {
    fn read_atom(reader: &mut R, size: u64) -> Result<Self> {
        let start = box_start(reader)?;

        let (version, flags) = read_atom_header_ext(reader)?;

        let entry_count = reader.read_u32::<BigEndian>()?;
        let mut entries = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            let entry = CttsEntry {
                sample_count: reader.read_u32::<BigEndian>()?,
                sample_offset: reader.read_i32::<BigEndian>()?,
            };
            entries.push(entry);
        }

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            entries,
        })
    }
}

impl<W: Write> WriteAtom<&mut W> for CttsAtom {
    fn write_atom(&self, writer: &mut W) -> Result<u64> {
        AtomHeader::new(self).write(writer)?;

        write_atom_header_ext(writer, self.version, self.flags)?;

        writer.write_u32::<BigEndian>(self.entries.len() as u32)?;
        for entry in &self.entries {
            writer.write_u32::<BigEndian>(entry.sample_count)?;
            writer.write_i32::<BigEndian>(entry.sample_offset)?;
        }

        Ok(self.size())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::atoms::AtomHeader;

    #[test]
    fn test_ctts() {
        let src_box = CttsAtom {
            version: 0,
            flags: 0,
            entries: vec![
                CttsEntry {
                    sample_count: 1,
                    sample_offset: 200,
                },
                CttsEntry {
                    sample_count: 2,
                    sample_offset: -100,
                },
            ],
        };
        let mut buf = Vec::new();
        src_box.write_atom(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.size() as usize);

        let mut reader = Cursor::new(&buf);
        let header = AtomHeader::read(&mut reader).unwrap();
        assert_eq!(header.name, CttsAtom::FOUR_CC);
        assert_eq!(src_box.size(), header.size);

        let dst_box = CttsAtom::read_atom(&mut reader, header.size).unwrap();
        assert_eq!(src_box, dst_box);
    }
}