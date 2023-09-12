//!
//! A Singular Process.
//! 
//! In NComm (as with Ros) the main idea is to encapsulate singular processes into a singular Node.  
//! The hope is that by encapsulating processes to specific structures it helps the readability and maintainability of robotics
//! projects.
//! 
//! Take, for example, a system consisting of a few sensors and a web server.  To enhance the readability and maintainability
//! of the project, it may be useful to separate each of the sensors into a specific Node that collects (and possibly processes)
//! the various pieces of sensor data.  Then, it may be necessary to combine the data at another node that is running the web server.
//! 
//! Each of the Modules below contains various examples used internally to test executor functionalities.
//! 
pub mod basic_node;
pub mod pubsub_node;
pub mod client_server_node;
pub mod update_client_server_node;
pub mod udp_pubsub_node;

/// A Node should represent a comprehensive object/block of code that functions relatively independently
/// from the other blocks of code.
/// 
/// All Nodes need to implement the following methods, which will be called by their executor.
pub trait Node: Send {
    // Returns the name of this node (used in testing)
    fn name(&self) -> String;
    
    /// Completes any important initialization methods for the node.
    fn start(&mut self);

    /// Called every update_rate ms and is where the majority of user code will be located (should be overridden for any user defined nodes)
    fn update(&mut self);

    /// Simply returns the node's update rate (in ms)
    fn get_update_delay(&self) -> u128;

    /// When the program is stopped the node must contain logic to end any affairs it is currently entangled in.
    fn shutdown(&mut self);

    /// Returns a string consisting of the debug information for this node
    fn debug(&self) -> String;
}