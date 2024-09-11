//!
//! The Minimal Client node sends requests to add two `u64`s and receives a response
//! containing the sum of the two `u64`s.
//!

use super::{AddTwoIntsRequest, AddTwoIntsResponse, NodeIdentifier};

use ncomm_clients_and_servers::local::LocalClient;
use ncomm_core::{Client, Node};

use rand::random;

/// A minimal client node example that sends requests to add two integers and receives
/// responses containing their sum.
pub struct MinimalClient {
    client: LocalClient<AddTwoIntsRequest, AddTwoIntsResponse>,
}

impl MinimalClient {
    /// Creates a new MinimalClient Node
    pub fn new(client: LocalClient<AddTwoIntsRequest, AddTwoIntsResponse>) -> Self {
        Self { client }
    }

    /// Generate two random numbers as a and b and send a request for their sum
    fn send_request(&mut self) {
        self.client
            .send_request(AddTwoIntsRequest {
                a: random(),
                b: random(),
            })
            .unwrap();
    }

    /// Poll for incoming responses and print the two input numbers and their sum.
    fn handle_responses(&mut self) {
        for response in self.client.poll_for_responses() {
            let Ok((request, response)) = response;
            println!("{} + {} = {}", request.a, request.b, response.sum);
        }
    }
}

impl Node<NodeIdentifier> for MinimalClient {
    fn get_id(&self) -> NodeIdentifier {
        NodeIdentifier::ClientNode
    }

    fn get_update_delay_us(&self) -> u128 {
        500_000
    }

    fn update(&mut self) {
        self.send_request();
        self.handle_responses();
    }
}
