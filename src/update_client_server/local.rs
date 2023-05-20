use std::sync::{mpsc, mpsc::{Sender, Receiver}};

use crate::client_server::{Request, Response};

use crate::update_client_server::{UpdateMessage, UpdateClient, UpdateServer};

pub struct LocalUpdateClient<Req: Request, Updt: UpdateMessage, Res: Response> {
    req_tx: Sender<Req>,
    updt_rx: Receiver<Updt>,
    res_rx: Receiver<Res>,
}

pub struct LocalUpdateServer<Req: Request, Updt: UpdateMessage, Res: Response> {
    req_rxs: Vec<Receiver<Req>>,
    updt_txs: Vec<Sender<Updt>>,
    res_txs: Vec<Sender<Res>>,
    idxs: Vec<usize>,
}

impl<Req: Request, Updt: UpdateMessage, Res: Response> LocalUpdateClient<Req, Updt, Res> {
    pub const fn new(req_tx: Sender<Req>, updt_rx: Receiver<Updt>, res_rx: Receiver<Res>) -> Self {
        Self { req_tx, updt_rx, res_rx }
    }
}

impl<Req: Request, Updt: UpdateMessage, Res: Response> LocalUpdateServer<Req, Updt, Res> {
    pub const fn new() -> Self {
        Self{ req_rxs: Vec::new(), updt_txs: Vec::new(), res_txs: Vec::new(), idxs: Vec::new() }
    }
}

impl<Req: Request, Updt: UpdateMessage, Res: Response> UpdateClient<Req, Updt, Res, mpsc::SendError<Req>> for LocalUpdateClient<Req, Updt, Res> {
    fn send_request(&self, request: Req) -> Result<(), mpsc::SendError<Req>> {
        self.req_tx.send(request)
    }

    fn receive_update(&self) -> Option<Updt> {
        if let Ok(update) = self.updt_rx.try_recv() {
            return Some(update);
        }
        return None;
    }

    fn receive_response(&self) -> Option<Res> {
        if let Ok(response) = self.res_rx.try_recv() {
            return Some(response);
        }
        return None;
    }
}

impl<Req: Request, Updt: UpdateMessage, Res: Response> UpdateServer<Req, Updt, Res, mpsc::SendError<Updt>, mpsc::SendError<Res>> for LocalUpdateServer<Req, Updt, Res> {
    type Client = LocalUpdateClient<Req, Updt, Res>;

    fn create_client(&mut self) -> Self::Client {
        let (req_tx, req_rx) = mpsc::channel();
        let (updt_tx, updt_rx) = mpsc::channel();
        let (res_tx, res_rx) = mpsc::channel();

        self.req_rxs.push(req_rx);
        self.updt_txs.push(updt_tx);
        self.res_txs.push(res_tx);

        return LocalUpdateClient::new(req_tx, updt_rx, res_rx);
    }

    fn get_request(&mut self) -> Vec<Req> {
        let mut idxs = Vec::new();
        let mut requests = Vec::new();

        for i in 0..self.req_rxs.len() {
            if let Ok(request) = self.req_rxs[i].try_recv() {
                idxs.push(i);
                requests.push(request);
            }
        }

        let mut merged_idxs = Vec::with_capacity(idxs.len() + self.idxs.len());
        while idxs.len() > 0 && self.idxs.len() > 0 {
            if idxs.first() > self.idxs.first() {
                merged_idxs.push(idxs.pop().unwrap());
            } else if idxs.first() < self.idxs.first() {
                merged_idxs.push(self.idxs.pop().unwrap());
            } else {
                if let Some(idx) = idxs.pop() {
                    merged_idxs.push(idx);
                } else if let Some(idx) = self.idxs.pop() {
                    merged_idxs.push(idx);
                }
            }
        }

        self.idxs = merged_idxs;
        return requests;
    }

    fn send_updates(&self, updates: Vec<Updt>) -> Vec<Result<(), mpsc::SendError<Updt>>> {
        let mut errors = Vec::new();

        for (i, idx) in self.idxs.iter().enumerate() {
            errors.push(self.updt_txs[self.idxs[*idx]].send(updates[i].clone()));
        }

        return errors;
    }

    fn send_responses(&mut self, responses: Vec<Res>) -> Vec<Result<(), mpsc::SendError<Res>>> {
        let mut errors = Vec::new();

        for (i, idx) in self.idxs.iter().enumerate() {
            errors.push(self.res_txs[self.idxs[*idx]].send(responses[i].clone()));
        }

        self.idxs = Vec::new();
        
        return errors;
    }
}