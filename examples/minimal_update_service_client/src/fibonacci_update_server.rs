use ncomm::update_client_server::{UpdateServer, local::{LocalUpdateClient, LocalUpdateServer}};
use ncomm::node::Node;

#[derive(PartialEq, Clone)]
pub struct FibonacciRequest {
    pub order: u128,
}

#[derive(PartialEq, Clone)]
pub struct FibonacciUpdate {
    pub last_num: u128,
    pub current_num: u128,
}

#[derive(PartialEq, Clone)]
pub struct FibonacciResponse {
    pub num: u128,
}

pub struct FibonacciUpdateServer<'a> {
    name: &'a str,
    last_num: u128,
    current_num: u128,
    order: u128,
    current_order: u128,
    update_server: LocalUpdateServer<FibonacciRequest, FibonacciUpdate, FibonacciResponse>,
}

impl<'a> FibonacciUpdateServer<'a> {
    pub fn new(name: &'a str) -> Self {
        Self { name, last_num: 0, current_num: 0, order: 0, current_order: 0, update_server: LocalUpdateServer::new() }
    }

    pub fn create_client(&mut self, client_name: String) -> LocalUpdateClient<FibonacciRequest, FibonacciUpdate, FibonacciResponse> {
        self.update_server.create_update_client(client_name)
    }
}

impl<'a> Node for FibonacciUpdateServer<'a> {
    fn name(&self) -> String { String::from(self.name) }

    fn get_update_rate(&self) -> u128 { 10u128 }

    fn start(&mut self) {}

    fn update(&mut self) {
        let requests = self.update_server.receive_requests();
        
        if requests.len() > 0 {
            self.last_num = 1;
            self.current_num = 1;
            self.current_order = 1;
            self.order = requests[0].1.order;
        } else {
            if self.current_order < self.order {
                let next_num = self.last_num + self.current_num;
                self.last_num = self.current_num;
                self.current_num = next_num;
                for client in self.update_server.get_clients() {
                    self.update_server.send_update(client, FibonacciUpdate { last_num: self.last_num, current_num: self.current_num });
                }
                self.current_order += 1;
            } else if self.current_order == self.order  {
                for client in self.update_server.get_clients() {
                    self.update_server.send_response(client, FibonacciResponse { num: self.current_num });
                }
                self.current_order += 1;
            }
        }
    }

    fn shutdown(&mut self) {}

    fn debug(&self) -> String {
        format!(
            "Fibonacci Update Server:\n{}",
            self.name()
        )
    }
}