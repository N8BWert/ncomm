//!
//! Minimal Subscriber Node that receives the message "Hello World! {count}" from
//! the publisher node.
//!

use super::NodeIdentifier;

use ncomm_core::{Node, Subscriber};
use ncomm_publishers_and_subscribers::local::LocalSubscriber;

/// Minimal Subscriber that receives the message "Hello World! {count}" and prints
/// the message to the console.
pub struct MinimalSubscriber {
    subscriber: LocalSubscriber<String>,
}

impl MinimalSubscriber {
    /// Creates a new MinimalSubscriber Node with a given subscriber
    pub fn new(subscriber: LocalSubscriber<String>) -> Self {
        Self { subscriber }
    }

    /// Print "I heard: {data}" with the data published to it by the minimal
    /// publisher
    fn print_subscriber_data(&mut self) {
        if let Some(data) = self.subscriber.get() {
            println!("I heard: {}", data);
        }
    }
}

impl Node<NodeIdentifier> for MinimalSubscriber {
    fn get_id(&self) -> NodeIdentifier {
        NodeIdentifier::SubscriberNode
    }

    fn get_update_delay_us(&self) -> u128 {
        500_000
    }

    fn update(&mut self) {
        self.print_subscriber_data();
    }
}
