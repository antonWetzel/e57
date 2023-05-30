use crate::bs_read::ByteStreamReadBuffer;
use crate::cv_section::CompressedVectorSectionHeader;
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
use std::collections::VecDeque;
use std::io::{Read, Seek};

/// Iterate over all points of an existing point cloud to read it.
pub struct PointCloudReader<'a, T: Read + Seek> {
	pc:           PointCloud,
	reader:       &'a mut PagedReader<T>,
	byte_streams: Vec<ByteStreamReadBuffer>,
	read:         u64,
	queue:        VecDeque<Point>,
	offsets:      Vec<usize>,
	current:      usize,
	avaible:      usize,
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

impl<'a, T: Read + Seek + 'a> PointCloudReader<'a, T> {
	pub(crate) fn new(pc: &PointCloud, reader: &'a mut PagedReader<T>) -> Result<Self> {
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

		Ok(PointCloudReader {
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

impl<'a, T: Read + Seek> Iterator for PointCloudReader<'a, T> {
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
			if let Err(err) = self.advance() {
				return Some(Err(err));
			}
		}
		let p = match self.queue.pop_front() {
			None => return None,
			Some(p) => p,
		};
		self.read += 1;
		self.current += 1;
		return Some(Ok(p));
	}
}
