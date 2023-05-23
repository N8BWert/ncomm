use ncomm_macro_derive::{Request, Update, Response};

use crate::node::Node;

use crate::client_server::{Request, Response};
use crate::update_client_server::{UpdateServer, UpdateClient};
use crate::update_client_server::{Update, local::{LocalUpdateClient, LocalUpdateServer}};

#[derive(PartialEq, Clone, Debug, Request)]
pub struct TestRequest {
    data: u128
}
impl TestRequest {
    pub const fn new(data: u128) -> Self {
        Self { data }
    }
}

#[derive(PartialEq, Clone, Debug, Update)]
pub struct TestUpdate {
    data: u128
}
impl TestUpdate {
    pub const fn new(data: u128) -> Self {
        Self { data }
    }
}

#[derive(PartialEq, Clone, Debug, Response)]
pub struct TestResponse {
    data: u128
}
impl TestResponse {
    pub const fn new(data: u128) -> Self {
        Self { data }
    }
}

pub struct UpdateServerNode<'a> {
    name: &'a str,
    update_rate: u128,
    test_number: u128,
    received_requests: u128,
    num_sent_updates: u8,
    test_server: LocalUpdateServer<TestRequest, TestUpdate, TestResponse>,
}

pub struct UpdateClientNode<'a> {
    name: &'a str,
    update_rate: u128,
    test_number: u128,
    test_client: LocalUpdateClient<TestRequest, TestUpdate, TestResponse>,
    sent_first_request: bool,
}

impl<'a> UpdateServerNode<'a> {
    pub fn new(name: &'a str, update_rate: u128) -> Self {
        Self {
            name,
            update_rate,
            test_number: 0,
            received_requests: 0,
            num_sent_updates: 0,
            test_server: LocalUpdateServer::new()
        }
    }

    pub fn create_client(&mut self, client: String) -> LocalUpdateClient<TestRequest, TestUpdate, TestResponse> {
        self.test_server.create_client(client)
    }
}

impl<'a> UpdateClientNode<'a> {
    pub const fn new(name: &'a str, update_rate: u128, client: LocalUpdateClient<TestRequest, TestUpdate, TestResponse>) -> Self {
        Self {
            name,
            update_rate,
            test_number: 0,
            test_client: client,
            sent_first_request: false,
        }
    }
}

impl<'a> Node for UpdateServerNode<'a> {
    fn name(&self) -> String {
        String::from(self.name)
    }

    fn start(&mut self) {
        self.test_number = 1;
    }

    fn update(&mut self) {
        if (self.received_requests as usize) < self.test_server.get_clients().len() {
            let requests = self.test_server.receive_requests();

            if requests.len() > 0 {
                self.test_number = requests[0].1.data;
                self.received_requests += requests.len() as u128;
            }
        } else {
            let clients = self.test_server.get_clients();
            if self.num_sent_updates < 5 {
                let mut updates = Vec::with_capacity(clients.len());
                for client in clients {
                    updates.push((client, TestUpdate::new((self.num_sent_updates as u128) * 5)));
                }
                self.test_server.send_updates(updates);
                self.num_sent_updates += 1;
            } else {
                let mut responses = Vec::with_capacity(clients.len());
                for client in clients {
                    responses.push((client, TestResponse::new(100)));
                }
                self.test_server.send_responses(responses);
                self.received_requests = 0;
                self.num_sent_updates = 0;
            }
        }
    }

    fn get_update_rate(&self) -> u128 {
        self.update_rate
    }

    fn shutdown(&mut self) {
        let clients = self.test_server.get_clients();
        let mut responses = Vec::with_capacity(clients.len());
        for client in clients {
            responses.push((client, TestResponse::new(u128::MAX)));
        }
        self.test_server.send_responses(responses);
        self.test_number = u128::MAX;
    }

    fn debug(&self) -> String {
        format!(
            "Update Server Node:\n{}\n{}\n{}",
            self.name(),
            self.update_rate,
            self.test_number
        )
    }
}

impl<'a> Node for UpdateClientNode<'a> {
    fn name(&self) -> String {
        String::from(self.name)
    }

    fn start(&mut self) {
        self.test_number = 1;
    }

    fn update(&mut self) {
        if !self.sent_first_request {
            let err = self.test_client.send_request(TestRequest::new(5));

            match err {
                Ok(()) => self.sent_first_request = true,
                _ => self.sent_first_request = false,
            }
        } else {
            if let Some(update) = self.test_client.receive_update() {
                self.test_number = update.data;
            }

            if let Some(response) = self.test_client.receive_response() {
                self.test_number = response.data;
                self.sent_first_request = false;
            }
        }
    }

    fn get_update_rate(&self) -> u128 {
        self.update_rate
    }

    fn shutdown(&mut self) {
        if let Some(response) = self.test_client.receive_response() {
            self.test_number = response.data;
        }
    }

