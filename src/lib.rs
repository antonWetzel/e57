#![doc = include_str!("../README.md")]
#![deny(
	clippy::unwrap_used,
	clippy::panic,
	clippy::large_stack_arrays,
	clippy::large_types_passed_by_value
)]
#![feature(thread_local)]

mod bounds;
mod error;
mod header;
mod mmap_paged;
mod pc_reader;
mod pointcloud;
mod reader;
mod record;
mod root;
mod transform;
mod xml;

pub use self::bounds::CartesianBounds;
pub use self::bounds::IndexBounds;
pub use self::bounds::SphericalBounds;
pub use self::error::Error;
pub use self::header::Header;
pub use self::pc_reader::*;
pub use self::pointcloud::PointCloud;
pub use self::reader::Reader;
pub use self::record::Record;
pub use self::record::RecordDataType;
pub use self::record::RecordName;
pub use self::record::RecordValue;
pub use self::transform::Quaternion;
pub use self::transform::Transform;
pub use self::transform::Translation;
