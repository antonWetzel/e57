pub trait PropertyConverter<V0, V1> {
	fn convert(&self, v: V0) -> V1;
}

pub struct ScaledIntConverter {
	pub scale: f64,
}

impl PropertyConverter<i64, f64> for ScaledIntConverter {
	fn convert(&self, v: i64) -> f64 {
		v as f64 * self.scale
	}
}

pub struct UnitIntConverter {
	pub min: i64,
	pub max: i64,
}

impl PropertyConverter<i64, f32> for UnitIntConverter {
	fn convert(&self, v: i64) -> f32 {
		(v - self.min) as f32 / (self.max - self.min) as f32
	}
}

pub struct U8Converter;
impl PropertyConverter<i64, u8> for U8Converter {
	fn convert(&self, v: i64) -> u8 {
		v as u8
	}
}

pub struct IdentityConverter;
impl<V> PropertyConverter<V, V> for IdentityConverter {
	fn convert(&self, v: V) -> V {
		v
	}
}

pub struct F32ToF64Converter;
impl PropertyConverter<f32, f64> for F32ToF64Converter {
	fn convert(&self, v: f32) -> f64 {
		v as f64
	}
}
