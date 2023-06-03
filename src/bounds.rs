use crate::xml::optional_double;
use crate::xml::optional_integer;
use crate::Error;
use roxmltree::Node;

/// Optional minimum and maximum values for Cartesian X, Y and Z coordinates.
#[derive(Clone, Debug, Default)]
pub struct CartesianBounds {
	pub x_min: Option<f64>,
	pub x_max: Option<f64>,
	pub y_min: Option<f64>,
	pub y_max: Option<f64>,
	pub z_min: Option<f64>,
	pub z_max: Option<f64>,
}

impl CartesianBounds {
	pub(crate) fn from_node(node: &Node) -> Result<Self, Error> {
		let x_min = optional_double(node, "xMinimum")?;
		let x_max = optional_double(node, "xMaximum")?;
		let y_min = optional_double(node, "yMinimum")?;
		let y_max = optional_double(node, "yMaximum")?;
		let z_min = optional_double(node, "zMinimum")?;
		let z_max = optional_double(node, "zMaximum")?;
		Ok(Self { x_min, x_max, y_min, y_max, z_min, z_max })
	}
}

/// Optional minimum and maximum values for spherical coordinates.
#[derive(Clone, Debug, Default)]
pub struct SphericalBounds {
	pub range_min:     Option<f64>,
	pub range_max:     Option<f64>,
	pub elevation_min: Option<f64>,
	pub elevation_max: Option<f64>,
	pub azimuth_start: Option<f64>,
	pub azimuth_end:   Option<f64>,
}

impl SphericalBounds {
	pub(crate) fn from_node(node: &Node) -> Result<Self, Error> {
		let range_min = optional_double(node, "rangeMinimum")?;
		let range_max = optional_double(node, "rangeMaximum")?;
		let elevation_min = optional_double(node, "elevationMinimum")?;
		let elevation_max = optional_double(node, "elevationMaximum")?;
		let azimuth_start = optional_double(node, "azimuthStart")?;
		let azimuth_end = optional_double(node, "azimuthEnd")?;
		Ok(Self {
			range_min,
			range_max,
			elevation_min,
			elevation_max,
			azimuth_start,
			azimuth_end,
		})
	}
}

/// Optional minimum and maximum values for the row, column and return indices.
#[derive(Clone, Debug, Default)]
pub struct IndexBounds {
	pub row_min:    Option<i64>,
	pub row_max:    Option<i64>,
	pub column_min: Option<i64>,
	pub column_max: Option<i64>,
	pub return_min: Option<i64>,
	pub return_max: Option<i64>,
}

impl IndexBounds {
	pub(crate) fn from_node(node: &Node) -> Result<Self, Error> {
		let row_min = optional_integer(node, "rowMinimum")?;
		let row_max = optional_integer(node, "rowMaximum")?;
		let column_min = optional_integer(node, "columnMinimum")?;
		let column_max = optional_integer(node, "columnMaximum")?;
		let return_min = optional_integer(node, "returnMinimum")?;
		let return_max = optional_integer(node, "returnMaximum")?;
		Ok(Self {
			row_min,
			row_max,
			column_min,
			column_max,
			return_min,
			return_max,
		})
	}
}
