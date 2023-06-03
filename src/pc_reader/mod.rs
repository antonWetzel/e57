mod converter;
mod loader;
mod saver;

use crate::error::INTERNAL_ERROR;
use crate::mmap_paged;
use crate::Error;
use crate::Point;
use crate::PointCloud;
use crate::RecordDataType;
use crate::RecordName;

use self::converter::F32ToF64Converter;
use self::converter::IdentityConverter;
use self::converter::PropertyConverter;
use self::converter::ScaledIntConverter;
use self::converter::U8Converter;
use self::converter::UnitIntConverter;
use self::loader::F32Loader;
use self::loader::F64Loader;
use self::loader::IntLoader;
use self::loader::PropertyLoader;
use self::saver::CartesionInvalidSaver;
use self::saver::CartesionXSaver;
use self::saver::CartesionYSaver;
use self::saver::CartesionZSaver;
use self::saver::ColorBlueSaver;
use self::saver::ColorGreenSaver;
use self::saver::ColorRedSaver;
use self::saver::PropertySaver;

trait PropertyReader {
	fn read(&mut self, mmap: &memmap2::Mmap, point: &mut Point, at_end: bool) -> Result<(), Error>;
}

struct GenPropertyReader<Loader, Saver, Converter, V0, V1>
where
	Loader: PropertyLoader<V0>,
	Converter: PropertyConverter<V0, V1>,
	Saver: PropertySaver<V1>,
{
	loader:    Loader,
	converter: Converter,
	_saver:    Saver,
	phantom:   std::marker::PhantomData<(V0, V1)>,
}

impl<Loader, Saver, Converter, V0, V1> GenPropertyReader<Loader, Saver, Converter, V0, V1>
where
	Loader: PropertyLoader<V0>,
	Converter: PropertyConverter<V0, V1>,
	Saver: PropertySaver<V1>,
{
	fn new(loader: Loader, converter: Converter, saver: Saver) -> Self {
		GenPropertyReader {
			loader,
			converter,
			_saver: saver,
			phantom: std::marker::PhantomData,
		}
	}

	fn boxed(loader: Loader, converter: Converter, saver: Saver) -> Box<Self> {
		Box::new(Self::new(loader, converter, saver))
	}
}

impl<Loader, Saver, Converter, V0, V1> PropertyReader for GenPropertyReader<Loader, Saver, Converter, V0, V1>
where
	Loader: PropertyLoader<V0>,
	Converter: PropertyConverter<V0, V1>,
	Saver: PropertySaver<V1>,
{
	fn read(&mut self, mmap: &memmap2::Mmap, point: &mut Point, at_end: bool) -> Result<(), Error> {
		let value = self.loader.load(mmap, at_end)?;
		let value = self.converter.convert(value);
		Saver::save(point, value);
		Ok(())
	}
}

/// Iterate over all points of an existing point cloud to read it.
pub struct PointCloudReader<'a> {
	pc:   PointCloud,
	read: u64,

	property_readers: Vec<Box<dyn PropertyReader>>,
	mmap:             &'a memmap2::Mmap,
}

