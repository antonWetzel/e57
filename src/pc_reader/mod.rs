mod converter;
mod loader;

use crate::error::INTERNAL_ERROR;
use crate::mmap_paged;
use crate::Error;
use crate::PointCloud;
use crate::RecordDataType;
use crate::RecordName;

pub use self::converter::F32ToF64Converter;
pub use self::converter::IdentityConverter;
pub use self::converter::PropertyConverter;
pub use self::converter::ScaledIntConverter;
pub use self::converter::U8Converter;
pub use self::converter::UnitIntConverter;
pub use self::loader::F32Loader;
pub use self::loader::F64Loader;
pub use self::loader::IntLoader;
pub use self::loader::PropertyLoader;

pub trait PropertySaver<Point, V> {
	fn save(point: &mut Point, value: V);
}

pub trait PropertyReader<Point> {
	fn read(&mut self, mmap: &memmap2::Mmap, point: &mut Point, at_end: bool) -> Result<(), Error>;
}

pub struct GenPropertyReader<Loader, Saver, Converter, Point, V0, V1>
where
	Loader: PropertyLoader<V0>,
	Converter: PropertyConverter<V0, V1>,
	Saver: PropertySaver<Point, V1>,
{
	loader:    Loader,
	converter: Converter,
	_saver:    Saver,
	phantom:   std::marker::PhantomData<(Point, V0, V1)>,
}

impl<Loader, Saver, Converter, Point, V0, V1> GenPropertyReader<Loader, Saver, Converter, Point, V0, V1>
where
	Loader: PropertyLoader<V0>,
	Converter: PropertyConverter<V0, V1>,
	Saver: PropertySaver<Point, V1>,
{
	pub fn new(loader: Loader, converter: Converter, saver: Saver) -> Self {
		GenPropertyReader {
			loader,
			converter,
			_saver: saver,
			phantom: std::marker::PhantomData,
		}
	}

	pub fn boxed(loader: Loader, converter: Converter, saver: Saver) -> Box<Self> {
		Box::new(Self::new(loader, converter, saver))
	}
}

impl<Loader, Saver, Converter, Point, V0, V1> PropertyReader<Point>
	for GenPropertyReader<Loader, Saver, Converter, Point, V0, V1>
where
	Loader: PropertyLoader<V0>,
	Converter: PropertyConverter<V0, V1>,
	Saver: PropertySaver<Point, V1>,
{
	fn read(&mut self, mmap: &memmap2::Mmap, point: &mut Point, at_end: bool) -> Result<(), Error> {
		let value = self.loader.load(mmap, at_end)?;
		let value = self.converter.convert(value);
		Saver::save(point, value);
		Ok(())
	}
}

/// Iterate over all points of an existing point cloud to read it.
pub struct PointCloudReader<'a, Point>
where
	Point: Default,
{
	pc:   PointCloud,
	read: u64,

	property_readers: Vec<Box<dyn PropertyReader<Point>>>,
	mmap:             &'a memmap2::Mmap,
}

impl<'a, Point> PointCloudReader<'a, Point>
where
	Point: Default,
{
	pub(crate) fn new<F>(pc: &PointCloud, mmap: &'a memmap2::Mmap, f: F) -> Result<Self, Error>
	where
		F: Fn(
			RecordName,
			RecordDataType,
			usize,
			usize,
			&'a memmap2::Mmap,
		) -> Result<Option<Box<dyn PropertyReader<Point>>>, Error>,
	{
		let mut buffer = [0_u8; 32];
		mmap_paged::read(&mut buffer, pc.file_offset as usize, mmap);

		let section_id = buffer[0];
		let section_length = u64::from_le_bytes(buffer[8..16].try_into().expect(INTERNAL_ERROR));
		let data_offset = u64::from_le_bytes(buffer[16..24].try_into().expect(INTERNAL_ERROR));
		let _index_offset = u64::from_le_bytes(buffer[24..32].try_into().expect(INTERNAL_ERROR));

		if section_id != 1 {
			return Error::Invalid("Section ID of the compressed vector section header is not 1".into()).throw();
		}
		if section_length % 4 != 0 {
			return Error::Invalid("Section length is not aligned and a multiple of four".into()).throw();
		}

		let pc = pc.clone();

		let logical_offset = data_offset as usize;
		let logical_offset = logical_offset - (logical_offset / 1024) * 4;

		let mut property_readers = Vec::<Box<dyn PropertyReader<Point>>>::new();

		for (index, prototype) in pc.prototype.iter().enumerate() {
			let reader = match f(
				prototype.name,
				prototype.data_type,
				logical_offset,
				index,
				mmap,
			)? {
				Some(v) => v,
				None => continue,
			};
			property_readers.push(reader);
		}

		Ok(PointCloudReader { mmap, property_readers, pc, read: 0 })
	}
}

impl<'a, Point> Iterator for PointCloudReader<'a, Point>
where
	Point: Default,
{
	type Item = Result<Point, Error>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.read >= self.pc.records {
			return None;
		}
		let mut p = Point::default();
		let at_end = self.read >= self.pc.records - 1;
		for reader in self.property_readers.iter_mut() {
			if let Err(err) = reader.read(self.mmap, &mut p, at_end) {
				return Some(Err(err));
			}
		}
		self.read += 1;

		Some(Ok(p))
	}
}
