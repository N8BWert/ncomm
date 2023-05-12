pub mod basic_node;
pub mod pubsub_node;

/// struct Node {
///     name/id: String/u64
///     update_rate: u64
///     list/vector<publishers> publishers
///     list/vector<subscribers> subscribers
///     list/vector<clients> clients
///     list/vector<servers> servers
///     list/vector<update_clients> update_clients
///     list/vector<update_servers> update_servers
/// }

/// A Node should represent a comprehensive object/block of code that functions relatively independantly
/// from the other blocks of code.
/// 
/// All Nodes need to implement the following methods, which will be called by their executor.
pub trait Node: Send {
    // Returns the name of this node (used in testing)
    fn name(&self) -> String;
    
    /// Completes any important initialization methods for the node.
    fn start(&mut self);

    /// Called every update_rate ms and is where the majority of user code will be located (should be overriden for any user defined nodes)
    fn update(&mut self);

    /// Simply returns the node's update rate (in ms)
    fn get_update_rate(&self) -> u128;

    /// When the program is stopped the node must contain logic to end any affairs it is currently entangled in.
    fn shutdown(&mut self);

    /// Returns a string consisting of the debug information for this node
    fn debug(&self) -> String;
}