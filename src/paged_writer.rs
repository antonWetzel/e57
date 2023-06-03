use crate::crc32::Crc32;
use std::io::{Read, Seek, SeekFrom, Write};

const PAGE_SIZE: u64 = 1024;
const CRC_SIZE: u64 = 4;
const PAGE_PAYLOAD_SIZE: usize = (PAGE_SIZE - CRC_SIZE) as usize;

pub struct PagedWriter<T: Write + Read + Seek> {
	writer:      T,
	offset:      usize,
	crc:         Crc32,
	page_buffer: Vec<u8>,
}

impl<T: Write + Read + Seek> Write for PagedWriter<T> {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		let remaining_page_bytes = PAGE_PAYLOAD_SIZE - self.offset;
		let writeable_bytes = buf.len().min(remaining_page_bytes);
		self.page_buffer[self.offset..self.offset + writeable_bytes].copy_from_slice(&buf[..writeable_bytes]);
		self.offset += writeable_bytes;
		if self.offset >= PAGE_PAYLOAD_SIZE {
			let crc = self.crc.calculate(&self.page_buffer);
			self.page_buffer.extend_from_slice(&crc.to_be_bytes());
			self.writer.write_all(&self.page_buffer)?;
			self.page_buffer.resize(PAGE_PAYLOAD_SIZE, 0_u8);
			self.page_buffer.fill(0_u8);
			self.offset = 0;
		}
		Ok(writeable_bytes)
	}

	fn flush(&mut self) -> std::io::Result<()> {
		// If the page buffer is empty we do not need to persist it
		if self.offset > 0 {
			// Store start posotion of current page
			let pos = self.writer.stream_position()?;

			// Write current page
			let crc = self.crc.calculate(&self.page_buffer);
			self.page_buffer.extend_from_slice(&crc.to_be_bytes());
			self.writer.write_all(&self.page_buffer)?;
			self.page_buffer.truncate(PAGE_PAYLOAD_SIZE);

			// Seek back to beginning of the page
			self.writer.seek(SeekFrom::Start(pos))?;
		}

		// Forward flush to underlying writer
		self.writer.flush()
	}
}

