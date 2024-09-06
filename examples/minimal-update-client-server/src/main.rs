//!
//! Example for a minimal update client and server where
//! the server requests a a fibonacci value of some order and
//! the server responds with updates containing the last and current
//! value in a fibonacci series before it finds the fibonacci value of
//! given degree which is returned as a response
//! 

#![deny(missing_docs)]

use ncomm_core::Executor;
use ncomm_executors::SimpleExecutor;

use crossbeam::channel::unbounded;

pub mod fibonacci_update_client;
use fibonacci_update_client::FibonacciUpdateClient;

pub mod fibonacci_update_server;
use fibonacci_update_server::FibonacciUpdateServer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A request asking for a the nth order of the fibonacci sequence.
pub struct FibonacciRequest {
    /// The nth term of the fibonacci sequence to compute
    pub order: u128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// An update containing the current last number and the current number
/// the update server has currently computed.
pub struct FibonacciUpdate {
    /// The order n-1 term of the fibonacci sequence
    pub last_num: u128,
    /// The order n term of the fibonacci sequence
    pub current_num: u128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A response containing the number at a given order of the fibonacci
/// sequence.
pub struct FibonacciResponse {
    /// The order n term of the fibonacci sequence
    pub num: u128,
}

fn main() {
    let mut update_server_node = FibonacciUpdateServer::new();
    let update_client_node = FibonacciUpdateClient::new(
        update_server_node.create_client(String::from("Fibonacci Update Client"))
    );

    let (tx, rx) = unbounded();
    ctrlc::set_handler(move || tx.send(true).expect("Unable to send data"))
        .expect("Error setting Ctrl-C handler");

    let mut executor = SimpleExecutor::new_with(
        rx,
        vec![
            Box::new(update_client_node),
            Box::new(update_server_node),
        ]
    );

    executor.update_loop();
}
