//! A pure Rust library for reading E57 files without unsafe code.
//!
//! Some example code can be found [here](https://github.com/cry-inc/e57/tree/master/tools) in the GitHub repository.
// #![forbid(unsafe_code)]
#![deny(
	clippy::unwrap_used,
	clippy::expect_used,
	clippy::panic,
	clippy::large_stack_arrays,
	clippy::large_types_passed_by_value
)]
#![feature(thread_local)]

mod blob;
mod bounds;
mod crc32;
mod cv_section;
mod date_time;
mod e57_reader;
mod error;
mod header;
mod limits;
mod paged_reader;
mod paged_writer;
mod pc_reader;
mod point;
mod pointcloud;
mod record;
mod root;
mod transform;
mod xml;

pub use self::bounds::CartesianBounds;
pub use self::bounds::IndexBounds;
pub use self::bounds::SphericalBounds;
pub use self::date_time::DateTime;
pub use self::e57_reader::E57Reader;
pub use self::error::Error;
pub use self::error::Result;
pub use self::header::Header;
pub use self::limits::ColorLimits;
pub use self::limits::IntensityLimits;
pub use self::pc_reader::PointCloudReader;
pub use self::point::CartesianCoordinate;
pub use self::point::Color;
pub use self::point::Point;
pub use self::point::SphericalCoordinate;
pub use self::pointcloud::PointCloud;
pub use self::record::Record;
pub use self::record::RecordDataType;
pub use self::record::RecordName;
pub use self::record::RecordValue;
pub use self::transform::Quaternion;
pub use self::transform::Transform;
pub use self::transform::Translation;

/// Storage container for a low level point data.
pub type RawValues = Vec<RecordValue>;
