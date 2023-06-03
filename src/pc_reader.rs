use crate::cv_section::CompressedVectorSectionHeader;
use crate::error::Converter;
use crate::paged_reader::PagedReader;
use crate::Point;
use crate::PointCloud;
use crate::RecordDataType;
use crate::RecordName;
use crate::Result;

// todo: use
const ALIGNMENT_SIZE: usize = 4;
const PHYSICAL_PAGE_SIZE: usize = 1024;
const LOGICAL_PAGE_SIZE: usize = PHYSICAL_PAGE_SIZE - ALIGNMENT_SIZE;

/// Iterate over all points of an existing point cloud to read it.
pub struct PointCloudReader {
	pc:   PointCloud,
	read: u64,

	property_readers: Vec<Box<dyn PropertyReader>>,
	mmap:             memmap2::Mmap,
}

trait PropertyLoader<V> {
	fn load(&mut self, mmap: &memmap2::Mmap, at_end: bool) -> V;
}

fn index_mmap(mmap: &memmap2::Mmap, start: usize, end: usize) -> &[u8] {
	#[thread_local]
	static mut BACKUP: [u8; 16] = [0u8; 16];
	if start / LOGICAL_PAGE_SIZE != (end - 1) / LOGICAL_PAGE_SIZE {
		let size = end - start;
		let remaining = LOGICAL_PAGE_SIZE - start % LOGICAL_PAGE_SIZE;
		let start = start + (start / LOGICAL_PAGE_SIZE) * 4;
		unsafe { BACKUP[0..remaining].copy_from_slice(&mmap[start..(start + remaining)]) };
		let start = start + remaining + 4;
		unsafe { BACKUP[remaining..size].copy_from_slice(&mmap[start..(start + size - remaining)]) };
		return unsafe { &BACKUP[0..size] };
	}
	let start = start + (start / LOGICAL_PAGE_SIZE) * 4;
	let end = end + ((end - 1) / LOGICAL_PAGE_SIZE) * 4;
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
	return pos + (pos / LOGICAL_PAGE_SIZE) * 4;
}

