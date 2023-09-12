//!
//! A local mpsc channel-based Client + Server.
//! 
//! The Local Client + Server sends data along a mpsc channel from the client
//! to the server and back to the client.
//! 

use std::sync::{mpsc, mpsc::{Sender, Receiver}};
use std::collections::HashMap;

use crate::client_server::{Client, Server};

/// Error from sending data along the local client + server.
#[derive(PartialEq, Debug)]
pub enum SendError<T> {
    NoError(String),
    ClientNotFound(String),
    SendIncomplete((String, mpsc::SendError<T>)),
}

/// A Local Client that sends data to a Local Server.
pub struct LocalClient<Req: PartialEq + Send + Clone,
                       Res: PartialEq + Send + Clone> {
    req_tx: Sender<Req>,
    res_rx: Receiver<Res>
}

/// A Singular Channel from a Local Client to the Local Server.
struct LocalServerChannels<Req: PartialEq + Send + Clone,
                           Res: PartialEq + Send + Clone> {
    req_rx: Receiver<Req>,
    res_tx: Sender<Res>,
}

/// A Local Server receives data from a client (of many) processes the data
/// and sends a response back to the client.
pub struct LocalServer<Req: PartialEq + Send + Clone,
                       Res: PartialEq + Send + Clone> {
    client_mappings: HashMap<String, LocalServerChannels<Req, Res>>,
}

impl<Req: PartialEq + Send + Clone,
     Res: PartialEq + Send + Clone> LocalClient<Req, Res> {
    /// Creates a new Local Client with Given sender and receiver.
    pub const fn new(req_tx: Sender<Req>, res_rx: Receiver<Res>) -> Self {
        Self {req_tx, res_rx }
    }
}

impl<Req: PartialEq + Send + Clone,
    Res: PartialEq + Send + Clone> LocalServerChannels<Req, Res> {
    
    const fn new(req_rx: Receiver<Req>, res_tx: Sender<Res>) -> Self {
        Self { req_rx, res_tx }
    }
}

impl<Req: PartialEq + Send + Clone,
     Res: PartialEq + Send + Clone> LocalServer<Req, Res> {
    /// Creates a new Local Server
    pub fn new() -> Self {
        Self { client_mappings: HashMap::new() }
    }
}

impl<Req: PartialEq + Send + Clone,
    Res: PartialEq + Send + Clone> Client<Req, Res, mpsc::SendError<Req>> for LocalClient<Req, Res> {
    fn send_request(&self, request: Req) -> Result<(), mpsc::SendError<Req>> {
        self.req_tx.send(request)
    }

    fn receive_response(&self) -> Option<Res> {
        let iter = self.res_rx.try_iter();

        if let Some(response) = iter.last() {
            return Some(response);
        }
        return None;
    }
}

