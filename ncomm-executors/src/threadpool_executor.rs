//!
//! The Threadpool Executor takes control of a number of threads and schedules
//! nodes to be run on a threadpool
//! 

use std::cmp::max;

use quanta::{Clock, Instant};

use threadpool::ThreadPool;

use crossbeam::channel::{Receiver, unbounded};

use ncomm_core::{Executor, ExecutorState, Node};

use crate::{NodeWrapper, insert_into};

/// ThreadPool Executor
/// 
/// The ThreadPool Executor stores Nodes in a sorted vector and sends them to
/// be executed by the threadPool.
/// 
/// Note: The ThreadPool Executor ca be interrupted by sending a true value
/// over the mpsc channel whose receiving end is owned by the ThreadPool
/// executor.
/// 
/// Addendum: The main thread of the ThreadPool is conducting the scheduling so
/// the ThreadPool will only have n-1 worker threads where n is the total number
/// of threads allocated to the threadpool executor.
pub struct ThreadPoolExecutor {
    // The sorted backing vector for the executor
    backing: Vec<NodeWrapper>,
    // The quanta high-precision clock backing the ThreadPoll scheduler
    clock: Clock,
    // The ThreadPool to execute nodes on
    pool: ThreadPool,
    // The current state of the executor
    state: ExecutorState,
    // The Instant the executor was started
    start_instant: Instant,
    // The Interrupt receiver channel
    interrupt: Receiver<bool>,
    // Whether or not the executor has been interrupted
    interrupted: bool,
}

impl ThreadPoolExecutor {
    /// Creates a new ThreadPool executor without any Nodes
    pub fn new(threads: usize, interrupt: Receiver<bool>) -> Self {
        let clock = Clock::new();
        let now = clock.now();
        let pool = ThreadPool::new(max(1, threads.saturating_sub(1)));

        Self {
            backing: Vec::new(),
            clock,
            pool,
            state: ExecutorState::Stopped,
            start_instant: now,
            interrupt,
            interrupted: false,
        }
    }

    /// Creates a new ThreadPool Executor with a number of Nodes
    pub fn new_with(threads: usize, interrupt: Receiver<bool>, mut nodes: Vec<Box<dyn Node>>) -> Self {
        let mut backing = Vec::new();
        for node in nodes.drain(..) {
            backing.push(NodeWrapper { priority: 0, node });
        }

        let clock = Clock::new();
        let now = clock.now();
        let pool = ThreadPool::new(max(1, threads.saturating_sub(1)));

        Self {
            backing,
            clock,
            pool,
            state: ExecutorState::Stopped,
            start_instant: now,
            interrupt,
            interrupted: false,
        }
    }
}

impl Executor for ThreadPoolExecutor {
    /// For each node in the ThreadPool executor the node will be updated
    /// and start_instant will be set to the current instant
    /// 
    /// Note: this should probably not be called individually because it will
    /// always be called at the beginning of `update_for_ms` or `update_loop`
    fn start(&mut self) {
        for node_wrapper in self.backing.iter_mut() {
            node_wrapper.priority = 0;
            node_wrapper.node.start();
        }

        self.interrupted = false;
        self.state = ExecutorState::Started;
        self.start_instant = self.clock.now();
    }

    fn update_for_ms(&mut self, ms: u128) {
        // Start the Executor
        self.start();

        // Run the Executor
        self.state = ExecutorState::Running;
        let (node_tx, node_rx) = unbounded();
        while self.clock.now().duration_since(self.start_instant).as_millis() < ms && !self.check_interrupt() {
            if self.backing.last().is_some() && self.clock.now().duration_since(self.start_instant).as_micros() >= self.backing.last().unwrap().priority {
                let mut node_wrapper = self.backing.pop().unwrap();
                let node_tx = node_tx.clone();
                self.pool.execute(move || {
                    node_wrapper.node.update();
                    node_wrapper.priority += node_wrapper.node.get_update_delay();
                    node_tx.send(node_wrapper).unwrap();
                });
            }

            if let Ok(node_wrapper) = node_rx.try_recv() {
                insert_into(&mut self.backing, node_wrapper);
            }
        }

        // Stop the Executor
        for node_wrapper in self.backing.iter_mut() {
            node_wrapper.priority = 0;
            node_wrapper.node.shutdown();
        }
        self.state = ExecutorState::Stopped;
    }

    fn update_loop(&mut self) {
        // Start the Executor
        self.start();

        // Run the Executor
        self.state = ExecutorState::Running;
        let (node_tx, node_rx) = unbounded();
        while !self.check_interrupt() {
            if self.backing.last().is_some() && self.clock.now().duration_since(self.start_instant).as_micros() >= self.backing.last().unwrap().priority {
                let mut node_wrapper = self.backing.pop().unwrap();
                let node_tx = node_tx.clone();
                self.pool.execute(move || {
                    node_wrapper.node.update();
                    node_wrapper.priority += node_wrapper.node.get_update_delay();
                    node_tx.send(node_wrapper).unwrap();
                });
            }

            if let Ok(node_wrapper) = node_rx.try_recv() {
                insert_into(&mut self.backing, node_wrapper);
            }
        }

        // Stop the Executor
        for node_wrapper in self.backing.iter_mut() {
            node_wrapper.priority = 0;
            node_wrapper.node.shutdown();
        }
        self.state = ExecutorState::Stopped;
    }

