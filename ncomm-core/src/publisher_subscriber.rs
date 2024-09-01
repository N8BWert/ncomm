//!
//! Publisher -> Subscriber Communication
//! 
//! Publishers should push data to some endpoint or location
//! so that subscribers can read the data published by the 
//! publishers.
//! 

/// The basic publisher trait that enables the publishing of data
/// to some endpoint for subscribers to read.
pub trait Publisher {
    /// The data to be published by the publisher
    type Data;
    /// The error type from attempting to publish data
    type Error;

    /// Publish a piece of data to the endpoint for clients to read.
    fn publish(&mut self, data: Self::Data) -> Result<(), Self::Error>;
}

/// The basic subscriber trait that enables for the reading of data
/// from some endpoint.
/// 
/// Note: For convenience sake, Subscriber is actually just an alias for the
/// Deref trait
pub trait Subscriber {
    /// The type of data stored in the subscriber
    type Target;

    /// Update the current data in the subscriber and return a reference to the
    /// current data
    fn get(&mut self) -> &Self::Target;
}