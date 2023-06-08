use crate::{error::INTERNAL_ERROR, Error};

const ALIGNMENT_SIZE: usize = 4;
const PHYSICAL_PAGE_SIZE: usize = 1024;
const LOGICAL_PAGE_SIZE: usize = PHYSICAL_PAGE_SIZE - ALIGNMENT_SIZE;

struct Position {
	offset:  usize,
	index:   usize,
	current: usize,
	end:     usize,
}

impl Position {
	fn new(prototype_offset: usize, prototype_index: usize, mmap: &memmap2::Mmap) -> Result<Self, Error> {
		let mut position = Self {
			offset:  prototype_offset,
			index:   prototype_index,
			current: 0,
			end:     0,
		};
		position.load_next(mmap)?;
		Ok(position)
	}

	fn load_next(&mut self, mmap: &memmap2::Mmap) -> Result<usize, Error> {
		let header = index_mmap(mmap, self.offset, self.offset + 6);
		if header[0] != 1 {
			return Err(Error::Invalid(format!(
				"only data headers (1) allowed, got ({})",
				header[0]
			)));
		}
		let _comp_restart_flag = header[1] & 1 != 0;
		let packet_length = u16::from_le_bytes(header[2..4].try_into().expect(INTERNAL_ERROR)) as usize + 1;
		let bytestream_count = u16::from_le_bytes(header[4..6].try_into().expect(INTERNAL_ERROR));

		let mut block_current = 6 + bytestream_count as usize * 2;
		let mut block_size = 0;
		for index in 0..=self.index {
			let data = index_mmap(
				mmap,
				self.offset + 6 + index * 2,
				self.offset + 6 + (index + 1) * 2,
			);
			let size = u16::from_le_bytes(data.try_into().expect(INTERNAL_ERROR)) as usize;
			block_current += size;
			block_size = size;
		}

		let diff = self.current - self.end;
		self.end = self.offset + block_current;
		self.current = self.end - block_size + diff;
		self.offset += packet_length;
		Ok(diff)
	}
}

pub trait PropertyLoader<V> {
	fn load(&mut self, mmap: &memmap2::Mmap, at_end: bool) -> Result<V, Error>;
}

fn index_mmap(mmap: &memmap2::Mmap, start: usize, end: usize) -> &[u8] {
	#[thread_local]
	static mut BACKUP: [u8; 16] = [0u8; 16];
	let pages_start = start / LOGICAL_PAGE_SIZE;
	let pages_end = (end - 1) / LOGICAL_PAGE_SIZE;
	if pages_start != pages_end {
		let size = end - start;
		let remaining = LOGICAL_PAGE_SIZE - start % LOGICAL_PAGE_SIZE;
		let start = start + (start / LOGICAL_PAGE_SIZE) * ALIGNMENT_SIZE;
		unsafe { BACKUP[0..remaining].copy_from_slice(&mmap[start..(start + remaining)]) };
		let start = start + remaining + ALIGNMENT_SIZE;
		unsafe { BACKUP[remaining..size].copy_from_slice(&mmap[start..(start + size - remaining)]) };
		return unsafe { &BACKUP[0..size] };
	}
	let start = start + pages_start * ALIGNMENT_SIZE;
	let end = end + pages_end * ALIGNMENT_SIZE;
	&mmap[start..end]
}

pub struct IntLoader {
	position: Position,
	min:      i64,
	offset:   u32,
	bits:     u32,
	mask:     u64,
}

impl IntLoader {
	pub fn new(
		prototype_offset: usize,
		prototype_index: usize,
		min: i64,
		max: i64,
		mmap: &memmap2::Mmap,
	) -> Result<Self, Error> {
		let range = max - min;
		let bits = u64::BITS - range.leading_zeros();
		let mask = (1u64 << bits) - 1;
		Ok(IntLoader {
			position: Position::new(prototype_offset, prototype_index, mmap)?,
			min,
			offset: 0,
			bits,
			mask,
		})
	}
}

impl PropertyLoader<i64> for IntLoader {
	fn load(&mut self, mmap: &memmap2::Mmap, at_end: bool) -> Result<i64, Error> {
		let end_offset = ((self.offset + self.bits + 7) / 8) as usize;
		let mut tmp = [0u8; 8];
		tmp[0..end_offset].copy_from_slice(index_mmap(
			mmap,
			self.position.current,
			self.position.current + end_offset,
		));

		let used_offset = ((self.offset + self.bits) / 8) as usize;
		self.position.current += used_offset;

		if self.position.current >= self.position.end && !at_end {
			let diff = self.position.load_next(mmap)?;
			if diff > 0 {
				tmp[(end_offset - diff)..end_offset].copy_from_slice(index_mmap(
					mmap,
					self.position.current - diff,
					self.position.current,
				));
			}
		}

		let uint_value = (u64::from_le_bytes(tmp) >> self.offset) & self.mask;
		let int_value = uint_value as i64 + self.min;
		self.offset = (self.offset + self.bits) % 8;
		Ok(int_value)
	}
}

pub struct F64Loader {
	position: Position,
}

impl F64Loader {
	pub fn new(prototype_offset: usize, prototype_index: usize, mmap: &memmap2::Mmap) -> Result<Self, Error> {
		Ok(Self {
			position: Position::new(prototype_offset, prototype_index, mmap)?,
		})
	}
}

impl PropertyLoader<f64> for F64Loader {
	fn load(&mut self, mmap: &memmap2::Mmap, at_end: bool) -> Result<f64, Error> {
		let mut tmp = [0u8; 8];
		tmp.copy_from_slice(index_mmap(
			mmap,
			self.position.current,
			self.position.current + 8,
		));
		self.position.current += 8;

		if self.position.current >= self.position.end && !at_end {
			let diff = self.position.load_next(mmap)?;
			if diff > 0 {
				tmp[(8 - diff)..8].copy_from_slice(index_mmap(
					mmap,
					self.position.current - diff,
					self.position.current,
				));
			}
		}
		Ok(f64::from_le_bytes(tmp))
	}
}

pub struct F32Loader {
	position: Position,
}

impl F32Loader {
	pub fn new(prototype_offset: usize, prototype_index: usize, mmap: &memmap2::Mmap) -> Result<Self, Error> {
		Ok(Self {
			position: Position::new(prototype_offset, prototype_index, mmap)?,
		})
	}
}

impl PropertyLoader<f32> for F32Loader {
	fn load(&mut self, mmap: &memmap2::Mmap, at_end: bool) -> Result<f32, Error> {
		let mut tmp = [0u8; 4];
		tmp.copy_from_slice(index_mmap(
			mmap,
			self.position.current,
			self.position.current + 4,
		));
		self.position.current += 4;

		if self.position.current >= self.position.end && !at_end {
			let diff = self.position.load_next(mmap)?;
			if diff > 0 {
				tmp[(4 - diff)..4].copy_from_slice(index_mmap(
					mmap,
					self.position.current - diff,
					self.position.current,
				));
			}
		}
		Ok(f32::from_le_bytes(tmp))
	}
}
