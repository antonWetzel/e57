use crate::bs_read::ByteStreamReadBuffer;
use crate::cv_section::CompressedVectorSectionHeader;
use crate::date_time::serialize_date_time;
use crate::error::Converter;
use crate::packet::PacketHeader;
use crate::paged_reader::PagedReader;
use crate::Error;
use crate::Point;
use crate::PointCloud;
use crate::Record;
use crate::RecordDataType;
use crate::RecordName;
use crate::Result;
use std::backtrace::Backtrace;
use std::collections::VecDeque;
use std::io::{Read, Seek};

/// Iterate over all points of an existing point cloud to read it.
pub struct PointCloudReader<'a> {
	pc:           PointCloud,
	reader:       &'a mut PagedReader,
	byte_streams: Vec<ByteStreamReadBuffer>,
	read:         u64,
	queue:        VecDeque<Point>,
	offsets:      Vec<usize>,
	current:      usize,
	avaible:      usize,

	test: GenPropertyReader<IntLoader, TestSaver, ScaledIntConverter, i64, f64>,
	mmap: memmap2::Mmap,
}

trait PropertyLoader<V> {
	fn load(&mut self, mmap: &memmap2::Mmap, at_end: bool) -> V;
}

fn index_mmap(mmap: &memmap2::Mmap, start: usize, end: usize) -> &[u8] {
	static mut BACKUP: [u8; 8] = [0u8; 8];
	if start / 1020 != (end - 1) / 1020 {
		let size = end - start;
		let remaining = 1020 - start % 1020;
		let start = start + (start / 1020) * 4;
		unsafe { BACKUP[0..remaining].copy_from_slice(&mmap[start..(start + remaining)]) };
		let start = start + remaining + 4;
		unsafe { BACKUP[remaining..size].copy_from_slice(&mmap[start..(start + size - remaining)]) };
		return unsafe { &BACKUP[0..size] };
	}
	let start = start + (start / 1020) * 4;
	let end = end + ((end - 1) / 1020) * 4;
	return &mmap[start..end];
}

struct IntLoader {
	prototype_offset: usize,
	prototype_index:  usize,
	current:          usize,
	end:              usize,

	min:    i64,
	max:    i64,
	offset: u32,
}

fn logic_to_phys(pos: usize) -> usize {
	return pos + (pos / 1020) * 4;
}

impl IntLoader {
	pub fn new(prototype_offset: usize, min: i64, max: i64, mmap: &memmap2::Mmap) -> Self {
		let mut loader = IntLoader {
			prototype_offset,
			prototype_index: 0,
			current: 0,
			end: 0,

			min,
			max,
			offset: 0,
		};
		loader.load_next(mmap);
		return loader;
	}

	fn load_next(&mut self, mmap: &memmap2::Mmap) -> usize {
		let header = index_mmap(mmap, self.prototype_offset, self.prototype_offset + 6);
		if header[0] != 1 {
			panic!(
				"a not data header at {}",
				logic_to_phys(self.prototype_offset)
			);
		}
		let comp_restart_flag = header[1] & 1 != 0;
		let packet_length = u16::from_le_bytes(header[2..4].try_into().unwrap()) as usize + 1;
		let bytestream_count = u16::from_le_bytes(header[4..6].try_into().unwrap());

		let mut block_current = 6 + bytestream_count as usize * 2;
		let mut block_size = 0;
		for index in 0..=self.prototype_index {
			let data = index_mmap(
				mmap,
				self.prototype_offset + 6 + index * 2,
				self.prototype_offset + 6 + (index + 1) * 2,
			);
			let size = u16::from_le_bytes(data.try_into().unwrap()) as usize;
			block_current += size;
			block_size = size;
		}

		let diff = self.current - self.end;
		self.end = self.prototype_offset + block_current;
		self.current = self.end - block_size + diff;
		self.prototype_offset += packet_length;
		return diff;
	}
}
impl PropertyLoader<i64> for IntLoader {
	fn load(&mut self, mmap: &memmap2::Mmap, at_end: bool) -> i64 {
		let range = self.max - self.min;
		let bits = u64::BITS - range.leading_zeros();
		let mask = (1u64 << bits) - 1;
		let end_offset = ((self.offset + bits + 7) / 8) as usize;
		let used_offset = ((self.offset + bits) / 8) as usize;
		let mut tmp = [0u8; 8];
		tmp[0..end_offset].copy_from_slice(index_mmap(mmap, self.current, self.current + end_offset));
		self.current += used_offset;

		if self.current >= self.end && at_end == false {
			let diff = self.load_next(mmap);
			tmp[(end_offset - diff)..end_offset].copy_from_slice(index_mmap(mmap, self.current - diff, self.current));
		}

		let uint_value = (u64::from_le_bytes(tmp) >> self.offset) & mask;
		let int_value = uint_value as i64 + self.min;
		self.offset = (self.offset + bits) % 8;
		return int_value;
	}
}

