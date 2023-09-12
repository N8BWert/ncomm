//!
//! A Network Udp-Based Publisher + Subscriber
//! 
//! The UDP Publisher sends data as a UDP Datagram to some other Subscriber.
//! 

use std::net::UdpSocket;
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

impl<'a, Data: Send + Clone, const DATA_SIZE: usize> UdpPublisher<'a, Data, DATA_SIZE> {
    /// Creates a new UdpPublisher with a UdpSocket bound to the bind address and a stored
    /// vector of references to the addresses of the subscribers.
    pub fn new(bind_address: &'a str, addresses: Vec<&'a str>) -> Self {
        let socket = UdpSocket::bind(bind_address).expect("couldn't bind to the given address");
        socket.set_nonblocking(true).unwrap();
        
        Self { tx: socket, addresses: addresses, phantom: PhantomData }
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

impl<'a, Data: Send + Clone, const DATA_SIZE: usize> Publish<Data> for UdpPublisher<'a, Data, DATA_SIZE> {
    fn send(&self, data: Data) {
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

impl<Data: Send + Clone, const DATA_SIZE: usize> Receive for  UdpSubscriber<Data, DATA_SIZE> {
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
        let mut subscriber:  UdpSubscriber<u8, 1> =  UdpSubscriber::new("127.0.0.1:8001", Some("127.0.0.1:8000"));
        let publisher: UdpPublisher<u8, 1> = UdpPublisher::new("127.0.0.1:8000", vec!["127.0.0.1:8001"]);

        publisher.send(5u8);

        // It takes a bit of time to actually send data so we'll sleep for 10 milliseconds
        thread::sleep(time::Duration::from_millis(10));

        subscriber.update_data();

        assert_eq!(subscriber.data.unwrap(), 5);
    }

    #[test]
    // Test that a udp publisher and subscriber can send multiple datas between each other.
    fn test_send_many_data_udp_publisher_and_subscriber() {
        let mut subscriber:  UdpSubscriber<u8, 1> =  UdpSubscriber::new("127.0.0.1:7001", Some("127.0.0.1:7000"));
        let publisher: UdpPublisher<u8, 1> = UdpPublisher::new("127.0.0.1:7000", vec!["127.0.0.1:7001"]);

        for i in 0..=8u8 {
            publisher.send(i);
        }

        thread::sleep(time::Duration::from_millis(10));

        subscriber.update_data();

        assert_eq!(subscriber.data.unwrap(), 8);
    }
}