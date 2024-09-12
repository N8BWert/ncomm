//!
//! The Threaded Executor takes control of a given number of threads and sends a simple
//! executor over each of the threads to execute Nodes.
//!
//! I would typically recommend utilizing the threadpool executor if each Node is equally
//! important to execute.  If, however, there is a specific Node that should always
//! execute at a given rate and needs to be on its own thread to accomplish that, the
//! Threaded Executor may be the best choice.
//!

use std::thread;

use quanta::{Clock, Instant};

use crossbeam::channel::{unbounded, Receiver, Sender};

use ncomm_core::{Executor, ExecutorState, Node};

use crate::{insert_into, NodeWrapper, SimpleExecutor};

/// Threaded Executor
///
/// The Threaded Executor stores nodes in a bunch of SimpleExecutors on
/// given threads.  On the update loop each of hte SimpleExecutors execute
/// their nodes in parallel
pub struct ThreadedExecutor<NID: PartialEq + Send, TID: PartialEq + Send> {
    /// The executors to run
    executors: Vec<(SimpleExecutor<NID>, TID)>,
    /// The backing for the main thread
    backing: Vec<NodeWrapper<NID>>,
    /// The thread id of the main thread
    thread_id: TID,
    /// The quanta high-prevision clock
    clock: Clock,
    /// The Instant the executor was started
    start_instant: Instant,
    /// The current state of the executor
    state: ExecutorState,
    /// The interrupt receiver channel
    interrupt: Receiver<bool>,
    /// The interrupt senders used to propagate the interrupt to other threads
    interrupt_propagators: Vec<Sender<bool>>,
    /// Whether or not the executor has been interrupted
    interrupted: bool,
}

impl<NID: PartialEq + Send, TID: PartialEq + Send> ThreadedExecutor<NID, TID> {
    /// Create a new Threaded Executor without any Nodes
    pub fn new(interrupt: Receiver<bool>, main_thread_id: TID) -> Self {
        let clock = Clock::new();
        let now = clock.now();

        Self {
            executors: Vec::new(),
            backing: Vec::new(),
            thread_id: main_thread_id,
            clock,
            state: ExecutorState::Stopped,
            start_instant: now,
            interrupt,
            interrupt_propagators: Vec::new(),
            interrupted: false,
        }
    }

    /// Creates a new Threaded executor with a given mapping for nodes
    #[allow(clippy::type_complexity)]
    pub fn new_with(
        interrupt: Receiver<bool>,
        main_thread_id: TID,
        mut nodes: Vec<(Vec<Box<dyn Node<NID>>>, TID)>,
    ) -> Self {
        let mut backing = Vec::new();
        if let Some(idx) = nodes.iter().position(|(_, tid)| tid.eq(&main_thread_id)) {
            let (mut node_list, _) = nodes.remove(idx);
            for node in node_list.drain(..) {
                backing.push(NodeWrapper { priority: 0, node });
            }
        }

        let mut executors = Vec::new();
        let mut interrupt_propagators = Vec::new();
        for (node_list, thread_id) in nodes.drain(..) {
            let (tx, rx) = unbounded();
            interrupt_propagators.push(tx);
            executors.push((SimpleExecutor::new_with(rx, node_list), thread_id));
        }

        let clock = Clock::new();
        let now = clock.now();

        Self {
            executors,
            backing,
            thread_id: main_thread_id,
            clock,
            start_instant: now,
            state: ExecutorState::Stopped,
            interrupt,
            interrupt_propagators,
            interrupted: false,
        }
    }

    fn start_self(&mut self) {
        for node_wrapper in self.backing.iter_mut() {
            node_wrapper.priority = 0;
            node_wrapper.node.start();
        }

        self.interrupted = false;
        self.state = ExecutorState::Started;
        self.start_instant = self.clock.now();
    }
}

