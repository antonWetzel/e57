use std::collections::VecDeque;

#[derive(Clone)]
pub struct ByteStreamReadBuffer {
	buffer:        VecDeque<Vec<u8>>,
	buffer_offset: usize,
	offset:        u32,
	available:     u32,
}

impl ByteStreamReadBuffer {
	pub fn new() -> Self {
		Self {
			buffer:        VecDeque::new(),
			offset:        0,
			buffer_offset: 0,
			available:     0,
		}
	}

	pub fn append(&mut self, data: Vec<u8>) {
		self.available += data.len() as u32 * 8;
		self.buffer.push_back(data);
	}

	pub fn extract_f32(&mut self) -> Option<f32> {
		if self.available < 32 {
			return None;
		}
		let mut data = [0u8; 4];
		for i in 0..4 {
			if self.buffer_offset >= self.buffer[0].len() {
				self.buffer_offset = 0;
				self.buffer.pop_front();
			}
			data[i] = self.buffer[0][self.buffer_offset];
			self.buffer_offset += 1;
		}
		return Some(f32::from_le_bytes(data));
	}

	fn get_value(&mut self) -> u8 {
		let res = self.buffer[0][self.buffer_offset];
		self.buffer_offset += 1;
		self.available -= 8;
		if self.buffer_offset >= self.buffer[0].len() {
			self.buffer_offset = 0;
			self.buffer.pop_front();
		}
		return res;
	}

	fn peek_value(&self) -> u8 {
		return self.buffer[0][self.buffer_offset];
	}

	pub fn extract_f64(&mut self) -> Option<f64> {
		if self.available < 64 {
			return None;
		}
		let mut data = [0u8; 8];
		for i in 0..8 {
			data[i] = self.get_value();
		}
		return Some(f64::from_le_bytes(data));
	}

	pub fn extract_int(&mut self, min: i64, max: i64) -> Option<i64> {
		let range = max - min;
		let bits = u64::BITS - range.leading_zeros();
		if self.available < bits {
			return None;
		}
		let mask = (1u64 << bits) - 1;
		let end_offset = ((self.offset + bits + 7) / 8) as usize;
		let used_offset = ((self.offset + bits) / 8) as usize;
		let mut tmp = [0u8; 8];
		for i in 0..used_offset {
			tmp[i] = self.get_value();
		}
		for i in used_offset..end_offset {
			tmp[i] = self.peek_value();
		}
		let uint_value = (u64::from_le_bytes(tmp) >> self.offset) & mask;
		let int_value = uint_value as i64 + min;
		self.offset = (self.offset + bits) % 8;
		return Some(int_value);
	}
}
