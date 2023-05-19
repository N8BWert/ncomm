use std::num::Wrapping;

use ncomm_macro_derive::{Request, Response};

use crate::node::Node;

use crate::client_server::{Request, Response, Server, Client};
use crate::client_server::local::{LocalClient, LocalServer};

#[derive(PartialEq, Clone, Debug, Request)]
pub struct TestRequest {
    data: u128
}

#[derive(PartialEq, Clone, Debug, Response)]
pub struct TestResponse {
    data: u128
}

pub struct ServerNode<'a> {
    name: &'a str,
    update_rate: u128,
    test_number: u128,
    test_server: LocalServer<TestRequest, TestResponse>,
}

pub struct ClientNode<'a> {
    name: &'a str,
    update_rate: u128,
    test_number: u128,
    test_client: LocalClient<TestRequest, TestResponse>,
}

impl<'a> ServerNode<'a> {
    pub const fn new(name: &'a str, update_rate: u128) -> Self {
        Self{
            name,
            update_rate,
            test_number: 0,
            test_server: LocalServer::new()
        }
    }

    pub fn create_client(&mut self) -> LocalClient<TestRequest, TestResponse> {
        self.test_server.create_client()
    }

    pub fn handle_requests(&mut self, requests: Vec<TestRequest>) -> Vec<TestResponse> {
        let mut responses = Vec::new();

        for request in requests {
            responses.push(TestResponse{ data: (Wrapping(request.data) + Wrapping(self.test_number)).0 })
        }

        self.test_number += 1;
        return responses;
    }
}

impl<'a> ClientNode<'a> {
    pub const fn new(name: &'a str, update_rate: u128, client: LocalClient<TestRequest, TestResponse>) -> Self {
        Self { name, update_rate, test_number: 0, test_client: client }
    }
}

impl<'a> Node for ServerNode<'a> {
    fn name(&self) -> String {
        String::from(self.name)
    }

    fn start(&mut self) {
        self.test_number = 1;
    }

    fn update(&mut self) {
        let requests = self.test_server.get_requests();
        let responses = self.handle_requests(requests);
        let _err = self.test_server.send_responses(responses);
    }

    fn get_update_rate(&self) -> u128 {
        self.update_rate
    }

    fn shutdown(&mut self) {}

    fn debug(&self) -> String {
        format!(
            "Server Node:\n{}\n{}\n{}",
            self.name(),
            self.update_rate,
            self.test_number
        )
    }
}

impl<'a> Node for ClientNode<'a> {
    fn name(&self) -> String {
        String::from(self.name)
    }

    fn start(&mut self) {
        self.test_number = 1
    }

    fn update(&mut self) {
        if let Some(response) = self.test_client.receive_response() {
            self.test_number = response.data;
        }

        let _err = self.test_client.send_request(TestRequest{ data: self.test_number });
    }

    fn get_update_rate(&self) -> u128 {
        self.update_rate
    }

    fn shutdown(&mut self) {}

    fn debug(&self) -> String {
        format!(
        "Client Node:\n{}\n{}\n{}",
        self.name(),
        self.update_rate,
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
        let client_node_one = ClientNode::new("test client node 1", 10, server_node.create_client());
        let client_node_two = ClientNode::new("test client node 2", 22, server_node.create_client());

        assert_eq!(server_node.name(), String::from("test server node"));
        assert_eq!(server_node.get_update_rate(), 10);
        assert_eq!(server_node.test_number, 0);

        assert_eq!(client_node_one.name(), String::from("test client node 1"));
        assert_eq!(client_node_one.get_update_rate(), 10);
        assert_eq!(client_node_one.test_number, 0);

        assert_eq!(client_node_two.name(), String::from("test client node 2"));
        assert_eq!(client_node_two.get_update_rate(), 22);
        assert_eq!(client_node_two.test_number, 0);
    }

    #[test]
    fn test_start_client_server_nodes() {
        let mut server_node = ServerNode::new("test server node", 12);
        let mut client_node_one = ClientNode::new("test client node 1", 10, server_node.create_client());
        let mut client_node_two = ClientNode::new("test client node 2", 22, server_node.create_client());

        server_node.start();
        client_node_one.start();
        client_node_two.start();

        assert_eq!(server_node.test_number, 1);
        assert_eq!(client_node_one.test_number, 1);
        assert_eq!(client_node_two.test_number, 1);
    }

    #[test]
    fn test_update_client_server_nodes() {
        let mut server_node = ServerNode::new("test server node", 12);
        let mut client_node_one = ClientNode::new("test client node 1", 12, server_node.create_client());
        let mut client_node_two = ClientNode::new("test client node 2", 22, server_node.create_client());

        server_node.start();
        client_node_one.start();
        client_node_two.start();

        client_node_one.update();
        client_node_two.update();
        server_node.update();
        client_node_one.update();
        client_node_two.update();

        assert_eq!(server_node.test_number, 2);
        assert_eq!(client_node_one.test_number, 2);
        assert_eq!(client_node_two.test_number, 2);
    }

    #[test]
    fn test_shutdown_client_server_nodes() {
        let mut server_node = ServerNode::new("test server node", 12);
        let mut client_node_one = ClientNode::new("test client node 1", 22, server_node.create_client());
        let mut client_node_two = ClientNode::new("test client node 2", 2, server_node.create_client());

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

        assert_eq!(server_node.test_number, 2);
        assert_eq!(client_node_one.test_number, 2);
        assert_eq!(client_node_two.test_number, 2);
    }
}