    fn debug(&self) -> String {
        format!(
            "Update Client Node:\n{}\n{}\n{}",
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
    fn test_create_update_client_server_nodes() {
        let mut server_node = UpdateServerNode::new("test update server node", 20);
        let client_node_one = UpdateClientNode::new("test update client node one", 20, server_node.create_client(String::from("test update client node one")));
        let client_node_two = UpdateClientNode::new("test update client node two", 20, server_node.create_client(String::from("test update client node two")));

        assert_eq!(server_node.name(), String::from("test update server node"));
        assert_eq!(server_node.get_update_rate(), 20);
        assert_eq!(server_node.test_number, 0);
        assert_eq!(server_node.test_server.get_clients().len(), 2);
        assert!(server_node.test_server.get_clients()[0] == String::from("test update client node one") || server_node.test_server.get_clients()[0] == String::from("test update client node two"));
        assert!(server_node.test_server.get_clients()[1] == String::from("test update client node one") || server_node.test_server.get_clients()[1] == String::from("test update client node two"));
        assert_eq!(server_node.received_requests, 0);
        assert_eq!(server_node.num_sent_updates, 0);

        assert_eq!(client_node_one.name(), String::from("test update client node one"));
        assert_eq!(client_node_one.get_update_rate(), 20);
        assert_eq!(client_node_one.test_number, 0);
        assert_eq!(client_node_one.sent_first_request, false);

        assert_eq!(client_node_two.name(), String::from("test update client node two"));
        assert_eq!(client_node_two.get_update_rate(), 20);
        assert_eq!(client_node_two.test_number, 0);
        assert_eq!(client_node_two.sent_first_request, false);
    }

    #[test]
    fn test_start_update_client_server_nodes() {
        let mut server_node = UpdateServerNode::new("test update server node", 20);
        let mut client_node_one = UpdateClientNode::new("test update client node one", 20, server_node.create_client(String::from("test update client node one")));
        let mut client_node_two = UpdateClientNode::new("test update client node two", 20, server_node.create_client(String::from("test update client node two")));

        server_node.start();
        client_node_one.start();
        client_node_two.start();

        assert_eq!(server_node.name(), String::from("test update server node"));
        assert_eq!(server_node.get_update_rate(), 20);
        assert_eq!(server_node.test_number, 1);
        assert_eq!(server_node.test_server.get_clients().len(), 2);
        assert!(server_node.test_server.get_clients()[0] == String::from("test update client node one") || server_node.test_server.get_clients()[0] == String::from("test update client node two"));
        assert!(server_node.test_server.get_clients()[1] == String::from("test update client node one") || server_node.test_server.get_clients()[1] == String::from("test update client node two"));
        assert_eq!(server_node.received_requests, 0);
        assert_eq!(server_node.num_sent_updates, 0);

        assert_eq!(client_node_one.name(), String::from("test update client node one"));
        assert_eq!(client_node_one.get_update_rate(), 20);
        assert_eq!(client_node_one.test_number, 1);
        assert_eq!(client_node_one.sent_first_request, false);

        assert_eq!(client_node_two.name(), String::from("test update client node two"));
        assert_eq!(client_node_two.get_update_rate(), 20);
        assert_eq!(client_node_two.test_number, 1);
        assert_eq!(client_node_two.sent_first_request, false);
    }

    #[test]
    fn test_update_update_client_server_nodes() {
        let mut server_node = UpdateServerNode::new("test update server node", 20);
        let mut client_node_one = UpdateClientNode::new("test update client node one", 20, server_node.create_client(String::from("test update client node one")));
        let mut client_node_two = UpdateClientNode::new("test update client node two", 20, server_node.create_client(String::from("test update client node two")));

        server_node.start();
        client_node_one.start();
        client_node_two.start();

        for _ in 0..3 {
            server_node.update();
            client_node_one.update();
            client_node_two.update();
        }

        assert_eq!(server_node.received_requests, 2);
        assert_eq!(server_node.num_sent_updates, 1);
        assert_eq!(server_node.test_number, 5);

        assert_eq!(client_node_one.sent_first_request, true);
        assert_eq!(client_node_one.test_number, 0);

        assert_eq!(client_node_two.sent_first_request, true);
        assert_eq!(client_node_two.test_number, 0);

        for _ in 0..5 {
            server_node.update();
            client_node_one.update();
            client_node_two.update();
        }

        assert_eq!(server_node.received_requests, 0);
        assert_eq!(server_node.num_sent_updates, 0);
        assert_eq!(server_node.test_number, 5);

        assert_eq!(client_node_one.sent_first_request, false);
        assert_eq!(client_node_one.test_number, 100);
        
        assert_eq!(client_node_two.sent_first_request, false);
        assert_eq!(client_node_two.test_number, 100);
    }

    #[test]
    fn test_shutdown_client_server_nodes() {
        let mut server_node = UpdateServerNode::new("test update server node", 20);
        let mut client_node_one = UpdateClientNode::new("test update client node one", 20, server_node.create_client(String::from("test update client node one")));
        let mut client_node_two = UpdateClientNode::new("test update client node two", 20, server_node.create_client(String::from("test update client node two")));

        server_node.start();
        client_node_one.start();
        client_node_two.start();

        server_node.shutdown();
        client_node_one.shutdown();
        client_node_two.shutdown();

        assert_eq!(server_node.test_number, u128::MAX);
        assert_eq!(client_node_one.test_number, u128::MAX);
        assert_eq!(client_node_two.test_number, u128::MAX);
    }
}