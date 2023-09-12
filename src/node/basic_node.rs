//!
//! Basic Node Example.
//! 

use crate::node::Node;

/// Test Basic Node
/// 
/// This node was meant to test the most basic possible node that contains its own data and
/// does its own processing.
pub struct BasicNode<'a> {
    name: &'a str,
    update_delay: u128,
    test_number: u128,
}

impl<'a> BasicNode<'a> {
    /// Creates a new Basic Node with given name and update delay
    pub const fn new(name: &'a str, update_delay: u128) -> Self {
        Self{ name, update_delay, test_number: 0}
    }
}

impl<'a> Node for BasicNode<'a> {
    fn name(&self) -> String {
        String::from(self.name)
    }

    fn start(&mut self) {
        self.test_number = 1;
    }

    fn update(&mut self) {
        self.test_number += 1;
    }

    fn get_update_delay(&self) -> u128 {
        self.update_delay
    }

    fn shutdown(&mut self) {
        self.test_number = u128::MAX;
    }

    fn debug(&self) -> String {
        format!(
            "Basic Node:\n{}\n{}\n{}\n",
            self.name(),
            self.update_delay,
            self.test_number,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_basic_node() {
        let basic_node = BasicNode::new("test node", 12);
        assert_eq!(basic_node.name, String::from("test node"));
        assert_eq!(basic_node.update_rate, 12);
        assert_eq!(basic_node.test_number, 0);
    }

    #[test]
    fn test_start_basic_node() {
        let mut basic_node = BasicNode::new("test node", 12);
        basic_node.start();
        assert_eq!(basic_node.test_number, 1);
    }

    #[test]
    fn test_update_basic_node() {
        let mut basic_node = BasicNode::new("test node", 12);
        for _ in 0..12 {
            basic_node.update();
        }
        assert_eq!(basic_node.test_number, 12);
    }

    #[test]
    fn test_get_update_rate() {
        let basic_node = BasicNode::new("test node", 12);
        assert_eq!(basic_node.get_update_rate(), 12);
    }

    #[test]
    fn test_shutdown_basic_node() {
        let mut basic_node = BasicNode::new("test node", 12);
        basic_node.shutdown();
        assert_eq!(basic_node.test_number, u128::MAX);
    }
}