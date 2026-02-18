pub mod clustering;
pub mod dedup;
pub mod entry;
pub mod error;
pub mod event;
pub mod gate;
pub mod near_dedup;
pub mod resolution;
pub mod scoring;
pub mod traits;
pub mod types;

pub use dedup::DedupChecker;
pub use entry::{MemoryEntry, MemoryEntryBuilder};
pub use error::{ShabtiError, ShabtiResult};
pub use event::Event;
pub use gate::{DataTier, FeatureGate};
pub use types::{LifecycleState, OriginType};
