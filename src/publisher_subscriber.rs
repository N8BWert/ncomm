pub mod local;

/// Trait for all Publishers that allows them to send data to subscribers
pub trait Publish<Data: Send + Sync + Clone> {
    /// Sends data to a subscriber
    /// 
    /// Args:
    ///     &self - the subscriber struct
    ///     data: T - the data to be sent
    fn send(&self, data: Data);
}

/// Trait for all Publishers that allows subscribers to subscribe to their
/// data broadcasts
pub trait Subscribe<Data: Send + Sync + Clone> {
    /// The type of the subscriber (local, bluetooth, etc...)
    type Subscriber;

    /// Creates a subscriber of specific type for a given publisher
    /// 
    /// Args:
    ///     &mut self - mutable reference to self to add a reference / way to
    ///         send data to the subscriber
    /// 
    /// Returns:
    ///     Self::Subscriber - a subscriber of type given in trait
    fn create_subscriber(&mut self) -> Self::Subscriber;
}

/// Trait for all Subscribers that allows them to receive and update their
/// internal data to reflect the newly received data
pub trait Receive {
    /// Updates the Subscriber's internal representation of the most recent
    /// piece of data
    /// 
    /// Args:
    ///     &mut self - mutable reference to self to edit the internal data to the
    ///         most recent piece of data
    fn update_data(&mut self);
}