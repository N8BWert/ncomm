# Minimal Client Server

## Description

In this example, you spin up two nodes in a single-threaded executor.  One of the nodes is a simple client who asks the server for the sum of two numbers (a and b).  The server node then responds with the sum of the two numbers.

## Running

To run this example ensure you are in the example's base directory and run:
```sh
cargo run
```

## Breakdown

### [NodeIdentifier](./src/main.rs)

In NComm, all nodes need some sort of identifier so they can be added and removed from the Executor.  The only requirement for this identifier is that the identifier must implement `PartialEq`.  In this example, I used an enum, but you can just as easily use any other equatable type.

### [AddTwoIntsRequest and AddTwoIntsResponse](./src/main.rs)

In NComm, there is no need to use message files because everything is Rust.  Instead, you can define your own types (as well as methods) as message data.  In this example, The `AddTwoIntsRequest` is a struct consisting of two fields `a` and `b` which are both `u64`s.  The response type, `AddTwoIntsResponse` consists of a singular field `sum` which is a `u64`.

### [Minimal Client](./src/minimal_client.rs)

The `MinimalClient` has a `LocalClient` that sends `AddTwoIntsRequest`s as requests and receives `AddTwoIntsResponses` as responses.  The `MinimalClient` also has an update_delay of 500,000us which means that its update function is called every 500,000us.  Every `update`, the `MinimalClient` sends a request containing two random `u64`s and checks for incoming responses.  If there are responses, the `MinimalClient` prints "{a} + {b} = {sum}", where a and b are the requested addends and sum is their sum.

### [Minimal Server](./src/minimal_server.rs)

The `MinimalServer` has a `LocalServer` that receives `AddTwoIntsRequest`s and responds with `AddTwoIntsResponses`.  The `MinimalServer` also has an update delay of 500,000us which means that its update function is called every 500,000 microseconds.  Every `update`, the server checks for incoming requests and responds with the sum of the addends.

### [Simple Executor](./src/main.rs)

In the main function, we create a `SimpleExecutor` to execute the Nodes.  The simple executor is a single threaded executor that maintains a queue of nodes to execute and runs their update function every update delay.  For more information on executors, see the `ncomm-executors` documentation.
