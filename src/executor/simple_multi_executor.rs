//!
//! Executor that manages multiple single-threaded executors (Preferred).
//! 
//! The Multi-Threaded Executor provides a simple-to-use api for users to add a large number
//! of nodes to a large number of possible threads.  The basic premise of this executor
//! is that there are a large number of single-threaded executors that are managed by a singular main
//! thread that has the ability to monitor and interrupt the other executors.
//! 

use crate::executor::{Executor, SingleThreadedExecutor, MultiThreadedExecutor, simple_executor::SimpleExecutor};
use crate::node::Node;

use std::collections::HashMap;
use std::time::Duration;
use std::thread;
use std::sync::{mpsc, mpsc::{Receiver, Sender}};

/// Multi-Threaded Executor that maps a large number of single executors to
/// strings (thread names) and manages the multi-threaded execution and interrupts for
/// each of these executors.
pub struct SimpleMultiExecutor<'a> {
    threads: HashMap<String, SimpleExecutor<'a>>,
    interrupt_txs: Vec<Sender<bool>>,
    interrupt_rx: Receiver<bool>,
    interrupt_tx: Sender<bool>,
    interrupted: bool,
}

impl<'a> SimpleMultiExecutor<'a> {
    /// Create a new empty Simple Executor
    pub fn new() -> Self {
        let (interrupt_tx, interrupt_rx) = mpsc::channel();

        Self {
            threads: HashMap::new(),
            interrupt_txs: Vec::new(),
            interrupt_rx,
            interrupt_tx,
            interrupted: false
        }
    }

    /// Creates a new Simple Executor with the nodes given
    pub fn new_with(nodes: Vec<(&str, &'a mut dyn Node)>) -> Self {
        let (interrupt_tx, interrupt_rx) = mpsc::channel();

        let mut map: HashMap<String, SimpleExecutor> = HashMap::new();
        let mut interrupt_txs = Vec::new();

        for (thread, node) in nodes {
            let thread = String::from(thread);
            match map.get_mut(&thread) {
                Some(executor) => {
                    executor.add_node(node);
                },
                None => {
                    let (interrupt_tx, interrupt_rx) = mpsc::channel();
                    interrupt_txs.push(interrupt_tx);
                    let mut executor = SimpleExecutor::new(interrupt_rx);
                    executor.add_node(node);
                    map.insert(thread, executor);
                }
            }
        }

        Self {
            threads: map,
            interrupt_txs,
            interrupt_rx,
            interrupt_tx,
            interrupted: false
        }
    }

    /// Returns a clone of the interrupt transmitter to this node
    /// 
    /// Returns:
    ///     Sender<bool>: the interrupt sender to this executor
    pub fn get_interrupt_tx(&self) -> Sender<bool> {
        self.interrupt_tx.clone()
    }
}

impl<'a> Default for SimpleMultiExecutor<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> MultiThreadedExecutor<'a> for SimpleMultiExecutor<'a> {
    fn add_node_to(&mut self, new_node: &'a mut dyn Node, thread: &str) {
        let thread = String::from(thread);

        match self.threads.get_mut(&thread) {
            Some(executor) => {
                executor.add_node(new_node);
            },
            None => {
                let (interrupt_tx, interrupt_rx) = mpsc::channel();
                self.interrupt_txs.push(interrupt_tx);
                let mut executor = SimpleExecutor::new(interrupt_rx);
                executor.add_node(new_node);
                self.threads.insert(thread, executor);
            }
        }
    }
}