impl<Req: PartialEq + Send + Clone,
     Res: PartialEq + Send + Clone> Server<Req, Res, SendError<Res>> for LocalServer<Req, Res> {
    type Client = LocalClient<Req, Res>;

    fn create_client(&mut self, client_name: String) -> Self::Client {
        let (req_tx, req_rx) = mpsc::channel();
        let (res_tx, res_rx) = mpsc::channel();

        let channels = LocalServerChannels::new(req_rx, res_tx);
        self.client_mappings.insert(client_name, channels);

        return LocalClient::new(req_tx, res_rx);
    }

    fn get_clients(&self) -> Vec<String> {
        let mut clients = Vec::with_capacity(self.client_mappings.len());

        for (client, _) in self.client_mappings.iter() {
            clients.push(client.clone());
        }

        return clients;
    }

    fn receive_requests(&self) -> Vec<(String, Req)> {
        let mut requests = Vec::new();

        for (client, channels) in self.client_mappings.iter() {
            let iter = channels.req_rx.try_iter();
            if let Some(request) = iter.last() {
                requests.push((client.clone(), request));
            }
        }

        return requests;
    }

    fn send_response(&self, client: String, response: Res) -> SendError<Res> {
        if let Some(channels) = self.client_mappings.get(&client) {
            if let Err(send_err) = channels.res_tx.send(response) {
                return SendError::<Res>::SendIncomplete((client, send_err));
            } else {
                return SendError::<Res>::NoError(client);
            }
        } else {
            return SendError::<Res>::ClientNotFound(client);
        }
    }

    fn send_responses(&self, responses: Vec<(String, Res)>) -> Vec<SendError<Res>> {
        let mut errors = Vec::with_capacity(responses.len());

        for (client, response) in responses {
            if let Some(channels) = self.client_mappings.get(&client) {
                if let Err(send_err) = channels.res_tx.send(response) {
                    errors.push(SendError::<Res>::SendIncomplete((client, send_err)));
                } else {
                    errors.push(SendError::<Res>::NoError(client));
                }
            } else {
                errors.push(SendError::ClientNotFound(client));
            }
        }

        return errors;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(PartialEq, Clone, Debug)]
    struct TestRequest {
        data: u8
    }
    impl TestRequest {
        pub const fn new(data: u8) -> Self {
            Self { data }
        }
    }

    #[derive(PartialEq, Clone, Debug)]
    struct TestResponse {
        data: u8
    }
    impl TestResponse {
        pub const fn new(data: u8) -> Self {
            Self { data }
        }
    }

    #[test]
    fn test_create_client_server() {
        let mut test_server: LocalServer<TestRequest, TestResponse> = LocalServer::new();
        let _: LocalClient<TestRequest, TestResponse> = test_server.create_client(String::from("test client"));

        assert_eq!(test_server.client_mappings.len(), 1);
    }

    #[test]
    fn test_get_clients_client_server() {
        let mut test_server: LocalServer<TestRequest, TestResponse> = LocalServer::new();
        let _ = test_server.create_client(String::from("test client one"));
        let _ = test_server.create_client(String::from("test client two"));
        let _ = test_server.create_client(String::from("test client three"));

        let clients = test_server.get_clients();
        assert_eq!(clients.len(), 3);
    }

    #[test]
    fn test_send_request_client_server() {
        let mut test_server: LocalServer<TestRequest, TestResponse> = LocalServer::new();
        let test_client = test_server.create_client(String::from("test client"));

        let request = TestRequest::new(7);
        let err = test_client.send_request(request);
        assert_eq!(Ok(()), err);

        let requests = test_server.receive_requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].0, String::from("test client"));
        assert_eq!(requests[0].1.data, 7);
    }

    #[test]
    fn test_send_multiple_requests_client_server() {
        let mut test_server: LocalServer<TestRequest, TestResponse> = LocalServer::new();
        let test_client_one = test_server.create_client(String::from("test client one"));
        let test_client_two = test_server.create_client(String::from("test client two"));

        let request_one = TestRequest::new(7);
        let err = test_client_one.send_request(request_one);
        assert_eq!(err, Ok(()));

        let request_two = TestRequest::new(8);
        let err = test_client_two.send_request(request_two);
        assert_eq!(err, Ok(()));

        let requests = test_server.receive_requests();
        assert_eq!(requests.len(), 2);

        match requests[0].0.as_str() {
            "test client one" => assert_eq!(requests[0].1.data, 7),
            "test client two" => assert_eq!(requests[0].1.data, 8),
            _ => assert_eq!(true, false),
        };

        match requests[1].0.as_str() {
            "test client one" => assert_eq!(requests[1].1.data, 7),
            "test client two" => assert_eq!(requests[1].1.data, 8),
            _ => assert_eq!(true, false),
        };
    }

    #[test]
    fn test_send_many_requests_from_same_client_client_server() {
        let mut test_server: LocalServer<TestRequest, TestResponse> = LocalServer::new();
        let test_client = test_server.create_client(String::from("test client"));

        let err = test_client.send_request(TestRequest::new(7));
        assert_eq!(err, Ok(()));

        let err = test_client.send_request(TestRequest::new(8));
        assert_eq!(err, Ok(()));

        let err = test_client.send_request(TestRequest::new(9));
        assert_eq!(err, Ok(()));

        let requests = test_server.receive_requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].0, String::from("test client"));
        assert_eq!(requests[0].1.data, 9);
    }

    #[test]
    fn test_send_response_client_server() {
        let mut test_server: LocalServer<TestRequest, TestResponse> = LocalServer::new();
        let test_client = test_server.create_client(String::from("test client"));

        let err = test_server.send_response(String::from("test client"), TestResponse::new(8));
        assert_eq!(err, SendError::<TestResponse>::NoError(String::from("test client")));

        let response = test_client.receive_response();
        assert_eq!(response.unwrap().data, 8);
    }

    #[test]
    fn test_receive_empty_response_client_server() {
        let mut test_server: LocalServer<TestRequest, TestResponse> = LocalServer::new();
        let test_client = test_server.create_client(String::from("test client"));

        let response = test_client.receive_response();
        assert_eq!(response, None);
    }

    #[test]
    fn test_send_multiple_responses_to_same_client_client_server() {
        let mut test_server: LocalServer<TestRequest, TestResponse> = LocalServer::new();
        let test_client = test_server.create_client(String::from("test client"));

        let err = test_server.send_response(String::from("test client"), TestResponse::new(7));
        assert_eq!(err, SendError::<TestResponse>::NoError(String::from("test client")));

        let err = test_server.send_response(String::from("test client"), TestResponse::new(8));
        assert_eq!(err, SendError::<TestResponse>::NoError(String::from("test client")));

        let err = test_server.send_response(String::from("test client"), TestResponse::new(9));
        assert_eq!(err, SendError::<TestResponse>::NoError(String::from("test client")));

        let response = test_client.receive_response();
        assert_eq!(response.unwrap().data, 9);
    }

    #[test]
    fn test_send_responses_to_multiple_clients_client_server() {
        let mut test_server: LocalServer<TestRequest, TestResponse> = LocalServer::new();
        let test_client_one = test_server.create_client(String::from("test client one"));
        let test_client_two = test_server.create_client(String::from("test client two"));

        let response_one = TestResponse::new(7);
        let response_two = TestResponse::new(8);

        let errs = test_server.send_responses(vec![
            (String::from("test client one"), response_one),
            (String::from("test client two"), response_two),
        ]);
        assert_eq!(errs[0], SendError::<TestResponse>::NoError(String::from("test client one")));
        assert_eq!(errs[1], SendError::<TestResponse>::NoError(String::from("test client two")));

        let response_one = test_client_one.receive_response();
        assert_eq!(response_one.unwrap().data, 7);

        let response_two = test_client_two.receive_response();
        assert_eq!(response_two.unwrap().data, 8);
    }

    #[test]
    fn test_send_multiple_responses_to_multiple_clients_client_server() {
        let mut test_server: LocalServer<TestRequest, TestResponse> = LocalServer::new();
        let test_client_one = test_server.create_client(String::from("test client one"));
        let test_client_two = test_server.create_client(String::from("test client two"));

        let response_one = TestResponse::new(7);
        let response_two = TestResponse::new(8);
        let response_three = TestResponse::new(9);
        let response_four = TestResponse::new(10);

        let errs = test_server.send_responses(vec![
            (String::from("test client one"), response_one),
            (String::from("test client two"), response_two),
        ]);
        assert_eq!(errs[0], SendError::<TestResponse>::NoError(String::from("test client one")));
        assert_eq!(errs[1], SendError::<TestResponse>::NoError(String::from("test client two")));

        let errs = test_server.send_responses(vec![
            (String::from("test client one"), response_three),
            (String::from("test client two"), response_four),
        ]);
        assert_eq!(errs[0], SendError::<TestResponse>::NoError(String::from("test client one")));
        assert_eq!(errs[1], SendError::<TestResponse>::NoError(String::from("test client two")));

        let response_three = test_client_one.receive_response();
        assert_eq!(response_three.unwrap().data, 9);

        let response_four = test_client_two.receive_response();
        assert_eq!(response_four.unwrap().data, 10);
    }

    #[test]
    fn test_send_resopnse_to_one_of_many_clients_client_server() {
        let mut test_server: LocalServer<TestRequest, TestResponse> = LocalServer::new();
        let _ = test_server.create_client(String::from("test client one"));
        let test_client_two = test_server.create_client(String::from("test client two"));

        let response_one = TestResponse::new(7);

        let errs = test_server.send_responses(vec![(String::from("test client two"), response_one)]);
        assert_eq!(errs[0], SendError::<TestResponse>::NoError(String::from("test client two")));

        let response_one = test_client_two.receive_response();
        assert_eq!(response_one.unwrap().data, 7);
    }
}