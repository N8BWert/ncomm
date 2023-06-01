pub mod local;

use std::error::Error;

/// "Empty" Request trait to be implemented by all Requests being sent by clients.
pub trait Request: PartialEq + Send + Clone {}

/// "Empty" Response trait to be implemented by all Responses being sent by servers.
pub trait Response: PartialEq + Send + Clone {}

/// Trait for all Clients that allows them to send requests and receive responses.
pub trait Client<Req: Request, Res: Response, SendErr: Error>: Send {
    /// Sends a request to a Server
    /// 
    /// Args:
    ///     Request: the request to send to the server
    /// 
    /// Returns:
    ///     Result<(), SendErr>: result of the request send
    fn send_request(&self, request: Req) -> Result<(), SendErr>;

    /// Receives a response from the server
    /// 
    /// Returns:
    ///     Option<Res>: an optional response from the server (if there is one)
    fn receive_response(&self) -> Option<Res>;
}

/// Trait for all Servers that allows them to receive requests and send responses.
pub trait Server<Req: Request, Res: Response, ResErr>: Send {
    /// The type of Client (loca, bluetooth, etc...)
    type Client;

    /// Creates a new client with a given name
    /// 
    /// Args:
    ///     client_name: the name of the new client
    /// 
    /// Returns:
    ///     Self::Client: a new client of the type specified
    fn create_client(&mut self, client_name: String) -> Self::Client;

    /// Gets a Vector of all of this server's clients
    /// 
    /// Returns:
    ///     Vec<String>: a vector of all this server's clients.
    fn get_clients(&self) -> Vec<String>;

    /// Receives a vector of requests from each client of this server
    /// 
    /// Returns:
    ///     Vec<(String, Req)> - a vector of (client names, requests)
    fn receive_requests(&self) -> Vec<(String, Req)>;

    /// Sends a response to a specified client
    /// 
    /// Args:
    ///     client: the name of the client to send the response to
    ///     response: the response to send to the client
    /// 
    /// Returns:
    ///     Error indicating success of the send.
    fn send_response(&self, client: String, response: Res) -> ResErr;

    /// Sends responses to the specified clients.
    /// 
    /// Args:
    ///     responses: the (client names, responses)
    /// 
    /// Returns:
    ///     A vector of errors indicating the success of the sends
    fn send_responses(&self, responses: Vec<(String, Res)>) -> Vec<ResErr>;
}