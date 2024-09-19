# Minimal Update Client Server

## Description

In this example, you spin up two nodes, the `FibonacciUpdateClient` node requests the 10th number in the fibonacci sequence from the `FibonacciUpdateServer` node.

## Running

To run this example ensure you are in the example's base directory and run:
```sh
cargo run
```

## Breakdown

### [NodeIdentifier](./src/main.rs)

In NComm, all nodes need some sort of identifier so they can be found inside of the Executor.  The only requirement for this identifier is that is must implement PartialEq so Node's with a given identifier can be found inside of an executor.

### [FibonacciRequest, FibonacciUpdate, and FibonacciResponse](./src/main.rs)

The update client sends a FibonacciRequest asking for the nth term in the Fibonacci sequence.  Then, as the update server is calculating the nth term, the update server will update the update client with a `FibonaciUpdate`, which consists of the current term in the sequence as well as the previous number in the sequence.  Finally, once the update server has reached the nth term in the Fibonacci sequence, the update server sends a `FibonacciResponse` containing the nth term in the sequence

### [FibonacciUpdateClient](./src/fibonacci_update_client.rs)

The `FibonacciUpdateClient` has a `LocalUpdateClient` that sends `FibonacciRequest`s and receives `FibonacciUpdate`s as updates and finally a `FibonacciResponse` as a response.  Every 100,000 microseconds, the update client checks for any incoming updates from the update server and prints "Last Num: {last_num} --- Current Num: {current_num}", where last_num is the last number in the Fibonacci sequence and current_num is the current number in the Fibonacci sequence.  Then, the update client checks if there are incoming responses.  If there is an incoming response, the update client prints "f(10) = {nth_term}", where nth_term is the nth term in the Fibonacci sequence before sending another request for the 10th term in the Fibonacci sequence.

### [FibonacciUpdateServer](./src/fibonacci_update_server.rs)

The `FibonacciUpdateServer` has a `LocalUpdateServer` that receives `FibonacciRequest`s and sends `FibonacciUpdate`s as updates before sending a `FibonacciResponse` as the final response.  Every 100,000 microseconds, the update client checks for incoming requests, setting any incoming requests as the current request to handle.  Then, it increments its current term in the Fibonacci sequence before sending an update the the client with the current and last number in the Fibonacci sequence.  Finally, if the update server has the 10th term in the Fibonacci sequence, the update server sends a response containing the 10th term in the Fibonacci sequence to the update client.

### [ThreadPoolExecutor](./src/main.rs)

In the main method, we create a `ThreadPoolExecutor` to execute the Nodes.  In all reality there is no need to use a `ThreadPoolExecutor` in this context as there are only two nodes so one thread can probably schedule the Nodes just fine.  However, to demonstrate how to use another one of the executors provided in the `ncomm-executors` crate, I figured it would make sense to use a ThreadPoolExecutor with 3 threads in the ThreadPool.  For more information on executors, see the `ncomm-executors` documentation.
