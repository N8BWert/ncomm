//!
//! Ncomm-Core is a collection of traits that layout the core of the
//! ncomm robotics framework.
//!

#![deny(unsafe_code)]
#![deny(missing_docs)]

pub mod node;
pub use node::Node;

pub mod executor;
pub use executor::{Executor, ExecutorState};

pub mod publisher_subscriber;
pub use publisher_subscriber::{Publisher, Subscriber};

pub mod client_server;
pub use client_server::{Client, Server};

pub mod update_client_server;
pub use update_client_server::{UpdateClient, UpdateServer};
