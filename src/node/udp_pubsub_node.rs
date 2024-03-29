//!
//! Basic UDP Publisher + Client Example.
//! 

use crate::node::Node;

use crate::publisher_subscriber::{Publish, Receive, udp::{UdpPublisher, UdpSubscriber}};

/// Test Udp Publisher Node
/// 
/// This node was created to both demo the usage of the UdpPublisher and to debug
/// the UdpPublisher to ensure it works correctly
pub struct UdpPublisherNode<'a> {
    name: &'a str,
    update_delay: u128,
    test_number: u8,
    num_publisher: UdpPublisher<'a, u8, 1>,
}

/// Test UdpSubscriber Node
/// 
/// This node was created to both demo the usage of the UdpSubsciber and to debug
/// the UdpSubscriber to ensure it works correctly
pub struct UdpSubscriberNode<'a> {
    name: &'a str,
    update_delay: u128,
    num_subscriber: UdpSubscriber<u8, 1>,
}

impl<'a> UdpPublisherNode<'a> {
    /// Creates a new UdpPublisherNode
    pub fn new(name: &'a str, update_delay: u128, bind_address: &'a str, addresses: Vec<&'a str>) -> Self {
        Self {
            name,
            update_delay,
            test_number: 0,
            num_publisher: UdpPublisher::new(bind_address, addresses),
        }
    }
}

impl<'a> UdpSubscriberNode<'a> {
    /// Creates a new UdpSubscriberNode
    pub fn new(name: &'a str, update_delay: u128, bind_address: &'a str, from_address: &'a str) -> Self {
        Self {
            name,
            update_delay,
            num_subscriber: UdpSubscriber::new(bind_address, Some(from_address)),
        }
    }
}

impl<'a> Node for UdpPublisherNode<'a> {
    fn name(&self) -> String { String::from(self.name) }

    fn get_update_delay(&self) -> u128 { self.update_delay }

    fn start(&mut self) {
        self.test_number = 1;
        self.num_publisher.send(self.test_number);
    }

    fn update(&mut self) {
        self.test_number += 1;
        self.num_publisher.send(self.test_number);
    }

    fn shutdown(&mut self) {
        self.test_number = u8::MAX;
    }

    fn debug(&self) -> String {
        format!(
            "UDP Publisher Node:\n{}\n{}\n{}",
            self.name(),
            self.update_delay,
            self.test_number,
        )
    }
}

impl<'a> Node for UdpSubscriberNode<'a> {
    fn name(&self) -> String { String::from(self.name) }

    fn get_update_delay(&self) -> u128 { self.update_delay }

    fn start(&mut self) {}

    fn update(&mut self) {
        self.num_subscriber.update_data();
    }

    fn shutdown(&mut self) {
        self.num_subscriber.update_data();
    }

    fn debug(&self) -> String {
        format!(
            "UDP Subscriber Node:\n{}\n{}\n{:?}",
            self.name(),
            self.update_delay,
            if let Some(data) = self.num_subscriber.data { data } else { 0 },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time};

    #[test]
    fn test_create_udp_pubsub_node() {
        let publisher_node = UdpPublisherNode::new(
            "publisher node",
            20,
            "127.0.0.1:8002",
            vec!["127.0.0.1:8003"],
        );
        let subscriber_node = UdpSubscriberNode::new(
            "subscriber node",
            20,
            "127.0.0.1:8003",
            "127.0.0.1:8002"
        );

        assert_eq!(publisher_node.name(), String::from("publisher node"));
        assert_eq!(publisher_node.get_update_delay(), 20);
        assert_eq!(publisher_node.test_number, 0);

        assert_eq!(subscriber_node.name(), String::from("subscriber node"));
        assert_eq!(subscriber_node.get_update_delay(), 20);
        assert!(subscriber_node.num_subscriber.data.is_none());
    }

    #[test]
    fn test_start_udp_pubsub_node() {
        let mut publisher_node = UdpPublisherNode::new(
            "publisher node",
            20,
            "127.0.0.1:8004",
            vec!["127.0.0.1:8005"],
        );
        let mut subscriber_node = UdpSubscriberNode::new(
            "subscriber node",
            20,
            "127.0.0.1:8005",
            "127.0.0.1:8004"
        );

        publisher_node.start();
        subscriber_node.start();

        assert_eq!(publisher_node.test_number, 1);
        assert!(subscriber_node.num_subscriber.data.is_none());
    }

    #[test]
    fn test_update_udp_pubsub_node() {
        let mut publisher_node = UdpPublisherNode::new(
            "publisher node",
            20,
            "127.0.0.1:8006",
            vec!["127.0.0.1:8007"],
        );
        let mut subscriber_node = UdpSubscriberNode::new(
            "subscriber node",
            30,
            "127.0.0.1:8007",
            "127.0.0.1:8006",
        );

        publisher_node.start();
        subscriber_node.start();
        publisher_node.update();
        thread::sleep(time::Duration::from_millis(10));
        subscriber_node.update();

        assert_eq!(publisher_node.test_number, 2);
        assert_eq!(subscriber_node.num_subscriber.data.unwrap(), 2);
    }

    #[test]
    fn test_shutdown_udp_pubsub_node() {
        let mut publisher_node = UdpPublisherNode::new(
            "publisher node",
            20,
            "127.0.0.1:8008",
            vec!["127.0.0.1:8009"],
        );
        let mut subscriber_node = UdpSubscriberNode::new(
            "subscriber node",
            30,
            "127.0.0.1:8009",
            "127.0.0.1:8008",
        );

        publisher_node.start();
        subscriber_node.start();
        publisher_node.shutdown();
        thread::sleep(time::Duration::from_millis(10));
        subscriber_node.shutdown();

        assert_eq!(publisher_node.test_number, u8::MAX);
        assert_eq!(subscriber_node.num_subscriber.data.unwrap(), 1);
    }
}