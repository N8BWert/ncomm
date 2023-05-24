use std::{sync::{mpsc, mpsc::{Sender, Receiver}}};
use std::collections::HashMap;

use crate::client_server::{Request, Response};
use crate::client_server::local::SendError;

use crate::update_client_server::{Update, UpdateClient, UpdateServer};

pub struct LocalUpdateClient<Req: Request, Updt: Update, Res: Response> {
    req_tx: Sender<Req>,
    updt_rx: Receiver<Updt>,
    res_rx: Receiver<Res>,
}

struct LocalUpdateServerChannels<Req: Request, Updt: Update, Res: Response> {
    req_rx: Receiver<Req>,
    updt_tx: Sender<Updt>,
    res_tx: Sender<Res>,
}

pub struct LocalUpdateServer<Req: Request, Updt: Update, Res: Response> {
    client_mappings: HashMap<String, LocalUpdateServerChannels<Req, Updt, Res>>,
}

impl<Req: Request, Updt: Update, Res: Response> LocalUpdateClient<Req, Updt, Res> {
    pub const fn new(req_tx: Sender<Req>, updt_rx: Receiver<Updt>, res_rx: Receiver<Res>) -> Self {
        Self { req_tx, updt_rx, res_rx }
    }
}

impl<Req: Request, Updt: Update, Res: Response> LocalUpdateServerChannels<Req, Updt, Res> {
    pub const fn new(req_rx: Receiver<Req>, updt_tx: Sender<Updt>, res_tx: Sender<Res>) -> Self {
        Self { req_rx, updt_tx, res_tx }
    }
}

impl<Req: Request, Updt: Update, Res: Response> LocalUpdateServer<Req, Updt, Res> {
    pub fn new() -> Self {
        Self { client_mappings: HashMap::new() }
    }
}

impl<Req: Request, Updt: Update, Res: Response> UpdateClient<Req, Updt, Res, mpsc::SendError<Req>> for LocalUpdateClient<Req, Updt, Res> {
    fn send_request(&self, request: Req) -> Result<(), mpsc::SendError<Req>> {
        self.req_tx.send(request)
    }

    fn receive_update(&self) -> Option<Updt> {
        let iter = self.updt_rx.try_iter();

        if let Some(update) = iter.last() {
            return Some(update);
        }
        return None;
    }

    fn receive_response(&self) -> Option<Res> {
        let iter = self.res_rx.try_iter();

        if let Some(response) = iter.last() {
            return Some(response);
        }
        return None;
    }
}

impl<Req: Request, Updt: Update, Res: Response> UpdateServer<Req, Updt, Res, SendError<Updt>, SendError<Res>> for LocalUpdateServer<Req, Updt, Res> {
    type Client = LocalUpdateClient<Req, Updt, Res>;