    /// Check the interrupt receiver for an interrupt
    fn check_interrupt(&mut self) -> bool {
        if let Ok(interrupt) = self.interrupt.try_recv() {
            self.interrupted = interrupt;
        }
        self.interrupted
    }

    /// Add a node to the ThreadPool Executor.
    /// 
    /// Note: If the executor is currently in `ExecutorState::Started` or
    /// `ExecutorState::Running` the node will be added with maximum
    /// priority to the backing vector.
    fn add_node(&mut self, node: Box<dyn Node>) {
        if self.state == ExecutorState::Stopped {
            self.backing.push(NodeWrapper { priority: 0, node });
        } else {
            insert_into(
                &mut self.backing,
                NodeWrapper {
                    priority: self.clock.now().duration_since(self.start_instant).as_micros(),
                    node,
                }
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{any::Any, time::Duration, thread};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum State {
        Stopped,
        Started,
        Updating,
    }

    struct SimpleNode {
        update_delay: u128,
        num: u8,
        state: State,
    }

    impl SimpleNode {
        pub fn new(update_delay: u128) -> Self {
            Self {
                update_delay,
                num: 0,
                state: State::Stopped,
            }
        }
    }

    impl Node for SimpleNode {
        fn start(&mut self) {
            self.state = State::Started;
        }

        fn update(&mut self) {
            self.state = State::Updating;
            self.num = self.num.wrapping_add(1);
        }

        fn shutdown(&mut self) {
            self.state = State::Stopped;
        }

        fn get_update_delay(&self) -> u128 {
            self.update_delay
        }
    }

    #[test]
    fn test_start() {
        let (_, rx) = unbounded();

        let mut executor = ThreadPoolExecutor::new_with(
            3,
            rx,
            vec![
                Box::new(SimpleNode::new(10_000)),
                Box::new(SimpleNode::new(25_000)),
            ]
       );
       let original_start_instant = executor.start_instant;

       executor.start();

       for node_wrapper in executor.backing.iter() {
        assert_eq!(node_wrapper.priority, 0);
        let simple_node: &dyn Any = &node_wrapper.node;
        let simple_node: &Box<SimpleNode> = unsafe { simple_node.downcast_ref_unchecked() };
        assert_eq!(simple_node.state, State::Started);
       }

       assert!(!executor.interrupted);
       assert_eq!(executor.state, ExecutorState::Started);
       assert!(executor.start_instant > original_start_instant);
    }

    #[test]
    fn test_update_for_ms() {
        let (_, rx) = unbounded();

        let mut executor = ThreadPoolExecutor::new_with(
            3,
            rx,
            vec![
                Box::new(SimpleNode::new(10_000)),
                Box::new(SimpleNode::new(25_000)),
            ]
       );

       let start = executor.clock.now();
       executor.update_for_ms(100);
       let end = executor.clock.now();

       // Check the nodes were started and updated
       for node_wrapper in executor.backing.iter() {
        assert_eq!(node_wrapper.priority, 0);
        let simple_node: &dyn Any = &node_wrapper.node;
        let simple_node: &Box<SimpleNode> = unsafe { simple_node.downcast_ref_unchecked() };
        assert_eq!(simple_node.state, State::Stopped);
        assert!([3, 4, 5, 9, 10, 11].contains(&simple_node.num));
       }

       assert!(Duration::from_millis(95) < end - start);
       assert!(end - start < Duration::from_millis(105));
    }

    #[test]
    fn test_check_interrupt() {
        let (tx, rx) = unbounded();

        let mut executor = ThreadPoolExecutor::new_with(
            3,
            rx,
            vec![
                Box::new(SimpleNode::new(10_000)),
                Box::new(SimpleNode::new(25_000)),
            ]
       );

       tx.send(true).unwrap();

       assert!(executor.check_interrupt());
    }

    #[test]
    fn test_add_node() {
        let (_, rx) = unbounded();

        let mut executor = ThreadPoolExecutor::new_with(
            3,
            rx,
            vec![
                Box::new(SimpleNode::new(10_000)),
                Box::new(SimpleNode::new(25_000)),
            ]
       );

       executor.add_node(Box::new(SimpleNode::new(1_000)));

       assert_eq!(executor.backing.len(), 3);
    }

    #[test]
    fn test_update_loop() {
        let (tx, rx) = unbounded();

        let mut executor = ThreadPoolExecutor::new_with(
            2,
            rx,
            vec![
                Box::new(SimpleNode::new(10_000)),
                Box::new(SimpleNode::new(25_000)),
            ]
        );

        let handle = thread::spawn(move || {
            executor.update_loop();
            executor
        });

        thread::sleep(Duration::from_millis(100));
        tx.send(true).unwrap();

        let executor = handle.join().unwrap();
        for node_wrapper in executor.backing.iter() {
            assert_eq!(node_wrapper.priority, 0);
            let simple_node: &dyn Any = &node_wrapper.node;
            let simple_node: &Box<SimpleNode> = unsafe { simple_node.downcast_ref_unchecked() };
            assert_eq!(simple_node.state, State::Stopped);
            assert!([3, 4, 5, 9, 10, 11].contains(&simple_node.num));
        }

        assert!(executor.interrupted);
        assert_eq!(executor.state, ExecutorState::Stopped);
    }
}