impl<T: Write + Read + Seek> Drop for PagedWriter<T> {
	fn drop(&mut self) {
		if self.flush().is_err() {
			// Cannot handle the error here :/
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::fs::{remove_file, File, OpenOptions};
	use std::path::Path;

	#[test]
	fn empty() {
		let path = Path::new("empty.bin");
		let file = File::create(&path).unwrap();
		let writer = PagedWriter::new(file).unwrap();
		drop(writer);
		assert_eq!(path.metadata().unwrap().len(), 0);
		remove_file(path).unwrap();
	}

	#[test]
	fn partial_page() {
		let path = Path::new("partial.bin");
		let file = File::create(&path).unwrap();

		// Write only three bytes
		let mut writer = PagedWriter::new(file).unwrap();
		writer.write_all(&[0_u8, 1_u8, 2_u8]).unwrap();
		drop(writer);
		assert_eq!(path.metadata().unwrap().len(), PAGE_SIZE);

		// Check file content
		let content = std::fs::read(path).unwrap();
		assert_eq!(content[0], 0_u8);
		assert_eq!(content[1], 1_u8);
		assert_eq!(content[2], 2_u8);
		for i in 3..PAGE_PAYLOAD_SIZE {
			assert_eq!(content[i], 0_u8);
		}
		assert_eq!(&content[PAGE_PAYLOAD_SIZE..], &[156, 69, 208, 231]);

		remove_file(path).unwrap();
	}

	#[test]
	fn single_page() {
		let path = Path::new("single.bin");
		let file = File::create(&path).unwrap();
		let mut writer = PagedWriter::new(file).unwrap();

		// Write exactly one page
		let data = vec![1_u8; PAGE_PAYLOAD_SIZE];
		writer.write_all(&data).unwrap();
		drop(writer);
		assert_eq!(path.metadata().unwrap().len(), PAGE_SIZE);

		// Check file content
		let content = std::fs::read(path).unwrap();
		for i in 0..PAGE_PAYLOAD_SIZE {
			assert_eq!(content[i], 1_u8);
		}
		assert_eq!(&content[PAGE_PAYLOAD_SIZE..], &[25, 85, 144, 35]);

		remove_file(path).unwrap();
	}

	#[test]
	fn multi_page() {
		let path = Path::new("multi.bin");
		let file = File::create(&path).unwrap();
		let mut writer = PagedWriter::new(file).unwrap();

		// Write a little bit more than one page
		let mut data = vec![1_u8; PAGE_PAYLOAD_SIZE + 1];
		data[PAGE_PAYLOAD_SIZE] = 2_u8;
		writer.write_all(&data).unwrap();
		drop(writer);
		assert_eq!(path.metadata().unwrap().len(), 2 * PAGE_SIZE);

		// Load file content
		let content = std::fs::read(path).unwrap();

		// Check first page with ones
		let offset = 0;
		for i in 0..PAGE_PAYLOAD_SIZE {
			assert_eq!(content[offset + i], 1_u8);
		}
		assert_eq!(
			&content[PAGE_PAYLOAD_SIZE..PAGE_PAYLOAD_SIZE + CRC_SIZE as usize],
			&[25, 85, 144, 35]
		);

		// Check second page with one two and lots of zeros
		let offset = PAGE_SIZE as usize;
		assert_eq!(content[offset], 2_u8);
		for i in 1..PAGE_PAYLOAD_SIZE {
			assert_eq!(content[offset + i], 0_u8);
		}
		assert_eq!(
			&content[(offset + PAGE_PAYLOAD_SIZE)..],
			&[40, 41, 250, 169]
		);

		remove_file(path).unwrap();
	}

	#[test]
	fn flush_in_page() {
		let path = Path::new("flush.bin");
		let file = File::create(&path).unwrap();
		let mut writer = PagedWriter::new(file).unwrap();

		// Partial page
		writer.write_all(&[0_u8, 1_u8, 2_u8]).unwrap();

		// Flush
		writer.flush().unwrap();

		// Write more data into page
		writer.write_all(&[3_u8, 4_u8, 5_u8]).unwrap();

		// Close and check size
		drop(writer);
		assert_eq!(path.metadata().unwrap().len(), PAGE_SIZE);

		// Check file content
		let content = std::fs::read(path).unwrap();
		for i in 0..6 {
			assert_eq!(content[i], i as u8);
		}
		for i in 6..PAGE_PAYLOAD_SIZE {
			assert_eq!(content[i], 0_u8);
		}
		assert_eq!(&content[PAGE_PAYLOAD_SIZE..], &[50, 14, 64, 153]);

		remove_file(path).unwrap();
	}

	#[test]
	fn seek_existing_page() {
		let mut options = OpenOptions::new();
		options.read(true);
		options.write(true);
		options.create(true);
		options.truncate(true);
		let path = Path::new("seek_existing.bin");
		let file = options.open(&path).unwrap();
		let mut writer = PagedWriter::new(file).unwrap();

		// Write two pages with ones
		let data = vec![1_u8; PAGE_PAYLOAD_SIZE * 2];
		writer.write_all(&data).unwrap();

		// Got back to start and write some twos
		writer.physical_seek(2).unwrap();
		writer.write_all(&[2_u8, 2_u8]).unwrap();
		drop(writer);

		// Check file content
		let content = std::fs::read(path).unwrap();
		assert_eq!(content[0], 1_u8);
		assert_eq!(content[1], 1_u8);
		assert_eq!(content[2], 2_u8);
		assert_eq!(content[3], 2_u8);
		assert_eq!(content[4], 1_u8);
		assert_eq!(content[5], 1_u8);

		remove_file(path).unwrap();
	}

	#[test]
	fn seek_after_end() {
		let path = Path::new("seek_after_end.bin");
		let file = OpenOptions::new()
			.create(true)
			.write(true)
			.read(true)
			.truncate(true)
			.open(path)
			.unwrap();
		let mut writer = PagedWriter::new(file).unwrap();

		// Seek to start should work
		writer.physical_seek(0).unwrap();

		// Seeking further fails
		assert!(writer.physical_seek(2).is_err());

		remove_file(path).unwrap();
	}

	#[test]
	fn phys_position_size() {
		let path = Path::new("phys_position_size.bin");
		let file = File::create(&path).unwrap();
		let mut writer = PagedWriter::new(file).unwrap();

		// Write a page and some bytes
		let data = vec![1_u8; 1028];
		writer.write_all(&data).unwrap();

		// We expect the physical position to be the logical + CRC size
		let pos = writer.physical_position().unwrap();
		assert_eq!(pos, 1028 + CRC_SIZE as u64);

		// We expect the physical size to be two pages with CRC sums
		let size = writer.physical_size().unwrap();
		assert_eq!(size, PAGE_SIZE * 2);

		remove_file(path).unwrap();
	}

	#[test]
	fn align() {
		let path = Path::new("align.bin");
		let file = File::create(&path).unwrap();
		let mut writer = PagedWriter::new(file).unwrap();

		writer.align().unwrap();
		assert_eq!(writer.physical_position().unwrap(), 0);

		let data = vec![1_u8; 2];
		writer.write_all(&data).unwrap();
		writer.align().unwrap();
		assert_eq!(writer.physical_position().unwrap(), 4);

		// Check file content
		drop(writer);
		let content = std::fs::read(path).unwrap();
		assert_eq!(content[0], 1_u8);
		assert_eq!(content[1], 1_u8);
		assert_eq!(content[2], 0_u8);
		assert_eq!(content[3], 0_u8);

		remove_file(path).unwrap();
	}
}
