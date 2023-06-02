use crate::executor::{Executor, SingleThreadedExecutor, MultiThreadedExecutor, node_wrapper::NodeWrapper, simple_executor::SimpleExecutor};
use crate::node::Node;

use std::{collections::{BinaryHeap, HashMap}, time::{Duration, SystemTime, UNIX_EPOCH}};
use std::thread;

pub struct SimpleMultiExecutor<'a> {
    threads: HashMap<String, SimpleExecutor<'a>>,
    start_time: u128,
    interrupted: bool
}

impl<'a> SimpleMultiExecutor<'a> {
    /// Create a new empty Simple Executor
    pub fn new() -> Self {
        Self {
            threads: HashMap::new(),
            start_time: 0,
            interrupted: false
        }
    }

    /// Creates a new Simple Executor with the nodes given
    pub fn new_with(nodes: Vec<(String, &'a mut dyn Node)>) -> Self {
        let mut map: HashMap<String, SimpleExecutor> = HashMap::new();

        for (thread, node) in nodes {
            match map.get_mut(&thread) {
                Some(executor) => {
                    executor.add_node(node);
                },
                None => {
                    let mut executor = SimpleExecutor::new();
                    executor.add_node(node);
                    map.insert(thread, executor);
                }
            }
        }

        Self {
            threads: map,
            start_time: 0,
            interrupted: false
        }
    }
}

impl<'a> MultiThreadedExecutor<'a> for SimpleMultiExecutor<'a> {
    fn add_node_to(&mut self, new_node: &'a mut dyn Node, thread: String) {
        match self.threads.get_mut(&thread) {
            Some(executor) => {
                executor.add_node(new_node);
            },
            None => {
                let mut executor = SimpleExecutor::new();
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
        
    }

    fn update_loop(&mut self) {
        
    }

    fn interrupt(&mut self) {
        
    }

    fn log(&self) {
        
    }
}