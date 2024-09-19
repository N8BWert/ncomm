# Minimal Publisher Subscriber

## Description

In this example you spin up two nodes in a single-threaded executor.  One of the nodes is a simple publisher who publishes the message "Hello, World! {count}" to a subscriber who prints the message to the console.

## Running

To run this example ensure you are in the example's base directory and run:
```sh
cargo run
```

## Breakdown

### [NodeIdentifier](./src/main.rs)

In NComm, all nodes need some sort of identifier so they can be found inside of the Executor.  The only requirement for this identifier is that is must implement PartialEq so Node's with a given identifier can be found inside of an executor.

### [Minimal Publisher](./src/minimal_publisher.rs)

The `MinimalPublisher` has a `LocalPublisher` that can be used to send strings.  Every 500,000 microseconds, the `MinimalPublisher` sends the string "Hello, World! {count}", where count is the number of times the publisher has published the string "Hello, World!" to the subscriber.

### [Minimal Subscriber](./src/minimal_subscriber.rs)

The `MinimalSubscriber` has a `LocalSubscriber` that subscribes to receive strings.  Every 500,000 microseconds, the `MinimalSubscriber` checks the currently published string message from the publisher and prints "I heard: {message}", where message is the "Hello, World! {count}" value from the publisher.

### [Simple Executor](./src/main.rs)

In the main file, a SimpleExecutor is created an populated with an owned heap pointer to the nodes.  This executor will then execute the `update` methods on the nodes at their update rate (500,000 microseconds).  For more information on executors, see the `ncomm-executors` documentation.
