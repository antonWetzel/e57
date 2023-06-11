use crate::xml::required_double;
use crate::Error;
use roxmltree::Node;

/// Describes the rotation of a point cloud.
#[derive(Clone, Debug)]
pub struct Quaternion {
	/// The scalar part of the quaternion. Shall be nonnegative.
	pub w: f64,
	/// The i coefficient of the quaternion.
	pub x: f64,
	/// The j coefficient of the quaternion.
	pub y: f64,
	/// The k coefficient of the quaternion.
	pub z: f64,
}

/// Describes the translation of a point cloud.
#[derive(Clone, Debug)]
pub struct Translation {
	/// The X coordinate of the translation in meters.
	pub x: f64,
	/// The Y coordinate of the translation in meters.
	pub y: f64,
	/// The Z coordinate of the translation in meters.
	pub z: f64,
}

/// Describes a transformation of a point cloud with a rotation and translation component.
#[derive(Clone, Debug)]
pub struct Transform {
	/// A unit quaternion representing the rotation of the transform.
	pub rotation:    Quaternion,
	/// The translation of the transform.
	pub translation: Translation,
}

pub fn transform_from_node(node: &Node) -> Result<Transform, Error> {
	let translation = match node.children().find(|n| n.has_tag_name("translation")) {
		Some(node) => translation_from_node(&node)?,
		None => Translation { x: 0.0, y: 0.0, z: 0.0 },
	};
	let rotation = match node.children().find(|n| n.has_tag_name("rotation")) {
		None => Quaternion { w: 1.0, x: 0.0, y: 0.0, z: 0.0 },
		Some(node) => quaternion_from_node(&node)?,
	};
	Ok(Transform { rotation, translation })
}

pub fn quaternion_from_node(node: &Node) -> Result<Quaternion, Error> {
	let w = required_double(node, "w")?;
	let x = required_double(node, "x")?;
	let y = required_double(node, "y")?;
	let z = required_double(node, "z")?;
	Ok(Quaternion { w, x, y, z })
}

pub fn translation_from_node(node: &Node) -> Result<Translation, Error> {
	let x = required_double(node, "x")?;
	let y = required_double(node, "y")?;
	let z = required_double(node, "z")?;
	Ok(Translation { x, y, z })
}
