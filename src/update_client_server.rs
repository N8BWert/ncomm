pub mod local;

use std::error::Error;

use crate::client_server::{Request, Response};

pub trait UpdateMessage: PartialEq + Send + Clone {}

pub trait UpdateClient<Req: Request, Updt: UpdateMessage, Res: Response, SendErr: Error>: Send {
    fn send_request(&self, request: Req) -> Result<(), SendErr>;

    fn receive_update(&self) -> Option<Updt>;

    fn receive_response(&self) -> Option<Res>;
}

pub trait UpdateServer<Req: Request, Updt: UpdateMessage, Res: Response, UpdtErr, ResErr>: Send {
    type Client;

    fn create_client(&mut self, client_name: String) -> Self::Client;

    fn receive_requests(&self) -> Vec<(String, Req)>;

    fn send_update(&self, client: String, update: Updt) -> UpdtErr;

    fn send_updates(&self, updates: Vec<(String, Updt)>) -> Vec<UpdtErr>;

    fn send_response(&self, client: String, response: Res) -> ResErr;

    fn send_responses(&self, responses: Vec<(String, Res)>) -> Vec<ResErr>;
}