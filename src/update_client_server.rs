pub mod local;

use std::error::Error;

use crate::client_server::{Request, Response};

/// "Empty" Update trait to be implemented by all Updates being sent by update servers.
pub trait Update: PartialEq + Send + Clone {}

/// Trait for UpdateClients to allow them to send requests and receive updates and responses.
pub trait UpdateClient<Req: Request, Updt: Update, Res: Response, SendErr: Error>: Send {
    /// Sends a request to an Update Server
    /// 
    /// Args:
    ///     Request: the request to send to the update server
    /// 
    /// Returns:
    ///     Result<(), SendErr>: the result of the request send
    fn send_request(&self, request: Req) -> Result<(), SendErr>;

    /// Receives an update from the Update Server
    /// 
    /// Returns:
    ///     Option<Updt>: an optional update from the server (if there is one)
    fn receive_update(&self) -> Option<Updt>;

    /// Receives a response from the Update Server
    /// 
    /// Returns:
    ///     Option<Res>: an optional response from the server (if there is one)
    fn receive_response(&self) -> Option<Res>;
}

/// Trait for all Update Servers that allows them to receive requests and send updates and responses
pub trait UpdateServer<Req: Request, Updt: Update, Res: Response, UpdtErr, ResErr>: Send {
    /// The type of Update Client (local, bluetooth, etc...)
    type UpdateClient;

    /// Creates a new Update Client with a given name
    /// 
    /// Args:
    ///     client_name: the name of the new client
    /// 
    /// Returns:
    ///     Self::UpdateClient: a new update client of the type specified
    fn create_update_client(&mut self, client_name: String) -> Self::UpdateClient;

    /// Gets a Vector of all of this update server's clients
    /// 
    /// Returns:
    ///     Vec<String>: a vector of all this server's clients
    fn get_clients(&self) -> Vec<String>;

    /// Receives a vector of requests from each client of this server
    /// 
    /// Returns:
    ///     Vec<(String, Req)>: a vector of (client names, requests)
    fn receive_requests(&self) -> Vec<(String, Req)>;

    /// Sends an update to a specified client
    /// 
    /// Args:
    ///     client: the name of the client to send the update to
    ///     update: the update to send to the client
    /// 
    /// Returns:
    ///     UpdtErr: an error indicating the status of the update send
    fn send_update(&self, client: String, update: Updt) -> UpdtErr;

    /// Sends updates to the specified clients.
    /// 
    /// Args:
    ///     updates: the (client names, updates)
    /// 
    /// Returns:
    ///     Vec<UpdtErr>: a vector of errors indicating the status of each update send
    fn send_updates(&self, updates: Vec<(String, Updt)>) -> Vec<UpdtErr>;

    /// Sends a response to a specified client
    /// 
    /// Args:
    ///     client: the name of the client to send the update to
    ///     response: the response to send to the client
    /// 
    /// Returns:
    ///     ResErr: an error indicating the status of the response send.
    fn send_response(&self, client: String, response: Res) -> ResErr;

    /// Sends responses to the specified clients.
    /// 
    /// Args:
    ///     responses: the (client names, responses)
    /// 
    /// Returns:
    ///     Vec<ResErr>: a vector of errors indicating the status of each response send.
    fn send_responses(&self, responses: Vec<(String, Res)>) -> Vec<ResErr>;
}