trait PropertyConverter<V0, V1> {
	fn convert(&self, v: V0) -> V1;
}

struct ScaledIntConverter {
	scale: f64,
}

impl PropertyConverter<i64, f64> for ScaledIntConverter {
	fn convert(&self, v: i64) -> f64 {
		return v as f64 * self.scale;
	}
}

trait PropertySaver<V> {
	fn save(point: &mut Point, value: V);
}

struct TestSaver {}
impl PropertySaver<f64> for TestSaver {
	fn save(point: &mut Point, value: f64) {
		point.cartesian.x = value;
	}
}

trait PropertyReader {
	fn read(&mut self, mmap: &memmap2::Mmap, point: &mut Point, at_end: bool);
}

struct GenPropertyReader<Loader, Saver, Converter, V0, V1>
where
	Loader: PropertyLoader<V0>,
	Converter: PropertyConverter<V0, V1>,
	Saver: PropertySaver<V1>,
{
	loader:    Loader,
	converter: Converter,
	phantom:   std::marker::PhantomData<(Saver, V0, V1)>,
}

impl<Loader, Saver, Converter, V0, V1> PropertyReader for GenPropertyReader<Loader, Saver, Converter, V0, V1>
where
	Loader: PropertyLoader<V0>,
	Converter: PropertyConverter<V0, V1>,
	Saver: PropertySaver<V1>,
{
	fn read(&mut self, mmap: &memmap2::Mmap, point: &mut Point, at_end: bool) {
		let value = self.loader.load(mmap, at_end);
		let value = self.converter.convert(value);
		Saver::save(point, value);
	}
}

fn skip_property(name: RecordName) -> bool {
	return match name {
		RecordName::CartesianX => false,
		RecordName::CartesianY => false,
		RecordName::CartesianZ => false,
		RecordName::ColorRed => false,
		RecordName::ColorGreen => false,
		RecordName::ColorBlue => false,
		_ => true,
	};
}

impl<'a> PointCloudReader<'a> {
	pub(crate) fn new(pc: &PointCloud, reader: &'a mut PagedReader) -> Result<Self> {
		reader
			.seek_physical(pc.file_offset)
			.read_err("Cannot seek to compressed vector header")?;
		let section_header = CompressedVectorSectionHeader::read(reader)?;
		reader
			.seek_physical(section_header.data_offset)
			.read_err("Cannot seek to packet header")?;
		let byte_streams = vec![ByteStreamReadBuffer::new(); pc.prototype.len()];
		let offsets = vec![0usize; pc.prototype.len()];
		let queue = VecDeque::new();
		let pc = pc.clone();

		let (min, max, scale) = match pc.prototype[0].data_type {
			RecordDataType::ScaledInteger { min, max, scale } => (min, max, scale),
			_ => panic!(),
		};

		let mmap = unsafe { memmap2::MmapOptions::new().map(&reader.reader).unwrap() };

		Ok(PointCloudReader {
			test: GenPropertyReader {
				loader: IntLoader::new(section_header.data_offset as usize, min, max, &mmap),

				converter: ScaledIntConverter { scale },
				phantom:   std::marker::PhantomData,
			},

			mmap,

			pc,
			reader,
			read: 0,
			byte_streams,
			queue,
			offsets,
			current: 0,
			avaible: 0,
		})
	}

