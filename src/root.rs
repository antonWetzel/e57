use crate::{
	xml::{optional_string, required_integer, required_string},
	Error,
};
use roxmltree::Document;

/// E57 XML Root structure with information shared by all elements in the file.
#[derive(Debug)]
#[non_exhaustive]
pub struct Root {
	pub format:              String,
	pub guid:                String,
	pub major_version:       i64,
	pub minor_version:       i64,
	pub library_version:     Option<String>,
	pub coordinate_metadata: Option<String>,
}

impl Default for Root {
	fn default() -> Self {
		Self {
			format:              String::from("ASTM E57 3D Imaging Data File"),
			guid:                String::new(),
			major_version:       1,
			minor_version:       0,
			coordinate_metadata: None,
			library_version:     None,
		}
	}
}

pub fn root_from_document(document: &Document) -> Result<Root, Error> {
	let root = document
		.descendants()
		.find(|n| n.has_tag_name("e57Root"))
		.ok_or(Error::Invalid(
			"Unable to find e57Root tag in XML document".into(),
		))?;

	// Required fields
	let format = required_string(&root, "formatName")?;
	let guid = required_string(&root, "guid")?;
	let major_version = required_integer(&root, "versionMajor")?;
	let minor_version = required_integer(&root, "versionMajor")?;

	// Optional fields
	let coordinate_metadata = optional_string(&root, "coordinateMetadata")?;
	let library_version = optional_string(&root, "e57LibraryVersion")?;

	Ok(Root {
		format,
		guid,
		major_version,
		minor_version,
		coordinate_metadata,
		library_version,
	})
}
