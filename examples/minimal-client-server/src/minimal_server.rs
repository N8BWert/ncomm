//!
//! The Minimal Server responds to requests seeking to add two u64s with
//! the wrapping sum of the two u64s.
//!

use super::{AddTwoIntsRequest, AddTwoIntsResponse};

use ncomm_clients_and_servers::local::{LocalClient, LocalServer};
use ncomm_core::{Node, Server};

/// A minimal server node example that receives requests to add two ints and
/// responds with the wrapping sum of the two integers.
pub struct MinimalServer {
    server: LocalServer<AddTwoIntsRequest, AddTwoIntsResponse, String>,
}

impl MinimalServer {
    /// Creates a new MinimalServer Node
    pub fn new() -> Self {
        Self {
            server: LocalServer::new(),
        }
    }

    /// Creates a client for the local server the node has
    pub fn create_client(
        &mut self,
        client_name: String,
    ) -> LocalClient<AddTwoIntsRequest, AddTwoIntsResponse> {
        self.server.create_client(client_name)
    }

    /// Poll for requests, find their sum and return a response to any
    /// incoming requests
    fn handle_requests(&mut self) {
        for request in self.server.poll_for_requests() {
            let Ok((client_name, request)) = request;
            self.server
                .send_response(
                    client_name,
                    request,
                    AddTwoIntsResponse {
                        sum: request.a.wrapping_add(request.b),
                    },
                )
                .unwrap();
        }
    }
}

impl Node for MinimalServer {
    fn get_update_delay_us(&self) -> u128 {
        500_000
    }

    fn update(&mut self) {
        self.handle_requests();
    }
}
