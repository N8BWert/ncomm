//!
//! A Network Udp-Based Publisher + Subscriber
//! 
//! The UDP Publisher sends data as a UDP Datagram to some other Subscriber.
//! 

use std::{net::UdpSocket, collections::HashMap};
use std::marker::PhantomData;

use crate::publisher_subscriber::{Publish, SubscribeRemote, Receive};

/// Basic UDP Publisher Type
/// 
/// Publishers can create subscriptions (in this case add addresses) and will send
/// the same data to each of its subscriptions.  This Publisher will use a UDP socket
/// to send data to the subscribers.
pub struct UdpPublisher<'a, Data: Send + Clone, const DATA_SIZE: usize> {
    tx: UdpSocket,
    addresses: Vec<&'a str>,
    phantom: PhantomData<Data>,
}

/// Basic UDP Subscriber Type
/// 
/// Subscribers will receive data from the subscriber and store only the most recent data.
/// In this case, the UDP Subscriber will listen on a UDP socket connection for incoming traffic.
/// It will then store the most recent datagram (decoded) internally
pub struct  UdpSubscriber<Data: Send + Clone, const DATA_SIZE: usize> {
    rx: UdpSocket,
    pub data: Option<Data>,
}

/// Buffered UDP Subscriber Type
/// 
/// The Buffered UDP Subscriber stores incoming data into a buffer that can be cleared on read 
/// (or whenever the user wants to clear it).
pub struct BufferedUdpSubscriber<Data: Send + Clone, const DATA_SIZE: usize> {
    rx: UdpSocket,
    pub data: Vec<Data>,
}

/// Local Subscriber that stores data in a HashMap
/// 
/// The Hash function is used to map a reference to a piece of data to its corresponding key
/// in the data HashMap.
pub struct MappedUdpSubscriber<Data: Send + Clone, const DATA_SIZE: usize> {
    rx: UdpSocket,
    pub data: HashMap<u128, Data>,
    hash: Box<dyn Fn(&Data) -> u128>,
}

impl<'a, Data: Send + Clone, const DATA_SIZE: usize> UdpPublisher<'a, Data, DATA_SIZE> {
    /// Creates a new UdpPublisher with a UdpSocket bound to the bind address and a stored
    /// vector of references to the addresses of the subscribers.
    pub fn new(bind_address: &'a str, addresses: Vec<&'a str>) -> Self {
        let socket = UdpSocket::bind(bind_address).expect("couldn't bind to the given address");
        socket.set_nonblocking(true).unwrap();
        
        Self { tx: socket, addresses, phantom: PhantomData }
    }
}

impl<'a, Data: Send + Clone, const DATA_SIZE: usize>  UdpSubscriber<Data, DATA_SIZE> {
    /// Creates a new UdpSubscriber with a UdpSocket bound to the bind address listening to the from
    /// address.
    /// 
    /// To only listen to communication to this bound address with a specific address, set from_address to Some value.
    pub fn new(bind_address: &'a str, from_address: Option<&'a str>) -> Self {
        let socket = UdpSocket::bind(bind_address).expect("couldn't bind to the given address");
        if let Some(from_address) = from_address {
            socket.connect(from_address).expect("couldn't connect to the given address");
        }
        socket.set_nonblocking(true).unwrap();

        Self { rx: socket, data: None }
    }
}

impl<'a, Data: Send + Clone, const DATA_SIZE: usize> BufferedUdpSubscriber<Data, DATA_SIZE> {
    /// Creates a new BufferedUdpSubscriber with a UdpSocket bound to the bind address listening to the
    /// from address.
    /// 
    /// To only listen to communication on a specific address specify the from_address
    pub fn new(bind_address: &'a str, from_address: Option<&'a str>) -> Self {
        let socket = UdpSocket::bind(bind_address).expect("couldn't bind to the given bind address");
        if let Some(from_address) = from_address {
            socket.connect(from_address).expect("couldn't connect to the given address");
        }
        socket.set_nonblocking(true).unwrap();

        Self { rx: socket, data: Vec::new() }
    }
}

