//!
//! Local Update Clients and Servers
//! 
//! Local Update Clients and Servers utilize crossbeam channels
//! to send requests from clients to servers, updates from servers to clients,
//! and responses from servers to clients
//! 

use std::{
    collections::HashMap,
    convert::Infallible,
    hash::Hash,
};

use crossbeam::channel::{self, Sender, Receiver};

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

    fn poll_for_updates(&mut self) -> Vec<Result<(Self::Request, Self::Update), Self::Error>> {
        let mut updates = Vec::new();
        for update in self.update_rx.try_iter() {
            updates.push(Ok(update));
        }
        updates
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
    client_map: HashMap<K, (Receiver<Req>, Sender<(Req, Updt)>, Sender<(Req, Res)>)>,
}

impl<Req: Clone, Updt, Res, K: Hash + Eq + Clone> LocalUpdateServer<Req, Updt, Res, K> {
    /// Create a new local update server
    pub fn new() -> Self {
        Self {
            client_map: HashMap::new(),
        }
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

impl<Req: Clone, Updt, Res, K: Hash + Eq + Clone> UpdateServer for LocalUpdateServer<Req, Updt, Res, K> {
    type Request = Req;
    type Update = Updt;
    type Response = Res;
    type Key = K;
    type Error = Infallible;

    fn poll_for_requests(&mut self) -> Vec<Result<(Self::Key, Self::Request), Self::Error>> {
        let mut requests = Vec::new();
        for (k, (rx, _, _)) in self.client_map.iter() {
            for request in rx.try_iter() {
                requests.push(Ok((k.clone(), request)));
            }
        }
        requests
    }

    fn send_update(&mut self, client_key: Self::Key, request: &Self::Request, update: Self::Update) -> Result<(), Self::Error> {
        if let Some((_, tx, _)) = self.client_map.get(&client_key) {
            tx.send((request.clone(), update)).unwrap();
        }
        Ok(())
    }

    fn send_response(&mut self, client_key: Self::Key, request: Self::Request, response: Self::Response) -> Result<(), Self::Error> {
        if let Some((_, _, tx)) = self.client_map.get(&client_key) {
            tx.send((request ,response)).unwrap();
        }
        Ok(())
    }
}