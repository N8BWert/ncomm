//!
//! The Node Wrapper wraps a mutable reference to a node with information regarding its next
//! update timestamp.
//! 

use crate::node::Node;

use std::cmp::{Ord, Ordering};

/// Wrapper for a node that gives it a priority based on its update rate
/// 
/// Params:
///     priorty: the priority of the node (i.e. the timestamp of the next update)
///     node: the node that will be updated after the priority timestamp it reached
pub(in crate::executor) struct NodeWrapper<'a> {
    pub priority: u128,
    pub node: &'a mut dyn Node,
}

impl Ord for NodeWrapper<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority).reverse()
    }
}

impl PartialOrd for NodeWrapper<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for NodeWrapper<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for NodeWrapper<'_> {}