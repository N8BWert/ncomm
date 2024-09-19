# NComm Examples

### Minimal Publisher Subscriber [`examples/minimal-publisher-subscriber](./minimal-publisher-subscriber/README.md)

The minimal publisher subscriber example shows how the ROS 2 [Writing a simple publisher and subscriber](https://docs.ros.org/en/humble/Tutorials/Beginner-Client-Libraries/Writing-A-Simple-Cpp-Publisher-And-Subscriber.html) tutorial would be completed in NComm.  In short, the principle of the example is that there are two Nodes, one that publishes the string "Hello, World! {num}", where num is an incrementing integer that contains the number of times the publisher has published.  The client receives this string and prints the string "I header: {message}", where message is the string the publisher published.

### Minimal Client Server [`examples/minimal-client-server`](./minimal-client-server/README.md)

The minimal client server example shows how the ROS 2 [Writing a simple service and client](https://docs.ros.org/en/humble/Tutorials/Beginner-Client-Libraries/Writing-A-Simple-Cpp-Service-And-Client.html) tutorial would be complete in NComm.  In short, the principle of the example is there are two Nodes, one that sends a request for the sum of two integers and one that sends the sum of the two addends back.

### Minimal Update Client Server [`examples/minimal-update-client-server](./minimal-update-client-server/README.md)

The minimal update client server example shows how the ROS 2 [Writing an action server and client](https://docs.ros.org/en/humble/Tutorials/Intermediate/Writing-an-Action-Server-Client/Cpp.html) tutorial would be completed in NComm.  In short, the principle of the example is that a client asks for the nth term in the Fibonacci sequence and the update server slowly increments from the 0th term to the nth term in the Fibonacci sequence sending the current and previous term in the sequence until it reaches the nth term when it sends the nth term in the sequence back to the client.

### Rerun Publisher [`examples/rerun-publisher](./rerun-publisher/README.md")

The rerun publisher example shows a contrived example of how you can connect to the Rerun with NComm.  In the example, a node publishes data sampled from a normal distribution to an instance of Rerun instantiated by creating a Rerun node.
