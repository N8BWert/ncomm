pub mod local;

use std::error::Error;

use crate::client_server::{Request, Response};

pub trait UpdateMessage: PartialEq + Send + Clone {}

pub trait UpdateClient<Req: Request, Updt: UpdateMessage, Res: Response, SendErr: Error>: Send {
    fn send_request(&self, request: Req) -> Result<(), SendErr>;

    fn receive_update(&self) -> Option<Updt>;

    fn receive_response(&self) -> Option<Res>;
}

pub trait UpdateServer<Req: Request, Updt: UpdateMessage, Res: Response, UpdtErr: Error, ResErr: Error>: Send {
    type Client;

    fn create_client(&mut self) -> Self::Client;

    fn get_request(&mut self) -> Vec<Req>;

    fn send_updates(&self, updates: Vec<Updt>) -> Vec<Result<(), UpdtErr>>;

    fn send_responses(&mut self, responses: Vec<Res>) -> Vec<Result<(), ResErr>>;
}