//!
//! A Singular Unit of Work.
//! 
//! In NComm, Nodes are thought of as an individual unit of work that is
//! performed every x microseconds.  The guiding principle of an NComm
//! Node is that units of work can be split up into individual microservices
//! that can execute on their own via data received from various communication
//! methods and send their outputs to the next process that processes their
//! information.
//! 

/// A Node represents a singular process that performs some singular
/// purpose
pub trait Node: Send {
    /// Return the node's update rate (in us)
    fn get_update_delay(&self) -> u128;

    /// Complete the necessary setup functionalities for a Node.
    /// 
    /// Note: this method is called on Start for the executor or
    /// (if the executor was not formally started) before the executor
    /// begins updating nodes.
    fn start(&mut self);

    /// Update is called by the executor every get_update_delay microseconds.
    /// 
    /// This can be compared to Arduino's `void loop` and should include the
    /// work completed by this node every "tick".
    fn update(&mut self);

    /// When an executor is stopped or has finished executing nodes, it will call
    /// this method on all of its nodes so this should clean up any work
    /// the node needs to do.
    fn shutdown(&mut self);
}