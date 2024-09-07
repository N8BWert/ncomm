//!
//! This example shows a minimal client and server based on the
//! [Ros 2 Foxy Tutorial]()
//!
//! The premise of this example is that the client requests the server to
//! add two integers together and the server responds with the sum of the
//! two incoming integers.
//!

#[deny(missing_docs)]
use ncomm_core::Executor;
use ncomm_executors::SimpleExecutor;

use crossbeam::channel::unbounded;
use ctrlc;

pub mod minimal_server;
use minimal_server::MinimalServer;

pub mod minimal_client;
use minimal_client::MinimalClient;

/// A request that asks for the sum of two `u64`s.
#[derive(Clone, Copy)]
pub struct AddTwoIntsRequest {
    pub a: u64,
    pub b: u64,
}

/// A response containing the sum of two `u64`s.
pub struct AddTwoIntsResponse {
    pub sum: u64,
}

fn main() {
    let mut server_node = MinimalServer::new();
    let client_node = MinimalClient::new(server_node.create_client(String::from("Minimal Client")));

    let (tx, rx) = unbounded();
    ctrlc::set_handler(move || tx.send(true).expect("Could not send data"))
        .expect("Error setting Ctrl-C handler");

    let mut executor =
        SimpleExecutor::new_with(rx, vec![Box::new(client_node), Box::new(server_node)]);

    executor.update_loop();
}
