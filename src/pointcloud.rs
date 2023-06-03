use crate::xml::{optional_double, optional_string, optional_transform, required_string};
use crate::{CartesianBounds, Error, IndexBounds, Record, RecordDataType, RecordName, SphericalBounds, Transform};
use roxmltree::{Document, Node};

/// Descriptor with metadata for a single point cloud.
///
/// This struct does not contain any actual point data,
/// it just describes the properties and attributes of a point cloud.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct PointCloud {
	/// Globally unique identifier for the point cloud.
	pub guid:        String,
	/// Physical file offset of the start of the associated binary section.
	pub file_offset: u64,
	/// Number of points in the point cloud.
	pub records:     u64,
	/// List of point attributes that exist for this point cloud.
	pub prototype:   Vec<Record>,

	/// Optional user-defined name for the point cloud.
	pub name:                 Option<String>,
	/// Optional user-defined description of the point cloud.
	pub description:          Option<String>,
	/// Optional Cartesian bounds for the point cloud.
	pub cartesian_bounds:     Option<CartesianBounds>,
	/// Optional spherical bounds for the point cloud.
	pub spherical_bounds:     Option<SphericalBounds>,
	/// Optional index bounds (row, column, return values) for the point cloud.
	pub index_bounds:         Option<IndexBounds>,
	/// Optional transformation to convert data from the local point cloud coordinates to the file-level coordinate system.
	pub transform:            Option<Transform>,
	/// Optional name of the manufacturer for the sensor used to capture the point cloud.
	pub sensor_vendor:        Option<String>,
	/// Optional model name of the sensor used for capturing.
	pub sensor_model:         Option<String>,
	/// Optional serial number of the sensor used for capturing.
	pub sensor_serial:        Option<String>,
	/// Optional version identifier for the sensor hardware used for capturing.
	pub sensor_hw_version:    Option<String>,
	/// Optional version identifier for the sensor software used for capturing.
	pub sensor_sw_version:    Option<String>,
	/// Optional version identifier for the sensor firmware used for capturing.
	pub sensor_fw_version:    Option<String>,
	/// Optional ambient temperature in degrees Celsius, measured at the sensor at the time of capturing.
	pub temperature:          Option<f64>,
	/// Optional percentage of relative humidity between 0 and 100, measured at the sensor at the time of capturing.
	pub humidity:             Option<f64>,
	/// Optional atmospheric pressure in Pascals, measured at the sensor at the time of capturing.
	pub atmospheric_pressure: Option<f64>,
}

pub fn pointclouds_from_document(document: &Document) -> Result<Vec<PointCloud>, Error> {
	let data3d_node = document
		.descendants()
		.find(|n| n.has_tag_name("data3D"))
		.ok_or(Error::Invalid(
			"Cannot find 'data3D' tag in XML document".into(),
		))?;

	let mut pointclouds = Vec::new();
	for n in data3d_node.children() {
		if n.has_tag_name("vectorChild") && n.attribute("type") == Some("Structure") {
			let pointcloud = extract_pointcloud(&n)?;
			pointclouds.push(pointcloud);
		}
	}
	Ok(pointclouds)
}

fn extract_pointcloud(node: &Node) -> Result<PointCloud, Error> {
	let guid = required_string(node, "guid")?;
	let name = optional_string(node, "name")?;
	let description = optional_string(node, "description")?;
	let sensor_model = optional_string(node, "sensorModel")?;
	let sensor_vendor = optional_string(node, "sensorVendor")?;
	let sensor_serial = optional_string(node, "sensorSerialNumber")?;
	let sensor_hw_version = optional_string(node, "sensorHardwareVersion")?;
	let sensor_sw_version = optional_string(node, "sensorSoftwareVersion")?;
	let sensor_fw_version = optional_string(node, "sensorFirmwareVersion")?;
	let temperature = optional_double(node, "temperature")?;
	let humidity = optional_double(node, "relativeHumidity")?;
	let atmospheric_pressure = optional_double(node, "atmosphericPressure")?;
	let transform = optional_transform(node, "pose")?;
	let cartesian_bounds = node.children().find(|n| n.has_tag_name("cartesianBounds"));
	let spherical_bounds = node.children().find(|n| n.has_tag_name("sphericalBounds"));
	let index_bounds = node.children().find(|n| n.has_tag_name("indexBounds"));

	let points_tag = node
		.children()
		.find(|n| n.has_tag_name("points") && n.attribute("type") == Some("CompressedVector"))
		.ok_or(Error::Invalid(
			"Cannot find 'points' tag inside 'data3D' child".into(),
		))?;
	let file_offset = points_tag
		.attribute("fileOffset")
		.ok_or(Error::Invalid(
			"Cannot find 'fileOffset' attribute in 'points' tag".into(),
		))?
		.parse::<u64>()?;
	let records = points_tag
		.attribute("recordCount")
		.ok_or(Error::Invalid(
			"Cannot find 'recordCount' attribute in 'points' tag".into(),
		))?
		.parse::<u64>()?;
	let prototype_tag = points_tag
		.children()
		.find(|n| n.has_tag_name("prototype") && n.attribute("type") == Some("Structure"))
		.ok_or(Error::Invalid(
			"Cannot find 'prototype' child in 'points' tag".into(),
		))?;
	let mut prototype = Vec::new();
	for n in prototype_tag.children() {
		if n.is_element() {
			let tag_name = n.tag_name().name();
			let name = RecordName::from_tag_name(tag_name)?;
			let data_type = RecordDataType::from_node(&n)?;
			prototype.push(Record { name, data_type });
		}
	}

	Ok(PointCloud {
		guid,
		name,
		file_offset,
		records,
		prototype,
		cartesian_bounds: if let Some(node) = cartesian_bounds {
			Some(CartesianBounds::from_node(&node)?)
		} else {
			None
		},
		spherical_bounds: if let Some(node) = spherical_bounds {
			Some(SphericalBounds::from_node(&node)?)
		} else {
			None
		},
		index_bounds: if let Some(node) = index_bounds {
			Some(IndexBounds::from_node(&node)?)
		} else {
			None
		},
		transform,
		description,
		sensor_vendor,
		sensor_model,
		sensor_serial,
		sensor_hw_version,
		sensor_sw_version,
		sensor_fw_version,
		temperature,
		humidity,
		atmospheric_pressure,
	})
}
