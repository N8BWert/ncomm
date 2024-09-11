//!
//! An update server example that calculates the nth term in the
//! fibonacci sequence, updating a client with each sequence of terms
//! into the series
//!

use super::{FibonacciRequest, FibonacciResponse, FibonacciUpdate, NodeIdentifier};

use ncomm_core::{Node, UpdateServer};
use ncomm_update_clients_and_servers::local::{LocalUpdateClient, LocalUpdateServer};

/// An update server node that calculates the nth term in the fibonacci
/// sequence
///
/// Note: for this example, the update server only expects a single client but
/// it would be trivial to make this update server capable of handling multiple
/// clients.
pub struct FibonacciUpdateServer {
    last_num: u128,
    current_num: u128,
    order: u128,
    current_order: u128,
    current_request: Option<(String, FibonacciRequest)>,
    update_server: LocalUpdateServer<FibonacciRequest, FibonacciUpdate, FibonacciResponse, String>,
}

impl FibonacciUpdateServer {
    /// Create a new fibonacci update server
    pub fn new() -> Self {
        Self {
            last_num: 1,
            current_num: 1,
            order: 1,
            current_order: 1,
            current_request: None,
            update_server: LocalUpdateServer::new(),
        }
    }

    /// Create a client for the fibonacci update server node
    pub fn create_client(
        &mut self,
        client_name: String,
    ) -> LocalUpdateClient<FibonacciRequest, FibonacciUpdate, FibonacciResponse> {
        self.update_server.create_update_client(client_name)
    }

    /// Check for requests
    fn handle_requests(&mut self) {
        if let Some(Ok((client, request))) = self.update_server.poll_for_requests().last() {
            self.current_request = Some((client.clone(), request.clone()));
            self.order = request.order;
            self.last_num = 1;
            self.current_num = 1;
            self.current_order = 1;
        }
    }

    /// Send Updates for the current client
    fn send_updates(&mut self) {
        if let Some((client_id, request)) = self.current_request.as_ref() {
            let next_num = self.last_num + self.current_num;
            self.last_num = self.current_num;
            self.current_num = next_num;
            self.update_server
                .send_update(
                    client_id.clone(),
                    &request,
                    FibonacciUpdate {
                        last_num: self.last_num,
                        current_num: self.current_num,
                    },
                )
                .unwrap();
            self.current_order += 1;
        }
    }

    /// Send final response for the current client
    fn send_response(&mut self) {
        if self.current_request.is_some() && self.current_order == self.order {
            let (client_id, request) = self.current_request.take().unwrap();
            self.update_server
                .send_response(
                    client_id,
                    request,
                    FibonacciResponse {
                        num: self.current_num,
                    },
                )
                .unwrap();
        }
    }
}

impl Node<NodeIdentifier> for FibonacciUpdateServer {
    fn get_id(&self) -> NodeIdentifier {
        NodeIdentifier::FibonacciServer
    }

    fn get_update_delay_us(&self) -> u128 {
        100_000
    }

    fn update(&mut self) {
        self.handle_requests();
        self.send_updates();
        self.send_response();
    }
}
