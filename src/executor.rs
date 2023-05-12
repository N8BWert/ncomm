pub mod simple_executor;

use crate::node::Node;

/// An executor should contain a large number of nodes which it will handle the calling
/// of the update, start and other functions (an executor will take place on a singular thread)
pub trait Executor<'a> {
    /// Adds a given node to this executors internal representation of nodes
    fn add_node(&'a mut self, new_node: &'a mut dyn Node);

    /// Sets the start time for executors allowing for the update loop to begin
    fn start(&mut self);

    /// The main loop for this executor where it will call update on all of its nodes
    fn update(&mut self);

    /// Run the update loop for a set amount of iterations (usefull for testing)
    fn update_for(&mut self, iterations: u128);

    /// Run the update loop for a set amount of time
    fn update_for_seconds(&mut self, seconds: u128);

    // Run the update loop until the program is terminated
    fn update_loop(&mut self);

    // Interrupt the execution of this executor
    fn interrupt(&mut self);

    /// A useful function to log any important information about this executor (will
    /// be mostly used for errors)
    fn log(&self);
}