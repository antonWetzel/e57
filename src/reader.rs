use crate::mmap_paged;
use crate::pc_reader::PointCloudReader;
use crate::pc_reader::PropertyReader;
use crate::pointcloud::pointclouds_from_document;
use crate::root::root_from_document;
use crate::root::Root;
use crate::Error;
use crate::Header;
use crate::PointCloud;
use crate::RecordDataType;
use crate::RecordName;
use roxmltree::Document;
use std::fs::File;
use std::path::Path;

/// Main interface for reading E57 files.
pub struct Reader {
	mmap:        memmap2::Mmap,
	header:      Header,
	root:        Root,
	pointclouds: Vec<PointCloud>,
}

impl Reader {
	/// Creates a new E57 instance for from a reader.
	pub fn new(mut reader: File) -> Result<Self, Error> {
		// Read, parse and validate E57 header
		let header = Header::read(&mut reader)?;

		// Set up paged reader for the CRC page layer

		// Read and parse XML data
		let mut xml_raw = vec![0_u8; header.xml_length as usize];
		let mmap = unsafe { memmap2::MmapOptions::new().map(&reader)? };

		mmap_paged::read(&mut xml_raw, header.phys_xml_offset as usize, &mmap);

		let xml = String::from_utf8(xml_raw)?;
		let document = Document::parse(&xml)?;
		let root = root_from_document(&document)?;
		let pointclouds = pointclouds_from_document(&document)?;
		Ok(Self { mmap, header, root, pointclouds })
	}

	/// Returns the contents of E57 binary file header structure.
	pub fn header(&self) -> Header {
		self.header.clone()
	}

	/// Returns format name stored in the XML section.
	pub fn format_name(&self) -> &str {
		&self.root.format
	}

	/// Returns GUID stored in the XML section.
	pub fn guid(&self) -> &str {
		&self.root.guid
	}

	/// Returns a list of all point clouds in the file.
	pub fn pointclouds(&self) -> Vec<PointCloud> {
		self.pointclouds.clone()
	}

	/// Returns an iterator for the requested point cloud.
	pub fn pointcloud<F, Point>(&mut self, pc: &PointCloud, f: F) -> Result<PointCloudReader<Point>, Error>
	where
		Point: Default,
		F: Fn(
			RecordName,
			RecordDataType,
			usize,
			usize,
			&memmap2::Mmap,
		) -> Result<Option<Box<dyn PropertyReader<Point>>>, Error>,
	{
		PointCloudReader::new(pc, &self.mmap, f)
	}

	/// Returns the optional coordinate system metadata.
	///
	/// This should contain a Coordinate Reference System that is specified by
	/// a string in a well-known text format for a spatial reference system,
	/// as defined by the Coordinate Transformation Service specification
	/// developed by the Open Geospatial Consortium.
	/// See also: <https://www.ogc.org/standard/wkt-crs/>
	pub fn coordinate_metadata(&self) -> Option<&str> {
		self.root.coordinate_metadata.as_deref()
	}
}

impl Reader {
	/// Creates an E57 instance from a Path.
	pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
		let file = File::open(path)?;
		Self::new(file)
	}
}
