//!
//! NComm-Executors provides a set of executors (kind of like schedulers)
//! for nodes.
//!
//! The main idea with this create is to make executing nodes fit the specific
//! use case.  Specifically, there may be times when a Threadpooled model works
//! best or another use case where a Green-threaded Tokio implementation works best.
//!
//! Ideally, this crate should contain all of the commonly used executors that
//! conform to the common ncomm-core traits to make creating robotics systems
//! as easy and pain-free as possible.
//!

#![deny(missing_docs)]
// To test the internal state of nodes, they need to be force
// downcasted into their respective type.
#![cfg_attr(test, feature(downcast_unchecked))]
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
pub mod simple_executor;
#[cfg(feature = "std")]
pub use simple_executor::SimpleExecutor;

#[cfg(feature = "std")]
pub mod threadpool_executor;
#[cfg(feature = "std")]
pub use threadpool_executor::ThreadPoolExecutor;

#[cfg(feature = "std")]
pub mod threaded_executor;
#[cfg(feature = "std")]
pub use threaded_executor::ThreadedExecutor;

use core::cmp::{Ord, Ordering};
use ncomm_core::node::Node;

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(feature = "std")]
use std::{boxed::Box, vec::Vec};

#[cfg(any(feature = "alloc", feature = "std"))]
/// The NodeWrapper wraps nodes giving them a priority based on the timestamp
/// of their next update.
///
/// This ensures that nodes are updated at the correct time
pub(crate) struct NodeWrapper<ID: PartialEq> {
    /// The timestamp of the nodes next update
    pub priority: u128,
    /// The nde this NodeWrapper is wrapping around
    pub node: Box<dyn Node<ID>>,
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl<ID: PartialEq> NodeWrapper<ID> {
    /// Destroy the node wrapper returning the node it was wrapping.
    pub fn destroy(self) -> Box<dyn Node<ID>> {
        self.node
    }
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl<ID: PartialEq> Ord for NodeWrapper<ID> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority).reverse()
    }
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl<ID: PartialEq> PartialOrd for NodeWrapper<ID> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl<ID: PartialEq> PartialEq for NodeWrapper<ID> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl<ID: PartialEq> Eq for NodeWrapper<ID> {}

#[cfg(any(feature = "alloc", feature = "std"))]
/// This method performs binary search insertion into the sorted vector
/// `vec` with the node `node`.
///
/// This is just a convenience method I found myself using a ton so I decided
/// to make it its own method.
#[inline(always)]
pub(crate) fn insert_into<ID: PartialEq>(vec: &mut Vec<NodeWrapper<ID>>, node: NodeWrapper<ID>) {
    // If another node is found with the same priority, insert the node after that
    // node.  Otherwise, insert the node into the position it should be in in the
    // sorted vector
    match vec.binary_search(&node) {
        Ok(idx) => vec.insert(idx + 1, node),
        Err(idx) => vec.insert(idx, node),
    }
}
