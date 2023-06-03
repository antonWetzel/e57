use crate::Error;
use roxmltree::Node;
use std::error::Error as StdError;
use std::fmt::{Debug, Display};
use std::str::FromStr;

/// Describes a record inside a E57 file with name and data type.
#[derive(Clone, Debug, Copy)]
pub struct Record {
	pub name:      RecordName,
	pub data_type: RecordDataType,
}

/// Basic primtive E57 data types that are used for the different point attributes.
#[derive(Clone, Debug, Copy)]
pub enum RecordDataType {
	/// 32-bit IEEE 754-2008 floating point value.
	Single { min: Option<f32>, max: Option<f32> },
	/// 64-bit IEEE 754-2008 floating point value.
	Double { min: Option<f64>, max: Option<f64> },
	/// Signed 64-bit integer scaled with a fixed 64-bit floating point value.
	ScaledInteger { min: i64, max: i64, scale: f64 },
	/// Signed 64-bit integer value.
	Integer { min: i64, max: i64 },
}

/// Used to describe the prototype records with all attributes that exit in the point cloud.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Copy)]
pub enum RecordName {
	/// Cartesian X coordinate (in meters).
	CartesianX,
	/// Cartesian Y coordinate (in meters).
	CartesianY,
	/// Cartesian Z coordinate (in meters).
	CartesianZ,
	/// Indicates whether the Cartesian coordinate or its magnitude is meaningful.
	/// Can have the value 0 (valid), 1 (XYZ is a direction vector) or 2 (invalid).
	CartesianInvalidState,

	/// Non-negative range (in meters) of the spherical coordinate.
	SphericalRange,
	/// Azimuth angle (in radians between -PI and PI) of the spherical coordinate.
	SphericalAzimuth,
	// Elevation angle (in radians between -PI/2 and PI/2) of the spherical coordinate.
	SphericalElevation,
	/// Indicates whether the spherical coordinate or its range is meaningful.
	/// Can have the value 0 (valid), 1 (range is not meaningful) or 2 (invalid).
	SphericalInvalidState,

	/// Point intensity. Unit is not specified.
	Intensity,
	/// Indicates whether the intensity value is meaningful.
	/// Can have the value 0 (valid) or 1 (invalid).
	IsIntensityInvalid,

	/// Red color value. Unit is not specified.
	ColorRed,
	/// Green color value. Unit is not specified.
	ColorGreen,
	/// Blue color value. Unit is not specified.
	ColorBlue,
	/// Indicates whether the color value is meaningful.
	/// Can have the value 0 (valid) or 1 (invalid).
	IsColorInvalid,

	/// Row number of the point (zero-based). Used for data that is stored in a grid.
	RowIndex,
	/// Column number of the point (zero-based). Used for data that is stored in a grid.
	ColumnIndex,

	/// For multi-return sensors. The total number of returns for the pulse that this point corresponds to.
	ReturnCount,
	/// For multi-return sensors. The number of this return (zero based). That is, 0 is the first, 1 is the second return etc.
	ReturnIndex,

	/// Non-negative time (in seconds) since the start time given by acquisition start in the parent point cloud.
	TimeStamp,
	/// Indicates whether the time stamp value is meaningful.
	/// Can have the value 0 (valid) or 1 (invalid).
	IsTimeStampInvalid,
}

/// Represents a raw value of records inside a point cloud.
///
/// For scaled integers the record data type with the scale is needed to calulcate the actual f64 value.
#[derive(Clone, Debug, PartialEq)]
pub enum RecordValue {
	Single(f32),
	Double(f64),
	ScaledInteger(i64),
	Integer(i64),
}

impl RecordName {
	pub(crate) fn from_tag_name(value: &str) -> Result<Self, Error> {
		Ok(match value {
			"cartesianX" => RecordName::CartesianX,
			"cartesianY" => RecordName::CartesianY,
			"cartesianZ" => RecordName::CartesianZ,
			"cartesianInvalidState" => RecordName::CartesianInvalidState,
			"sphericalRange" => RecordName::SphericalRange,
			"sphericalAzimuth" => RecordName::SphericalAzimuth,
			"sphericalElevation" => RecordName::SphericalElevation,
			"sphericalInvalidState" => RecordName::SphericalInvalidState,
			"intensity" => RecordName::Intensity,
			"isIntensityInvalid" => RecordName::IsIntensityInvalid,
			"colorRed" => RecordName::ColorRed,
			"colorGreen" => RecordName::ColorGreen,
			"colorBlue" => RecordName::ColorBlue,
			"isColorInvalid" => RecordName::IsColorInvalid,
			"rowIndex" => RecordName::RowIndex,
			"columnIndex" => RecordName::ColumnIndex,
			"returnCount" => RecordName::ReturnCount,
			"returnIndex" => RecordName::ReturnIndex,
			"timeStamp" => RecordName::TimeStamp,
			"isTimeStampInvalid" => RecordName::IsTimeStampInvalid,
			name => return Error::Unimplemented(format!("Found unknown record name: '{name}'")).throw(),
		})
	}
}

