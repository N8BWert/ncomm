//!
//! Basic Client + Server Node Example.
//! 

use std::num::Wrapping;

use crate::node::Node;

use crate::client_server::{Server, Client};
use crate::client_server::local::{LocalClient, LocalServer};

/// Request Sent from the Client to the Server.
#[derive(PartialEq, Clone, Debug)]
pub struct TestRequest {
    data: u128
}
impl TestRequest {
    /// Creates a new Request with given data.
    pub const fn new(data: u128) -> Self {
        Self { data }
    }
}

/// Response Sent back from the Server to the Client
#[derive(PartialEq, Clone, Debug)]
pub struct TestResponse {
    data: u128
}
impl TestResponse {
    /// Creates a new Response with given data.
    pub const fn new(data: u128) -> Self {
        Self { data }
    }
}

/// Test Server Node
/// 
/// This node is used to test the most basic possible node that contains a
/// local server.
pub struct ServerNode<'a> {
    name: &'a str,
    update_delay: u128,
    test_server: LocalServer<TestRequest, TestResponse>,
}

/// Test Client Node
/// 
/// This node is used to test the most basic possible node that contains a
/// local client.
pub struct ClientNode<'a> {
    name: &'a str,
    update_delay: u128,
    test_number: u128,
    test_client: LocalClient<TestRequest, TestResponse>,
}

impl<'a> ServerNode<'a> {
    /// Creates a new Server node with given name and update delay
    pub fn new(name: &'a str, update_delay: u128) -> Self {
        Self{
            name,
            update_delay,
            test_server: LocalServer::new()
        }
    }

    /// Create a new client for this node's server
    pub fn create_client(&mut self, client_name: String) -> LocalClient<TestRequest, TestResponse> {
        self.test_server.create_client(client_name)
    }
}

impl<'a> ClientNode<'a> {
    /// Create a new Client Node with given name, update delay and client.
    pub const fn new(name: &'a str, update_delay: u128, client: LocalClient<TestRequest, TestResponse>) -> Self {
        Self { name, update_delay, test_number: 0, test_client: client }
    }
}

impl<'a> Node for ServerNode<'a> {
    fn name(&self) -> String {
        String::from(self.name)
    }

    fn start(&mut self) {}

    fn update(&mut self) {
        let requests = self.test_server.receive_requests();
        let mut responses = Vec::with_capacity(requests.len());
        for (client, request) in requests {
            responses.push((client, TestResponse::new((Wrapping(request.data) * Wrapping(2)).0)));
        }
        self.test_server.send_responses(responses);
    }

    fn get_update_delay(&self) -> u128 {
        self.update_delay
    }

    fn shutdown(&mut self) {
        let clients = self.test_server.get_clients();
        let mut responses = Vec::with_capacity(clients.len());
        for client in clients {
            responses.push((client, TestResponse::new(u128::MAX)));
        }
        self.test_server.send_responses(responses);
    }

    fn debug(&self) -> String {
        format!(
            "Server Node:\n{}\n{}",
            self.name(),
            self.update_delay,
        )
    }
}

impl<'a> Node for ClientNode<'a> {
    fn name(&self) -> String {
        String::from(self.name)
    }

    fn start(&mut self) {
        println!("starting client node");
        self.test_number = 1
    }

    fn update(&mut self) {
        if let Some(response) = self.test_client.receive_response() {
            println!("previous test number: {}", self.test_number);
            self.test_number = (Wrapping(self.test_number) + Wrapping(response.data)).0;
            println!("new test number: {}", self.test_number);
        }

        let _err = self.test_client.send_request(TestRequest{ data: self.test_number });
    }

    fn get_update_delay(&self) -> u128 {
        self.update_delay
    }

    fn shutdown(&mut self) {
        if let Some(response) = self.test_client.receive_response() {
            self.test_number = response.data;
        }
    }

    fn debug(&self) -> String {
        format!(
        "Client Node:\n{}\n{}\n{}",
        self.name(),
        self.update_delay,
        self.test_number
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_client_server_nodes() {
        let mut server_node = ServerNode::new("test server node", 10);
        let client_node_one = ClientNode::new("test client node 1", 10, server_node.create_client(String::from("test client node 1")));
        let client_node_two = ClientNode::new("test client node 2", 22, server_node.create_client(String::from("test client node 2")));

        assert_eq!(server_node.name(), String::from("test server node"));
        assert_eq!(server_node.get_update_delay(), 10);

        assert_eq!(client_node_one.name(), String::from("test client node 1"));
        assert_eq!(client_node_one.get_update_delay(), 10);
        assert_eq!(client_node_one.test_number, 0);

        assert_eq!(client_node_two.name(), String::from("test client node 2"));
        assert_eq!(client_node_two.get_update_delay(), 22);
        assert_eq!(client_node_two.test_number, 0);
    }

    #[test]
    fn test_start_client_server_nodes() {
        let mut server_node = ServerNode::new("test server node", 12);
        let mut client_node_one = ClientNode::new("test client node 1", 10, server_node.create_client(String::from("test client node 1")));
        let mut client_node_two = ClientNode::new("test client node 2", 22, server_node.create_client(String::from("test client node 2")));

        server_node.start();
        client_node_one.start();
        client_node_two.start();

        assert_eq!(client_node_one.test_number, 1);
        assert_eq!(client_node_two.test_number, 1);
    }

    #[test]
    fn test_update_client_server_nodes() {
        let mut server_node = ServerNode::new("test server node", 12);
        let mut client_node_one = ClientNode::new("test client node 1", 12, server_node.create_client(String::from("test client node 1")));
        let mut client_node_two = ClientNode::new("test client node 2", 22, server_node.create_client(String::from("test client node 2")));

        server_node.start();
        client_node_one.start();
        client_node_two.start();

        client_node_one.update();
        client_node_two.update();
        server_node.update();
        client_node_one.update();
        client_node_two.update();

        assert_eq!(client_node_one.test_number, 3);
        assert_eq!(client_node_two.test_number, 3);
    }

    #[test]
    fn test_shutdown_client_server_nodes() {
        let mut server_node = ServerNode::new("test server node", 12);
        let mut client_node_one = ClientNode::new("test client node 1", 22, server_node.create_client(String::from("test client node 1")));
        let mut client_node_two = ClientNode::new("test client node 2", 2, server_node.create_client(String::from("test client node 2")));

        server_node.start();
        client_node_one.start();
        client_node_two.start();

        client_node_one.update();
        client_node_two.update();
        server_node.update();
        client_node_one.update();
        client_node_two.update();

        server_node.shutdown();
        client_node_one.shutdown();
        client_node_two.shutdown();

        assert_eq!(client_node_one.test_number, u128::MAX);
        assert_eq!(client_node_two.test_number, u128::MAX);
    }
}