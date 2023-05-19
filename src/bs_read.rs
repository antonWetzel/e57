use std::collections::VecDeque;

#[derive(Clone)]
pub struct ByteStreamReadBuffer {
	buffer: VecDeque<u8>,
	offset: u64,
}

impl ByteStreamReadBuffer {
	pub fn new() -> Self {
		Self { buffer: VecDeque::new(), offset: 0 }
	}

	pub fn append(&mut self, data: Vec<u8>) {
		self.buffer.append(&mut VecDeque::from(data));
	}

	pub fn extract_f32(&mut self) -> Option<f32> {
		if self.available() < 32 {
			return None;
		}
		let mut data = [0u8; 4];
		for i in 0..4 {
			data[i] = self.buffer.pop_front().unwrap();
		}
		return Some(f32::from_le_bytes(data));
	}

	pub fn extract_f64(&mut self) -> Option<f64> {
		if self.available() < 64 {
			return None;
		}
		let mut data = [0u8; 8];
		for i in 0..8 {
			data[i] = self.buffer.pop_front().unwrap();
		}
		return Some(f64::from_le_bytes(data));
	}

	pub fn extract_int(&mut self, bits: u64, min: i64, mask: u64) -> Option<i64> {
		if self.available() < bits {
			return None;
		}
		let end_offset = ((self.offset + bits + 7) / 8) as usize;
		let used_offset = ((self.offset + bits) / 8) as usize;
		let mut tmp = [0u8; 8];
		for i in 0..used_offset {
			tmp[i] = self.buffer.pop_front().unwrap();
		}
		for i in used_offset..end_offset {
			tmp[i] = *self.buffer.front().unwrap();
		}
		let uint_value = (u64::from_le_bytes(tmp) >> self.offset) & mask;
		let int_value = uint_value as i64 + min;
		self.offset = (self.offset + bits) % 8;
		return Some(int_value);
	}

	pub fn available(&self) -> u64 {
		(self.buffer.len() as u64 * 8) - self.offset
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn empty() {
		let mut bs = ByteStreamReadBuffer::new();
		assert_eq!(bs.available(), 0);
		let result = bs.extract(0).unwrap();
		assert_eq!(result.bits, 0);
		assert_eq!(result.offset, 0);
		assert_eq!(result.data, Vec::new());

		assert_eq!(bs.available(), 0);
		assert!(bs.extract(1).is_none());
	}

	#[test]
	fn append_and_extract_bits() {
		let mut bs = ByteStreamReadBuffer::new();
		bs.append(vec![255]);

		assert_eq!(bs.available(), 8);
		let result = bs.extract(2).unwrap();
		assert_eq!(result.bits, 2);
		assert_eq!(result.offset, 0);
		assert_eq!(result.data, vec![255_u8]);

		assert_eq!(bs.available(), 6);
		let result = bs.extract(6).unwrap();
		assert_eq!(result.bits, 6);
		assert_eq!(result.offset, 2);
		assert_eq!(result.data, vec![255]);

		assert_eq!(bs.available(), 0);
		assert!(bs.extract(1).is_none());
	}

	#[test]
	fn append_and_extract_bytes() {
		let mut bs = ByteStreamReadBuffer::new();
		bs.append(vec![23, 42, 13]);
		bs.extract(2).unwrap();

		assert_eq!(bs.available(), 22);
		let result = bs.extract(22).unwrap();
		assert_eq!(result.bits, 22);
		assert_eq!(result.offset, 2);
		assert_eq!(result.data, vec![23, 42, 13]);
	}

	#[test]
	fn remove_consume_when_appending() {
		let mut bs = ByteStreamReadBuffer::new();
		bs.append(vec![1, 2, 3, 4, 5]);
		bs.extract(4 * 8 + 2).unwrap();

		// We append one byte and the buffer should become smaller
		// because all fully consumed bytes are removed.
		bs.append(vec![6]);
		assert!(bs.buffer.len() == 2);

		// Offsets are updated correctly appended
		// data can be extracted as expected.
		let result = bs.extract(14).unwrap();
		assert_eq!(result.bits, 14);
		assert_eq!(result.offset, 2);
		assert_eq!(result.data, vec![5, 6]);
	}
}
