//!
//! A local mpsc channel-based Publisher + Subscriber
//! 
//! The Local Publisher sends data along a mpsc channel (cloned which may change)
//! to a set of subscribers.
//! 

use std::sync::{mpsc, mpsc::{Sender, Receiver}};

use crate::publisher_subscriber::{Publish, Subscribe, Receive};

/// Basic Local Publisher Type
/// 
/// Publishers can create subscriptions and will send the same data
/// (cloned) to each of its subscriptions
pub struct LocalPublisher<Data: Send + Clone> {
    txs: Vec<Sender<Data>>,
}

/// Basic Local Subscriber Type
/// 
/// Subscribers will receive data from a subscriber and store only the most recent
/// data internally allowing for Nodes that contain subscribers to be able to access
/// the most recent data.
pub struct LocalSubscriber<Data: Send + Clone> {
    rx: Receiver<Data>,
    pub data: Option<Data>,
}

impl<Data: Send + Clone> LocalPublisher<Data> {
    /// Creates a new Publisher with empty vector of Sender<T> ends
    pub const fn new() -> Self {
        Self{ txs: Vec::new() }
    }
}

impl<Data: Send + Clone> Publish<Data> for LocalPublisher<Data> {
    fn send(&self, data: Data) {
        for tx in self.txs.iter() {
            tx.send(data.clone()).expect("Data Not Sendable");
        }
    }
}

impl<Data: Send + Clone> Subscribe<Data> for LocalPublisher<Data> {
    /// The Local Subscriber Type associated with the Local Publisher
    type Subscriber = LocalSubscriber<Data>;

    fn create_subscriber(&mut self) -> LocalSubscriber<Data> {
        let (tx, rx): (Sender<Data>, Receiver<Data>) = mpsc::channel();
        self.txs.push(tx);
        LocalSubscriber::new_empty(rx)
    }
}

impl <Data: Send + Clone> LocalSubscriber<Data> {
    pub fn new(rx: Receiver<Data>, data: Option<Data>) -> Self {
        Self{ rx, data }
    }

    pub fn new_empty(rx: Receiver<Data>) -> Self {
        Self { rx, data: None }
    }
}

impl<Data: Send + Clone> Receive for LocalSubscriber<Data> {
    fn update_data(&mut self) {
        let iter = self.rx.try_iter();

        if let Some(data) = iter.last() {
            self.data = Some(data);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Test that a publisher can create a subscriber with no data
    fn test_create_publisher_and_subscriber() {
        let mut publisher: LocalPublisher<u8> = LocalPublisher::new();
        let subscriber = publisher.create_subscriber();
        assert_eq!(subscriber.data, None);
    }

    #[test]
    /// Test that a publisher can send 1 piece of data to a subscriber
    fn test_send_data() {
        let mut publisher: LocalPublisher<u8> = LocalPublisher::new();
        let mut subscriber = publisher.create_subscriber();
        publisher.send(18u8);
        subscriber.update_data();
        assert_eq!(subscriber.data, Some(18u8));
    }

    #[test]
    /// Test that a publisher can send many pieces of data to a subscriber and
    /// that only the latest data is stored by the subscriber.
    fn test_send_many_data() {
        let mut publisher: LocalPublisher<u8> = LocalPublisher::new();
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
        let mut publisher: LocalPublisher<u8> = LocalPublisher::new();
        let mut subscriber = publisher.create_subscriber();
        subscriber.update_data();
        assert_eq!(subscriber.data, None);
    }
}