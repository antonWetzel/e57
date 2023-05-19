use std::collections::VecDeque;

use crate::bs_read::ByteStreamReadBuffer;
use crate::error::Converter;
use crate::error::WRONG_OFFSET;
use crate::Error;
use crate::RecordValue;
use crate::Result;

#[inline]
fn unpack_int<Conv: Fn(i64) -> RecordValue>(
	stream: &mut ByteStreamReadBuffer,
	min: i64,
	max: i64,
	queue: &mut VecDeque<RecordValue>,
	conv: Conv,
) -> Result<()> {
	let range = max - min;
	let bit_size = f64::ceil(f64::log2(range as f64 + 1.0)) as u64;
	if bit_size > 56 && bit_size != 64 {
		// These values can require 9 bytes before alignment
		// which would not fit into the u64 used for decoding!
		Error::not_implemented(format!("Integers with {bit_size} bits are not supported"))?
	}
	let mask = (1u64 << bit_size) - 1;
	loop {
		let value = match stream.extract_int(bit_size, min, mask) {
			Some(v) => v,
			None => break,
		};
		queue.push_back(conv(value));
	}
	return Ok(());
}

pub fn unpack_doubles(stream: &mut ByteStreamReadBuffer, queue: &mut VecDeque<RecordValue>) -> Result<()> {
	let av_bits = stream.available();
	let bits = 64;
	if av_bits % bits != 0 {
		Error::invalid(format!(
			"Available bits {av_bits} do not match expected type size of {bits} bits"
		))?
	}
	loop {
		let v = match stream.extract_f64() {
			Some(v) => v,
			None => break,
		};
		queue.push_back(RecordValue::Double(v));
	}
	return Ok(());
}

pub fn unpack_singles(stream: &mut ByteStreamReadBuffer, queue: &mut VecDeque<RecordValue>) -> Result<()> {
	let av_bits = stream.available();
	let bits = 32;
	if av_bits % bits != 0 {
		Error::invalid(format!(
			"Available bits {av_bits} do not match expected type size of {bits} bits"
		))?
	}
	loop {
		let v = match stream.extract_f32() {
			Some(v) => v,
			None => break,
		};
		queue.push_back(RecordValue::Single(v));
	}
	return Ok(());
}

pub fn unpack_ints(
	stream: &mut ByteStreamReadBuffer,
	min: i64,
	max: i64,
	queue: &mut VecDeque<RecordValue>,
) -> Result<()> {
	return unpack_int(stream, min, max, queue, |i| RecordValue::Integer(i));
}

pub fn unpack_scaled_ints(
	stream: &mut ByteStreamReadBuffer,
	min: i64,
	max: i64,
	queue: &mut VecDeque<RecordValue>,
) -> Result<()> {
	return unpack_int(stream, min, max, queue, |i| RecordValue::ScaledInteger(i));
}

trait FromBytes: Sized {
	fn from_le_bytes(bytes: &[u8]) -> Result<Self>;
	fn bits() -> u64 {
		std::mem::size_of::<Self>() as u64 * 8
	}
}

impl FromBytes for f64 {
	#[inline]
	fn from_le_bytes(bytes: &[u8]) -> Result<Self> {
		Ok(f64::from_le_bytes(
			bytes.try_into().internal_err(WRONG_OFFSET)?,
		))
	}
}

impl FromBytes for f32 {
	#[inline]
	fn from_le_bytes(bytes: &[u8]) -> Result<Self> {
		Ok(f32::from_le_bytes(
			bytes.try_into().internal_err(WRONG_OFFSET)?,
		))
	}
}