	fn extract_values<Extract, Insert, V>(
		mut offset: usize,
		byte_stream: &mut ByteStreamReadBuffer,
		queue: &mut VecDeque<Point>,
		extract: Extract,
		insert: Insert,
	) -> usize
	where
		Extract: Fn(&mut ByteStreamReadBuffer) -> Option<V>,
		Insert: Fn(&mut Point, V),
	{
		loop {
			let v = match extract(byte_stream) {
				Some(v) => v,
				None => break,
			};
			if offset >= queue.len() {
				queue.push_back(Point::default());
			}
			insert(&mut queue[offset], v);
			offset += 1;
		}
		return offset;
	}

	fn advance(&mut self) -> Result<()> {
		let packet_header = PacketHeader::read(self.reader)?;
		match packet_header {
			PacketHeader::Index(_) => Error::not_implemented("Index packets are not yet supported")?,
			PacketHeader::Ignored(_) => Error::not_implemented("Ignored packets are not yet supported")?,
			PacketHeader::Data(header) => {
				if header.bytestream_count as usize != self.byte_streams.len() {
					Error::invalid("Bytestream count does not match prototype size")?
				}

				let mut buffer_sizes = Vec::with_capacity(self.byte_streams.len());
				for _ in 0..header.bytestream_count {
					let mut buf = [0_u8; 2];
					self.reader
						.read_exact(&mut buf)
						.read_err("Failed to read data packet buffer sizes")?;
					let len = u16::from_le_bytes(buf) as usize;
					buffer_sizes.push(len);
				}

				for (i, bs) in buffer_sizes.into_iter().enumerate() {
					if skip_property(self.pc.prototype[i].name) {
						self.reader.skip(bs);
						continue;
					}
					let mut buffer = vec![0u8; bs];
					self.reader
						.read_exact(&mut buffer)
						.read_err("Failed to read data packet buffers")?;
					self.byte_streams[i].append(buffer);
				}
				let mut avaible = usize::MAX;

				for (i, r) in self.pc.prototype.iter().enumerate() {
					if skip_property(r.name) {
						continue;
					}
					let offset = self.offsets[i] - self.avaible;
					let byte_stream = &mut self.byte_streams[i];
					self.offsets[i] = Self::match_record(r, offset, byte_stream, &mut self.queue);
					avaible = std::cmp::min(avaible, self.offsets[i]);
				}
				self.current = 0;
				self.avaible = avaible;
			},
		};

		self.reader
			.align()
			.read_err("Failed to align reader on next 4-byte offset after reading packet")?;

		Ok(())
	}

