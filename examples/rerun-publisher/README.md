# Rerun Publisher

## Description

In this example, one Node is spun up that publishes data randomly selected from a normal distribution with a mean of 1 and standard deviation of 1 to a spawned instance of the Rerun data visualizer.  Although this is arguably a very contrived example of using Rerun with NComm, I would argue that it gives the user a good idea of how to use Rerun with NComm.

## Running

To run this example ensure you are in the example's base directory and run:
```sh
# If you don't already have the rerun-cli installed
cargo install rerun-cli
cargo run

# If you already have the rerun-cli installed
cargo run
```

## Breakdown

### [NodeIdentifier](./src/main.rs)

In NComm, all nodes need some sort of identifier so they can be added and removed from the Executor.  The only requirement for this identifier is that the identifier must implement `PartialEq`.  In this example, I used an enum, but you can just as easily use any other equatable type.

### [Data Collection Node](./src/data_collection_node.rs)

The data collection node has an OS Random number generator, a Normal distribution, and a RerunPublisher that publishes Scalars to a string path.  The node's update loop is run every 10,000 microseconds and involves the node selecting a piece of data from the normal distribution and publishing that data to the running Rerun instance.

### [Rerun Node](./src/main.rs)

The Rerun node is more or less a convenience Node.  All it does is during the `start` state it connects to or spawns a rerun instance and saves the data from the current Rerun session to an optional output_path.  It also makes it easy to create publishers for the Rerun session it is managing by offering a convenience method to create rerun publishers.

### [Simple Executor](./src/main.rs)

In the main method, we create a `SimpleExecutor` to execute the Nodes.  In reality, the `RerunNode` doesn't do anything during the `update` state so the simple executor is only calling `update` on the `DataCollectionNode` so a `SimpleExecutor` is more than enough for the situation.

## Note

This example only opens Rerun the first time it is run, so if you don't see any data logged to Rerun the first time you run the example, run it again and then you'll see the data populate the Rerun viewer.
