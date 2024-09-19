# NComm

NComm is a robotics framework designed for developing large parallel robotics projects.

## Description

The idea behind NComm is that the robotics framework should provide structure and communication primitives but, other than that, the user should be free to use whatever they want.

## Technical Overview

In NComm (like Ros) a single "task" or unit of work is considered a Node.  In NComm, nodes implement the Node interface which defines specific actions and properties of a node.

For example, imagine a single unit of work that receives sensor data and is responsible for performing fusion on the various pieces of sensor data.  This node would typically look something like:

```rust
pub struct SensorFusionNode {
    // Sensor subscribers
    temperature_sensor_subscriber,
    imu_sensor_subscriber,
    ...

    // Data publishers
    state_estimation_publisher,
}
```

Then implementing the Node interface for the SensorFusionNode requires specifying an update delay for the node as well as an optional behavior for the node given specific states.  First, I'll explain what the various states entail.

### The `Start` State

The start state is run just before normal execution begins.  This makes start the perfect place to perform any necessary initialization logic that must occur before a system begins running.

If the user is familiar with Arduino or Game Development, this is typically notated as `void setup()` and can be used to configure peripherals before the normal execution of the system begins.

For our SensorFusionNode, the `Start` state could involve resetting internal state
parameters or publishing an initial known state over the `state_estimation_publisher`.

### The `Update` State

The update state is effectively the main operation of a Node.  This method is called every `update_delay` microseconds and almost certainly contains the actual 
runtime functionality or the node.

If the user is familiar with Arduino or Game Development, this is typically notated as `void update()`.

For our SensorFusionNode, the `Update` state would involve taking the incoming sensor values, calculating some state estimate and publishing that state estimate via the `state_estimate_publisher`.

### The `Shutdown` State

The shutdown state is used to clean up any necessary work that was started during the `Update` state.  This could involve publishing a final message over the publisher or storing data for the next execution of the system.

For our SensorFusionNode, the `Stop` state could involve saving the current state estimation to some log file so it can be used when the system begins again.

## Getting Started

To get started create a new rust binary crate via:
```sh
cargo new --bin
```

Then, add ncomm as a dependency by running:
```sh
cargo add ncomm
```

This will add ncomm with all standard library features enabled.  Currently, this is the most complete version of NComm, but I will be adding more support to no_std and alloc environments in the future, in which case you can use NComm in those environments by instead running the following:
```sh
# for no_std
cargo add ncomm --default-features=false --features=nostd
# or for alloc (which automatically also includes no_std)
cargo add ncomm --default-features=false --features=alloc
```

## API Documentation

The API documentation for NComm can be found on [crates.io](https://crates.io/crates/ncomm) for release versions.

To view the API documentation for the current dev version of NComm, clone the repo and run:
```sh
cargo doc --no-deps
```

## Integrations

Currently NComm is has integration with the following packages:

* Rerun - NComm has integration with the Rerun data visualizer as both a publisher and node.  To enable Rerun integration add the feature "rerun" to ncomm, ncomm-nodes, or ncomm-publishers-and-subscribers.

## Examples

NComm has a number of example projects to show just a small amount of what NComm can do.  The documentation for these examples can be found [here](./examples/README.md), or by navigating to the [examples](./examples/) folder.

## Features

To allow NComm to have the possibility of being used on as many types of devices as possible I'm using a heavy dose of Rust features to conditionally compile parts of the crates.  Specifically, each crate in the NComm collection has the following features:
* nostd - Use the nostd feature when running on a target that doesn't support the Rust standard library (i.e. resource constrained embedded devices)
* alloc - Use the alloc feature when running ona target that doesn't support the Rust standard library but there is some global allocator available so allocating a bit of memory isn't out of the question.  As a note, this will almost always also include the nostd feature of the crate.  This is because it is impossible to really run Rust without the core library (i.e. no-std) so a lot of the bare necessities of the NComm crate come from the nostd compatibility.
* std (default) - The full availability of the Rust standard library is at your disposal.

Note: I don't think there is any reason to have both "alloc" and "nostd" selected at the same time so I would highly recommend doing that as I'm pretty sure weird things will happen.

In addition to the above features, NComm also has the following feature:
* rerun - Enable Rerun integration support (available in ncomm, ncomm-nodes, and ncomm-publishers-and-subscribers)
* rerun-web-viewer - Enable the Rerun web viewer (available in ncomm-nodes)

## Why?

Why NComm?  Well that's a great question.  I created NComm because I feel like Ros had the right idea it just executed on the idea poorly.  Specifically, I love the idea of Nodes, publishers and subscribers, clients and servers, and action clients and servers but their performance is just plainly laughable in Ros because all types of communication need to be able to be sent between C++ code and Python code.

In addition, I feel like the split of the project across Python and C++ is necessitated by the verbosity of C++ which quickly leads to a split between software developers between the C++ and Python stack.  Additionally, Rust can facilitate incredibly nice zero-cost abstraction that makes it significantly less verbose than C++ while maintaining equal-to-better performance.

Every Rust project would also be amiss if it didn't mention something about memory safety so here's that part.  Rust is memory-safe and thread-safe by design (and by association so is NComm).  This means that with NComm you are very unlikely to encounter NREs and other common pitfalls that significantly impede the progress of C++ development.  NComm is also natively thread-safe so on multi-core systems NComm can often execute with significant performance improvements as more cores are added.

## Projects Using NComm

As NComm grows, I'd like to keep this README updated with links to any projects that use NComm both so new users can see how to use NComm in real projects and because I think its cool to highlight interesting projects.

The following projects use NComm:

* RoboJackets [robocup-base-station](https://github.com/RoboJackets/robocup-base-station) (NComm 0.4.1) - The RoboJackets robocup-base-station runs on a Raspberry Pi and translates commands to and from a team of autonomous soccer playing robots.

## Future

Currently, NComm is only available on Rust with the `std` toolchain.  This is fine, but I would love to have `no_std` versions of the communication primitives for embedded development.  However, I don't think it is likely I will make `no_std` executors and instead work on adding communication primitive support to current RTOS's like RTIC and Embassy (both of which much better than I could likely create on my own).  In general, I would encourage people to add anything they think is missing to this project.  I can already see a possibility for building secure network communication primitives and creating standard nodes for data visualization and logging so I encourage people to build whatever they see fit.
