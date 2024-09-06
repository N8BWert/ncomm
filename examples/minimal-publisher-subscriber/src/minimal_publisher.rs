//!
//! The minimal publisher keeps track of a count and publishes
//! the message "Hello, World! {count}" to the subscriber.
//! 

use ncomm_core::{Node, Publisher};
use ncomm_publishers_and_subscribers::local::{LocalPublisher, LocalSubscriber};

/// A minimal publisher node that publishes the string "Hello World! {count}"
pub struct MinimalPublisher {
    count: u128,
    publisher: LocalPublisher<String>,
}

impl MinimalPublisher {
    /// Create a new minimal publisher
    pub fn new() -> Self {
        Self {
            count: 0,
            publisher: LocalPublisher::new(),
        }
    }

    /// Creates a new subscriber for this node's publisher
    pub fn create_subscriber(&mut self) -> LocalSubscriber<String> {
        self.publisher.subscribe()
    }

    /// Publish the string "Hello World! {count}" and increment
    /// the current count.
    fn publish_message(&mut self) {
        let message = format!("Hello World! {}", self.count);
        println!("Publishing: {}", message);
        self.publisher.publish(message).unwrap();
        self.count = self.count.wrapping_add(1);
    }
}

impl Node for MinimalPublisher {
    fn get_update_delay_us(&self) -> u128 {
        500_000u128
    }

    fn update(&mut self) {
        self.publish_message()
    }
}