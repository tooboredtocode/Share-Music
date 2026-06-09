use std::sync::atomic::{AtomicUsize, Ordering};

pub struct AtomicQueue {
    entries: AtomicUsize, // Number of times the queue has been entered
    leaves: AtomicUsize,  // Number of times the queue has been left
}

pub struct AtomicQueueGuard<'a> {
    position: usize,
    queue: &'a AtomicQueue,
}

impl AtomicQueue {
    pub fn new() -> Self {
        Self {
            entries: AtomicUsize::new(0),
            leaves: AtomicUsize::new(0),
        }
    }

    pub fn enter(&self) -> AtomicQueueGuard<'_> {
        let position = self.entries.fetch_add(1, Ordering::Relaxed);
        AtomicQueueGuard {
            position,
            queue: self,
        }
    }
}

impl<'a> AtomicQueueGuard<'a> {
    pub fn position(&self) -> usize {
        self.position
            .saturating_sub(self.queue.leaves.load(Ordering::Relaxed))
    }

    pub fn leave(self) {
        // Empty method to consume the guard and trigger the drop, which will increment the leaves count
    }
}

impl<'a> Drop for AtomicQueueGuard<'a> {
    fn drop(&mut self) {
        self.queue.leaves.fetch_add(1, Ordering::Relaxed);
    }
}
