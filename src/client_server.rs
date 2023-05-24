pub mod local;

use std::error::Error;

pub trait Request: PartialEq + Send + Clone {}

pub trait Response: PartialEq + Send + Clone {}

pub trait Client<Req: Request, Res: Response, SendErr: Error>: Send {
    fn send_request(&self, request: Req) -> Result<(), SendErr>;

    fn receive_response(&self) -> Option<Res>;
}

pub trait Server<Req: Request, Res: Response, ResErr>: Send {
    type Client;

    fn create_client(&mut self, client_Name: String) -> Self::Client;

    fn get_clients(&self) -> Vec<String>;

    fn receive_requests(&self) -> Vec<(String, Req)>;

    fn send_response(&self, client: String, response: Res) -> ResErr;

    fn send_responses(&self, responses: Vec<(String, Res)>) -> Vec<ResErr>;
}