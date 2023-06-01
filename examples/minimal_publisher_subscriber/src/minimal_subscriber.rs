use ncomm::publisher_subscriber::Receive;
use ncomm::publisher_subscriber::local::LocalSubscriber;
use ncomm::node::Node;

pub struct MinimalSubscriber<'a> {
    name: &'a str,
    subscriber: LocalSubscriber<String>,
}

impl<'a> MinimalSubscriber<'a> {
    pub const fn new(name: &'a str, subscriber: LocalSubscriber<String>) -> Self {
        Self { name, subscriber }
    }

    fn print_subscriber_data(&mut self) {
        self.subscriber.update_data();

        if let Some(data) = self.subscriber.data.as_ref() {
            println!("I heard: {}", data);
        }
    }
}

impl<'a> Node for MinimalSubscriber<'a> {
    fn name(&self) -> String { String::from(self.name) }

    fn get_update_rate(&self) -> u128 { 500u128 }

    fn start(&mut self) {
        self.print_subscriber_data();
    }

    fn update(&mut self) {
        self.print_subscriber_data();
    }

    fn shutdown(&mut self) { }

    fn debug(&self) -> String {
        format!(
            "Minimal Publisher:\n{}",
            self.name()
        )
    }
}