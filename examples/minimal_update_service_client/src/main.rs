extern crate ncomm;

use ncomm::executor::{Executor, simple_executor::SimpleExecutor};

pub mod fibonacci_update_server;
pub mod fibonacci_update_client;

use fibonacci_update_server::FibonacciUpdateServer;
use fibonacci_update_client::FibonacciUpdateClient;

fn main() {
    let mut server = FibonacciUpdateServer::new("fibonacci update server");
    let mut client = FibonacciUpdateClient::new("fibonacci update client", server.create_client(String::from("fibonacci update client")));
    let mut executor = SimpleExecutor::new();
    executor.add_node(&mut server);
    executor.add_node(&mut client);
    executor.start();
    executor.update_for_seconds(1);
    executor.interrupt();
}
