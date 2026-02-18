pub mod entry;
pub mod error;
pub mod types;
pub mod traits;
pub mod dedup;
pub mod gate;

pub use entry::{MemoryEntry, MemoryEntryBuilder};
pub use error::{ShabtiError, ShabtiResult};
pub use types::{LifecycleState, OriginType};
pub use dedup::DedupChecker;
pub use gate::{DataTier, FeatureGate};