impl<'a> PointCloudReader<'a> {
	pub(crate) fn new(pc: &PointCloud, mmap: &'a memmap2::Mmap) -> Result<Self, Error> {
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

		let mut property_readers = Vec::<Box<dyn PropertyReader>>::new();

		for (index, prototype) in pc.prototype.iter().enumerate() {
			let reader: Box<dyn PropertyReader> = match (prototype.name, prototype.data_type) {
				(RecordName::CartesianX, RecordDataType::ScaledInteger { min, max, scale }) => {
					GenPropertyReader::boxed(
						IntLoader::new(logical_offset, index, min, max, mmap)?,
						ScaledIntConverter { scale },
						CartesionXSaver,
					)
				},
				(RecordName::CartesianX, RecordDataType::Double { min: _, max: _ }) => GenPropertyReader::boxed(
					F64Loader::new(logical_offset, index, mmap)?,
					IdentityConverter,
					CartesionXSaver,
				),
				(RecordName::CartesianX, RecordDataType::Single { min: _, max: _ }) => GenPropertyReader::boxed(
					F32Loader::new(logical_offset, index, mmap)?,
					F32ToF64Converter,
					CartesionXSaver,
				),
				(RecordName::CartesianY, RecordDataType::ScaledInteger { min, max, scale }) => {
					GenPropertyReader::boxed(
						IntLoader::new(logical_offset, index, min, max, mmap)?,
						ScaledIntConverter { scale },
						CartesionYSaver,
					)
				},
				(RecordName::CartesianY, RecordDataType::Double { min: _, max: _ }) => GenPropertyReader::boxed(
					F64Loader::new(logical_offset, index, mmap)?,
					IdentityConverter,
					CartesionYSaver,
				),
				(RecordName::CartesianY, RecordDataType::Single { min: _, max: _ }) => GenPropertyReader::boxed(
					F32Loader::new(logical_offset, index, mmap)?,
					F32ToF64Converter,
					CartesionYSaver,
				),
				(RecordName::CartesianZ, RecordDataType::ScaledInteger { min, max, scale }) => {
					GenPropertyReader::boxed(
						IntLoader::new(logical_offset, index, min, max, mmap)?,
						ScaledIntConverter { scale },
						CartesionZSaver,
					)
				},
				(RecordName::CartesianZ, RecordDataType::Double { min: _, max: _ }) => GenPropertyReader::boxed(
					F64Loader::new(logical_offset, index, mmap)?,
					IdentityConverter,
					CartesionZSaver,
				),
				(RecordName::CartesianZ, RecordDataType::Single { min: _, max: _ }) => GenPropertyReader::boxed(
					F32Loader::new(logical_offset, index, mmap)?,
					F32ToF64Converter,
					CartesionZSaver,
				),
				(RecordName::ColorRed, RecordDataType::Integer { min, max }) => GenPropertyReader::boxed(
					IntLoader::new(logical_offset, index, min, max, mmap)?,
					UnitIntConverter { min, max },
					ColorRedSaver,
				),
				(RecordName::ColorGreen, RecordDataType::Integer { min, max }) => GenPropertyReader::boxed(
					IntLoader::new(logical_offset, index, min, max, mmap)?,
					UnitIntConverter { min, max },
					ColorGreenSaver,
				),
				(RecordName::ColorBlue, RecordDataType::Integer { min, max }) => GenPropertyReader::boxed(
					IntLoader::new(logical_offset, index, min, max, mmap)?,
					UnitIntConverter { min, max },
					ColorBlueSaver,
				),
				(RecordName::Intensity, _) => continue,
				(RecordName::RowIndex, _) => continue,
				(RecordName::ColumnIndex, _) => continue,
				(RecordName::CartesianInvalidState, RecordDataType::Integer { min, max }) => GenPropertyReader::boxed(
					IntLoader::new(logical_offset, index, min, max, mmap)?,
					U8Converter,
					CartesionInvalidSaver,
				),
				(name, data_type) => unimplemented!("not handled or ignored: {:?} {:?}", name, data_type),
			};
			property_readers.push(reader);
		}

		Ok(PointCloudReader { mmap, property_readers, pc, read: 0 })
	}
}

impl<'a> Iterator for PointCloudReader<'a> {
	type Item = Result<Point, Error>;

	fn next(&mut self) -> Option<Self::Item> {
		while self.read < self.pc.records {
			let mut p = Point::default();
			let at_end = self.read >= self.pc.records - 1;
			for reader in self.property_readers.iter_mut() {
				if let Err(err) = reader.read(self.mmap, &mut p, at_end) {
					return Some(Err(err));
				}
			}
			self.read += 1;

			if p.cartesian_invalid != 0 {
				continue;
			}

			return Some(Ok(p));
		}
		None
	}
}
