use std::io::{Read, Seek, Write};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use serde::Serialize;

use super::{
    box_start, read_atom_header_ext, skip_bytes_to, write_atom_header_ext, Atom, AtomHeader,
    ReadAtom, WriteAtom, HEADER_EXT_SIZE, HEADER_SIZE,
};
use crate::{FourCC, Result};

/// Sample Offset Atom
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct StcoAtom {
    pub version: u8,
    pub flags: u32,

    #[serde(skip_serializing)]
    pub entries: Vec<u32>,
}

impl Atom for StcoAtom {
    const FOUR_CC: FourCC = FourCC::new(b"stco");

    fn size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 4 + (4 * self.entries.len() as u64)
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("entries={}", self.entries.len());
        Ok(s)
    }
}

impl<R: Read + Seek> ReadAtom<&mut R> for StcoAtom {
    fn read_atom(reader: &mut R, size: u64) -> Result<Self> {
        let start = box_start(reader)?;

        let (version, flags) = read_atom_header_ext(reader)?;

        let entry_count = reader.read_u32::<BigEndian>()?;
        let mut entries = Vec::with_capacity(entry_count as usize);
        for _i in 0..entry_count {
            let chunk_offset = reader.read_u32::<BigEndian>()?;
            entries.push(chunk_offset);
        }

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            entries,
        })
    }
}

impl<W: Write> WriteAtom<&mut W> for StcoAtom {
    fn write_atom(&self, writer: &mut W) -> Result<u64> {
        AtomHeader::new(self).write(writer)?;

        write_atom_header_ext(writer, self.version, self.flags)?;

        writer.write_u32::<BigEndian>(self.entries.len() as u32)?;
        for chunk_offset in &self.entries {
            writer.write_u32::<BigEndian>(*chunk_offset)?;
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
    fn test_stco() {
        let src_box = StcoAtom {
            version: 0,
            flags: 0,
            entries: vec![267, 1970, 2535, 2803, 11843, 22223, 33584],
        };
        let mut buf = Vec::new();
        src_box.write_atom(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.size() as usize);

        let mut reader = Cursor::new(&buf);
        let header = AtomHeader::read(&mut reader).unwrap();
        assert_eq!(header.name, StcoAtom::FOUR_CC);
        assert_eq!(src_box.size(), header.size);

        let dst_box = StcoAtom::read_atom(&mut reader, header.size).unwrap();
        assert_eq!(src_box, dst_box);
    }
}