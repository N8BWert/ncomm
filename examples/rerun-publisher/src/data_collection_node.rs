//!
//! The Data Collection Node "collects" data from a normal distribution
//! and publishes the data to rerun to view the data
//!

use super::NodeIdentifier;

use rand::{self, rngs::OsRng};
use rand_distr::{Distribution, Normal};

use ncomm_core::{Node, Publisher};
use ncomm_publishers_and_subscribers::rerun::RerunPublisher;

use rerun::Scalar;

/// Node that "collects" data from a normal distribution and publishes
/// the results to the Rerun data visualizer.
pub struct DataCollectionNode {
    /// The random number generator for this node
    rng: OsRng,
    /// The normal distribution to sample from
    distribution: Normal<f64>,
    /// The publisher to publish rerun data to
    publisher: RerunPublisher<String, Scalar>,
}

impl DataCollectionNode {
    /// Create a new DataCollectionNode from a Rerun Publisher
    pub fn new(publisher: RerunPublisher<String, Scalar>) -> Self {
        let distribution = Normal::new(1.0, 1.0).unwrap();
        let rng = OsRng::default();

        Self {
            rng,
            distribution,
            publisher,
        }
    }
}

impl Node<NodeIdentifier> for DataCollectionNode {
    fn get_id(&self) -> NodeIdentifier {
        NodeIdentifier::DataCollectionNode
    }

    fn get_update_delay_us(&self) -> u128 {
        10_000
    }

    fn update(&mut self) {
        let data = self.distribution.sample(&mut self.rng);
        println!("Collected {}", data);
        self.publisher.publish(Scalar::new(data)).unwrap();
    }
}
