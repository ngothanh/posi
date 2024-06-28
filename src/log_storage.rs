use std::time::Duration;
use moka::sync::Cache;
use uuid::Uuid;

pub trait LogStorage {
    fn store(&self, attempts: usize, duration: Duration);

    fn count(&self) -> usize;
}

pub struct InMemoryLogStorage {
    cache: Cache<String, usize>,
}

impl InMemoryLogStorage {
    pub fn new(size: usize, duration: Duration) -> InMemoryLogStorage {
        InMemoryLogStorage {
            cache: Cache::builder()
                .max_capacity(size as u64)
                .time_to_live(duration)
                .build(),
        }
    }
}

impl LogStorage for InMemoryLogStorage {
    fn store(&self, attempts: usize, duration: Duration) {
        for _ in 0..attempts {
            self.cache.insert(Uuid::new_v4().to_string(), 1);
        }
    }

    fn count(&self) -> usize {
        self.cache.iter().count()
    }
}