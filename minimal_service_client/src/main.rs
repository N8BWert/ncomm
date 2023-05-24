extern crate ncomm;

use ncomm::executor::{Executor, simple_executor::SimpleExecutor};

pub mod minimal_service;
pub mod minimal_client;

use minimal_client::MinimalClient;
use minimal_service::MinimalService;

fn main() {
    let mut minimal_service = MinimalService::new("minimal service");
    let mut minimal_client = MinimalClient::new("minimal client", minimal_service.create_client(String::from("minimal client")));
    let mut executor = SimpleExecutor::new();
    executor.add_node(&mut minimal_service);
    executor.add_node(&mut minimal_client);
    executor.start();
    executor.update_for_seconds(1);
    executor.interrupt();
}