impl<NID: PartialEq + Send + 'static, TID: PartialEq + Send + 'static> Executor<NID>
    for ThreadedExecutor<NID, TID>
{
    type Context = TID;

    fn start(&mut self) {
        let mut handles = Vec::new();
        for (mut executor, tid) in self.executors.drain(..) {
            handles.push(thread::spawn(move || {
                executor.start();
                (executor, tid)
            }));
        }

        self.start_self();

        for handle in handles {
            self.executors.push(handle.join().unwrap());
        }
    }

    fn update_for_ms(&mut self, ms: u128) {
        // Dispatch the other threads
        let mut handles = Vec::new();
        for (mut executor, tid) in self.executors.drain(..) {
            handles.push(thread::spawn(move || {
                executor.update_for_ms(ms);
                (executor, tid)
            }));
        }

        // Start this exector
        self.start_self();

        // Run this executor
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

        for handle in handles {
            self.executors.push(handle.join().unwrap());
        }
    }

    fn update_loop(&mut self) {
        // Dispatch the other threads
        let mut handles = Vec::new();
        for (mut executor, tid) in self.executors.drain(..) {
            handles.push(thread::spawn(move || {
                executor.update_loop();
                (executor, tid)
            }));
        }

        // Start this executor
        self.start_self();

        // Run the executor
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

        // Stop this executor
        for node_wrapper in self.backing.iter_mut() {
            node_wrapper.priority = 0;
            node_wrapper.node.shutdown();
        }
        self.state = ExecutorState::Stopped;

        for handle in handles {
            self.executors.push(handle.join().unwrap());
        }
    }

    fn check_interrupt(&mut self) -> bool {
        if let Ok(interrupt) = self.interrupt.try_recv() {
            self.interrupted = interrupt;
            for tx in self.interrupt_propagators.iter_mut() {
                tx.send(interrupt).unwrap();
            }
        }

        self.interrupted
    }

    fn add_node(&mut self, node: Box<dyn Node<NID>>) {
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

    fn add_node_with_context(&mut self, node: Box<dyn Node<NID>>, _ctx: Self::Context) {
        if _ctx == self.thread_id {
            self.add_node(node);
        } else if let Some((executor, _)) = self.executors.iter_mut().find(|(_, tid)| tid.eq(&_ctx))
        {
            executor.add_node(node);
        } else {
            let (tx, rx) = unbounded();
            self.interrupt_propagators.push(tx);
            self.executors
                .push((SimpleExecutor::new_with(rx, vec![node]), _ctx));
        }
    }

    fn remove_node(&mut self, id: &NID) -> Option<Box<dyn Node<NID>>> {
        if let Some(idx) = self
            .backing
            .iter()
            .position(|node_wrapper| node_wrapper.node.get_id().eq(id))
        {
            return Some(self.backing.remove(idx).destroy());
        }

        let mut found_node = None;
        let mut delete_executor = None;
        for (idx, (executor, _)) in self.executors.iter_mut().enumerate() {
            if let Some(node) = executor.remove_node(id) {
                found_node = Some(node);
                if executor.backing.is_empty() {
                    delete_executor = Some(idx);
                }
            }
        }

        if let Some(idx) = delete_executor {
            self.executors.remove(idx);
        }

        found_node
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{any::Any, time::Duration};

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
    fn test_start() {
        let (_, rx) = unbounded();

        let mut executor = ThreadedExecutor::new_with(
            rx,
            0,
            vec![
                (vec![Box::new(SimpleNode::new(0, 10_000))], 0),
                (vec![Box::new(SimpleNode::new(1, 100_000))], 1),
                (vec![Box::new(SimpleNode::new(2, 110_000))], 2),
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

        for executor in executor.executors.iter() {
            for node_wrapper in executor.0.backing.iter() {
                assert_eq!(node_wrapper.priority, 0);
                let simple_node: &dyn Any = &node_wrapper.node;
                let simple_node: &Box<SimpleNode> = unsafe { simple_node.downcast_ref_unchecked() };
                assert_eq!(simple_node.state, State::Started);
            }
        }

        assert!(!executor.interrupted);
        assert_eq!(executor.state, ExecutorState::Started);
        assert!(executor.start_instant > original_start_instant);
    }

    #[test]
    fn test_check_interrupt() {
        let (tx, rx) = unbounded();

        let mut executor = ThreadedExecutor::new_with(
            rx,
            0,
            vec![
                (vec![Box::new(SimpleNode::new(0, 10_000))], 0),
                (vec![Box::new(SimpleNode::new(1, 100_000))], 1),
                (vec![Box::new(SimpleNode::new(2, 110_000))], 2),
            ],
        );

        tx.send(true).unwrap();

        assert!(executor.check_interrupt());
        for executor in executor.executors.iter_mut() {
            assert!(executor.0.check_interrupt());
        }
    }

    #[test]
    fn test_add_node() {
        let (_, rx) = unbounded();

        let mut executor = ThreadedExecutor::new_with(
            rx,
            0,
            vec![
                (vec![Box::new(SimpleNode::new(0, 10_000))], 0),
                (vec![Box::new(SimpleNode::new(1, 100_000))], 1),
                (vec![Box::new(SimpleNode::new(2, 110_000))], 2),
            ],
        );

        executor.add_node(Box::new(SimpleNode::new(3, 22_000)));

        assert_eq!(executor.backing.len(), 2);
    }

    #[test]
    fn test_add_node_with_context_backing() {
        let (_, rx) = unbounded();

        let mut executor = ThreadedExecutor::new_with(
            rx,
            0,
            vec![
                (vec![Box::new(SimpleNode::new(0, 10_000))], 0),
                (vec![Box::new(SimpleNode::new(1, 100_000))], 1),
                (vec![Box::new(SimpleNode::new(2, 110_000))], 2),
            ],
        );

        executor.add_node_with_context(Box::new(SimpleNode::new(3, 10_000)), 0);
        assert_eq!(executor.backing.len(), 2);
    }

    #[test]
    fn test_add_node_with_context_other_executor() {
        let (_, rx) = unbounded();

        let mut executor = ThreadedExecutor::new_with(
            rx,
            0,
            vec![
                (vec![Box::new(SimpleNode::new(0, 10_000))], 0),
                (vec![Box::new(SimpleNode::new(1, 100_000))], 1),
                (vec![Box::new(SimpleNode::new(2, 110_000))], 2),
            ],
        );

        executor.add_node_with_context(Box::new(SimpleNode::new(3, 10_000)), 1);
        assert_eq!(
            executor
                .executors
                .iter()
                .find(|(_, tid)| tid.eq(&1))
                .unwrap()
                .0
                .backing
                .len(),
            2
        );
    }

    #[test]
    fn test_add_node_same_id() {
        let (_, rx) = unbounded();

        let mut executor = ThreadedExecutor::new_with(
            rx,
            0,
            vec![
                (vec![Box::new(SimpleNode::new(0, 10_000))], 0),
                (vec![Box::new(SimpleNode::new(1, 100_000))], 1),
                (vec![Box::new(SimpleNode::new(2, 110_000))], 2),
            ],
        );

        executor.add_node(Box::new(SimpleNode::new(0, 100_000)));
        assert_eq!(executor.backing.len(), 1);
        assert_eq!(executor.backing[0].node.get_update_delay_us(), 100_000);
    }

    #[test]
    fn test_remove_node_backing() {
        let (_, rx) = unbounded();

        let mut executor = ThreadedExecutor::new_with(
            rx,
            0,
            vec![
                (vec![Box::new(SimpleNode::new(0, 10_000))], 0),
                (vec![Box::new(SimpleNode::new(1, 100_000))], 1),
                (vec![Box::new(SimpleNode::new(2, 110_000))], 2),
            ],
        );

        executor.remove_node(&0);
        assert_eq!(executor.backing.len(), 0);
    }

    #[test]
    fn test_remove_node_executors_no_remove_executor() {
        let (_, rx) = unbounded();

        let mut executor = ThreadedExecutor::new_with(
            rx,
            0,
            vec![
                (vec![Box::new(SimpleNode::new(0, 10_000))], 0),
                (
                    vec![
                        Box::new(SimpleNode::new(1, 100_000)),
                        Box::new(SimpleNode::new(2, 111_111)),
                    ],
                    1,
                ),
                (vec![Box::new(SimpleNode::new(3, 110_000))], 2),
            ],
        );

        executor.remove_node(&1);
        assert_eq!(executor.executors[0].0.backing.len(), 1);
    }

    #[test]
    fn test_remove_node_executors_remove_executor() {
        let (_, rx) = unbounded();

        let mut executor = ThreadedExecutor::new_with(
            rx,
            0,
            vec![
                (vec![Box::new(SimpleNode::new(0, 10_000))], 0),
                (vec![Box::new(SimpleNode::new(1, 100_000))], 1),
                (vec![Box::new(SimpleNode::new(2, 110_000))], 2),
            ],
        );

        executor.remove_node(&1);
        assert_eq!(executor.executors.len(), 1);
    }

    #[test]
    fn test_update_ms() {
        let (_, rx) = unbounded();

        let mut executor = ThreadedExecutor::new_with(
            rx,
            0,
            vec![
                (vec![Box::new(SimpleNode::new(0, 10_000))], 0),
                (vec![Box::new(SimpleNode::new(1, 10_000))], 1),
                (vec![Box::new(SimpleNode::new(2, 10_000))], 2),
            ],
        );

        let start = executor.clock.now();
        executor.update_for_ms(100);
        let end = executor.clock.now();

        // Check that the nodes were  started and updated
        for node_wrapper in executor.backing.iter() {
            assert!(node_wrapper.priority == 0);
            let simple_node: &dyn Any = &node_wrapper.node;
            let simple_node: &Box<SimpleNode> = unsafe { simple_node.downcast_ref_unchecked() };
            assert_eq!(simple_node.state, State::Stopped);
            assert!([9, 10, 11].contains(&simple_node.num));
        }

        for (executor, _) in executor.executors.iter() {
            for node_wrapper in executor.backing.iter() {
                assert!(node_wrapper.priority == 0);
                let simple_node: &dyn Any = &node_wrapper.node;
                let simple_node: &Box<SimpleNode> = unsafe { simple_node.downcast_ref_unchecked() };
                assert_eq!(simple_node.state, State::Stopped);
                assert!([9, 10, 11].contains(&simple_node.num));
            }
        }

        assert!(Duration::from_millis(95) < end - start);
        assert!(end - start < Duration::from_millis(105));
    }

    #[test]
    fn test_update_loop() {
        let (tx, rx) = unbounded();

        let mut executor = ThreadedExecutor::new_with(
            rx,
            0,
            vec![
                (vec![Box::new(SimpleNode::new(0, 10_000))], 0),
                (vec![Box::new(SimpleNode::new(1, 10_000))], 1),
                (vec![Box::new(SimpleNode::new(2, 10_000))], 2),
            ],
        );

        let handle = thread::spawn(move || {
            executor.update_loop();
            executor
        });

        thread::sleep(Duration::from_millis(100));
        tx.send(true).unwrap();

        let executor = handle.join().unwrap();

        // Check that the nodes were  started and updated
        for node_wrapper in executor.backing.iter() {
            assert!(node_wrapper.priority == 0);
            let simple_node: &dyn Any = &node_wrapper.node;
            let simple_node: &Box<SimpleNode> = unsafe { simple_node.downcast_ref_unchecked() };
            assert_eq!(simple_node.state, State::Stopped);
            assert!([9, 10, 11].contains(&simple_node.num));
        }

        for (executor, _) in executor.executors.iter() {
            for node_wrapper in executor.backing.iter() {
                assert!(node_wrapper.priority == 0);
                let simple_node: &dyn Any = &node_wrapper.node;
                let simple_node: &Box<SimpleNode> = unsafe { simple_node.downcast_ref_unchecked() };
                assert_eq!(simple_node.state, State::Stopped);
                assert!([9, 10, 11].contains(&simple_node.num));
            }
        }
    }
}
