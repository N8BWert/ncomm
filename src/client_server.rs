use std::sync::mpsc::TryRecvError;

pub mod local;

pub trait Request: PartialEq + Send {}

pub trait Response: PartialEq + Send {}

pub trait Client<Req: Request, Res: Response>: Send {
    fn sendRequest(&self, request: Req);

    fn receiveResponse(&self) -> Result<Res, TryRecvError>;
}

pub trait Server<Req: Request, Res: Response>: Send {
    type Client;

    fn createClient(&mut self) -> Self::Client;

    fn handleRequests(&self);
}