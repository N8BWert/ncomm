use crate::fibonacci_update_server::{FibonacciRequest, FibonacciUpdate, FibonacciResponse};

use ncomm::update_client_server::{UpdateClient, local::LocalUpdateClient};
use ncomm::node::Node;

pub struct FibonacciUpdateClient<'a> {
    name: &'a str,
    update_client: LocalUpdateClient<FibonacciRequest, FibonacciUpdate, FibonacciResponse>
}

impl<'a> FibonacciUpdateClient<'a> {
    pub fn new(name: &'a str, update_client: LocalUpdateClient<FibonacciRequest, FibonacciUpdate, FibonacciResponse>) -> Self {
        Self { name, update_client }
    }
}

impl<'a> Node for FibonacciUpdateClient<'a> {
    fn name(&self) -> String { String::from(self.name) }

    fn get_update_delay(&self) -> u128 { 10u128 }

    fn start(&mut self) {
        self.update_client.send_request(FibonacciRequest { order: 10 }).unwrap();
    }

    fn update(&mut self) {
        if let Some(update) = self.update_client.receive_update() {
            println!("Received Update:");
            println!("Last Num: {} --- Current Num: {}", update.last_num, update.current_num);
        }

        if let Some(response) = self.update_client.receive_response() {
            println!("f(10) = {}", response.num);
        }
    }

    fn shutdown(&mut self) {}

    fn debug(&self) -> String {
        format!(
            "FIbonacci Update Client:\n{}",
            self.name()
        )
    }
}