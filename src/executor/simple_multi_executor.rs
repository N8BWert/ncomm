use crate::executor::{Executor, SingleThreadedExecutor, MultiThreadedExecutor, simple_executor::SimpleExecutor};
use crate::node::Node;

use std::collections::HashMap;
use std::time::{Duration};
use std::thread;
use std::sync::{mpsc, mpsc::{Receiver, Sender}};

pub struct SimpleMultiExecutor<'a> {
    threads: HashMap<String, SimpleExecutor<'a>>,
    interrupt_txs: Vec<Sender<bool>>,
    interrupt_rx: Receiver<bool>,
    interrupted: bool,
}

impl<'a> SimpleMultiExecutor<'a> {
    /// Create a new empty Simple Executor
    pub fn new(interrupt_rx: Receiver<bool>) -> Self {
        Self {
            threads: HashMap::new(),
            interrupt_txs: Vec::new(),
            interrupt_rx,
            interrupted: false
        }
    }

    /// Creates a new Simple Executor with the nodes given
    pub fn new_with(nodes: Vec<(String, &'a mut dyn Node)>, interrupt_rx: Receiver<bool>) -> Self {
        let mut map: HashMap<String, SimpleExecutor> = HashMap::new();
        let mut interrupt_txs = Vec::new();

        for (thread, node) in nodes {
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
                    return executor;
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

            return (thread_names, executors);
        });

        let mut map = HashMap::new();
        for (thread_name, executor) in thread_names.iter().zip(executors) {
            map.insert(thread_name.clone(), executor);
        }

        self.threads = map;
    }

    fn update_loop(&mut self) {
        let (thread_names, executors) = thread::scope(|scope| {
            let mut thread_names = Vec::with_capacity(self.threads.len());
            let mut handles = Vec::with_capacity(self.threads.len());
            let mut executors = Vec::with_capacity(self.threads.len());

            for (thread_name, mut executor) in self.threads.drain() {
                thread_names.push(thread_name);
                handles.push(scope.spawn(|| {
                    executor.start();
                    executor.update_loop();
                    return executor;
                }));
            }

            while !self.check_interrupt() {
                // TODO: In the future the main thread can probably do something useful
                // other than just sleeping
                thread::sleep(Duration::from_secs(2));
            }

            for interrupt_tx in &self.interrupt_txs {
                interrupt_tx.send(true).unwrap();
            }

            self.log("Interrupt Sent Waiting For Wind Down");

            for handle in handles.drain(..) {
                executors.push(handle.join().unwrap());
            }

            self.log("Wind Down Complete");

            return (thread_names, executors);
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
        return self.interrupted;
    }

    fn log(&self, message: &str) {
        // TODO: In the future we'll probably do something more than just print to the console
        println!("{}", message);
    }
}