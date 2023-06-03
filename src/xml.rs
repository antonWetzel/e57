use crate::{transform::transform_from_node, Error, Transform};
use roxmltree::Node;
use std::str::FromStr;

pub fn optional_string(parent_node: &Node, tag_name: &str) -> Result<Option<String>, Error> {
	if let Some(tag) = parent_node.children().find(|n| n.has_tag_name(tag_name)) {
		let expected_type = "String";
		if let Some(found_type) = tag.attribute("type") {
			if found_type != expected_type {
				return Error::Invalid(format!(
					"Found XML tag '{tag_name}' with type '{found_type}' instead of '{expected_type}'"
				))
				.throw();
			}
		} else {
			return Error::Invalid(format!("XML tag '{tag_name}' has no 'type' attribute")).throw();
		}
		let text = tag.text().unwrap_or("");
		Ok(Some(text.to_string()))
	} else {
		Ok(None)
	}
}

pub fn required_string(parent_node: &Node, tag_name: &str) -> Result<String, Error> {
	optional_string(parent_node, tag_name)?.ok_or(Error::Invalid(format!(
		"XML tag '{tag_name}' was not found"
	)))
}

fn optional_number<T: FromStr + Sync + Send>(
	parent_node: &Node,
	tag_name: &str,
	expected_type: &str,
) -> Result<Option<T>, Error> {
	let tag = match parent_node.children().find(|n| n.has_tag_name(tag_name)) {
		Some(tag) => tag,
		None => return Ok(None),
	};

	
	if let Some(found_type) = tag.attribute("type") {
		if found_type != expected_type {
			return Error::Invalid(format!(
				"Found XML tag '{tag_name}' with type '{found_type}' instead of '{expected_type}'"
			))
			.throw();
		}
	} else {
		return Error::Invalid(format!("XML tag '{tag_name}' has no 'type' attribute")).throw();
	}
	let text = tag.text().unwrap_or("0");
	if let Ok(parsed) = text.parse::<T>() {
		Ok(Some(parsed))
	} else {
		Error::Invalid(format!(
			"Cannot parse value '{text}' of XML tag '{tag_name}' as '{expected_type}'"
		))
		.throw()
	}
}

pub fn optional_double(parent_node: &Node, tag_name: &str) -> Result<Option<f64>, Error> {
	optional_number(parent_node, tag_name, "Float")
}

pub fn required_double(parent_node: &Node, tag_name: &str) -> Result<f64, Error> {
	optional_number(parent_node, tag_name, "Float")?.ok_or(Error::Invalid(format!(
		"XML tag '{tag_name}' was not found"
	)))
}

pub fn optional_integer<T: FromStr + Sync + Send>(parent_node: &Node, tag_name: &str) -> Result<Option<T>, Error> {
	optional_number(parent_node, tag_name, "Integer")
}

pub fn required_integer<T: FromStr + Send + Sync>(parent_node: &Node, tag_name: &str) -> Result<T, Error> {
	optional_number(parent_node, tag_name, "Integer")?.ok_or(Error::Invalid(format!(
		"XML tag '{tag_name}' was not found"
	)))
}

pub fn optional_transform(parent_node: &Node, tag_name: &str) -> Result<Option<Transform>, Error> {
	let node = parent_node.children().find(|n| n.has_tag_name(tag_name));
	if let Some(node) = node {
		Ok(Some(transform_from_node(&node)?))
	} else {
		Ok(None)
	}
}
