//!
//! Basic Publisher + Server Node Example.
//! 

use crate::node::Node;

use crate::publisher_subscriber::{Publish, Subscribe, Receive, local::{LocalPublisher, LocalSubscriber}};

/// Test Local Publisher Node
/// 
/// This node was created to both demo the usage of the LocalPublisher and to debug
/// the LocalPublisher to ensure it works correctly.
pub struct PublisherNode<'a> {
    name: &'a str,
    update_delay: u128,
    test_number: u128,
    num_publisher: LocalPublisher<u128>,
}

/// Test Local Subscriber Node
/// 
/// This node was created to both demo the usage of the LocalSubscriber and to
/// debug the LocalSubscriber to ensure it works correctly
pub struct SubscriberNode<'a> {
    name: &'a str,
    update_delay: u128,
    num_subscriber: Option<LocalSubscriber<u128>>,
}

impl<'a> PublisherNode<'a> {
    /// Creates a new PublisherNode
    pub const fn new(name: &'a str, update_delay: u128) -> Self {
        Self{
            name,
            update_delay,
            test_number: 0,
            num_publisher: LocalPublisher::new(),
        }
    }

    /// Returns the Subscriber end of the publisher node's publisher
    pub fn subscribe_to_num_publisher(&mut self) -> LocalSubscriber<u128> {
        self.num_publisher.create_subscriber()
    }
}

impl<'a> SubscriberNode<'a> {
    /// Creates a new subscriber node
    pub const fn new(name: &'a str, update_delay: u128) -> Self {
        Self{
            name,
            update_delay,
            num_subscriber: None,
        }
    }

    /// Adds a subscriber endpoint to this Node's num_subscriber endpoint.
    pub fn add_num_subscriber_subscriber(&mut self, subscriber: LocalSubscriber<u128>) {
        self.num_subscriber = Some(subscriber);
    }
}

impl<'a> Node for PublisherNode<'a> {
    fn name(&self) -> String {
        String::from(self.name)
    }

    fn start(&mut self) {
        self.test_number = 1;
        self.num_publisher.send(self.test_number);
    }

    fn update(&mut self) {
        self.test_number += 1;
        self.num_publisher.send(self.test_number);
    }

    fn get_update_delay(&self) -> u128 {
        self.update_delay
    }

    fn shutdown(&mut self) {
        self.test_number = u128::MAX;
    }

    fn debug(&self) -> String {
        format!(
            "Publisher Node:\n{}\n{}\n{}",
            self.name(),
            self.update_delay,
            self.test_number,
        )
    }
}

impl<'a> Node for SubscriberNode<'a> {
    fn name(&self) -> String {
        String::from(self.name)
    }

    fn start(&mut self) {
        self.num_subscriber.as_mut().unwrap().update_data();
    }

    fn update(&mut self) {
        self.num_subscriber.as_mut().unwrap().update_data();
    }

    fn get_update_delay(&self) -> u128 {
        self.update_delay
    }

    fn shutdown(&mut self) {}

    fn debug(&self) -> String {
        format!(
            "Subscriber Node:\n{}\n{}\n{}",
            self.name(),
            self.update_delay,
            self.num_subscriber.as_ref().unwrap().data.unwrap(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_pubsub_node() {
        let publisher_node = PublisherNode::new("test publisher", 12);
        let subscriber_node = SubscriberNode::new("test subscriber", 10);

        assert_eq!(publisher_node.name(), String::from("test publisher"));
        assert_eq!(publisher_node.update_delay, 12);
        assert_eq!(publisher_node.test_number, 0);
        
        assert_eq!(subscriber_node.name(), String::from("test subscriber"));
        assert_eq!(subscriber_node.update_delay, 10);
        assert!(subscriber_node.num_subscriber.is_none());
    }

    #[test]
    fn test_start_pubsub_node() {
        let mut publisher_node = PublisherNode::new("test publisher", 13);
        let mut subscriber_node = SubscriberNode::new("test subscriber", 10);

        subscriber_node.add_num_subscriber_subscriber(publisher_node.subscribe_to_num_publisher());

        publisher_node.start();
        subscriber_node.start();

        assert_eq!(publisher_node.name(), String::from("test publisher"));
        assert_eq!(publisher_node.get_update_delay(), 13);
        assert_eq!(publisher_node.test_number, 1);

        assert_eq!(subscriber_node.name(), String::from("test subscriber"));
        assert_eq!(subscriber_node.get_update_delay(), 10);
        assert_eq!(subscriber_node.num_subscriber.unwrap().data.unwrap(), 1);
    }

    #[test]
    fn test_update_pubsub_node() {
        // Create the publisher and subscriber nodes
        let mut publisher_node = PublisherNode::new("test publisher", 12);
        let mut subscriber_node = SubscriberNode::new("test subscriber", 10);

        // Subscribe to the publisher node and update the subscriber data
        subscriber_node.add_num_subscriber_subscriber(publisher_node.subscribe_to_num_publisher());
        publisher_node.start();
        subscriber_node.start();
        publisher_node.update();
        subscriber_node.update();

        assert_eq!(publisher_node.name(), String::from("test publisher"));
        assert_eq!(publisher_node.get_update_delay(), 12);
        assert_eq!(publisher_node.test_number, 2);

        assert_eq!(subscriber_node.name(), String::from("test subscriber"));
        assert_eq!(subscriber_node.get_update_delay(), 10);
        assert_eq!(subscriber_node.num_subscriber.unwrap().data.unwrap(), 2);
    }
}