	fn match_record(
		record: &Record,
		offset: usize,
		byte_stream: &mut ByteStreamReadBuffer,
		queue: &mut VecDeque<Point>,
	) -> usize {
		return match (record.name, record.data_type) {
			(RecordName::CartesianX, RecordDataType::Double { min: _min, max: _max }) => Self::extract_values(
				offset,
				byte_stream,
				queue,
				|byte_stream| byte_stream.extract_f64(),
				|p, v| p.cartesian.x = v,
			),
			(RecordName::CartesianX, RecordDataType::ScaledInteger { min, max, scale }) => Self::extract_values(
				offset,
				byte_stream,
				queue,
				|byte_stream| byte_stream.extract_int(min, max),
				|p, v| p.cartesian.x = v as f64 * scale,
			),
			(RecordName::CartesianY, RecordDataType::Double { min: _min, max: _max }) => Self::extract_values(
				offset,
				byte_stream,
				queue,
				|byte_stream| byte_stream.extract_f64(),
				|p, v| p.cartesian.y = v,
			),
			(RecordName::CartesianY, RecordDataType::ScaledInteger { min, max, scale }) => Self::extract_values(
				offset,
				byte_stream,
				queue,
				|byte_stream| byte_stream.extract_int(min, max),
				|p, v| p.cartesian.y = v as f64 * scale,
			),
			(RecordName::CartesianZ, RecordDataType::Double { min: _min, max: _max }) => Self::extract_values(
				offset,
				byte_stream,
				queue,
				|byte_stream| byte_stream.extract_f64(),
				|p, v| p.cartesian.z = v,
			),
			(RecordName::CartesianZ, RecordDataType::ScaledInteger { min, max, scale }) => Self::extract_values(
				offset,
				byte_stream,
				queue,
				|byte_stream| byte_stream.extract_int(min, max),
				|p, v| p.cartesian.z = v as f64 * scale,
			),
			(RecordName::Intensity, RecordDataType::Single { min: _min, max: _max }) => Self::extract_values(
				offset,
				byte_stream,
				queue,
				|byte_stream| byte_stream.extract_f32(),
				|p, v| p.intensity = v,
			),
			(RecordName::Intensity, RecordDataType::ScaledInteger { min, max, scale }) => Self::extract_values(
				offset,
				byte_stream,
				queue,
				|byte_stream| byte_stream.extract_int(min, max),
				|p, v| p.intensity = (v as f64 * scale) as f32,
			),
			(RecordName::ColorRed, RecordDataType::Single { min: Some(min), max: Some(max) }) => Self::extract_values(
				offset,
				byte_stream,
				queue,
				|byte_stream| byte_stream.extract_f32(),
				|p, v| p.color.red = (v - min) / (max - min),
			),
			(RecordName::ColorRed, RecordDataType::Integer { min, max }) => Self::extract_values(
				offset,
				byte_stream,
				queue,
				|byte_stream| byte_stream.extract_int(min, max),
				|p, v| p.color.red = (v - min) as f32 / (max - min) as f32,
			),
			(RecordName::ColorGreen, RecordDataType::Single { min: Some(min), max: Some(max) }) => {
				Self::extract_values(
					offset,
					byte_stream,
					queue,
					|byte_stream| byte_stream.extract_f32(),
					|p, v| p.color.green = (v - min) / (max - min),
				)
			},
			(RecordName::ColorGreen, RecordDataType::Integer { min, max }) => Self::extract_values(
				offset,
				byte_stream,
				queue,
				|byte_stream| byte_stream.extract_int(min, max),
				|p, v| p.color.green = (v - min) as f32 / (max - min) as f32,
			),
			(RecordName::ColorBlue, RecordDataType::Single { min: Some(min), max: Some(max) }) => Self::extract_values(
				offset,
				byte_stream,
				queue,
				|byte_stream| byte_stream.extract_f32(),
				|p, v| p.color.blue = (v - min) as f32 / (max - min) as f32,
			),
			(RecordName::ColorBlue, RecordDataType::Integer { min, max }) => Self::extract_values(
				offset,
				byte_stream,
				queue,
				|byte_stream| byte_stream.extract_int(min, max),
				|p, v| p.color.blue = (v - min) as f32 / (max - min) as f32,
			),
			(RecordName::RowIndex, RecordDataType::Integer { min, max }) => Self::extract_values(
				offset,
				byte_stream,
				queue,
				|byte_stream| byte_stream.extract_int(min, max),
				|p, v| p.row = Some(v),
			),
			(RecordName::ColumnIndex, RecordDataType::Integer { min, max }) => Self::extract_values(
				offset,
				byte_stream,
				queue,
				|byte_stream| byte_stream.extract_int(min, max),
				|p, v| p.row = Some(v),
			),
			(RecordName::CartesianInvalidState, RecordDataType::Integer { min, max }) => Self::extract_values(
				offset,
				byte_stream,
				queue,
				|byte_stream| byte_stream.extract_int(min, max),
				|p, v| p.cartesian_invalid = Some(v as u8),
			),
			_ => {
				panic!("todo: handle {:?} {:?}", record.name, record.data_type);
			},
		};
	}
}

impl<'a> Iterator for PointCloudReader<'a> {
	/// Each iterator item is a result for an extracted point.
	type Item = Result<Point>;

	/// Returns the next available point or None if the end was reached.
	fn next(&mut self) -> Option<Self::Item> {
		// Already read all points?

		if self.read >= self.pc.records {
			return None;
		}

		// Refill property queues if required
		if self.current == self.avaible {
			match self.advance() {
				Ok(_) => {},
				Err(err) => return Some(Err(err)),
			};
		}
		let mut p = match self.queue.pop_front() {
			None => return None,
			Some(p) => p,
		};

		let before = p.cartesian.x;
		self.test
			.read(&self.mmap, &mut p, self.read >= self.pc.records - 1);
		if before != p.cartesian.x {
			println!(
				"missmatch at point {} with {} and {}",
				self.read, before, p.cartesian.x
			);
			// panic!();
		}

		self.read += 1;
		self.current += 1;
		return Some(Ok(p));
	}
}
