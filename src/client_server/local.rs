use std::{sync::{mpsc, mpsc::{Sender, Receiver}}};

use crate::client_server::{Request, Response, Client, Server};

pub struct LocalClient<Req: Request, Res: Response> {
    req_tx: Sender<Req>,
    res_rx: Receiver<Res>
}

pub struct LocalServer<Req: Request, Res: Response> {
    req_rxs: Vec<Receiver<Req>>,
    res_txs: Vec<Sender<Res>>,
    idxs: Vec<usize>,
}

impl<Req: Request, Res: Response> LocalClient<Req, Res> {
    pub const fn new(req_tx: Sender<Req>, res_rx: Receiver<Res>) -> Self {
        Self {req_tx, res_rx }
    }
}

impl<Req: Request, Res: Response> LocalServer<Req, Res> {
    pub const fn new() -> Self {
        Self { req_rxs: Vec::new(), res_txs: Vec::new(), idxs: Vec::new() }
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

impl<Req: Request, Res: Response> Server<Req, Res, mpsc::SendError<Res>> for LocalServer<Req, Res> {
    type Client = LocalClient<Req, Res>;

    fn create_client(&mut self) -> LocalClient<Req, Res> {
        let (req_tx, req_rx) = mpsc::channel();
        let (res_tx, res_rx) = mpsc::channel();

        self.req_rxs.push(req_rx);
        self.res_txs.push(res_tx);

        return LocalClient::new(req_tx, res_rx);
    }

    fn get_requests(&mut self) -> Vec<Req> {
        let mut idxs = Vec::new();
        let mut requests = Vec::new();

        for i in 0..self.req_rxs.len() {
            if let Ok(request) = self.req_rxs[i].try_recv() {
                idxs.push(i);
                requests.push(request);
            }
        }
        
        self.idxs = idxs;
        return requests;
    }

    fn send_responses(&self, responses: Vec<Res>) -> Vec<Result<(), mpsc::SendError<Res>>> {
        let mut errors = Vec::new();

        for (i, idx) in self.idxs.iter().enumerate() {
            errors.push(self.res_txs[self.idxs[*idx]].send(responses[i].clone()));
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