//! Test helper modules for integration tests

#![allow(dead_code, reason = "each test binary uses a subset of the shared helpers")]

pub mod event_collector;
pub mod exif_fixture;
pub mod indexing_harness;
pub mod snapshot;
pub mod sync_harness;
pub mod sync_transport;
pub mod test_data;
pub mod test_volumes;
pub mod wait;

