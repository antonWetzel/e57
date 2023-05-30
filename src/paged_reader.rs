use std::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom};

const CHECKSUM_SIZE: u64 = 4;
const ALIGNMENT_SIZE: u64 = 4;
const MAX_PAGE_SIZE: u64 = 1024 * 1024;

pub struct PagedReader<T: Read + Seek> {
	page_size: u64,
	reader:    T,
	offset:    u64,
}

impl<T: Read + Seek> PagedReader<T> {
	/// Create and initialize a paged reader that abstracts the E57 CRC scheme
	pub fn new(mut reader: T, page_size: u64) -> Result<Self> {
		if page_size > MAX_PAGE_SIZE {
			Err(Error::new(
				ErrorKind::InvalidInput,
				format!("Page size {page_size} is bigger than the allowed maximum page size of {MAX_PAGE_SIZE} bytes"),
			))?;
		}
		if page_size <= CHECKSUM_SIZE {
			Err(Error::new(
				ErrorKind::InvalidInput,
				format!("Page size {page_size} needs to be bigger than checksum ({CHECKSUM_SIZE} bytes)"),
			))?;
		}

		let phy_file_size = reader.seek(SeekFrom::End(0))?;
		if phy_file_size == 0 {
			let msg = "A file size of zero is not allowed";
			Err(Error::new(ErrorKind::InvalidData, msg))?;
		}
		if phy_file_size % page_size != 0 {
			Err(Error::new(
				ErrorKind::InvalidData,
				format!("File size {phy_file_size} is not a multiple of the page size {page_size}"),
			))?;
		}

		Ok(Self { reader, page_size, offset: 0 })
	}

	pub fn seek_physical(&mut self, offset: u64) -> Result<()> {
		self.reader.seek(SeekFrom::Start(offset))?;
		self.offset = offset % self.page_size;
		return Ok(());
	}

	pub fn align(&mut self) -> Result<()> {
		let off_alignment = self.offset.overflowing_neg().0 % ALIGNMENT_SIZE;
		self.reader
			.seek(SeekFrom::Current(off_alignment as i64))
			.unwrap();
		self.offset += off_alignment;
		return Ok(());
	}

	pub fn skip(&mut self, length: usize) {
		let mut length = length as u64;
		let skips = (self.offset + length) / (self.page_size - CHECKSUM_SIZE);
		length += skips * CHECKSUM_SIZE;
		self.offset = (self.offset + length) % self.page_size;
		self.reader.seek(SeekFrom::Current(length as i64)).unwrap();
	}
}

impl<T: Read + Seek> Read for PagedReader<T> {
	fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
		if self.offset == self.page_size - CHECKSUM_SIZE {
			self.offset = 0;
			self.reader
				.seek(SeekFrom::Current(CHECKSUM_SIZE as i64))
				.unwrap();
		} else if self.offset > self.page_size - CHECKSUM_SIZE {
			unreachable!();
		}

		let readable = std::cmp::min(
			buf.len() as u64,
			self.page_size - CHECKSUM_SIZE - self.offset,
		);

		let read = self.reader.read(&mut buf[0..readable as usize])?;
		self.offset = self.offset + read as u64;

		return Ok(read);
	}
}
