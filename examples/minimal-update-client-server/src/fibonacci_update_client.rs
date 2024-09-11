//!
//! A Simple Update Client Example that sends a request for the nth term in the
//! fibonacci series.
//!

use super::{FibonacciRequest, FibonacciResponse, FibonacciUpdate, NodeIdentifier};

use ncomm_core::{Node, UpdateClient};
use ncomm_update_clients_and_servers::local::LocalUpdateClient;

/// A simple update client node that sends a request for the nth term in the
/// fibonacci series, printing the numbers in the series before receiving the nth
/// number in the series
pub struct FibonacciUpdateClient {
    update_client: LocalUpdateClient<FibonacciRequest, FibonacciUpdate, FibonacciResponse>,
}

impl FibonacciUpdateClient {
    /// Create a new Fibonacci Update Client
    pub fn new(
        update_client: LocalUpdateClient<FibonacciRequest, FibonacciUpdate, FibonacciResponse>,
    ) -> Self {
        Self { update_client }
    }

    /// Print the update messages received
    pub fn handle_updates(&mut self) {
        let updates = self.update_client.poll_for_updates();
        for update in updates.iter() {
            if let Ok((_request, update)) = update {
                println!(
                    "Last Num: {} --- Current Num: {}",
                    update.last_num, update.current_num
                );
            }
        }
    }

    /// Print the response message received and send a new request for the 10th order of
    /// a fibonacci sequence
    pub fn handle_response(&mut self) {
        let responses = self.update_client.poll_for_responses();
        for response in responses.iter() {
            if let Ok((request, response)) = response {
                println!("f(10) = {}", response.num);
                self.update_client.send_request(request.clone()).unwrap();
            }
        }
    }
}

impl Node<NodeIdentifier> for FibonacciUpdateClient {
    fn get_id(&self) -> NodeIdentifier {
        NodeIdentifier::FibonacciClient
    }

    fn get_update_delay_us(&self) -> u128 {
        100_000
    }

    fn start(&mut self) {
        self.update_client
            .send_request(FibonacciRequest { order: 10 })
            .unwrap();
    }

    fn update(&mut self) {
        self.handle_updates();
        self.handle_response();
    }
}
