//! Platform-free core of TheOrbit.
//!
//! Everything here is plain Rust so it can be shared by the desktop app today
//! and by the mobile targets later.

pub mod config;
pub mod geometry;

pub use config::{ActionType, OrbitConfig, OrbitItem, RingTheme};
pub use geometry::segment_at;
