use crate::error::Converter;
use crate::error::WRONG_OFFSET;
use crate::paged_reader::PagedReader;
use crate::Error;
use crate::Result;
use std::io::Read;

#[derive(Debug)]
pub struct CompressedVectorSectionHeader {
	section_id:         u8,
	pub section_length: u64,
	pub data_offset:    u64,
	pub index_offset:   u64,
}

impl CompressedVectorSectionHeader {
	pub const SIZE: usize = 32;

	pub fn read(reader: &mut PagedReader) -> Result<CompressedVectorSectionHeader> {
		let mut buffer = [0_u8; Self::SIZE as usize];
		reader
			.read_exact(&mut buffer)
			.read_err("Failed to read compressed vector section header")?;

		let header = Self {
			section_id:     buffer[0],
			section_length: u64::from_le_bytes(buffer[8..16].try_into().internal_err(WRONG_OFFSET)?),
			data_offset:    u64::from_le_bytes(buffer[16..24].try_into().internal_err(WRONG_OFFSET)?),
			index_offset:   u64::from_le_bytes(buffer[24..32].try_into().internal_err(WRONG_OFFSET)?),
		};

		if header.section_id != 1 {
			Error::invalid("Section ID of the compressed vector section header is not 1")?
		}
		if header.section_length % 4 != 0 {
			Error::invalid("Section length is not aligned and a multiple of four")?
		}

		Ok(header)
	}
}

impl Default for CompressedVectorSectionHeader {
	fn default() -> Self {
		Self {
			section_id:     1,
			section_length: 0,
			data_offset:    0,
			index_offset:   0,
		}
	}
}