    fn create_client(&mut self, client_name: String) -> Self::Client {
        let (req_tx, req_rx) = mpsc::channel();
        let (updt_tx, updt_rx) = mpsc::channel();
        let (res_tx, res_rx) = mpsc::channel();

        let channels = LocalUpdateServerChannels::new(req_rx, updt_tx, res_tx);
        self.client_mappings.insert(client_name, channels);

        return LocalUpdateClient::new(req_tx, updt_rx, res_rx);
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

    fn send_update(&self, client: String, update: Updt) -> SendError<Updt> {
        if let Some(channels) = self.client_mappings.get(&client) {
            if let Err(send_err) = channels.updt_tx.send(update) {
                return SendError::<Updt>::SendIncomplete((client, send_err));
            } else {
                return SendError::<Updt>::NoError(client);
            }
        } else {
            return SendError::<Updt>::ClientNotFound(client);
        }
    }

    fn send_updates(&self, updates: Vec<(String, Updt)>) -> Vec<SendError<Updt>> {
        let mut errors = Vec::with_capacity(updates.len());

        for (client, update) in updates {
            if let Some(channels) = self.client_mappings.get(&client) {
                if let Err(send_err) = channels.updt_tx.send(update) {
                    errors.push(SendError::<Updt>::SendIncomplete((client, send_err)));
                } else {
                    errors.push(SendError::<Updt>::NoError(client));
                }
            } else {
                errors.push(SendError::<Updt>::ClientNotFound(client));
            }
        }

        return errors;
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
    use ncomm_macro_derive::{Request, Response, Update};

    #[derive(PartialEq, Clone, Request, Debug)]
    struct TestRequest {
        data: u8
    }
    impl TestRequest {
        pub const fn new(data: u8) -> Self {
            Self { data }
        }
    }

    #[derive(PartialEq, Clone, Update, Debug)]
    struct TestUpdate {
        data: u8
    }
    impl TestUpdate {
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
    fn test_create_update_client_server() {
        let mut test_server: LocalUpdateServer<TestRequest, TestUpdate, TestResponse> = LocalUpdateServer::new();
        let _ = test_server.create_client(String::from("test client"));

        assert_eq!(test_server.client_mappings.len(), 1);
    }

    #[test]
    fn test_get_clients_update_client_server() {
        let mut test_server: LocalUpdateServer<TestRequest, TestUpdate, TestResponse> = LocalUpdateServer::new();
        let _ = test_server.create_client(String::from("test client one"));
        let _ = test_server.create_client(String::from("test client two"));
        let _ = test_server.create_client(String::from("test client three"));

        let clients = test_server.get_clients();
        assert_eq!(clients.len(), 3);
    }

    #[test]
    fn test_send_request_update_client_server() {
        let mut test_server: LocalUpdateServer<TestRequest, TestUpdate, TestResponse> = LocalUpdateServer::new();
        let test_client = test_server.create_client(String::from("test client"));

        let request = TestRequest::new(7);
        let err = test_client.send_request(request);
        assert_eq!(Ok(()), err);

        let requests = test_server.receive_requests();
        assert_eq!(requests[0].0, String::from("test client"));
        assert_eq!(requests[0].1.data, 7);
    }
    #[test]
    fn test_send_multiple_requests_update_client_server() {
        let mut test_server: LocalUpdateServer<TestRequest, TestUpdate, TestResponse> = LocalUpdateServer::new();
        let test_client_one = test_server.create_client(String::from("test client one"));
        let test_client_two = test_server.create_client(String::from("test client two"));

        let request_one = TestRequest::new(7);
        let err = test_client_one.send_request(request_one);
        assert_eq!(err, Ok(()));

        let request_two = TestRequest::new(8);
        let err = test_client_two.send_request(request_two);
        assert_eq!(err, Ok(()));

        let requests = test_server.receive_requests();

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
    fn test_send_many_requests_from_same_client_update_client_server() {
        let mut test_server: LocalUpdateServer<TestRequest, TestUpdate, TestResponse> = LocalUpdateServer::new();
        let test_client = test_server.create_client(String::from("test client"));

        let err = test_client.send_request(TestRequest::new(7));
        assert_eq!(err, Ok(()));

        let err = test_client.send_request(TestRequest::new(8));
        assert_eq!(err, Ok(()));

        let err = test_client.send_request(TestRequest::new(9));
        assert_eq!(err, Ok(()));

        let requests = test_server.receive_requests();
        assert_eq!(requests[0].0, String::from("test client"));
        assert_eq!(requests[0].1.data, 9);
    }

    #[test]
    fn test_send_update_update_client_server() {
        let mut test_server: LocalUpdateServer<TestRequest, TestUpdate, TestResponse> = LocalUpdateServer::new();
        let test_client = test_server.create_client(String::from("test client"));

        let err = test_server.send_update(String::from("test client"), TestUpdate::new(7));
        assert_eq!(err, SendError::<TestUpdate>::NoError(String::from("test client")));

        let update = test_client.receive_update();
        assert_eq!(update.unwrap().data, 7);
    }

    #[test]
    fn test_receive_empty_update_update_client_server() {
        let mut test_server: LocalUpdateServer<TestRequest, TestUpdate, TestResponse> = LocalUpdateServer::new();
        let test_client = test_server.create_client(String::from("test client"));

        let update = test_client.receive_update();
        assert_eq!(None, update);
    }

    #[test]
    fn test_send_multiple_updates_to_same_client_update_client_server() {
        let mut test_server: LocalUpdateServer<TestRequest, TestUpdate, TestResponse> = LocalUpdateServer::new();
        let test_client = test_server.create_client(String::from("test client"));

        let err = test_server.send_update(String::from("test client"), TestUpdate::new(7));
        assert_eq!(err, SendError::<TestUpdate>::NoError(String::from("test client")));

        let err = test_server.send_update(String::from("test client"), TestUpdate::new(8));
        assert_eq!(err, SendError::<TestUpdate>::NoError(String::from("test client")));

        let err = test_server.send_update(String::from("test client"), TestUpdate::new(9));
        assert_eq!(err, SendError::<TestUpdate>::NoError(String::from("test client")));

        let update = test_client.receive_update();
        assert_eq!(update.unwrap().data, 9);
    }

    #[test]
    fn test_send_updates_to_multiple_clients_update_client_server() {
        let mut test_server: LocalUpdateServer<TestRequest, TestUpdate, TestResponse> = LocalUpdateServer::new();
        let test_client_one = test_server.create_client(String::from("test client one"));
        let test_client_two = test_server.create_client(String::from("test client two"));

        let update_one = TestUpdate::new(7);
        let update_two = TestUpdate::new(8);
        
        let errs = test_server.send_updates(vec![
            (String::from("test client one"), update_one),
            (String::from("test client two"), update_two),
        ]);
        assert_eq!(errs[0], SendError::<TestUpdate>::NoError(String::from("test client one")));
        assert_eq!(errs[1], SendError::<TestUpdate>::NoError(String::from("test client two")));

        let update_one = test_client_one.receive_update();
        assert_eq!(update_one.unwrap().data, 7);

        let update_two = test_client_two.receive_update();
        assert_eq!(update_two.unwrap().data, 8);
    }

    #[test]
    fn test_send_multiple_updates_to_multiple_clients_update_client_server() {
        let mut test_server: LocalUpdateServer<TestRequest, TestUpdate, TestResponse> = LocalUpdateServer::new();
        let test_client_one = test_server.create_client(String::from("test client one"));
        let test_client_two = test_server.create_client(String::from("test client two"));

        let update_one = TestUpdate::new(7);
        let update_two = TestUpdate::new(8);
        let update_three = TestUpdate::new(9);
        let update_four = TestUpdate::new(10);

        let errs = test_server.send_updates(vec![
            (String::from("test client one"), update_one),
            (String::from("test client two"), update_two),
        ]);
        assert_eq!(errs[0], SendError::<TestUpdate>::NoError(String::from("test client one")));
        assert_eq!(errs[1], SendError::<TestUpdate>::NoError(String::from("test client two")));

        let errs = test_server.send_updates(vec![
            (String::from("test client one"), update_three),
            (String::from("test client two"), update_four),
        ]);
        assert_eq!(errs[0], SendError::<TestUpdate>::NoError(String::from("test client one")));
        assert_eq!(errs[1], SendError::<TestUpdate>::NoError(String::from("test client two")));

        let update_three = test_client_one.receive_update();
        assert_eq!(update_three.unwrap().data, 9);
        
        let update_four = test_client_two.receive_update();
        assert_eq!(update_four.unwrap().data, 10);
    }

    #[test]
    fn test_send_update_to_one_of_many_clients_update_client_server() {
        let mut test_server: LocalUpdateServer<TestRequest, TestUpdate, TestResponse> = LocalUpdateServer::new();
        let _ = test_server.create_client(String::from("test client one"));
        let test_client_two = test_server.create_client(String::from("test client two"));

        let update_one = TestUpdate::new(7);

        let errs = test_server.send_updates(vec![(String::from("test client two"), update_one)]);
        assert_eq!(errs[0], SendError::<TestUpdate>::NoError(String::from("test client two")));

        let update_one = test_client_two.receive_update();
        assert_eq!(update_one.unwrap().data, 7);
    }

    // BEGIN RESPONSE TESTING //

    #[test]
    fn test_send_response_update_client_server() {
        let mut test_server: LocalUpdateServer<TestRequest, TestUpdate, TestResponse> = LocalUpdateServer::new();
        let test_client = test_server.create_client(String::from("test client"));

        let err = test_server.send_response(String::from("test client"), TestResponse::new(7));
        assert_eq!(err, SendError::<TestResponse>::NoError(String::from("test client")));

        let response = test_client.receive_response();
        assert_eq!(response.unwrap().data, 7);
    }

    #[test]
    fn test_receive_empty_response_update_client_server() {
        let mut test_server: LocalUpdateServer<TestRequest, TestUpdate, TestResponse> = LocalUpdateServer::new();
        let test_client = test_server.create_client(String::from("test client"));

        let response = test_client.receive_response();
        assert_eq!(None, response);
    }

    #[test]
    fn test_send_multiple_responses_to_same_client_update_client_server() {
        let mut test_server: LocalUpdateServer<TestRequest, TestUpdate, TestResponse> = LocalUpdateServer::new();
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
    fn test_send_responses_to_multiple_clients_update_client_server() {
        let mut test_server: LocalUpdateServer<TestRequest, TestUpdate, TestResponse> = LocalUpdateServer::new();
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
    fn test_send_multiple_responses_to_multiple_clients_update_client_server() {
        let mut test_server: LocalUpdateServer<TestRequest, TestUpdate, TestResponse> = LocalUpdateServer::new();
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
    fn test_send_response_to_one_of_many_clients_update_client_server() {
        let mut test_server: LocalUpdateServer<TestRequest, TestUpdate, TestResponse> = LocalUpdateServer::new();
        let _ = test_server.create_client(String::from("test client one"));
        let test_client_two = test_server.create_client(String::from("test client two"));

        let response_one = TestResponse::new(7);

        let errs = test_server.send_responses(vec![(String::from("test client two"), response_one)]);
        assert_eq!(errs[0], SendError::<TestResponse>::NoError(String::from("test client two")));

        let response_one = test_client_two.receive_response();
        assert_eq!(response_one.unwrap().data, 7);
    }
}