impl<'a, Data: Send + Clone, const DATA_SIZE: usize> MappedUdpSubscriber<Data, DATA_SIZE> {
    /// Create a new MappedUdpSubscriber with a UdpSocket bound to the bind address listening to the
    /// from address.
    /// 
    /// To only listen to communication on a specific address specify the from_address
    pub fn new(bind_address: &'a str, from_address: Option<&'a str>, hash_function: Box<dyn Fn(&Data) -> u128>) -> Self {
        let socket = UdpSocket::bind(bind_address).expect("couldn't bind to the given bind address");
        if let Some(from_address) = from_address {
            socket.connect(from_address).expect("couldn't connect to the given address");
        }
        socket.set_nonblocking(true).unwrap();

        Self {
            rx: socket,
            data: HashMap::new(),
            hash: hash_function,
        }
    }
}

impl<'a, Data: Send + Clone, const DATA_SIZE: usize> Publish<Data> for UdpPublisher<'a, Data, DATA_SIZE> {
    fn send(&mut self, data: Data) {
        let buf: [u8; DATA_SIZE] = unsafe { std::mem::transmute_copy(&data) };
        for address in self.addresses.iter() {
            if let Err(err) = self.tx.send_to(&buf, address) {
                println!("Received Error on Send: {}", err)
            }
        }
    }
}

impl<'a, Data: Send + Clone, const DATA_SIZE: usize> SubscribeRemote<'a> for UdpPublisher<'a, Data, DATA_SIZE> {
    fn add_subscriber(&mut self, address: &'a str) {
        self.addresses.push(address);
    }
}

impl<Data: Send + Clone, const DATA_SIZE: usize> Receive for UdpSubscriber<Data, DATA_SIZE> {
    fn update_data(&mut self) {
        let mut data: Option<Data> = None;
        loop {
            let mut buf = [0u8; DATA_SIZE];
            match self.rx.recv(&mut buf) {
                Ok(_received) => unsafe { data = Some(std::mem::transmute_copy(&buf)); },
                Err(_) => break,
            }
        }
        if let Some(data) = data {
            self.data = Some(data);
        }
    }
}

impl<Data: Send + Clone, const DATA_SIZE: usize> Receive for BufferedUdpSubscriber<Data, DATA_SIZE> {
    fn update_data(&mut self) {
        loop {
            let mut buf = [0u8; DATA_SIZE];
            match self.rx.recv(&mut buf) {
                Ok(_received) => unsafe { self.data.push(std::mem::transmute_copy(&buf)); },
                Err(_) => break,
            }
        }
    }
}