impl<'a> Executor<'a> for SimpleMultiExecutor<'a> {
    fn start(&mut self) {
        for (_thread, executor) in self.threads.iter_mut() {
            executor.start();
        }
    }

    fn update_for_seconds(&mut self, seconds: u128) {
        let (thread_names, executors) = thread::scope(|scope| {
            let mut thread_names = Vec::with_capacity(self.threads.len());
            let mut handles = Vec::with_capacity(self.threads.len());
            let mut executors = Vec::with_capacity(self.threads.len());

            for (thread_name, mut executor) in self.threads.drain() {
                thread_names.push(thread_name);
                handles.push(scope.spawn(|| {
                    executor.start();
                    executor.update_loop();
                    executor
                }));
            }

            // TODO: In the future the main thread can probably do something useful
            // other than just sleeping
            thread::sleep(Duration::from_secs(seconds as u64));

            for interrupt_tx in &self.interrupt_txs {
                interrupt_tx.send(true).unwrap();
            }

            self.log("Interrupt Sent Waiting For Wind Down");

            for handle in handles.drain(..) {
                executors.push(handle.join().unwrap());
            }

            self.log("Wind Down Complete");

            (thread_names, executors)
        });

        let mut map = HashMap::new();
        for (thread_name, executor) in thread_names.iter().zip(executors) {
            map.insert(thread_name.clone(), executor);
        }

        self.threads = map;
    }

    fn update_loop(&mut self) {
        let mut txs = self.interrupt_txs.clone();
        
        ctrlc::set_handler(move || {
            for interrupt_tx in txs.drain(..) {
                interrupt_tx.send(true).unwrap();
            }
        }).expect("Error Setting Loop Interrupt Handler");

        let (thread_names, executors) = thread::scope(|scope| {
            let mut thread_names = Vec::with_capacity(self.threads.len());
            let mut handles = Vec::with_capacity(self.threads.len());
            let mut executors = Vec::with_capacity(self.threads.len());

            for (thread_name, mut executor) in self.threads.drain() {
                thread_names.push(thread_name);
                handles.push(scope.spawn(|| {
                    executor.start();
                    executor.update_loop();
                    executor
                }));
            }

            while !self.check_interrupt() {
                // TODO: In the future the main thread can probably do something useful
                // other than just sleeping
                thread::sleep(Duration::from_millis(10));
            }

            for interrupt_tx in &self.interrupt_txs {
                interrupt_tx.send(true).unwrap();
            }

            self.log("Interrupt Sent Waiting For Wind Down");

            for handle in handles.drain(..) {
                executors.push(handle.join().unwrap());
            }

            self.log("Wind Down Complete");

            (thread_names, executors)
        });

        let mut map = HashMap::new();
        for (thread_name, executor) in thread_names.iter().zip(executors) {
            map.insert(thread_name.clone(), executor);
        }

        self.threads = map;
    }

    fn check_interrupt(&mut self) -> bool {
        let iter = self.interrupt_rx.try_iter();
        if let Some(interrupt) = iter.last() {
            if interrupt != self.interrupted {
                self.interrupted = interrupt;
            }
        }
        self.interrupted
    }

    fn log(&self, message: &str) {
        // TODO: In the future we'll probably do something more than just print to the console
        println!("{}", message);
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::node::basic_node::BasicNode;

    use super::*;

    #[test]
    fn test_simple_multi_executor_new() {
        let executor = SimpleMultiExecutor::new();

        assert_eq!(executor.threads.len(), 0);
        assert_eq!(executor.interrupt_txs.len(), 0);
        assert_eq!(executor.interrupted, false);
    }

    #[test]
    fn test_simple_multi_executor_new_with() {
        let mut node_one = BasicNode::new("node one", 10);
        let mut node_two = BasicNode::new("node two", 33);

        let executor = SimpleMultiExecutor::new_with(
            vec![
                ("thread_one", &mut node_one),
                ("thread two", &mut node_two),
            ]
        );

        assert_eq!(executor.threads.len(), 2);
        assert_eq!(executor.interrupt_txs.len(), 2);
        assert_eq!(executor.interrupted, false);
    }

    #[test]
    fn test_simple_multi_executor_add_new_node() {
        let mut node = BasicNode::new("Node one", 22);

        let mut executor = SimpleMultiExecutor::new();
        executor.add_node_to(&mut node, "thread two");

        assert_eq!(executor.threads.len(), 1);
        assert_eq!(executor.interrupt_txs.len(), 1);
        assert_eq!(executor.interrupted, false);
    }

    #[test]
    fn test_simple_multi_executor_start() {
        let mut node_one = BasicNode::new("node one", 22);
        let mut node_two = BasicNode::new("node two", 33);

        let mut executor = SimpleMultiExecutor::new_with(
            vec![
                ("thread one", &mut node_one),
                ("thread two", &mut node_two)
            ]
        );

        executor.start();

        let exec_one = executor.threads.get_mut(&String::from("thread one")).unwrap();
        let node_one = exec_one.heap.pop().unwrap().node;
        assert_eq!(node_one.debug(), String::from("Basic Node:\nnode one\n22\n1\n"));

        let exec_two = executor.threads.get_mut(&String::from("thread two")).unwrap();
        let node_two = exec_two.heap.pop().unwrap().node;
        assert_eq!(node_two.debug(), String::from("Basic Node:\nnode two\n33\n1\n"));
    }

    #[test]
    fn test_simple_multi_executor_update_for_seconds() {
        let mut node_one = BasicNode::new("node one", 22);
        let mut node_two = BasicNode::new("node two", 10);

        let mut executor = SimpleMultiExecutor::new_with(
            vec![
                ("thread one", &mut node_one),
                ("thread two", &mut node_two)
            ]
        );

        executor.start();

        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

        executor.update_for_seconds(1);

        let end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

        assert!(1000 <= end_time - start_time);
        assert!(end_time - start_time <= 1000 + 22);
    }

    #[test]
    fn test_simple_multi_executor_update_loop_for_one_second() {
        let mut node_one = BasicNode::new("node one", 22);
        let mut node_two = BasicNode::new("node two", 10);

        let mut executor = SimpleMultiExecutor::new_with(
            vec![
                ("thread one", &mut node_one),
                ("thread two", &mut node_two)
            ]
        );

        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        
        let interrupt_tx = executor.get_interrupt_tx();

        let exec = thread::scope(|scope| {
            let handle = scope.spawn(|| {
                executor.start();
                executor.update_loop();
                return executor;
            });

            thread::sleep(Duration::from_secs(1));
            interrupt_tx.send(true).unwrap();

            return handle.join();
        });

        let end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

        match exec {
            Ok(_) => assert!(true),
            Err(_) => assert!(false),
        };

        println!("{}", end_time - start_time);

        assert!(1000 <= end_time - start_time);
        assert!(end_time - start_time <= 1032);
    }
}