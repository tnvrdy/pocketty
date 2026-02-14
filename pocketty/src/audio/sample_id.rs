use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SampleId(pub u64);

// fancy atomic counter lets us generate unique ids while in threads
pub fn next_sample_id() -> SampleId {
    SampleId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
}
