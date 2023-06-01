use ncomm::client_server::{Server, local::{LocalClient, LocalServer}};
use ncomm::node::Node;

#[derive(PartialEq, Clone)]
pub struct AddTwoIntsRequest {
    pub a: u8,
    pub b: u8,
}

#[derive(PartialEq, Clone)]
pub struct AddTwoIntsResponse {
    pub sum: u8,
}

pub struct MinimalService<'a> {
    name: &'a str,
    server: LocalServer<AddTwoIntsRequest, AddTwoIntsResponse>,
}

impl<'a> MinimalService<'a> {
    pub fn new(name: &'a str) -> Self {
        Self { name, server: LocalServer::new() }
    }

    pub fn create_client(&mut self, client_name: String) -> LocalClient<AddTwoIntsRequest, AddTwoIntsResponse> {
        self.server.create_client(client_name)
    }

    fn handle_requests(&mut self) {
        let requests = self.server.receive_requests();

        let mut responses = Vec::with_capacity(requests.len());
        for (client, request) in requests {
            responses.push((client, AddTwoIntsResponse { sum: request.a + request.b }));
        }

        self.server.send_responses(responses);
    }
}

impl<'a> Node for MinimalService<'a> {
    fn name(&self) -> String { String::from(self.name) }

    fn get_update_rate(&self) -> u128 { 10u128 }

    fn start(&mut self) { }

    fn update(&mut self) {
        self.handle_requests();
    }

    fn shutdown(&mut self) {
        self.handle_requests();
    }

    fn debug(&self) -> String {
        format!(
            "Minimal Service:\n{}",
            self.name()
        )
    }
}
