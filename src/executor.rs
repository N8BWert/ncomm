//!
//! Wrapper that manages the scheduling of Nodes.
//! 
//! An Executor should maintain references to a number of nodes and schedule their
//! updates.  The idea here is that each executor can pretty much be its own thread
//! that manages its own set of processes.
//! 

pub mod simple_executor;
pub mod simple_multi_executor;
pub mod node_wrapper;

use crate::node::Node;

/// An executor should contain a large number of nodes which it will handle the calling
/// of the update, start and other functions (an executor will take place on a singular thread)
pub trait Executor<'a> {
    /// Sets the start time for executors allowing for the update loop to begin
    fn start(&mut self);

    /// Run the update loop for a set amount of time
    fn update_for_seconds(&mut self, seconds: u128);

    // Run the update loop until the program is terminated
    fn update_loop(&mut self);

    // Interrupt the execution of this executor
    fn check_interrupt(&mut self) -> bool;

    /// A useful function to log any important information about this executor (will
    /// be mostly used for errors)
    fn log(&self, message: &str);
}

/// Trait to be implemented by all single threaded executors to allow the addition of nodes
/// in the single threaded context.
pub trait SingleThreadedExecutor<'a> {
    /// Adds a given node to the single threaded executor's internal representation of nodes
    fn add_node(&mut self, new_node: &'a mut dyn Node);

    /// Run the update loop for one iteration
    fn update(&mut self);

    /// Run the update loop for a set number of iterations
    fn update_for(&mut self, iterations: u128);
}

/// Trait to be implemented by all multi threaded executors to allow the addition
/// of nodes into specific threads of the multi threaded executor.
pub trait MultiThreadedExecutor<'a> {
    /// Adds a given node to the specific thread of the multi threaded executor
    fn add_node_to(&mut self, new_node: &'a mut dyn Node, thread: &str);
}