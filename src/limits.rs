use crate::error::Converter;
use crate::Error;
use crate::RecordValue;
use crate::Result;
use roxmltree::Node;

fn extract_limit(bounds: &Node, tag_name: &str) -> Result<Option<RecordValue>> {
	if let Some(tag) = bounds.descendants().find(|n| n.has_tag_name(tag_name)) {
		let type_str = tag
			.attribute("type")
			.invalid_err(format!("Cannot find type attribute of limit '{tag_name}'"))?;
		let value_str = tag.text().unwrap_or("0");
		Ok(match type_str {
			"Integer" => Some(RecordValue::Integer(
				value_str
					.parse::<i64>()
					.invalid_err("Cannot parse integer limit value")?,
			)),
			"ScaledInteger" => Some(RecordValue::ScaledInteger(
				value_str
					.parse::<i64>()
					.invalid_err("Cannot parse scaled integer limit value")?,
			)),
			"Float" => {
				let single = tag.attribute("precision").unwrap_or("double") == "single";
				if single {
					Some(RecordValue::Single(
						value_str
							.parse::<f32>()
							.invalid_err("Cannot parse single limit value")?,
					))
				} else {
					Some(RecordValue::Double(
						value_str
							.parse::<f64>()
							.invalid_err("Cannot parse double limit value")?,
					))
				}
			},
			_ => Error::not_implemented(format!(
				"Found unsupported limit of type '{type_str}' for '{tag_name}'"
			))?,
		})
	} else {
		Ok(None)
	}
}

/// Optional minimum and maximum values for intensity.
#[derive(Clone, Debug)]
pub struct IntensityLimits {
	pub intensity_min: Option<RecordValue>,
	pub intensity_max: Option<RecordValue>,
}

impl IntensityLimits {
	pub(crate) fn from_node(node: &Node) -> Result<Self> {
		let intensity_min = extract_limit(node, "intensityMinimum")?;
		let intensity_max = extract_limit(node, "intenstiyMaximum")?;
		Ok(Self { intensity_min, intensity_max })
	}
}

/// Optional minimum and maximum values for the colors red, green and blue.
#[derive(Clone, Debug)]
pub struct ColorLimits {
	pub red_min:   Option<RecordValue>,
	pub red_max:   Option<RecordValue>,
	pub green_min: Option<RecordValue>,
	pub green_max: Option<RecordValue>,
	pub blue_min:  Option<RecordValue>,
	pub blue_max:  Option<RecordValue>,
}

impl ColorLimits {
	pub(crate) fn from_node(node: &Node) -> Result<Self> {
		let red_min = extract_limit(node, "colorRedMinimum")?;
		let red_max = extract_limit(node, "colorRedMaximum")?;
		let green_min = extract_limit(node, "colorGreenMinimum")?;
		let green_max = extract_limit(node, "colorGreenMaximum")?;
		let blue_min = extract_limit(node, "colorBlueMinimum")?;
		let blue_max = extract_limit(node, "colorBlueMaximum")?;
		Ok(Self {
			red_min,
			red_max,
			green_min,
			green_max,
			blue_min,
			blue_max,
		})
	}
}
