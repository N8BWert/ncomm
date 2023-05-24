use ncomm::client_server::{Client, local::LocalClient};
use ncomm::node::Node;

use crate::minimal_service::{AddTwoIntsRequest, AddTwoIntsResponse};

pub struct MinimalClient<'a> {
    name: &'a str,
    client: LocalClient<AddTwoIntsRequest, AddTwoIntsResponse>,
}

impl<'a> MinimalClient<'a> {
    pub fn new(name: &'a str, client: LocalClient<AddTwoIntsRequest, AddTwoIntsResponse>) -> Self {
        Self { name, client }
    }
}

impl<'a> Node for MinimalClient<'a> {
    fn name(&self) -> String { String::from(self.name) }

    fn get_update_rate(&self) -> u128 { 10u128 }

    fn start(&mut self) {
        self.client.send_request(AddTwoIntsRequest { a: 10, b: 20}).unwrap();
    }

    fn update(&mut self) {
        if let Some(response) = self.client.receive_response() {
            println!("Sum: {}", response.sum);
        }
    }

    fn shutdown(&mut self) { }

    fn debug(&self) -> String {
        format!(
            "Minimal Client:\n{}",
            self.name()
        )
    }
}