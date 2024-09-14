//!
//! Local Update Clients and Servers
//!
//! Local Update Clients and Servers utilize crossbeam channels
//! to send requests from clients to servers, updates from servers to clients,
//! and responses from servers to clients
//!

use std::{collections::HashMap, convert::Infallible, hash::Hash};

use crossbeam::channel::{self, Receiver, Sender};

use ncomm_core::{UpdateClient, UpdateServer};

/// A local client that sends requests via a crossbeam channel, receives updates
/// via another channel, and finally receives a response from another channel.
pub struct LocalUpdateClient<Req, Updt, Res> {
    /// The receiving end of a crossbeam channel for updates
    update_rx: Receiver<(Req, Updt)>,
    /// The receiving end of a crossbeam channel for responses
    response_rx: Receiver<(Req, Res)>,
    /// The sending end of a crossbeam channel to send requests
    tx: Sender<Req>,
}

impl<Req, Updt, Res> UpdateClient for LocalUpdateClient<Req, Updt, Res> {
    type Request = Req;
    type Update = Updt;
    type Response = Res;
    type Error = Infallible;

    fn send_request(&mut self, request: Self::Request) -> Result<(), Self::Error> {
        self.tx.send(request).unwrap();
        Ok(())
    }

    fn poll_for_update(&mut self) -> Result<Option<(Self::Request, Self::Update)>, Self::Error> {
        match self.update_rx.try_recv() {
            Ok(update) => Ok(Some(update)),
            Err(_) => Ok(None),
        }
    }

    fn poll_for_updates(&mut self) -> Vec<Result<(Self::Request, Self::Update), Self::Error>> {
        let mut updates = Vec::new();
        for update in self.update_rx.try_iter() {
            updates.push(Ok(update));
        }
        updates
    }

    fn poll_for_response(
        &mut self,
    ) -> Result<Option<(Self::Request, Self::Response)>, Self::Error> {
        match self.response_rx.try_recv() {
            Ok(response) => Ok(Some(response)),
            Err(_) => Ok(None),
        }
    }

    fn poll_for_responses(&mut self) -> Vec<Result<(Self::Request, Self::Response), Self::Error>> {
        let mut responses = Vec::new();
        for response in self.response_rx.try_iter() {
            responses.push(Ok(response));
        }
        responses
    }
}

/// A local update server that receives requests via a crossbeam channel, responds
/// with updates via another channel, and finally sends a response via a final
/// channel
pub struct LocalUpdateServer<Req: Clone, Updt, Res, K: Hash + Eq + Clone> {
    /// A map between client identifiers and their channels
    #[allow(clippy::type_complexity)]
    client_map: HashMap<K, (Receiver<Req>, Sender<(Req, Updt)>, Sender<(Req, Res)>)>,
}

impl<Req: Clone, Updt, Res, K: Hash + Eq + Clone> Default for LocalUpdateServer<Req, Updt, Res, K> {
    fn default() -> Self {
        Self {
            client_map: HashMap::new(),
        }
    }
}

impl<Req: Clone, Updt, Res, K: Hash + Eq + Clone> LocalUpdateServer<Req, Updt, Res, K> {
    /// Create a new local update server
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new local update client for this local server
    pub fn create_update_client(&mut self, key: K) -> LocalUpdateClient<Req, Updt, Res> {
        let (req_tx, req_rx) = channel::unbounded();
        let (updt_tx, updt_rx) = channel::unbounded();
        let (res_tx, res_rx) = channel::unbounded();
        self.client_map.insert(key, (req_rx, updt_tx, res_tx));
        LocalUpdateClient {
            update_rx: updt_rx,
            response_rx: res_rx,
            tx: req_tx,
        }
    }
}

impl<Req: Clone, Updt, Res, K: Hash + Eq + Clone> UpdateServer
    for LocalUpdateServer<Req, Updt, Res, K>
{
    type Request = Req;
    type Update = Updt;
    type Response = Res;
    type Key = K;
    type Error = Infallible;

    fn poll_for_request(&mut self) -> Result<Option<(Self::Key, Self::Request)>, Self::Error> {
        for (k, (rx, _, _)) in self.client_map.iter() {
            if let Ok(request) = rx.try_recv() {
                return Ok(Some((k.clone(), request)));
            }
        }
        Ok(None)
    }

    fn poll_for_requests(&mut self) -> Vec<Result<(Self::Key, Self::Request), Self::Error>> {
        let mut requests = Vec::new();
        for (k, (rx, _, _)) in self.client_map.iter() {
            for request in rx.try_iter() {
                requests.push(Ok((k.clone(), request)));
            }
        }
        requests
    }

    fn send_update(
        &mut self,
        client_key: Self::Key,
        request: &Self::Request,
        update: Self::Update,
    ) -> Result<(), Self::Error> {
        if let Some((_, tx, _)) = self.client_map.get(&client_key) {
            tx.send((request.clone(), update)).unwrap();
        }
        Ok(())
    }

    fn send_response(
        &mut self,
        client_key: Self::Key,
        request: Self::Request,
        response: Self::Response,
    ) -> Result<(), Self::Error> {
        if let Some((_, _, tx)) = self.client_map.get(&client_key) {
            tx.send((request, response)).unwrap();
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
            Self { num: random() }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Update {
        num: u64,
    }

    impl Update {
        pub fn new(request: Request) -> Self {
            Self {
                num: request.num.wrapping_mul(2),
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
    fn test_local_update_client_server() {
        let mut server = LocalUpdateServer::new();
        let mut client = server.create_update_client(0u8);

        let original_request = Request::new();
        let original_update = Update::new(original_request.clone());
        let original_response = Response::new(original_request.clone());

        client.send_request(original_request.clone()).unwrap();

        for request in server.poll_for_requests() {
            let Ok((client, request)) = request;
            assert_eq!(request, original_request);
            server
                .send_update(client, &request, Update::new(request.clone()))
                .unwrap();
            server
                .send_response(client, request, Response::new(request.clone()))
                .unwrap();
        }

        for update in client.poll_for_updates() {
            let Ok((request, update)) = update;
            assert_eq!(request, original_request);
            assert_eq!(update, original_update);
        }

        for response in client.poll_for_responses() {
            let Ok((request, response)) = response;
            assert_eq!(request, original_request);
            assert_eq!(response, original_response);
        }
    }

    #[test]
    fn test_local_update_client_server_singular_request() {
        let mut server = LocalUpdateServer::new();
        let mut client = server.create_update_client(0u8);

        let original_request = Request::new();
        let original_update = Update::new(original_request.clone());
        let original_response = Response::new(original_request.clone());

        client.send_request(original_request.clone()).unwrap();

        if let Ok(Some((client, request))) = server.poll_for_request() {
            assert_eq!(request, original_request);
            server
                .send_update(client, &request, Update::new(request.clone()))
                .unwrap();
            server
                .send_response(client, request, Response::new(request.clone()))
                .unwrap();
        } else {
            assert!(false, "Expected a request to be received");
        }

        if let Ok(Some((request, update))) = client.poll_for_update() {
            assert_eq!(request, original_request);
            assert_eq!(update, original_update);
        } else {
            assert!(false, "Expected an update to be received");
        }

        if let Ok(Some((request, response))) = client.poll_for_response() {
            assert_eq!(request, original_request);
            assert_eq!(response, original_response);
        } else {
            assert!(false, "Expected a response to be received");
        }
    }
}
