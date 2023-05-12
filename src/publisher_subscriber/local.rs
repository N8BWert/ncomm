use std::sync::{mpsc, mpsc::{Sender, Receiver}};

use crate::publisher_subscriber::{Publish, Subscribe, Receive};

/// Basic Local Publisher Type
/// 
/// Publishers can create subscriptions and will send the same data
/// (cloned) to each of its subscriptions
/// 
/// Params:
///     txs: a list of the Sender<T> ends for the publisher subscriber channel
pub struct Publisher<Data: Send + Sync + Clone> {
    txs: Vec<Sender<Data>>,
}

/// Basic Local Subscriber Type
/// 
/// Subscribers will receive data from a subscriber and store only the most recent
/// data internally allowing for Nodes that contain subscribers to be able to access
/// the most recent data.
/// 
/// Params:
///     rx: the Receiver<T> end for the publisher's channel
///     data: the most recent data from the publisher (None on init)
pub struct Subscriber<Data: Send + Sync + Clone> {
    rx: Receiver<Data>,
    pub data: Option<Data>,
}

impl<Data: Send + Sync + Clone> Publisher<Data> {
    /// Creates a new Publisher with empty vector of Sender<T> ends
    /// 
    /// Returns:
    ///     Publisher<T> - new Publisher
    pub const fn new() -> Self {
        Self{ txs: Vec::new() }
    }
}

impl<Data: Send + Sync + Clone> Publish<Data> for Publisher<Data> {
    /// Sends a given piece of clonable data to each of the subscribers
    /// subscribing to this publisher
    /// 
    /// Arsg:
    ///     &self - the publisher sending the data
    ///     data: T - the data to send to the subscribers
    fn send(&self, data: Data) {
        for tx in self.txs.iter() {
            tx.send(data.clone()).expect("Data Not Sendable");
        }
    }
}

impl<Data: Send + Sync + Clone> Subscribe<Data> for Publisher<Data> {
    /// The Local Subscriber Type associated with the Local Publisher
    type Subscriber = Subscriber<Data>;

    /// Creates a new local subscriber for this publisher
    /// 
    /// Args:
    ///     &mut self - mutable reference to self (adds a tx channel)
    /// 
    /// 
    /// Returns:
    ///     Subscriber<T> - a new local subscriber
    fn create_subscriber(&mut self) -> Subscriber<Data> {
        let (tx, rx): (Sender<Data>, Receiver<Data>) = mpsc::channel();
        self.txs.push(tx);
        return Subscriber::new_empty(rx);
    }
}

impl <Data: Send + Sync + Clone> Subscriber<Data> {
    /// Creates a new local subscriber (called in create_subscriber)
    /// 
    /// Args:
    ///     rx: Receiver<T> - the receiving end of a publisher channel
    ///     data: Option<T> - the original data to hold in the subscriber
    /// 
    /// Returns:
    ///     Subscriber<T> - a new subscriber object
    pub fn new(rx: Receiver<Data>, data: Option<Data>) -> Self {
        Self{ rx, data }
    }

    /// Creates a new local subscriber with data = None (called in create_subsciber)
    /// 
    /// Args:
    ///     rx: Receiver<T> - the receiving end of a publisher channel
    /// 
    /// Returns:
    ///     Subscriber<T> - a new subscriber object
    pub fn new_empty(rx: Receiver<Data>) -> Self {
        Self { rx, data: None }
    }
}

impl<Data: Send + Sync + Clone> Receive for Subscriber<Data> {
    /// Updates the internal data of a local subscriber with data from the
    /// receiver.  The only data stored is the most recent data
    /// 
    /// Args:
    ///     &mut self - a mutable reference to self to get the receiver and to
    ///         update the most recent data
    fn update_data(&mut self) {
        let iter = self.rx.try_iter();
        match iter.last() {
            Some(data) => self.data = Some(data),
            _ => return,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Test that a publisher can create a subscriber with no data
    fn test_create_publisher_and_subscriber() {
        let mut publisher: Publisher<u8> = Publisher::new();
        let subscriber = publisher.create_subscriber();
        assert_eq!(subscriber.data, None);
    }

    #[test]
    /// Test that a publisher can send 1 piece of data to a subscriber
    fn test_send_data() {
        let mut publisher: Publisher<u8> = Publisher::new();
        let mut subscriber = publisher.create_subscriber();
        publisher.send(18u8);
        subscriber.update_data();
        assert_eq!(subscriber.data, Some(18u8));
    }

    #[test]
    /// Test that a publisher can send many pieces of data to a subscriber and
    /// that only the latest data is stored by the subscriber.
    fn test_send_many_data() {
        let mut publisher: Publisher<u8> = Publisher::new();
        let mut subscriber = publisher.create_subscriber();
        for i in 0..=10 {
            publisher.send(i);
        }
        subscriber.update_data();
        assert_eq!(subscriber.data, Some(10u8));
    }

    #[test]
    /// Test that a subscriber who has not received data has None as data
    fn test_no_data_sent() {
        let mut publisher: Publisher<u8> = Publisher::new();
        let mut subscriber = publisher.create_subscriber();
        subscriber.update_data();
        assert_eq!(subscriber.data, None);
    }
}