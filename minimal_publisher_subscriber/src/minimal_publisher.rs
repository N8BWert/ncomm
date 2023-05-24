use ncomm::publisher_subscriber::{Publish, Subscribe, local::{Publisher, Subscriber}};
use ncomm::node::Node;

pub struct MinimalPublisher<'a> {
    name: &'a str,
    count: u128,
    publisher: Publisher<String>,
}

impl<'a> MinimalPublisher<'a> {
    pub const fn new(name: &'a str) -> Self {
        Self { name, count: 0, publisher: Publisher::new() }
    }

    pub fn create_subscriber(&mut self) -> Subscriber<String> {
        self.publisher.create_subscriber()
    }

    fn publish_message(&mut self) {
        let message = format!("Hello, world! {}", self.count);
        println!("Publishing: {}", message);
        self.publisher.send(message);
        self.count += 1;
    }
}

impl<'a> Node for MinimalPublisher<'a> {
    fn name(&self) -> String { String::from(self.name) }

    fn get_update_rate(&self) -> u128 { 500u128 }

    fn start(&mut self) {
        self.publish_message();
    }

    fn update(&mut self) {
        self.publish_message();
    }

    fn shutdown(&mut self) { }

    fn debug(&self) -> String {
        format!(
            "Minimal Publisher:\n{}",
            self.name()
        )
    }
}