impl RecordDataType {
	pub(crate) fn from_node(node: &Node) -> Result<Self, Error> {
		let tag_name = node.tag_name().name();
		let type_name = node.attribute("type").ok_or(Error::Invalid(format!(
			"Missing type attribute for XML tag '{tag_name}'"
		)))?;
		Ok(match type_name {
			"Float" => {
				let precision = node.attribute("precision").unwrap_or("double");
				if precision == "double" {
					let min = optional_attribute(node, "minimum", tag_name, type_name)?;
					let max = optional_attribute(node, "maximum", tag_name, type_name)?;
					RecordDataType::Double { min, max }
				} else if precision == "single" {
					let min = optional_attribute(node, "minimum", tag_name, type_name)?;
					let max = optional_attribute(node, "maximum", tag_name, type_name)?;
					RecordDataType::Single { min, max }
				} else {
					return Error::Invalid(format!(
						"Float 'precision' attribute value '{precision}' for 'Float' type is unknown"
					))
					.throw();
				}
			},
			"Integer" => {
				let min = required_attribute(node, "minimum", tag_name, type_name)?;
				let max = required_attribute(node, "maximum", tag_name, type_name)?;
				if max <= min {
					return Error::Invalid(format!(
						"Maximum value '{max}' and minimum value '{min}' of type '{type_name}' in XML tag \
						 '{tag_name}' are inconsistent"
					))
					.throw();
				}
				RecordDataType::Integer { min, max }
			},
			"ScaledInteger" => {
				let min = required_attribute(node, "minimum", tag_name, type_name)?;
				let max = required_attribute(node, "maximum", tag_name, type_name)?;
				if max <= min {
					return Error::Invalid(format!(
						"Maximum value '{max}' and minimum value '{min}' of type '{type_name}' in XML tag \
						 '{tag_name}' are inconsistent"
					))
					.throw();
				}
				let scale = required_attribute(node, "scale", tag_name, type_name)?;
				RecordDataType::ScaledInteger { min, max, scale }
			},
			_ => {
				return Error::Unimplemented(format!(
					"Unsupported type '{type_name}' in XML tag '{tag_name}' detected"
				))
				.throw()
			},
		})
	}
}

impl RecordValue {
	// pub fn to_f64(&self, dt: &RecordDataType) -> Result<f64, Error> {
	// 	match self {
	// 		RecordValue::Single(s) => Ok(*s as f64),
	// 		RecordValue::Double(d) => Ok(*d),
	// 		RecordValue::ScaledInteger(i) => {
	// 			if let RecordDataType::ScaledInteger { scale, .. } = dt {
	// 				Ok(*i as f64 * *scale)
	// 			} else {
	// 				Error::internal("Tried to convert scaled integer value with wrong data type")
	// 			}
	// 		},
	// 		RecordValue::Integer(i) => Ok(*i as f64),
	// 	}
	// }

	// pub fn to_unit_f32(&self, dt: &RecordDataType) -> Result<f32, Error> {
	// 	match self {
	// 		RecordValue::Single(s) => {
	// 			if let RecordDataType::Single { min: Some(min), max: Some(max) } = dt {
	// 				Ok((s - min) / (max - min))
	// 			} else {
	// 				Error::internal("Tried to convert single value with wrong data type or without min/max")
	// 			}
	// 		},
	// 		RecordValue::Double(d) => {
	// 			if let RecordDataType::Double { min: Some(min), max: Some(max) } = dt {
	// 				Ok(((d - min) / (max - min)) as f32)
	// 			} else {
	// 				Error::internal("Tried to convert double value with wrong data type or without min/max")
	// 			}
	// 		},
	// 		RecordValue::ScaledInteger(si) => {
	// 			if let RecordDataType::ScaledInteger { min, max, .. } = dt {
	// 				Ok((si - min) as f32 / (max - min) as f32)
	// 			} else {
	// 				Error::internal("Tried to convert scaled integer value with wrong data type")
	// 			}
	// 		},
	// 		RecordValue::Integer(i) => {
	// 			if let RecordDataType::Integer { min, max } = dt {
	// 				Ok((i - min) as f32 / (max - min) as f32)
	// 			} else {
	// 				Error::internal("Tried to convert integer value with wrong data type")
	// 			}
	// 		},
	// 	}
	// }

