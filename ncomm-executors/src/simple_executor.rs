//!
//! The Simple Executor
//!
//! The simple executor is the most simple and easy to understand
//! executor in the NComm system.  Basically, the simple executor is
//! a singular thread that stores each node in a sorted vector and
//! pops off the highest priority Node, executes its update method
//! and then inserts it into the sorted vector with an updated priority.
//!
//! In practice, I would say it is unlikely for the simple executor
//! to find a lot of use out in the wild but it is probably the best
//! executor for single threaded execution.
//!

use std::any::Any;

use crossbeam::channel::Receiver;

use quanta::{Clock, Instant};

use ncomm_core::{Executor, ExecutorState, Node};

use crate::{insert_into, NodeWrapper};

/// Simple Executor
///
/// This simple executor stores Nodes in a sorted vector where the
/// priority is higher the closer to the current timestamp the Node's
/// next update is.
///
/// Note: The Simple Executor can be interrupted by sending a true value
/// over the mpsc channel whose receiving end is owned by the SimpleExecutor
///
/// Addendum: The Simple Executor will also busy wait between node executions
/// so do not expect the SimpleExecutor to yield CPU time to other processes while
/// it is running.
pub struct SimpleExecutor<ID: PartialEq> {
    /// The sorted backing vector for the executor
    pub(crate) backing: Vec<NodeWrapper<ID>>,
    /// The quanta high-precision clock backing the SimplExecutor
    clock: Clock,
    /// The current state of the executor
    state: ExecutorState,
    /// The Instant the executor was started
    start_instant: Instant,
    /// The Interrupt receiver channel
    interrupt: Receiver<bool>,
    /// Whether or not the executor has been interrupted
    interrupted: bool,
}

impl<ID: PartialEq> SimpleExecutor<ID> {
    /// Create a new Simple Executor without any Nodes
    pub fn new(interrupt: Receiver<bool>) -> Self {
        let clock = Clock::new();
        let now = clock.now();

        Self {
            backing: Vec::new(),
            clock,
            start_instant: now,
            state: ExecutorState::Stopped,
            interrupt,
            interrupted: false,
        }
    }

    /// Creates a new Simple Executor with a number of Nodes
    pub fn new_with(interrupt: Receiver<bool>, mut nodes: Vec<Box<dyn Node<ID>>>) -> Self {
        let mut backing = Vec::new();
        for node in nodes.drain(..) {
            backing.push(NodeWrapper { priority: 0, node });
        }

        let clock = Clock::new();
        let now = clock.now();

        Self {
            backing,
            clock,
            start_instant: now,
            state: ExecutorState::Stopped,
            interrupt,
            interrupted: false,
        }
    }
}

impl<ID: PartialEq> Executor<ID> for SimpleExecutor<ID> {
    /// Context doesn't really apply to SimpleExecutors
    type Context = Box<dyn Any>;

    /// For each node in the simple executor we should reset their priority to 0
    /// and start the node.  We should also set the start_instant to the current time.
    ///
    /// Note: this method should not be called individually as it will always be
    /// called during the `update_for_ms` and `update_loop` methods so running
    /// it here is completely redundant.
    fn start(&mut self) {
        for node_wrapper in self.backing.iter_mut() {
            node_wrapper.priority = 0;
            node_wrapper.node.start();
        }

        self.interrupted = false;
        self.state = ExecutorState::Started;
        self.start_instant = self.clock.now();
    }

    /// Start the executor and run the executor for a given number of milliseconds before
    /// stopping the executor.  An interrupt will also stop the executor early.
    ///
    /// Note: if there are no Nodes currently in the executor it will busy wait until the
    /// time has passed or an interrupt occurs
    fn update_for_ms(&mut self, ms: u128) {
        // Start the Executor
        self.start();

        // Run the Executor
        self.state = ExecutorState::Running;
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
                node_wrapper.node.update();
                node_wrapper.priority += node_wrapper.node.get_update_delay_us();
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

    /// Start the executor and run until an interrupt is received.
    ///
    /// Note: if there are no Nodes currently in the executor it will busy wait until it
    /// receives an interrupt
    fn update_loop(&mut self) {
        // Start the Executor
        self.start();

        // Run the Executor
        self.state = ExecutorState::Running;
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
                node_wrapper.node.update();
                node_wrapper.priority += node_wrapper.node.get_update_delay_us();
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

    /// Check the interrupt receiver for an interrupt.  If an interrupt
    /// signal was sent over the channel then this node should report that
    /// it was interrupted.
    fn check_interrupt(&mut self) -> bool {
        if let Ok(interrupt) = self.interrupt.try_recv() {
            self.interrupted = interrupt;
        }
        self.interrupted
    }

    /// Add a node to the Simple Executor.
    ///
    /// Note: Nodes can only be added to the executor when it is not running.
    ///
    /// Additionally, only 1 node can exist per id so additional nodes added with
    /// the same id will replace the previous node of a given id.
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

    /// Remove a node from the Simple Executor.
    ///
    /// Note: Nodes can only be removed from the executor when it is not running.
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

    use crossbeam::channel::unbounded;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum State {
        Stopped,
        Started,
        Updating,
    }

    pub struct SimpleNode {
        id: u8,
        pub update_delay: u128,
        pub num: u8,
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
    /// Start should set the priority of all nodes to 0, start all nodes, set its
    /// interrupted value to false, enter the ExecutorState::Started state and set its
    /// start instant
    fn test_simple_executor_start() {
        let (_, rx) = unbounded();

        let mut executor = SimpleExecutor::new_with(
            rx,
            vec![
                Box::new(SimpleNode::new(0, 100_000)),
                Box::new(SimpleNode::new(1, 250_000)),
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

        let mut executor = SimpleExecutor::new_with(
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
            // Priority should have been reset to 0
            assert!(node_wrapper.priority == 0);
            let simple_node: &dyn Any = &node_wrapper.node;
            let simple_node: &Box<SimpleNode> = unsafe { simple_node.downcast_ref_unchecked() };
            assert_eq!(simple_node.state, State::Stopped);
            // Check the node has been updated a valid number of times
            assert!([9, 10, 11, 3, 4, 5].contains(&simple_node.num));
        }

        assert!(Duration::from_millis(95) < end - start);
        assert!(end - start < Duration::from_millis(105));
    }

    #[test]
    fn test_check_interrupt() {
        let (tx, rx) = unbounded();

        let mut executor = SimpleExecutor::new_with(
            rx,
            vec![
                Box::new(SimpleNode::new(0, 110_000)),
                Box::new(SimpleNode::new(1, 25_000)),
            ],
        );

        tx.send(true).unwrap();

        assert!(executor.check_interrupt());
    }

    #[test]
    fn test_add_node_stopped() {
        let (_, rx) = unbounded();

        let mut executor = SimpleExecutor::new_with(
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

        let mut executor = SimpleExecutor::new_with(
            rx,
            vec![
                Box::new(SimpleNode::new(0, 10_000)),
                Box::new(SimpleNode::new(1, 25_000)),
            ],
        );

        executor.add_node(Box::new(SimpleNode::new(0, 1_000)));

        assert_eq!(executor.backing.len(), 2);
        let zero_id = executor
            .backing
            .iter()
            .find(|node_wrapper| node_wrapper.node.get_id().eq(&0))
            .unwrap();
        assert_eq!(zero_id.node.get_update_delay_us(), 1_000);
    }

    #[test]
    fn test_remove_node() {
        let (_, rx) = unbounded();

        let mut executor = SimpleExecutor::new_with(
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

        let mut executor = SimpleExecutor::new_with(
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
