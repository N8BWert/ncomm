//!
//! Executes the update step of nodes on a singular thread.
//! 
//! A simple executor that stores nodes in a binary head updating the nodes one by one
//! and adding them back to the heap with priority equal to their next update timestamp.
//! 
//! This executor is "scoped" thread safe so it can be sent to scoped threads to update its
//! nodes as another process.
//! 

use crate::executor::{Executor, SingleThreadedExecutor, node_wrapper::NodeWrapper};
use crate::node::Node;

use std::collections::BinaryHeap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::thread;
use std::sync::mpsc::Receiver;

/// Simple Implementation of an executor
/// 
/// This simple executor stores Nodes in a Binary heap with with priority equal to the
/// timestamp of the node's next update.
pub struct SimpleExecutor<'a> {
    pub(in crate::executor) heap: BinaryHeap<NodeWrapper<'a>>,
    start_time: u128,
    interrupt_rx: Receiver<bool>,
    interrupted: bool,
}

impl<'a> SimpleExecutor<'a> {
    /// Creates a new Simple Executor
    pub fn new(interrupt_rx: Receiver<bool>) -> Self {
        Self {
            heap: BinaryHeap::new(),
            start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
            interrupt_rx,
            interrupted: false
        }
    }

    /// Creates a new Simple Executor with the nodes given
    pub fn new_with(nodes: Vec<&'a mut dyn Node>, interrupt_rx: Receiver<bool>) -> Self {
        let mut heap = BinaryHeap::new();

        for node in nodes {
            node.start();
            heap.push(
                NodeWrapper {
                    priority: node.get_update_delay(),
                    node
                }
            );
        }

        Self {
            heap,
            start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
            interrupt_rx,
            interrupted: false
        }
    }
}

impl<'a> SingleThreadedExecutor<'a> for SimpleExecutor<'a> {
    fn add_node(&mut self, new_node: &'a mut dyn Node) {
        self.heap.push(
            NodeWrapper{
                priority: new_node.get_update_delay(),
                node: new_node
            }
        );
    }

    fn update(&mut self) {
        // Sleep until time to update the next node
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        let start_time = self.start_time + self.heap.peek().unwrap().priority;
        if start_time > current_time {
            let sleep_time = start_time - current_time;
            thread::sleep(Duration::from_millis(sleep_time as u64));
        }

        // Get and update the next node
        let mut node_wrapper = self.heap.pop().unwrap();
        node_wrapper.node.update();
        node_wrapper.priority += node_wrapper.node.get_update_delay();
        self.heap.push(node_wrapper);
    }

    fn update_for(&mut self, iterations: u128) {
        self.start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

        for _ in 0..iterations {
            self.update();
        }
    }
}

