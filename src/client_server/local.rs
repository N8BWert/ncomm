use std::sync::{mpsc, mpsc::{Sender, Receiver, TryRecvError}};

use crate::node::Node;

use crate::client_server::{Request, Response, Client, Server};

pub struct LocalClient<Req: Request, Res: Response> {
    req_tx: Sender<Req>,
    res_rx: Receiver<Res>
}

pub struct LocalServer<'a, Req: Request, Res: Response> {
    handler: fn(& dyn Node, Req) -> Res,
    node: &'a dyn Node,
    req_rxs: Vec<Receiver<Req>>,
    res_txs: Vec<Sender<Res>>,
}

impl<Req: Request, Res: Response> LocalClient<Req, Res> {
    pub fn new(req_tx: Sender<Req>, res_rx: Receiver<Res>) -> Self {
        Self {req_tx, res_rx }
    }
}

impl<'a, Req: Request, Res: Response> LocalServer<'a, Req, Res> {
    pub fn new(node: &'a dyn Node, handler: fn(& dyn Node, Req) -> Res) -> Self {
        Self { node, handler, req_rxs: Vec::new(), res_txs: Vec::new() }
    }
}

impl<Req: Request, Res: Response> Client<Req, Res> for LocalClient<Req, Res> {
    fn sendRequest(&self, request: Req) {
        // TODO: handle error from send
        self.req_tx.send(request).unwrap();
    }

    fn receiveResponse(&self) -> Result<Res, TryRecvError> {
        self.res_rx.try_recv()
    }
}

impl<'a, Req: Request, Res: Response> Server<Req, Res> for LocalServer<'a, Req, Res> {
    type Client = LocalClient<Req, Res>;

    fn createClient(&mut self) -> LocalClient<Req, Res> {
        let (req_tx, req_rx) = mpsc::channel();
        let (res_tx, res_rx) = mpsc::channel();

        self.req_rxs.push(req_rx);
        self.res_txs.push(res_tx);

        return LocalClient::new(req_tx, res_rx);
    }

    fn handleRequests(&self) {
        for i in 0..self.req_rxs.len() {
            if let Ok(request) = self.req_rxs[i].try_recv() {
                let response = (self.handler)(self.node, request);
                // TODO: handle errors with sending
                self.res_txs[i].send(response);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(PartialEq)]
    struct TestRequest {
        data: u8
    }
    impl TestRequest {
        pub const fn new(data: u8) -> Self {
            Self { data }
        }
    }
    impl Request for TestRequest {}

    #[derive(PartialEq)]
    struct TestResponse {
        data: u8
    }
    impl Response for TestResponse {}

    struct TestServerNode<'a> {
        name: &'a str,
        update_rate: u128,
        num: u8,
    }

    impl<'a> TestServerNode<'a> {
        pub fn new(num: u8) -> Self {
            Self { name: "test server node", update_rate: 0, num }
        }

        pub fn add_num_handler(&self, value: TestRequest) -> TestResponse {
            TestResponse {data: value.data + self.num }
        }
    }

    impl<'a> Node for TestServerNode<'a> {
        fn name(&self) -> String {
            String::from(self.name)
        }

        fn start(&mut self) {}

        fn update(&mut self) {}

        fn get_update_rate(&self) -> u128 {
            self.update_rate
        }

        fn shutdown(&mut self) {}

        fn debug(&self) -> String {
            format!("Test Server Node:\n{}\n", self.num)
        }
    }

    #[test]
    fn test_create_client_server() {
        let testNode = TestServerNode::new(10);
        let mut testServer: LocalServer<_, _> = LocalServer::new(&testNode, testNode.add_num_handler);
        let mut testClient: LocalClient<_, _> = testServer.createClient();
        
        assert!(testServer.req_rxs.len() == 1);
        assert!(testServer.res_txs.len() == 1);
        assert_eq!(testServer.node, testNode);
    }

    #[test]
    fn test_send_data_client_server() {
        let testNode = TestServerNode::new(10);
        let mut testServer = LocalServer::new(&testNode, testNode.add_num_handler);
        let mut testClient = testServer.createClient();

        let request = TestRequest::new(7);
        testClient.sendRequest(request);
        testServer.handleRequests();
        if let Ok(response) = testClient.receiveResponse() {
            assert_eq!(response.data, 17);
        } else {
            assert!(false);
        }
    }
}