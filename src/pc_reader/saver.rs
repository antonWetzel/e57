use crate::Point;

pub trait PropertySaver<V> {
	// maybe: add '&self' for consistany, but it should never be nessecary
	fn save(point: &mut Point, value: V);
}

pub struct CartesionXSaver;
impl PropertySaver<f64> for CartesionXSaver {
	fn save(point: &mut Point, value: f64) {
		point.cartesian.x = value;
	}
}

pub struct CartesionYSaver;
impl PropertySaver<f64> for CartesionYSaver {
	fn save(point: &mut Point, value: f64) {
		point.cartesian.y = value;
	}
}

pub struct CartesionZSaver;
impl PropertySaver<f64> for CartesionZSaver {
	fn save(point: &mut Point, value: f64) {
		point.cartesian.z = value;
	}
}

pub struct ColorRedSaver;
impl PropertySaver<f32> for ColorRedSaver {
	fn save(point: &mut Point, value: f32) {
		point.color.red = value;
	}
}

pub struct ColorGreenSaver;
impl PropertySaver<f32> for ColorGreenSaver {
	fn save(point: &mut Point, value: f32) {
		point.color.green = value;
	}
}

pub struct ColorBlueSaver;
impl PropertySaver<f32> for ColorBlueSaver {
	fn save(point: &mut Point, value: f32) {
		point.color.blue = value;
	}
}

pub struct CartesionInvalidSaver;
impl PropertySaver<u8> for CartesionInvalidSaver {
	fn save(point: &mut Point, value: u8) {
		point.cartesian_invalid = value;
	}
}
