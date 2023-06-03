extern crate ncomm;

use std::sync::mpsc;

use ncomm::executor::{Executor, SingleThreadedExecutor, simple_executor::SimpleExecutor};

pub mod fibonacci_update_server;
pub mod fibonacci_update_client;

use fibonacci_update_server::FibonacciUpdateServer;
use fibonacci_update_client::FibonacciUpdateClient;

fn main() {
    let (_interrupt_tx, interrupt_rx) = mpsc::channel();

    let mut server = FibonacciUpdateServer::new("fibonacci update server");
    let mut client = FibonacciUpdateClient::new("fibonacci update client", server.create_client(String::from("fibonacci update client")));
    let mut executor = SimpleExecutor::new(interrupt_rx);

    executor.add_node(&mut server);
    executor.add_node(&mut client);
    executor.start();
    executor.update_for_seconds(1);
}
