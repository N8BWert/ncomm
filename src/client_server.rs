pub mod local;

use std::error::Error;

pub trait Request: PartialEq + Send + Clone {}

pub trait Response: PartialEq + Send + Clone {}

pub trait Client<Req: Request, Res: Response, SendErr: Error>: Send {
    fn send_request(&self, request: Req) -> Result<(), SendErr>;

    fn receive_response(&self) -> Option<Res>;
}

pub trait Server<Req: Request, Res: Response, SendErr: Error>: Send {
    type Client;

    fn create_client(&mut self) -> Self::Client;

    fn get_requests(&mut self) -> Vec<Req>;

    fn send_responses(&self, responses: Vec<Res>) -> Vec<Result<(), SendErr>>;
}