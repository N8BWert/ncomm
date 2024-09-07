//!
//! NComm Client Server Traits.
//!
//! Servers should be a single unique entity that can have multiple clients
//! that request something from them (in the form of a request).
//!

/// A common abstraction for all NComm clients to allow for the creation
/// of a common method of sending requests and receiving responses.
pub trait Client {
    /// The type of data used as a request by the client
    type Request;
    /// The type of data used as a response from the server
    type Response;
    /// The type of error from sending or receiving data from
    /// the server
    type Error;

    /// Send a request to the server this client is associated with
    fn send_request(&mut self, request: Self::Request) -> Result<(), Self::Error>;

    /// Check for a response from the server containing both the sent
    /// request and the response from the server
    #[allow(clippy::type_complexity)]
    fn poll_for_responses(&mut self) -> Vec<Result<(Self::Request, Self::Response), Self::Error>>;
}

/// A common abstraction for all NComm servers that outlines the necessary
/// base requirements for all NComm servers
pub trait Server {
    /// The type of data received as a request from the client
    type Request;
    /// The type of data sent as a response to the client
    type Response;
    /// The unique identifier type for the various clients
    type Key;
    /// The type of error from sending or receiving data from
    /// the client
    type Error;

    /// Check for incoming requests from the client
    #[allow(clippy::type_complexity)]
    fn poll_for_requests(&mut self) -> Vec<Result<(Self::Key, Self::Request), Self::Error>>;

    /// Send a response to a specific client
    fn send_response(
        &mut self,
        client_key: Self::Key,
        request: Self::Request,
        response: Self::Response,
    ) -> Result<(), Self::Error>;

    /// Send a collection of responses to specified clients
    fn send_responses(
        &mut self,
        mut responses: Vec<(Self::Key, Self::Request, Self::Response)>,
    ) -> Vec<Result<(), Self::Error>> {
        responses
            .drain(..)
            .map(|response| self.send_response(response.0, response.1, response.2))
            .collect()
    }
}
