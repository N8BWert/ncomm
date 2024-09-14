//!
//! An executor handles the scheduling and execution of nodes.
//!
//! In all likelihood most users should use one of the executors provided
//! in ncomm-executors.  This trait should, however, create a common interface
//! for interfacing with any and all of the executors that are part of NComm
//! and allows users to write their own executors if desired.
//!

use crate::node::Node;

#[cfg(feature = "alloc")]
use alloc::boxed::Box;
#[cfg(feature = "std")]
use std::boxed::Box;

/// The current state an executor is in.
///
/// This should be taken into account whenever the start or update methods
/// are called on an executor so that an executor may be put into the correct
/// state before executing a method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutorState {
    /// The nodes in the executor are currently stopped.
    ///
    /// This means the Nodes must be started before update can begin
    Stopped,
    /// The nodes in the executor have been started and can now be updated.
    Started,
    /// The nodes in the executor are current being updated
    Running,
}

/// An executor handles the scheduling and execution of nodes
///
/// All nodes should have some unique ID that makes them identifiable
/// as trait objects
pub trait Executor<ID: PartialEq> {
    /// Optional Context for adding nodes with specific conditions
    type Context;

    /// Starts the nodes contained by the executor
    fn start(&mut self);

    /// Run the update loop for a set amount of time (in milliseconds)
    fn update_for_ms(&mut self, ms: u128);

    /// Run the update loop until the executor's interrupt is called
    fn update_loop(&mut self);

    /// Check whether the program has been interrupted
    ///
    /// Note: This should be called between each Node execution
    fn check_interrupt(&mut self) -> bool;

    /// Add a node to the executor.
    fn add_node(&mut self, node: Box<dyn Node<ID>>);

    /// Add a node to the executor with some given context.
    ///
    /// Note: The context is mainly to allow for extra configuration when
    /// adding nodes.
    fn add_node_with_context(&mut self, node: Box<dyn Node<ID>>, _ctx: Self::Context) {
        self.add_node(node);
    }

    /// Remove a node from the executor.
    fn remove_node(&mut self, id: &ID) -> Option<Box<dyn Node<ID>>>;
}
