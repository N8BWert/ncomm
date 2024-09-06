//!
//! This example shows a minimal publisher based on the
//! [Ros 2 Foxy Tutorial](https://docs.ros.org/en/foxy/Tutorials/Beginner-Client-Libraries/Writing-A-Simple-Cpp-Publisher-And-Subscriber.html).
//!
//! The premise of this example is that the publisher will publish the
//! string "Hello World! <NUM>", where NUM is incremented with each incremental
//! publish.
//!

#![deny(missing_docs)]

use ncomm_core::Executor;
use ncomm_executors::SimpleExecutor;

use crossbeam::channel::unbounded;
use ctrlc;

pub mod minimal_publisher;
use minimal_publisher::MinimalPublisher;

pub mod minimal_subscriber;
use minimal_subscriber::MinimalSubscriber;

fn main() {
    let mut publisher_node = MinimalPublisher::new();
    let subscriber_node = MinimalSubscriber::new(publisher_node.create_subscriber());

    let (tx, rx) = unbounded();
    ctrlc::set_handler(move || tx.send(true).expect("Could not send interrupt"))
        .expect("Error setting Ctrl-C handler");

    let mut executor = SimpleExecutor::new_with(
        rx,
        vec![Box::new(publisher_node), Box::new(subscriber_node)],
    );

    executor.update_loop();
}