impl IntLoader {
	pub fn new(prototype_offset: usize, prototype_index: usize, min: i64, max: i64, mmap: &memmap2::Mmap) -> Self {
		let mut loader = IntLoader {
			prototype_offset,
			prototype_index,
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
				"only data headers allowed, got other at {}",
				logic_to_phys(self.prototype_offset)
			);
		}
		let _comp_restart_flag = header[1] & 1 != 0;
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
			if diff > 0 {
				tmp[(end_offset - diff)..end_offset].copy_from_slice(index_mmap(
					mmap,
					self.current - diff,
					self.current,
				));
			}
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

struct UnitIntConverter {
	min: i64,
	max: i64,
}

impl PropertyConverter<i64, f32> for UnitIntConverter {
	fn convert(&self, v: i64) -> f32 {
		return (v - self.min) as f32 / (self.max - self.min) as f32;
	}
}

trait PropertySaver<V> {
	fn save(point: &mut Point, value: V);
}

struct CartesionXSaver;
impl PropertySaver<f64> for CartesionXSaver {
	fn save(point: &mut Point, value: f64) {
		point.cartesian.x = value;
	}
}

struct CartesionYSaver;
impl PropertySaver<f64> for CartesionYSaver {
	fn save(point: &mut Point, value: f64) {
		point.cartesian.y = value;
	}
}

struct CartesionZSaver;
impl PropertySaver<f64> for CartesionZSaver {
	fn save(point: &mut Point, value: f64) {
		point.cartesian.z = value;
	}
}

struct ColorRedSaver;
impl PropertySaver<f32> for ColorRedSaver {
	fn save(point: &mut Point, value: f32) {
		point.color.red = value;
	}
}

struct ColorGreenSaver;
impl PropertySaver<f32> for ColorGreenSaver {
	fn save(point: &mut Point, value: f32) {
		point.color.green = value;
	}
}

struct ColorBlueSaver;
impl PropertySaver<f32> for ColorBlueSaver {
	fn save(point: &mut Point, value: f32) {
		point.color.blue = value;
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

impl<Loader, Saver, Converter, V0, V1> GenPropertyReader<Loader, Saver, Converter, V0, V1>
where
	Loader: PropertyLoader<V0>,
	Converter: PropertyConverter<V0, V1>,
	Saver: PropertySaver<V1>,
{
	fn new(loader: Loader, converter: Converter) -> Self {
		return GenPropertyReader {
			loader:    loader,
			converter: converter,
			phantom:   std::marker::PhantomData,
		};
	}
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

impl PointCloudReader {
	pub(crate) fn new(pc: &PointCloud, reader: &mut PagedReader) -> Result<Self> {
		reader
			.seek_physical(pc.file_offset)
			.read_err("Cannot seek to compressed vector header")?;
		let section_header = CompressedVectorSectionHeader::read(reader)?;
		reader
			.seek_physical(section_header.data_offset)
			.read_err("Cannot seek to packet header")?;
		let pc = pc.clone();

		let mmap = unsafe { memmap2::MmapOptions::new().map(&reader.reader).unwrap() };
		let logical_offset = section_header.data_offset as usize;
		let logical_offset = logical_offset - (logical_offset / 1024) * 4;

		let mut property_readers = Vec::<Box<dyn PropertyReader>>::new();

		for (index, prototype) in pc.prototype.iter().enumerate() {
			let reader: Box<dyn PropertyReader> = match (prototype.name, prototype.data_type) {
				(RecordName::CartesianX, RecordDataType::ScaledInteger { min, max, scale }) => {
					Box::new(GenPropertyReader::<_, CartesionXSaver, _, _, _>::new(
						IntLoader::new(logical_offset, index, min, max, &mmap),
						ScaledIntConverter { scale },
					))
				},
				(RecordName::CartesianY, RecordDataType::ScaledInteger { min, max, scale }) => {
					Box::new(GenPropertyReader::<_, CartesionYSaver, _, _, _>::new(
						IntLoader::new(logical_offset, index, min, max, &mmap),
						ScaledIntConverter { scale },
					))
				},
				(RecordName::CartesianZ, RecordDataType::ScaledInteger { min, max, scale }) => {
					Box::new(GenPropertyReader::<_, CartesionZSaver, _, _, _>::new(
						IntLoader::new(logical_offset, index, min, max, &mmap),
						ScaledIntConverter { scale },
					))
				},
				(RecordName::ColorRed, RecordDataType::Integer { min, max }) => {
					Box::new(GenPropertyReader::<_, ColorRedSaver, _, _, _>::new(
						IntLoader::new(logical_offset, index, min, max, &mmap),
						UnitIntConverter { min, max },
					))
				},
				(RecordName::ColorGreen, RecordDataType::Integer { min, max }) => {
					Box::new(GenPropertyReader::<_, ColorGreenSaver, _, _, _>::new(
						IntLoader::new(logical_offset, index, min, max, &mmap),
						UnitIntConverter { min, max },
					))
				},
				(RecordName::ColorBlue, RecordDataType::Integer { min, max }) => {
					Box::new(GenPropertyReader::<_, ColorBlueSaver, _, _, _>::new(
						IntLoader::new(logical_offset, index, min, max, &mmap),
						UnitIntConverter { min, max },
					))
				},
				(RecordName::Intensity, _) => continue,
				(RecordName::RowIndex, _) => continue,
				(RecordName::ColumnIndex, _) => continue,
				(RecordName::CartesianInvalidState, _) => continue, //todo: use
				(name, data_type) => panic!("not handled or ignored: {:?} {:?}", name, data_type),
			};
			property_readers.push(reader);
		}

		Ok(PointCloudReader { mmap, property_readers, pc, read: 0 })
	}
}

impl Iterator for PointCloudReader {
	type Item = Point;

	fn next(&mut self) -> Option<Self::Item> {
		if self.read >= self.pc.records {
			return None;
		}

		let mut p = Point::default();

		let at_end = self.read >= self.pc.records - 1;
		for reader in self.property_readers.iter_mut() {
			reader.read(&self.mmap, &mut p, at_end);
		}

		self.read += 1;
		return Some(p);
	}
}
