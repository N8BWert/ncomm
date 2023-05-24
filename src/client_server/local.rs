use std::{sync::{mpsc, mpsc::{Sender, Receiver}}};
use std::collections::HashMap;

use crate::client_server::{Request, Response, Client, Server};

#[derive(PartialEq, Debug)]
pub enum SendError<T> {
    NoError(String),
    ClientNotFound(String),
    SendIncomplete((String, mpsc::SendError<T>)),
}

pub struct LocalClient<Req: Request, Res: Response> {
    req_tx: Sender<Req>,
    res_rx: Receiver<Res>
}

struct LocalServerChannels<Req: Request, Res: Response> {
    req_rx: Receiver<Req>,
    res_tx: Sender<Res>,
}

pub struct LocalServer<Req: Request, Res: Response> {
    client_mappings: HashMap<String, LocalServerChannels<Req, Res>>,
}

impl<Req: Request, Res: Response> LocalClient<Req, Res> {
    pub const fn new(req_tx: Sender<Req>, res_rx: Receiver<Res>) -> Self {
        Self {req_tx, res_rx }
    }
}

impl<Req: Request, Res: Response> LocalServerChannels<Req, Res> {
    pub const fn new(req_rx: Receiver<Req>, res_tx: Sender<Res>) -> Self {
        Self { req_rx, res_tx }
    }
}

impl<Req: Request, Res: Response> LocalServer<Req, Res> {
    pub const fn new() -> Self {
        Self { client_mappings: HashMap::new() }
    }
}

impl<Req: Request, Res: Response> Client<Req, Res, mpsc::SendError<Req>> for LocalClient<Req, Res> {
    fn send_request(&self, request: Req) -> Result<(), mpsc::SendError<Req>> {
        self.req_tx.send(request)
    }

    fn receive_response(&self) -> Option<Res> {
        if let Ok(response) = self.res_rx.try_recv() {
            return Some(response);
        }
        return None;
    }
}

impl<Req: Request, Res: Response> Server<Req, Res, SendError<Res>> for LocalServer<Req, Res> {
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
                requests.push((*client, request));
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
    use ncomm_macro_derive::{Request, Response};

    #[derive(PartialEq, Clone, Request, Debug)]
    struct TestRequest {
        data: u8
    }
    impl TestRequest {
        pub const fn new(data: u8) -> Self {
            Self { data }
        }
    }

    #[derive(PartialEq, Clone, Response, Debug)]
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
        let _: LocalClient<TestRequest, TestResponse> = test_server.create_client();
        
        assert_eq!(test_server.req_rxs.len(), 1);
        assert_eq!(test_server.res_txs.len(), 1);
    }

    #[test]
    fn test_send_data_client_server() {
        let mut test_server: LocalServer<TestRequest, TestResponse> = LocalServer::new();
        let test_client: LocalClient<TestRequest, TestResponse> = test_server.create_client();

        let request = TestRequest::new(7);
        let err = test_client.send_request(request);
        assert_eq!(Ok(()), err);

        let requests = test_server.get_requests();
        test_server.send_responses(vec!(TestResponse::new(requests[0].data + 1)));

        let response = test_client.receive_response();

        assert_eq!(response.unwrap().data, 8);
    }

    #[test]
    fn test_server_receive_data_dne() {
        let mut test_server: LocalServer<TestRequest, TestResponse> = LocalServer::new();
        let test_client = test_server.create_client();

        let response = test_client.receive_response();
        
        assert_eq!(response, None);
    }
}