	// pub fn to_u8(&self, dt: &RecordDataType) -> Result<u8, Error> {
	// 	if let (RecordValue::Integer(i), RecordDataType::Integer { min, max }) = (self, dt) {
	// 		if *min >= 0 && *max <= 255 {
	// 			Ok(*i as u8)
	// 		} else {
	// 			Error::internal("Integer range is too big for u8")
	// 		}
	// 	} else {
	// 		Error::internal("Tried to convert value to u8 with unsupported value or data type")
	// 	}
	// }

	// pub fn to_i64(&self, dt: &RecordDataType) -> Result<i64, Error> {
	// 	if let (RecordValue::Integer(i), RecordDataType::Integer { .. }) = (self, dt) {
	// 		Ok(*i)
	// 	} else {
	// 		Error::internal("Tried to convert value to i64 with unsupported data type")
	// 	}
	// }
}

impl Display for RecordValue {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			RecordValue::Single(v) => write!(f, "{v}"),
			RecordValue::Double(v) => write!(f, "{v}"),
			RecordValue::ScaledInteger(v) => write!(f, "{v}"),
			RecordValue::Integer(v) => write!(f, "{v}"),
		}
	}
}

fn optional_attribute<T>(node: &Node, attribute: &str, tag_name: &str, type_name: &str) -> Result<Option<T>, Error>
where
	T: FromStr,
	T::Err: StdError + Send + Sync + 'static,
{
	if let Some(attr) = node.attribute(attribute) {
		let parsed = attr.parse::<T>();
		let v = match parsed {
			Err(_) => {
				return Error::Invalid(format!(
					"Failed to parse attribute '{}' for type '{}' in XML tag '{}'",
					attribute, type_name, tag_name
				))
				.throw()
			},
			Ok(v) => v,
		};
		Ok(Some(v))
	} else {
		Ok(None)
	}
}

fn required_attribute<T>(node: &Node, attribute: &str, tag_name: &str, type_name: &str) -> Result<T, Error>
where
	T: FromStr,
	T::Err: StdError + Send + Sync + 'static,
{
	optional_attribute(node, attribute, tag_name, type_name)?.ok_or(Error::Invalid(format!(
		"Cannot find '{}' for type '{}' in XML tag '{}'",
		attribute, type_name, tag_name
	)))
}

impl RecordDataType {
	pub const F32: RecordDataType = RecordDataType::Single { min: None, max: None };

	pub const UNIT_F32: RecordDataType = RecordDataType::Single { min: Some(0.0), max: Some(1.0) };

	pub const F64: RecordDataType = RecordDataType::Double { min: None, max: None };

	pub const U8: RecordDataType = RecordDataType::Integer { min: 0, max: u8::MAX as i64 };

	pub const U16: RecordDataType = RecordDataType::Integer { min: 0, max: u16::MAX as i64 };
}

impl Record {
	pub const CARTESIAN_X_F32: Record = Record {
		name:      RecordName::CartesianX,
		data_type: RecordDataType::F32,
	};

	pub const CARTESIAN_Y_F32: Record = Record {
		name:      RecordName::CartesianY,
		data_type: RecordDataType::F32,
	};

	pub const CARTESIAN_Z_F32: Record = Record {
		name:      RecordName::CartesianZ,
		data_type: RecordDataType::F32,
	};

	pub const CARTESIAN_X_F64: Record = Record {
		name:      RecordName::CartesianX,
		data_type: RecordDataType::F64,
	};

	pub const CARTESIAN_Y_F64: Record = Record {
		name:      RecordName::CartesianY,
		data_type: RecordDataType::F64,
	};

	pub const CARTESIAN_Z_F64: Record = Record {
		name:      RecordName::CartesianZ,
		data_type: RecordDataType::F64,
	};

	pub const COLOR_RED_U8: Record = Record {
		name:      RecordName::ColorRed,
		data_type: RecordDataType::U8,
	};

	pub const COLOR_GREEN_U8: Record = Record {
		name:      RecordName::ColorGreen,
		data_type: RecordDataType::U8,
	};

	pub const COLOR_BLUE_U8: Record = Record {
		name:      RecordName::ColorBlue,
		data_type: RecordDataType::U8,
	};

	pub const INTENSITY_U16: Record = Record {
		name:      RecordName::Intensity,
		data_type: RecordDataType::U16,
	};

	pub const COLOR_RED_UNIT_F32: Record = Record {
		name:      RecordName::ColorRed,
		data_type: RecordDataType::UNIT_F32,
	};

	pub const COLOR_GREEN_UNIT_F32: Record = Record {
		name:      RecordName::ColorGreen,
		data_type: RecordDataType::UNIT_F32,
	};

	pub const COLOR_BLUE_UNIT_F32: Record = Record {
		name:      RecordName::ColorBlue,
		data_type: RecordDataType::UNIT_F32,
	};

	pub const INTENSITY_UNIT_F32: Record = Record {
		name:      RecordName::Intensity,
		data_type: RecordDataType::UNIT_F32,
	};
}
