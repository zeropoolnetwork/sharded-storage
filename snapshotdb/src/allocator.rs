//! Slot-based memory allocation system with reference counting
//! 
//! This module provides an `Allocator` that manages a pool of slots with reference counting.
//! It's designed for concurrent access and automatically grows the slot pool when needed.
//! The allocator maintains a minimum reserve of free slots to ensure efficient allocation.
//!
//! # Features
//! - Thread-safe concurrent access
//! - Automatic pool growth
//! - Reference counting for each slot
//! - Batch operations for multiple slots
//! - Asynchronous API
//!
//! # Example
//! ```
//! use allocator::Allocator;
//! 
//! #[tokio::main]
//! async fn main() {
//!     // Initialize with empty slots
//!     let allocator = Allocator::from_link_counter(vec![0, 0, 0]);
//!     
//!     // Allocate a new slot
//!     let slot = allocator.pop().await;
//!     
//!     // Increment reference count
//!     allocator.inc(slot).await;
//!     
//!     // Decrement reference count, potentially returning slot to free pool
//!     allocator.dec(slot).await;
//! }
//! ```
//! 


use flume::{Sender, Receiver, unbounded};
use tokio::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};

pub const FREE_SLOTS_MIN_RESERVE: usize = 1024;



/// A concurrent slot allocator with reference counting capabilities.
/// 
/// The allocator maintains a pool of slots where each slot has an associated
/// reference count. When a slot's reference count reaches zero, it's automatically
/// returned to the pool of free slots.
pub struct Allocator {
    link_counter: RwLock<Vec<AtomicUsize>>,
    free_slots: (Sender<usize>, Receiver<usize>),
}

impl Allocator {
    /// Creates a new `Allocator` from an initial vector of reference counts.
    ///
    /// This constructor initializes the allocator with the given reference counts and
    /// ensures a minimum number of free slots is available (defined by `FREE_SLOTS_MIN_RESERVE`).
    ///
    /// # Arguments
    /// * `link_counter` - Initial vector where each element represents a slot's reference count
    ///
    /// # Returns
    /// A new `Allocator` instance
    pub fn from_link_counter(link_counter: Vec<usize>) -> Self {
        let len = link_counter.len();
        let (tx, rx) = unbounded();

        let mut num_free_slots = 0;
        for (i, &item) in link_counter.iter().enumerate() {
            if item == 0 {
                tx.send(i).unwrap();
                num_free_slots += 1;
            }
        }

        let mut link_counter = link_counter.into_iter().map(AtomicUsize::new).collect::<Vec<_>>();

        if num_free_slots < FREE_SLOTS_MIN_RESERVE {
            for i in len .. len + FREE_SLOTS_MIN_RESERVE {
                tx.send(i).unwrap();
            }

            link_counter.resize_with(len + FREE_SLOTS_MIN_RESERVE, || AtomicUsize::new(0));
        }

        
        Self { link_counter: RwLock::new(link_counter), free_slots: (tx, rx) }
    }

    /// Allocates a new slot and increments its reference count.
    ///
    /// If the number of free slots falls below `FREE_SLOTS_MIN_RESERVE`,
    /// the allocator automatically grows the pool.
    ///
    /// # Returns
    /// The index of the allocated slot
    pub async fn pop(&self) -> usize {
        let len = self.free_slots.1.len();
        if len < FREE_SLOTS_MIN_RESERVE {
            let mut guard = self.link_counter.write().await;
            let len = guard.len();
            guard.resize_with(len + FREE_SLOTS_MIN_RESERVE, || AtomicUsize::new(0));
            drop(guard);

            let tx = self.free_slots.0.clone();
            tokio::spawn(async move {
                for i in len .. len + FREE_SLOTS_MIN_RESERVE {
                    tx.send_async(i).await.unwrap();
                }
            });
        }

        let slot = self.free_slots.1.recv_async().await.unwrap();
        self.inc(slot).await;
        slot
    }

    /// Increments the reference count for a given slot.
    ///
    /// # Arguments
    /// * `slot` - The slot index whose reference count should be incremented
    pub async fn inc(&self, slot: usize) {
        self.link_counter.read().await[slot].fetch_add(1, Ordering::Relaxed);
    }

    /// Increments reference counts for multiple slots atomically.
    ///
    /// # Arguments
    /// * `slots` - Slice of slot indices whose reference counts should be incremented
    pub async fn inc_many(&self, slots: &[usize]) {
        let guard = self.link_counter.read().await;
        for &slot in slots {
            guard[slot].fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Decrements the reference count for a given slot.
    ///
    /// If the reference count reaches zero, the slot is automatically
    /// returned to the pool of free slots.
    ///
    /// # Arguments
    /// * `slot` - The slot index whose reference count should be decremented
    pub async fn dec(&self, slot: usize) {
        let val = self.link_counter.read().await[slot].fetch_sub(1, Ordering::Relaxed);
        if val == 1 {
            self.free_slots.0.send_async(slot).await.unwrap();
        }
    }

    /// Decrements reference counts for multiple slots.
    ///
    /// Any slots whose reference counts reach zero are automatically
    /// returned to the pool of free slots.
    ///
    /// # Arguments
    /// * `slots` - Slice of slot indices whose reference counts should be decremented
    pub async fn dec_many(&self, slots: &[usize]) {
        let mut freed_slots = Vec::new();
        let guard = self.link_counter.read().await;
        for &slot in slots {
            let val = guard[slot].fetch_sub(1, Ordering::Relaxed);
            if val == 1 {
                freed_slots.push(slot);
            }
        }
        drop(guard);

        let tx = self.free_slots.0.clone();
        tokio::spawn(async move {
            for slot in freed_slots {
                tx.send_async(slot).await.unwrap();
            }
        });
    }


    pub async fn len(&self) -> usize {
        self.link_counter.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
}
