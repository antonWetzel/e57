use crate::error::INTERNAL_ERROR;
use crate::Error;
use std::io::Read;

// const SIGNATURE: [u8; 8] = [b'A', b'S', b'T', b'M', b'-', b'E', b'5', b'7'];
const SIGNATURE: &[u8; 8] = b"ASTM-E57";
const MAJOR_VERSION: u32 = 1;
const MINOR_VERSION: u32 = 0;
const PAGE_SIZE: u64 = 1024;

/// Represents the file structure from the start of an E57 file.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Header {
	/// File header signature that must be always "ASTM-E57".
	pub signature: [u8; 8],

	/// Major version number of the E57 format of the file.
	pub major: u32,

	/// Minor version number of the E57 format of the file.
	pub minor: u32,

	/// Physical length of the E57 file on disk or in memory.
	pub phys_length: u64,

	/// Physical offset of the XML data inside the XML file.
	pub phys_xml_offset: u64,

	/// Logical (without CRC bytes) length of the XML data.
	pub xml_length: u64,

	/// Page size of the E57 file.
	pub page_size: u64,
}

impl Header {
	/// Reads an E57 file header structure.
	pub fn read(reader: &mut dyn Read) -> Result<Self, Error> {
		let mut data = [0_u8; 48];
		reader.read_exact(&mut data)?;

		let header = Header {
			signature:       data[0..8].try_into().expect(INTERNAL_ERROR),
			major:           u32::from_le_bytes(data[8..12].try_into().expect(INTERNAL_ERROR)),
			minor:           u32::from_le_bytes(data[12..16].try_into().expect(INTERNAL_ERROR)),
			phys_length:     u64::from_le_bytes(data[16..24].try_into().expect(INTERNAL_ERROR)),
			phys_xml_offset: u64::from_le_bytes(data[24..32].try_into().expect(INTERNAL_ERROR)),
			xml_length:      u64::from_le_bytes(data[32..40].try_into().expect(INTERNAL_ERROR)),
			page_size:       u64::from_le_bytes(data[40..48].try_into().expect(INTERNAL_ERROR)),
		};

		if &header.signature != SIGNATURE {
			return Error::Invalid("Found unsupported signature in header".into()).throw();
		}
		if header.major != MAJOR_VERSION {
			return Error::Invalid("Found unsupported major version in header".into()).throw();
		}
		if header.minor != MINOR_VERSION {
			return Error::Invalid("Found unsupported minor version in header".into()).throw();
		}
		if header.page_size != PAGE_SIZE {
			return Error::Invalid("Found unsupported page size in header".into()).throw();
		}

		Ok(header)
	}
}
