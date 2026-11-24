//! Buffer Pool manager and frame cache for TheresavDB.

pub mod frame;
pub mod manager;
pub mod replacer;

pub use frame::Frame;
pub use manager::BufferPoolManager;
pub use replacer::ClockReplacer;