impl<'a> Executor<'a> for SimpleExecutor<'a> {
    fn start(&mut self) {
        let mut vec = Vec::with_capacity(self.heap.len());

        for node_wrapper in self.heap.drain() {
            node_wrapper.node.start();
            vec.push(node_wrapper);
        }

        for node_wrapper in vec {
            self.heap.push(node_wrapper);
        }
    }

    fn update_for_seconds(&mut self, seconds: u128) {
        self.start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

        let seconds = seconds * 1000;
        while SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() - self.start_time < seconds && !self.check_interrupt() {
            self.update();
        }
    }

    fn update_loop(&mut self) {
        self.start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

        while !self.check_interrupt() {
            self.update();
        }
    }

    fn check_interrupt(&mut self) -> bool {
        let iter = self.interrupt_rx.try_iter();
        if let Some(interrupt) = iter.last() {
            if interrupt != self.interrupted {
                self.interrupted = interrupt;
            }
        }
        return self.interrupted;
    }

    fn log(&self, message: &str) {
        // TODO: In the future more complicated logging is likely
        println!("{}", message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::basic_node::BasicNode;
    use crate::node::client_server_node::{ServerNode, ClientNode};
    use crate::node::pubsub_node::{PublisherNode, SubscriberNode};
    use crate::node::update_client_server_node::{UpdateServerNode, UpdateClientNode};
    use crate::node::udp_pubsub_node::{UdpPublisherNode, UdpSubscriberNode};
    use std::thread::scope;
    use std::sync::mpsc;

    #[test]
    fn test_simple_executor_basic_node() {
        let (_interrupt_tx, interrupt_rx)= mpsc::channel();

        // Create the basic nodes and executor
        let mut basic_node_one = BasicNode::new("test node 1", 12);
        let mut basic_node_two = BasicNode::new("test node 2", 13);
        let mut basic_node_three = BasicNode::new("test node 3", 3);
        let mut simple_executor = SimpleExecutor::new(interrupt_rx);
        simple_executor.add_node(&mut basic_node_one);
        simple_executor.add_node(&mut basic_node_two);
        simple_executor.add_node(&mut basic_node_three);

        // Run the update loop for 10 iterations
        simple_executor.start();
        simple_executor.update_for(10);

        // Check the first node is node 3
        let node_three = simple_executor.heap.pop().unwrap().node;
        assert_eq!(node_three.name(), String::from("test node 3"));
        assert_eq!(node_three.get_update_rate(), 3);

        // Check the second node is node 2
        let node_two = simple_executor.heap.pop().unwrap().node;
        assert_eq!(node_two.name(), String::from("test node 2"));
        assert_eq!(node_two.get_update_rate(), 13);

        // Check the third node is node 3
        let node_one = simple_executor.heap.pop().unwrap().node;
        assert_eq!(node_one.name(), String::from("test node 1"));
        assert_eq!(node_one.get_update_rate(), 12);
    }

    #[test]
    fn test_two_simple_executors() {
        let (_interrupt_tx_one, interrupt_rx_one) = mpsc::channel();
        let (_interrupt_tx_two, interrupt_rx_two) = mpsc::channel();

        // Initialize the nodes
        let mut basic_node_one: BasicNode = BasicNode::new("test node 1", 9);
        let mut basic_node_two: BasicNode = BasicNode::new("test node 2", 13);
        let mut basic_node_three: BasicNode = BasicNode::new("test node 3", 12);
        let mut basic_node_four: BasicNode = BasicNode::new("test node 4", 15);
        let mut basic_node_five: BasicNode = BasicNode::new("test node 5", 5);
        let mut basic_node_six: BasicNode = BasicNode::new("test node 6", 20);

        // Initialize the executors
        let mut executor_one = SimpleExecutor::new(interrupt_rx_one);
        executor_one.add_node(&mut basic_node_one);
        executor_one.add_node(&mut basic_node_two);
        executor_one.add_node(&mut basic_node_three);

        let mut executor_two = SimpleExecutor::new(interrupt_rx_two);
        executor_two.add_node(&mut basic_node_four);
        executor_two.add_node(&mut basic_node_five);
        executor_two.add_node(&mut basic_node_six);

        let (exec_one, exec_two) = scope(|scope| {
            let thread_one = scope.spawn(|| {
                executor_one.start();
                executor_one.update_for(20);
                return executor_one;
            });

            let thread_two = scope.spawn(|| {
                executor_two.start();
                executor_two.update_for(20);
                return executor_two;
            });

            (thread_one.join(), thread_two.join())
        });

        match (exec_one, exec_two) {
            (Ok(mut executor_one), Ok(mut executor_two)) => {
                let node_one = executor_one.heap.pop().unwrap().node;
                assert_eq!(node_one.name(), String::from("test node 1"));
                assert_eq!(node_one.get_update_rate(), 9);

                let node_three = executor_one.heap.pop().unwrap().node;
                assert_eq!(node_three.name(), String::from("test node 3"));
                assert_eq!(node_three.get_update_rate(), 12);

                let node_two = executor_one.heap.pop().unwrap().node;
                assert_eq!(node_two.name(), String::from("test node 2"));
                assert_eq!(node_two.get_update_rate(), 13);

                let node_five = executor_two.heap.pop().unwrap().node;
                assert_eq!(node_five.name(), String::from("test node 5"));
                assert_eq!(node_five.get_update_rate(), 5);

                let node_four = executor_two.heap.pop().unwrap().node;
                assert_eq!(node_four.name(), String::from("test node 4"));
                assert_eq!(node_four.get_update_rate(), 15);
                
                let node_six = executor_two.heap.pop().unwrap().node;
                assert_eq!(node_six.name(), String::from("test node 6"));
                assert_eq!(node_six.get_update_rate(), 20);
            },
            _ => assert_eq!(false, true),
        };
    }

    #[test]
    fn test_simple_executor_time() {
        let (_interrupt_tx, interrupt_rx) = mpsc::channel();

        // Create the basic nodes and executor
        let mut basic_node_one = BasicNode::new("test node 1", 15);
        let mut basic_node_two = BasicNode::new("test node 2", 4);
        let mut basic_node_three = BasicNode::new("test node 3", 5);
        let mut basic_node_four = BasicNode::new("test node 4", 2);
        let mut simple_executor = SimpleExecutor::new(interrupt_rx);
        simple_executor.add_node(&mut basic_node_one);
        simple_executor.add_node(&mut basic_node_two);
        simple_executor.add_node(&mut basic_node_three);
        simple_executor.add_node(&mut basic_node_four);

        // Run the update loop for 2 seconds
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        simple_executor.start();
        simple_executor.update_for_seconds(2);
        let end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

        let wrapper_two = simple_executor.heap.pop().unwrap();
        let node_two = wrapper_two.node;
        assert_eq!(node_two.name(), String::from("test node 2"));
        assert_eq!(node_two.get_update_rate(), 4);
        assert_eq!(wrapper_two.priority, 2000);

        let wrapper_four = simple_executor.heap.pop().unwrap();
        let node_four = wrapper_four.node;
        assert_eq!(node_four.name(), String::from("test node 4"));
        assert_eq!(node_four.get_update_rate(), 2);
        assert_eq!(wrapper_four.priority, 2000);

        let wrapper_three = simple_executor.heap.pop().unwrap();
        let node_three = wrapper_three.node;
        assert_eq!(node_three.name(), String::from("test node 3"));
        assert_eq!(node_three.get_update_rate(), 5);
        assert_eq!(wrapper_three.priority, 2005);

        let wrapper_one = simple_executor.heap.pop().unwrap();
        let node_one = wrapper_one.node;
        assert_eq!(node_one.name(), String::from("test node 1"));
        assert_eq!(node_one.get_update_rate(), 15);
        assert_eq!(wrapper_one.priority, 2010);

        assert!(2000 <= end_time - start_time && end_time - start_time <= 2002);
    }

    #[test]
    fn test_two_simple_executors_time() {
        let (_interrupt_tx_one, interrupt_rx_one) = mpsc::channel();
        let (_interrupt_tx_two, interrupt_rx_two) = mpsc::channel();

        // Initialize the nodes
        let mut basic_node_one: BasicNode = BasicNode::new("test node 1", 9);
        let mut basic_node_two: BasicNode = BasicNode::new("test node 2", 25);

        // Initialize the executors
        let mut executor_one = SimpleExecutor::new(interrupt_rx_one);
        executor_one.add_node(&mut basic_node_one);

        let mut executor_two = SimpleExecutor::new(interrupt_rx_two);
        executor_two.add_node(&mut basic_node_two);

        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

        let (exec_one, exec_two) = scope(|scope| {
            let thread_one = scope.spawn(|| {
                executor_one.update_for_seconds(2);
                return executor_one;
            });

            let thread_two = scope.spawn(|| {
                executor_two.update_for_seconds(2);
                return executor_two;
            });

            (thread_one.join(), thread_two.join())
        });

        match (exec_one, exec_two) {
            (Ok(mut executor_one), Ok(mut executor_two)) => {
                let end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                
                let node_one = executor_one.heap.pop().unwrap().node;
                assert_eq!(node_one.name(), String::from("test node 1"));

                let node_two = executor_two.heap.pop().unwrap().node;
                assert_eq!(node_two.name(), String::from("test node 2"));

                println!("Execution Time: {}", end_time - start_time);
                assert!(2000 <= end_time - start_time && end_time - start_time <= 2200);
            },
            _ => assert_eq!(false, true),
        };
    }

    #[test]
    fn test_simple_executor_pubsub_node() {
        let (_interrupt_tx, interrupt_rx) = mpsc::channel();

        let mut publisher_node = PublisherNode::new("publisher node", 13);
        let mut subscriber_node = SubscriberNode::new("subscriber node", 10);
        let mut simple_executor = SimpleExecutor::new(interrupt_rx);
        subscriber_node.add_num_subscriber_subscriber(publisher_node.subscribe_to_num_publisher());
        simple_executor.add_node(&mut publisher_node);
        simple_executor.add_node(&mut subscriber_node);

        // Run the update loop for 10 iterations
        simple_executor.start();
        simple_executor.update_for(10);

        // Check the first node is the subscriber
        let publisher = simple_executor.heap.pop().unwrap().node;
        assert_eq!(publisher.debug(), "Publisher Node:\npublisher node\n13\n5");

        // Check the second node is the publisher
        let subscriber = simple_executor.heap.pop().unwrap().node;
        assert_eq!(subscriber.debug(), "Subscriber Node:\nsubscriber node\n10\n5");
    }

    #[test]
    fn test_simple_executor_pubsub_node_different_executors() {
        let (_interrupt_tx_one, interrupt_rx_one) = mpsc::channel();
        let (_interrupt_tx_two, interrupt_rx_two) = mpsc::channel();

        let mut publisher_node = PublisherNode::new("publisher node", 13);
        let mut subscriber_node = SubscriberNode::new("subscriber node", 10);
        let mut pub_executor = SimpleExecutor::new(interrupt_rx_one);
        let mut sub_executor = SimpleExecutor::new(interrupt_rx_two);
        subscriber_node.add_num_subscriber_subscriber(publisher_node.subscribe_to_num_publisher());
        pub_executor.add_node(&mut publisher_node);
        sub_executor.add_node(&mut subscriber_node);

        let (exec_one, exec_two) = scope(|scope| {
            let thread_one = scope.spawn(|| {
                pub_executor.start();
                pub_executor.update_for(10);
                return pub_executor;
            });

            let thread_two = scope.spawn(|| {
                sub_executor.start();
                sub_executor.update_for(10);
                return sub_executor;
            });

            (thread_one.join(), thread_two.join())
        });

        match (exec_one, exec_two) {
            (Ok(mut pub_executor), Ok(mut sub_executor)) => {
                let publisher = pub_executor.heap.pop().unwrap().node;
                assert_eq!(publisher.debug(), "Publisher Node:\npublisher node\n13\n11");

                let subscriber = sub_executor.heap.pop().unwrap().node;
                assert_eq!(subscriber.debug(), "Subscriber Node:\nsubscriber node\n10\n8");
            },
            _ => assert_eq!(false, true),
        };
    }

    #[test]
    fn test_simple_executor_pubsub_node_different_executors_time() {
        let (_interrupt_tx_one, interrupt_rx_one) = mpsc::channel();
        let (_interrupt_tx_two, interrupt_rx_two) = mpsc::channel();
    
        let mut publisher_node = PublisherNode::new("publisher node", 13);
        let mut subscriber_node = SubscriberNode::new("subscriber node", 10);
        let mut pub_executor = SimpleExecutor::new(interrupt_rx_one);
        let mut sub_executor = SimpleExecutor::new(interrupt_rx_two);
        subscriber_node.add_num_subscriber_subscriber(publisher_node.subscribe_to_num_publisher());
        pub_executor.add_node(&mut publisher_node);
        sub_executor.add_node(&mut subscriber_node);

        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

        let (exec_one, exec_two) = scope(|scope| {
            let thread_one = scope.spawn(|| {
                pub_executor.start();
                pub_executor.update_for_seconds(2);
                return pub_executor;
            });

            let thread_two = scope.spawn(|| {
                sub_executor.start();
                sub_executor.update_for_seconds(2);
                return sub_executor;
            });

            (thread_one.join(), thread_two.join())
        });

        match (exec_one, exec_two) {
            (Ok(mut pub_executor), Ok(mut sub_executor)) => {
                let end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

                let publisher = pub_executor.heap.pop().unwrap().node;
                assert_eq!(publisher.debug(), "Publisher Node:\npublisher node\n13\n155");

                let subscriber = sub_executor.heap.pop().unwrap().node;
                assert_eq!(subscriber.debug(), "Subscriber Node:\nsubscriber node\n10\n154");

                println!("Elapsed Time {}", end_time - start_time);
                assert!(2000 <= end_time - start_time && end_time - start_time <= 2020);
            },
            _ => assert_eq!(false, true)
        };
    }

    #[test]
    fn test_simple_executor_client_server_nodes() {
        let (_interrupt_tx_one, interrupt_rx_one) = mpsc::channel();

        let mut server_node = ServerNode::new("server node", 10);
        let mut client_node_one = ClientNode::new("client node 1", 22, server_node.create_client(String::from("client node 1")));
        let mut client_node_two = ClientNode::new("client node 2", 25, server_node.create_client(String::from("client node 2")));
        let mut simple_executor = SimpleExecutor::new(interrupt_rx_one);
        simple_executor.add_node(&mut server_node);
        simple_executor.add_node(&mut client_node_one);
        simple_executor.add_node(&mut client_node_two);

        // Run the update loop for 10 iterations
        simple_executor.start();
        simple_executor.update_for(10);

        // Check the first node is client node 1
        let client_one = simple_executor.heap.pop().unwrap().node;
        assert_eq!(client_one.debug(), "Client Node:\nclient node 1\n22\n3");

        // Check the second node is server node
        let server = simple_executor.heap.pop().unwrap().node;
        assert_eq!(server.debug(), "Server Node:\nserver node\n10");

        // Check the third node is client node 2
        let client_two = simple_executor.heap.pop().unwrap().node;
        assert_eq!(client_two.debug(), "Client Node:\nclient node 2\n25\n3");
    }

    #[test]
    fn test_simple_executor_client_server_nodes_different_executors() {
        let (_interrupt_tx_one, interrupt_rx_one) = mpsc::channel();
        let (_interrupt_tx_two, interrupt_rx_two) = mpsc::channel();

        let mut server_node = ServerNode::new("server node", 10);
        let mut client_node_one = ClientNode::new("client node 1", 15, server_node.create_client(String::from("client node 1")));
        let mut client_node_two = ClientNode::new("client node 2", 22, server_node.create_client(String::from("client node 2")));
        let mut executor_one = SimpleExecutor::new(interrupt_rx_one);
        let mut executor_two = SimpleExecutor::new(interrupt_rx_two);
        executor_one.add_node(&mut server_node);
        executor_one.add_node(&mut client_node_one);
        executor_two.add_node(&mut client_node_two);

        let (exec_one, exec_two) = scope(|scope| {
            let thread_one = scope.spawn(|| {
                executor_one.start();
                executor_one.update_for(10);
                return executor_one;
            });

            let thread_two = scope.spawn(|| {
                executor_two.start();
                executor_two.update_for(10);
                return executor_two;
            });

            (thread_one.join(), thread_two.join())
        });

        match (exec_one, exec_two) {
            (Ok(mut executor_one), Ok(mut executor_two)) => {
                let server = executor_one.heap.pop().unwrap().node;
                assert_eq!(server.debug(), "Server Node:\nserver node\n10");

                let client_one = executor_one.heap.pop().unwrap().node;
                assert_eq!(client_one.debug(), "Client Node:\nclient node 1\n15\n27");

                let client_two = executor_two.heap.pop().unwrap().node;
                assert_eq!(client_two.debug(), "Client Node:\nclient node 2\n22\n9");
            },
            _ => assert_eq!(false, true),
        }
    }

    #[test]
    fn test_simple_executor_client_server_nodes_different_executors_time() {
        let (_interrupt_tx_one, interrupt_rx_one) = mpsc::channel();
        let (_interrupt_tx_two, interrupt_rx_two) = mpsc::channel();

        let mut server_node = ServerNode::new("server node", 12);
        let mut client_node_one = ClientNode::new("client node 1", 11, server_node.create_client(String::from("client node 1")));
        let mut client_node_two = ClientNode::new("client node 2", 25, server_node.create_client(String::from("client node 2")));
        let mut executor_one = SimpleExecutor::new(interrupt_rx_one);
        let mut executor_two = SimpleExecutor::new(interrupt_rx_two);
        executor_one.add_node(&mut server_node);
        executor_one.add_node(&mut client_node_one);
        executor_two.add_node(&mut client_node_two);

        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

        let (exec_one, exec_two) = scope(|scope| {
            let thread_one = scope.spawn(|| {
                executor_one.start();
                executor_one.update_for_seconds(2);
                return executor_one;
            });

            let thread_two = scope.spawn(|| {
                executor_two.start();
                executor_two.update_for_seconds(2);
                return executor_two;
            });

            (thread_one.join(), thread_two.join())
        });

        match (exec_one, exec_two) {
            (Ok(mut executor_one), Ok(mut executor_two)) => {
                let end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

                let server = executor_one.heap.pop().unwrap().node;
                assert_eq!(server.name(), String::from("server node"));

                let client_one = executor_one.heap.pop().unwrap().node;
                assert_eq!(client_one.name(), String::from("client node 1"));

                let client_two = executor_two.heap.pop().unwrap().node;
                assert_eq!(client_two.name(), String::from("client node 2"));

                assert!(2000 <= end_time - start_time && end_time - start_time <= 2010);
            },
            _ => assert_eq!(true, false)
        };
    }

    #[test]
    fn test_simple_executor_update_client_server_nodes() {
        let (_interrupt_tx, interrupt_rx) = mpsc::channel();

        let mut server_node = UpdateServerNode::new("update server node", 20);
        let mut client_node = UpdateClientNode::new("update client node", 20, server_node.create_update_client(String::from("update client node")));
        let mut simple_executor = SimpleExecutor::new(interrupt_rx);
        simple_executor.add_node(&mut server_node);
        simple_executor.add_node(&mut client_node);

        // Run the update loop for 10 iterations
        simple_executor.start();
        simple_executor.update_for(10);

        // Check the first node is the server
        let server = simple_executor.heap.pop().unwrap().node;
        assert_eq!(server.debug(), "Update Server Node:\nupdate server node\n20\n5");

        // Check the second node is the client
        let client = simple_executor.heap.pop().unwrap().node;
        assert_eq!(client.debug(), "Update Client Node:\nupdate client node\n20\n10");
    }

    #[test]
    fn test_simple_executor_update_client_server_nodes_different_executors() {
        let (_interrupt_tx_one, interrupt_rx_one) = mpsc::channel();
        let (_interrupt_tx_two, interrupt_rx_two) = mpsc::channel();

        let mut server_node = UpdateServerNode::new("update server node", 20);
        let mut client_node = UpdateClientNode::new("update client node", 20, server_node.create_update_client(String::from("update client node")));
        let mut server_executor = SimpleExecutor::new(interrupt_rx_one);
        let mut client_executor = SimpleExecutor::new(interrupt_rx_two);
        server_executor.add_node(&mut server_node);
        client_executor.add_node(&mut client_node);

        let (server_exec, client_exec) = scope(|scope| {
            let thread_one = scope.spawn(|| {
                server_executor.start();
                server_executor.update_for(10);
                return server_executor;
            });

            let thread_two = scope.spawn(|| {
                client_executor.start();
                client_executor.update_for(10);
                return client_executor;
            });

            (thread_one.join(), thread_two.join())
        });

        match (server_exec, client_exec) {
            (Ok(mut server_executor), Ok(mut client_executor)) => {
                let server = server_executor.heap.pop().unwrap().node;
                assert_eq!(server.debug(), "Update Server Node:\nupdate server node\n20\n5");

                let client = client_executor.heap.pop().unwrap().node;
                assert_eq!(client.debug(), "Update Client Node:\nupdate client node\n20\n100");
            },
            _ => assert_eq!(true, false),
        };
    }

    #[test]
    fn test_simple_executor_update_client_server_nodes_different_executors_time() {
        let (_interrupt_tx_one, interrupt_rx_one) = mpsc::channel();
        let (_interrupt_tx_two, interrupt_rx_two) = mpsc::channel();

        let mut server_node = UpdateServerNode::new("update server node", 20);
        let mut client_node = UpdateClientNode::new("update client node", 20, server_node.create_update_client(String::from("update client node")));
        let mut server_executor = SimpleExecutor::new(interrupt_rx_one);
        let mut client_executor = SimpleExecutor::new(interrupt_rx_two);
        server_executor.add_node(&mut server_node);
        client_executor.add_node(&mut client_node);

        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

        let (server_exec, client_exec) = scope(|scope| {
            let thread_one = scope.spawn(|| {
                server_executor.start();
                server_executor.update_for_seconds(1);
                return server_executor;
            });

            let thread_two = scope.spawn(|| {
                client_executor.start();
                client_executor.update_for_seconds(1);
                return client_executor;
            });

            (thread_one.join(), thread_two.join())
        });

        match (server_exec, client_exec) {
            (Ok(_), Ok(_)) => {
                let end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                
                assert!(1000 <= end_time - start_time && end_time - start_time <= 1010);
            },
            _ => assert_eq!(true, false),
        };
    }

    #[test]
    fn test_simple_executor_udp_pubsub_nodes() {
        let (_interrupt_tx, interrupt_rx) = mpsc::channel();

        let mut publisher = UdpPublisherNode::new(
            "udp publisher node",
            20,
            "127.0.0.1:8020",
            vec!["127.0.0.1:8021"],
        );
        let mut subscriber = UdpSubscriberNode::new(
            "udp subscriber node",
            30,
            "127.0.0.1:8021",
            "127.0.0.1:8020",
        );
        let mut executor = SimpleExecutor::new(interrupt_rx);
        executor.add_node(&mut publisher);
        executor.add_node(&mut subscriber);

        // Run the update loop for 10 iterations
        executor.start();
        executor.update_for(10);

        // Check the first node is the publisher
        let publisher = executor.heap.pop().unwrap().node;
        let binding = publisher.debug();
        let mut parts = binding.split("\n");
        assert_eq!(Some("UDP Publisher Node:"), parts.next());
        assert_eq!(Some("udp publisher node"), parts.next());
        assert_eq!(Some("20"), parts.next());
        assert_eq!(Some("7"), parts.next());

        // Check the first node is the subscriber
        let subscriber = executor.heap.pop().unwrap().node;
        let binding = subscriber.debug();
        let mut parts = binding.split("\n");
        assert_eq!(Some("UDP Subscriber Node:"), parts.next());
        assert_eq!(Some("udp subscriber node"), parts.next());
        assert_eq!(Some("30"), parts.next());
        match parts.next().to_owned().unwrap().to_string().parse::<u8>() {
            Ok(4..=6) => assert!(true),
            _ => assert!(false),
        };
    }

    #[test]
    fn test_simple_executor_udp_pubsub_nodes_different_executors() {
        let (_interrupt_tx_one, interrupt_rx_one) = mpsc::channel();
        let (_interrupt_tx_two, interrupt_rx_two) = mpsc::channel();

        let mut publisher = UdpPublisherNode::new(
            "udp publisher node",
            20,
            "127.0.0.1:8022",
            vec!["127.0.0.1:8023"],
        );
        let mut subscriber = UdpSubscriberNode::new(
            "udp subscriber node",
            30,
            "127.0.0.1:8023",
            "127.0.0.1:8022",
        );
        let mut pub_executor = SimpleExecutor::new(interrupt_rx_one);
        let mut sub_executor = SimpleExecutor::new(interrupt_rx_two);
        pub_executor.add_node(&mut publisher);
        sub_executor.add_node(&mut subscriber);

        let (pub_exec, sub_exec) = scope(|scope| {
            let thread_one = scope.spawn(|| {
                pub_executor.start();
                pub_executor.update_for(10);
                return pub_executor;
            });

            let thread_two = scope.spawn(|| {
                sub_executor.start();
                sub_executor.update_for(10);
                return sub_executor;
            });

            (thread_one.join(), thread_two.join())
        });

        match (pub_exec, sub_exec) {
            (Ok(mut pub_executor), Ok(mut sub_executor)) => {
                let publisher = pub_executor.heap.pop().unwrap().node;
                let binding = publisher.debug();
                let mut parts = binding.split("\n");
                assert_eq!(Some("UDP Publisher Node:"), parts.next());
                assert_eq!(Some("udp publisher node"), parts.next());
                assert_eq!(Some("20"), parts.next());
                assert_eq!(Some("11"), parts.next());

                let subscriber = sub_executor.heap.pop().unwrap().node;
                let binding = subscriber.debug();
                let mut parts = binding.split("\n");
                assert_eq!(Some("UDP Subscriber Node:"), parts.next());
                assert_eq!(Some("udp subscriber node"), parts.next());
                assert_eq!(Some("30"), parts.next());
                assert_eq!(Some("11"), parts.next());
            },
            _ => assert_eq!(true, false),
        }
    }

    #[test]
    fn test_simple_executor_new_with() {
        let (_interrupt_tx, interrupt_rx) = mpsc::channel();

        // Initialize the nodes
        let mut basic_node_one = BasicNode::new("basic node", 10);
        let mut server_node = ServerNode::new("server node", 11);
        let mut client_node = ClientNode::new("client node", 12, server_node.create_client(String::from("client node")));
        let mut publisher_node = PublisherNode::new("publisher node", 13);
        let mut subscriber_node = SubscriberNode::new("subscriber node", 14);
        subscriber_node.add_num_subscriber_subscriber(publisher_node.subscribe_to_num_publisher());
        let mut update_server = UpdateServerNode::new("update server node", 15);
        let mut update_client = UpdateClientNode::new("update client node", 16, update_server.create_update_client(String::from("update client node")));

        let mut executor = SimpleExecutor::new_with(
            vec![&mut basic_node_one, &mut server_node, &mut client_node,
                       &mut publisher_node, &mut subscriber_node, &mut update_server,
                       &mut update_client], interrupt_rx
        );

        let basic = executor.heap.pop().unwrap().node;
        assert!(basic.debug().contains("basic node"));

        let server = executor.heap.pop().unwrap().node;
        assert!(server.debug().contains("server node"));

        let client = executor.heap.pop().unwrap().node;
        assert!(client.debug().contains("client node"));

        let publisher = executor.heap.pop().unwrap().node;
        assert!(publisher.debug().contains("publisher node"));

        let subscriber = executor.heap.pop().unwrap().node;
        assert!(subscriber.debug().contains("subscriber node"));

        let update_server = executor.heap.pop().unwrap().node;
        assert!(update_server.debug().contains("update server node"));

        let update_client = executor.heap.pop().unwrap().node;
        assert!(update_client.debug().contains("update client node"));
    }

    #[test]
    fn test_simple_executor_update_loop_interrupt() {
        let (interrupt_tx, interrupt_rx) = mpsc::channel();

        let mut basic_node_one = BasicNode::new("basic node one", 100);
        let mut basic_node_two = BasicNode::new("basic node two", 222);
        let mut executor = SimpleExecutor::new_with(vec![&mut basic_node_one, &mut basic_node_two], interrupt_rx);

        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

        let exec = scope(|scope| {
            let thread_handle = scope.spawn(|| {
                executor.start();
                executor.update_loop();
                return executor;
            });

            thread::sleep(Duration::from_millis(1000));
            interrupt_tx.send(true).unwrap();

            thread_handle.join()
        });

        let end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

        match exec {
            Ok(_) => {
                assert!(1000 <= end_time - start_time && end_time - start_time <= 1222);
            },
            _ => assert_eq!(true, false),
        }
    }
}