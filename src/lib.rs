//!
//! NComm is a rust-based replacement for Ros's communication system.
//! 
//! The main idea of NComm is that there should be many types of publishers, subscribers, clients, servers, update clients,
//! and update servers.  Each of the types of communication should specialize on something.  For example, when publishing data
//! between nodes in the same Rust program, there is no need to make this communication accessible via the internet as this will
//! only slow down communication.  Instead, it makes more sense to use the local publisher (a channels based publisher) to send
//! data efficiently.
//! 
//! Over time, I would like to expand the reaches of this library by adding various new communication types as I find them necessary.
//! 
//! I am also going to try my best to link projects that use this library so that new users can get a good idea of what NComm enables.
//! 
//! ## Projects
//! * RoboJackets Robocup Base Station - <https://github.com/RoboJackets/robocup-base-station>
//! 

pub mod node;
pub mod executor;
pub mod publisher_subscriber;
pub mod client_server;
pub mod update_client_server;