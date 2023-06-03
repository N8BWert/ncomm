extern crate ncomm;

use std::sync::mpsc;

use ncomm::executor::{Executor, SingleThreadedExecutor, simple_executor::SimpleExecutor};

pub mod minimal_publisher;
pub mod minimal_subscriber;

use minimal_publisher::MinimalPublisher;
use minimal_subscriber::MinimalSubscriber;

fn main() {
    let (_interrupt_tx, interrupt_rx) = mpsc::channel();

    let mut minimal_publisher = MinimalPublisher::new("minimal publisher");
    let mut minimal_subscriber = MinimalSubscriber::new("minimal subscriber", minimal_publisher.create_subscriber());
    let mut executor = SimpleExecutor::new(interrupt_rx);

    executor.add_node(&mut minimal_publisher);
    executor.add_node(&mut minimal_subscriber);
    executor.start();
    executor.update_for_seconds(5);
}
