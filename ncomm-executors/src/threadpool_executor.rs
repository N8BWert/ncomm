//!
//! The Threadpool Executor takes control of a number of threads and schedules
//! nodes to be run on a threadpool
//!

use std::{any::Any, cmp::max};

use quanta::{Clock, Instant};

use threadpool::ThreadPool;

use crossbeam::channel::{unbounded, Receiver};

use ncomm_core::{Executor, ExecutorState, Node};

use crate::{insert_into, NodeWrapper};

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
pub struct ThreadPoolExecutor<ID: PartialEq> {
    /// The sorted backing vector for the executor
    backing: Vec<NodeWrapper<ID>>,
    /// The quanta high-precision clock backing the ThreadPoll scheduler
    clock: Clock,
    /// The ThreadPool to execute nodes on
    pool: ThreadPool,
    /// The current state of the executor
    state: ExecutorState,
    /// The Instant the executor was started
    start_instant: Instant,
    /// The Interrupt receiver channel
    interrupt: Receiver<bool>,
    /// Whether or not the executor has been interrupted
    interrupted: bool,
}

impl<ID: PartialEq> ThreadPoolExecutor<ID> {
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
    pub fn new_with(
        threads: usize,
        interrupt: Receiver<bool>,
        mut nodes: Vec<Box<dyn Node<ID>>>,
    ) -> Self {
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

impl<ID: PartialEq + 'static> Executor<ID> for ThreadPoolExecutor<ID> {
    /// Context doesn't really apply to Threadpool executors
    type Context = Box<dyn Any>;

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
        while self
            .clock
            .now()
            .duration_since(self.start_instant)
            .as_millis()
            < ms
            && !self.check_interrupt()
        {
            if self.backing.last().is_some()
                && self
                    .clock
                    .now()
                    .duration_since(self.start_instant)
                    .as_micros()
                    >= self.backing.last().unwrap().priority
            {
                let mut node_wrapper = self.backing.pop().unwrap();
                let node_tx = node_tx.clone();
                self.pool.execute(move || {
                    node_wrapper.node.update();
                    node_wrapper.priority += node_wrapper.node.get_update_delay_us();
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
            if self.backing.last().is_some()
                && self
                    .clock
                    .now()
                    .duration_since(self.start_instant)
                    .as_micros()
                    >= self.backing.last().unwrap().priority
            {
                let mut node_wrapper = self.backing.pop().unwrap();
                let node_tx = node_tx.clone();
                self.pool.execute(move || {
                    node_wrapper.node.update();
                    node_wrapper.priority += node_wrapper.node.get_update_delay_us();
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
    /// Note: Nodes can only be added to the executor when it is not running.
    ///
    /// Additionally, only 1 node can exist per id so additional nodes added with the same
    /// id will replace the previous node of a given id
    fn add_node(&mut self, node: Box<dyn Node<ID>>) {
        if let Some(idx) = self
            .backing
            .iter()
            .position(|node_wrapper| node_wrapper.node.get_id().eq(&node.get_id()))
        {
            self.backing.remove(idx);
        }

        if self.state == ExecutorState::Stopped {
            self.backing.push(NodeWrapper { priority: 0, node });
        } else if self.state == ExecutorState::Started {
            insert_into(
                &mut self.backing,
                NodeWrapper {
                    priority: self
                        .clock
                        .now()
                        .duration_since(self.start_instant)
                        .as_micros(),
                    node,
                },
            );
        }
    }

    /// Remove a node from the Threadpool Executor.
    ///
    /// Note: Nodes can only be removed from hte executor when it is not running
    fn remove_node(&mut self, id: &ID) -> Option<Box<dyn Node<ID>>> {
        if self.state != ExecutorState::Running {
            let idx = self
                .backing
                .iter()
                .position(|node_wrapper| node_wrapper.node.get_id().eq(id));
            if let Some(idx) = idx {
                Some(self.backing.remove(idx).destroy())
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{any::Any, thread, time::Duration};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum State {
        Stopped,
        Started,
        Updating,
    }

    struct SimpleNode {
        id: u8,
        update_delay: u128,
        num: u8,
        state: State,
    }

    impl SimpleNode {
        pub fn new(id: u8, update_delay: u128) -> Self {
            Self {
                id,
                update_delay,
                num: 0,
                state: State::Stopped,
            }
        }
    }

    impl Node<u8> for SimpleNode {
        fn get_id(&self) -> u8 {
            self.id
        }

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

        fn get_update_delay_us(&self) -> u128 {
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
                Box::new(SimpleNode::new(0, 10_000)),
                Box::new(SimpleNode::new(1, 25_000)),
            ],
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
                Box::new(SimpleNode::new(0, 10_000)),
                Box::new(SimpleNode::new(1, 25_000)),
            ],
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
                Box::new(SimpleNode::new(0, 10_000)),
                Box::new(SimpleNode::new(1, 25_000)),
            ],
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
                Box::new(SimpleNode::new(0, 10_000)),
                Box::new(SimpleNode::new(1, 25_000)),
            ],
        );

        executor.add_node(Box::new(SimpleNode::new(2, 1_000)));

        assert_eq!(executor.backing.len(), 3);
    }

    #[test]
    fn test_add_node_same_id() {
        let (_, rx) = unbounded();

        let mut executor = ThreadPoolExecutor::new_with(
            3,
            rx,
            vec![
                Box::new(SimpleNode::new(0, 10_000)),
                Box::new(SimpleNode::new(1, 25_000)),
            ],
        );

        executor.add_node(Box::new(SimpleNode::new(0, 1_000)));

        assert_eq!(executor.backing.len(), 2);
        let node_zero = executor
            .backing
            .iter()
            .find(|node_wrapper| node_wrapper.node.get_id().eq(&0))
            .unwrap();
        assert_eq!(node_zero.node.get_update_delay_us(), 1_000);
    }

    #[test]
    fn test_remove_node() {
        let (_, rx) = unbounded();

        let mut executor = ThreadPoolExecutor::new_with(
            3,
            rx,
            vec![
                Box::new(SimpleNode::new(0, 10_000)),
                Box::new(SimpleNode::new(1, 25_000)),
            ],
        );

        executor.remove_node(&0);

        assert_eq!(executor.backing.len(), 1);
        assert_eq!(executor.backing[0].node.get_id(), 1);
    }

    #[test]
    fn test_update_loop() {
        let (tx, rx) = unbounded();

        let mut executor = ThreadPoolExecutor::new_with(
            2,
            rx,
            vec![
                Box::new(SimpleNode::new(0, 10_000)),
                Box::new(SimpleNode::new(1, 25_000)),
            ],
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