impl<Data: Send + Clone, const DATA_SIZE: usize> Receive for MappedUdpSubscriber<Data, DATA_SIZE> {
    fn update_data(&mut self) {
        loop {
            let mut buf = [0u8; DATA_SIZE];
            match self.rx.recv(&mut buf) {
                Ok(_received) => {
                    let data: Data = unsafe { std::mem::transmute_copy(&buf) };
                    let label = (self.hash)(&data);
                    self.data.insert(label, data);
                },
                Err(_) => break,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time};
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

    #[test]
    // Test that a udp publisher and subscriber can be created.
    fn test_create_udp_publisher_and_subscriber() {
        let subscriber:  UdpSubscriber<u8, 1> =  UdpSubscriber::new("127.0.0.1:8001", Some("127.0.0.1:8000"));
        let publisher: UdpPublisher<u8, 1> = UdpPublisher::new("127.0.0.1:8000", vec!["127.0.0.1:8001"]);

        assert_eq!(subscriber.rx.local_addr().unwrap(), SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8001)));
        assert_eq!(subscriber.data, None);

        assert_eq!(publisher.tx.local_addr().unwrap(), SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8000)));
        assert_eq!(String::from(publisher.addresses[0]), String::from("127.0.0.1:8001"));
    }

    #[test]
    // Test that a udp publisher and subscriber can send data between each other.
    fn test_send_data_udp_publisher_and_subscriber() {
        let mut subscriber:  UdpSubscriber<u8, 1> =  UdpSubscriber::new("127.0.0.1:8003", Some("127.0.0.1:8002"));
        let mut publisher: UdpPublisher<u8, 1> = UdpPublisher::new("127.0.0.1:8002", vec!["127.0.0.1:8003"]);

        publisher.send(5u8);

        // It takes a bit of time to actually send data so we'll sleep for 10 milliseconds
        thread::sleep(time::Duration::from_millis(10));

        subscriber.update_data();

        assert_eq!(subscriber.data.unwrap(), 5);
    }

    #[test]
    // Test that a udp publisher and subscriber can send multiple pieces of data between each other.
    fn test_send_many_data_udp_publisher_and_subscriber() {
        let mut subscriber:  UdpSubscriber<u8, 1> =  UdpSubscriber::new("127.0.0.1:7001", Some("127.0.0.1:7000"));
        let mut publisher: UdpPublisher<u8, 1> = UdpPublisher::new("127.0.0.1:7000", vec!["127.0.0.1:7001"]);

        for i in 0..=8u8 {
            publisher.send(i);
        }

        thread::sleep(time::Duration::from_millis(10));

        subscriber.update_data();

        assert_eq!(subscriber.data.unwrap(), 8);
    }

    #[test]
    // Test that a buffered udp subscriber can be created.
    fn test_create_buffered_udp_subscriber() {
        let subscriber:  BufferedUdpSubscriber<u8, 1> =  BufferedUdpSubscriber::new("127.0.0.1:9001", Some("127.0.0.1:9000"));
        let publisher: UdpPublisher<u8, 1> = UdpPublisher::new("127.0.0.1:9000", vec!["127.0.0.1:9001"]);

        assert_eq!(subscriber.rx.local_addr().unwrap(), SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9001)));
        assert_eq!(subscriber.data.len(), 0);

        assert_eq!(publisher.tx.local_addr().unwrap(), SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9000)));
        assert_eq!(String::from(publisher.addresses[0]), String::from("127.0.0.1:9001"));
    }

    #[test]
    // Test that a udp publisher can send data to the buffered udp subscriber.
    fn test_send_data_udp_publisher_and_buffered_subscriber() {
        let mut subscriber:  BufferedUdpSubscriber<u8, 1> =  BufferedUdpSubscriber::new("127.0.0.1:9003", Some("127.0.0.1:9002"));
        let mut publisher: UdpPublisher<u8, 1> = UdpPublisher::new("127.0.0.1:9002", vec!["127.0.0.1:9003"]);

        publisher.send(5u8);

        // It takes a bit of time to actually send data
        thread::sleep(time::Duration::from_millis(10));

        subscriber.update_data();

        assert_eq!(subscriber.data.len(), 1);
        assert_eq!(subscriber.data[0], 5);
    }

    #[test]
    // Test that a udp publisher can send multiple pieces of data that are cached by the subscriber
    fn test_send_multiple_data_udp_publisher_and_buffered_subscriber() {
        let mut subscriber:  BufferedUdpSubscriber<u8, 1> =  BufferedUdpSubscriber::new("127.0.0.1:9005", Some("127.0.0.1:9004"));
        let mut publisher: UdpPublisher<u8, 1> = UdpPublisher::new("127.0.0.1:9004", vec!["127.0.0.1:9005"]);

        for i in 0..=8u8 {
            publisher.send(i);
        }

        thread::sleep(time::Duration::from_millis(10));

        subscriber.update_data();

        assert_eq!(subscriber.data.len(), 9);
        for i in 0..=8u8 {
            assert_eq!(subscriber.data[i as usize], i);
        }
    }
}