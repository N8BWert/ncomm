//!
//! NComm Update Server and Client.
//! 
//! An Update Client and Server is pretty much the same as an action client in
//! ROS in which a server is given some long running task that they routinely
//! update the client on.
//! 

/// A common abstraction for all NComm update clients
pub trait UpdateClient {
    /// The type of data used as a request by the client
    type Request;
    /// The type of data used as an update from the server
    type Update;
    /// The type of data used as a response from the server
    type Response;
    /// The type of error from sending or receiving data from the update
    /// server
    type Error;

    /// Send a request to the server this client is associated with
    fn send_request(&mut self, request: Self::Request) -> Result<(), Self::Error>;

    /// Poll for updates from the server
    fn poll_for_updates(&mut self) -> Vec<Result<(Self::Request, Self::Update), Self::Error>>;

    /// Poll for responses from the server
    fn poll_for_responses(&mut self) -> Vec<Result<(Self::Request, Self::Response), Self::Error>>;
}

/// A common abstraction for all NComm update servers
pub trait UpdateServer {
    /// The type of data received as a request from the client
    type Request: Clone;
    /// The type of data sent as an update to the client
    type Update;
    /// The type of data sent as a response to the client
    type Response;
    /// The unique identifier type for the various clients
    type Key;
    /// The type of error from sending or receiving data from and
    /// to the update client
    type Error;

    /// Check for incoming requests from the client
    fn poll_for_requests(&mut self) -> Vec<Result<(Self::Key, Self::Request), Self::Error>>;

    /// Send an update to a specific client
    fn send_update(&mut self, client_key: Self::Key, request: &Self::Request, update: Self::Update) -> Result<(), Self::Error>;

    /// Send a collection of updates to specified client
    fn send_updates(&mut self, mut updates: Vec<(Self::Key, &Self::Request, Self::Update)>) -> Vec<Result<(), Self::Error>> {
        updates.drain(..).map(|update| self.send_update(update.0, update.1, update.2)).collect()
    }

    /// Send a response to a specific client
    fn send_response(&mut self, client_key: Self::Key, request: Self::Request, response: Self::Response) -> Result<(), Self::Error>;

    /// Send a collection of responses to specific clients
    fn send_responses(&mut self, mut responses: Vec<(Self::Key, Self::Request, Self::Response)>) -> Vec<Result<(), Self::Error>> {
        responses.drain(..).map(|response| self.send_response(response.0, response.1, response.2)).collect()
    }
}