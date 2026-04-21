//! Command module containing all CLI subcommands
//!
//! Each command is implemented in its own module file for
//! better organization and maintainability.

pub mod compile_map;
pub mod extract_overture;
pub mod extract_osm;
pub mod optimize;
pub mod clean;
pub mod validate;
pub mod pipeline;
pub mod status;
pub mod logs;
