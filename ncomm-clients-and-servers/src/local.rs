//!
//! Local Clients and Servers
//! 
//! Local Clients and Servers utilize crossbeam channels
//! to send requests from clients to servers and from responses
//! from servers to clients.
//!

use std::{
    collections::HashMap,
    convert::Infallible,
    hash::Hash
};

use crossbeam::channel::{self, Sender, Receiver};

use ncomm_core::{Client, Server};

/// A local client that sends requests via a crossbeam channel and receives
/// data via another channel
pub struct LocalClient<Req, Res> {
    /// The receiving end of a crossbeam channel for responses
    rx: Receiver<(Req, Res)>,
    /// The sending end of a crossbeam channel to send requests
    tx: Sender<Req>,
}

impl<Req, Res> Client for LocalClient<Req, Res> {
    type Request = Req;
    type Response = Res;
    type Error = Infallible;

    fn send_request(&mut self, request: Self::Request) -> Result<(), Self::Error> {
        self.tx.send(request).unwrap();
        Ok(())
    }

    fn poll_for_responses(&mut self) -> Vec<Result<(Self::Request, Self::Response), Self::Error>> {
        let mut responses = Vec::new();
        for response in self.rx.try_iter() {
            responses.push(Ok(response));
        }
        responses
    }
}

/// A local server that receives data via a crossbeam channel and sends
/// data back via another crossbeam channel.
pub struct LocalServer<Req, Res, K: Hash + Eq + Clone> {
    /// A map between client identifiers and their respective
    /// request receivers and response senders
    client_map: HashMap<K, (Receiver<Req>, Sender<(Req, Res)>)>,
}

impl<Req, Res, K: Hash + Eq + Clone> LocalServer<Req, Res, K> {
    /// Create a new Local Server
    pub fn new() -> Self {
        Self {
            client_map: HashMap::new(),
        }
    }

    /// Create a new local client for this local server
    pub fn create_client(&mut self, key: K) -> LocalClient<Req, Res> {
        let (req_tx, req_rx) = channel::unbounded();
        let (res_tx, res_rx) = channel::unbounded();
        self.client_map.insert(key, (req_rx, res_tx));
        LocalClient {
            rx: res_rx,
            tx: req_tx
        }
    }
}

impl<Req, Res, K: Hash + Eq + Clone> Server for LocalServer<Req, Res, K> {
    type Request = Req;
    type Response = Res;
    type Key = K;
    type Error = Infallible;

    fn poll_for_requests(&mut self) -> Vec<Result<(Self::Key, Self::Request), Self::Error>> {
        let mut requests = Vec::new();
        for (k, (rx, _)) in self.client_map.iter() {
            for request in rx.try_iter() {
                requests.push(Ok((k.clone(), request)));
            }
        }
        requests
    }

    fn send_response(&mut self, client_key: Self::Key, request: Self::Request, response: Self::Response) -> Result<(), Self::Error> {
        if let Some((_, tx)) = self.client_map.get(&client_key) {
            tx.send((request ,response)).unwrap();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::random;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Request {
        num: u64,
    }

    impl Request {
       pub fn new() -> Self {
        Self {
            num: random(),
        }
       } 
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Response {
        num: u64,
    }

    impl Response {
        pub fn new(request: Request) -> Self {
            Self {
                num: request.num.wrapping_mul(4),
            }
        }
    }

    #[test]
    fn test_local_client_server() {
        let mut server = LocalServer::new();
        let mut client = server.create_client(0u8);

        let original_request = Request::new();
        let original_response = Response::new(original_request);
        client.send_request(original_request).unwrap();
        for request in server.poll_for_requests() {
            let Ok((client, request)) = request;
            assert_eq!(request, original_request);
            server.send_response(client, request, Response::new(request.clone())).unwrap();
        }
        for response in client.poll_for_responses() {
            let Ok((request, response)) = response;
            assert_eq!(request, original_request);
            assert_eq!(response, original_response);
        }
    }
}