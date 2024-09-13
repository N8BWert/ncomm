//!
//! This example outlines how to utilize NComm with the rerun data
//! visualization framework.
//!
//! Before running this example, please ensure the rerun-cli is installed
//! by running:
//! ```sh
//! cargo install rerun-cli --locked
//! ```
//!
//! This example is an incredibly trivial example of how to utilize Rerun with
//! NComm and involves a node publishing randomly sampled data from a normal
//! distribution every 10 milliseconds.
//!

#[deny(missing_docs)]
use ncomm_core::Executor;
use ncomm_executors::SimpleExecutor;
use ncomm_nodes::RerunNode;

use crossbeam::channel::unbounded;
use ctrlc;

mod data_collection_node;
use data_collection_node::DataCollectionNode;

/// Identifier for the different nodes in the system
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NodeIdentifier {
    /// The Rerun Node
    RerunNode,
    /// The Data Collection Node
    DataCollectionNode,
}

fn main() {
    println!("Creating Rerun Node");
    let mut rerun_node = RerunNode::new_rerun_spawn(
        "ncomm-example-project",
        1_000_000,
        Some("ncomm-example.rrd"),
        NodeIdentifier::RerunNode,
    )
    .unwrap();

    println!("Creating Data Collection Node");
    let data_collection_node =
        DataCollectionNode::new(rerun_node.create_rerun_publisher("normal/scalar".to_string()));

    let (tx, rx) = unbounded();
    ctrlc::set_handler(move || tx.send(true).expect("Unable to send data"))
        .expect("Error setting Ctrl-C handler");

    println!("Creating Executor");
    let mut executor = SimpleExecutor::new_with(
        rx,
        vec![Box::new(rerun_node), Box::new(data_collection_node)],
    );

    println!("Updating Nodes");
    executor.update_for_ms(1_000);
}
