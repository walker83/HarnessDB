use common::{DrorisError, Result, StorageError};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug)]
pub struct MemoryTracker {
    current_usage: AtomicU64,
    max_usage: AtomicU64,
    quota: u64,
}

impl MemoryTracker {
    pub fn new(quota: u64) -> Self {
        Self {
            current_usage: AtomicU64::new(0),
            max_usage: AtomicU64::new(0),
            quota,
        }
    }

    pub fn try_allocate(self: &Arc<Self>, size: u64) -> Result<MemoryGuard> {
        let current = self.current_usage.load(Ordering::Relaxed);
        let new_usage = current + size;
        
        if new_usage > self.quota {
            return Err(DrorisError::storage(
                StorageError::MemoryLimitExceeded,
                format!("memory limit exceeded: current={}, requested={}, quota={}", 
                    current, size, self.quota)
            ));
        }

        self.current_usage.store(new_usage, Ordering::Relaxed);
        
        let max = self.max_usage.load(Ordering::Relaxed);
        if new_usage > max {
            self.max_usage.store(new_usage, Ordering::Relaxed);
        }

        Ok(MemoryGuard {
            tracker: Arc::clone(self),
            size,
        })
    }

    pub fn current_usage(&self) -> u64 {
        self.current_usage.load(Ordering::Relaxed)
    }

    pub fn max_usage(&self) -> u64 {
        self.max_usage.load(Ordering::Relaxed)
    }

    pub fn quota(&self) -> u64 {
        self.quota
    }
}

#[derive(Debug)]
pub struct MemoryGuard {
    tracker: Arc<MemoryTracker>,
    size: u64,
}

impl MemoryGuard {
    pub fn size(&self) -> u64 {
        self.size
    }
}

impl Drop for MemoryGuard {
    fn drop(&mut self) {
        let current = self.tracker.current_usage.load(Ordering::Relaxed);
        self.tracker.current_usage.store(current.saturating_sub(self.size), Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_allocation() {
        let tracker = Arc::new(MemoryTracker::new(1000));
        
        let guard1 = tracker.try_allocate(100).unwrap();
        assert_eq!(tracker.current_usage(), 100);
        
        let guard2 = tracker.try_allocate(200).unwrap();
        assert_eq!(tracker.current_usage(), 300);
        
        drop(guard1);
        assert_eq!(tracker.current_usage(), 200);
        
        drop(guard2);
        assert_eq!(tracker.current_usage(), 0);
    }

    #[test]
    fn test_memory_limit() {
        let tracker = Arc::new(MemoryTracker::new(1000));
        
        let guard1 = tracker.try_allocate(800).unwrap();
        assert!(tracker.try_allocate(300).is_err());
        
        drop(guard1);
        assert!(tracker.try_allocate(300).is_ok());
    }

    #[test]
    fn test_max_usage_tracking() {
        let tracker = Arc::new(MemoryTracker::new(1000));
        
        let guard1 = tracker.try_allocate(500).unwrap();
        let guard2 = tracker.try_allocate(300).unwrap();
        assert_eq!(tracker.max_usage(), 800);
        
        drop(guard1);
        drop(guard2);
        assert_eq!(tracker.max_usage(), 800);
        
        let _guard3 = tracker.try_allocate(900).unwrap();
        assert_eq!(tracker.max_usage(), 900);
    }
}