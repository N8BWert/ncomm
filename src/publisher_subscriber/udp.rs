use std::net::UdpSocket;
use std::marker::PhantomData;

use crate::publisher_subscriber::{Publish, SubscribeRemote, Receive};

pub struct Publisher<'a, Data: Send + Clone, const DATA_SIZE: usize> {
    tx: UdpSocket,
    addresses: Vec<&'a str>,
    phantom: PhantomData<Data>,
}

pub struct Subscriber<Data: Send + Clone, const DATA_SIZE: usize> {
    rx: UdpSocket,
    pub data: Option<Data>,
}

impl<'a, Data: Send + Clone, const DATA_SIZE: usize> Publisher<'a, Data, DATA_SIZE> {
    pub fn new(bind_address: &'a str, addresses: Vec<&'a str>) -> Self {
        let socket = UdpSocket::bind(bind_address).expect("couldn't bind to the given address");
        socket.set_nonblocking(true).unwrap();
        
        Self { tx: socket, addresses: addresses, phantom: PhantomData }
    }
}

impl<'a, Data: Send + Clone, const DATA_SIZE: usize> Subscriber<Data, DATA_SIZE> {
    pub fn new(bind_address: &'a str, from_address: &'a str) -> Self {
        let socket = UdpSocket::bind(bind_address).expect("couldn't bind to the given address");
        socket.connect(from_address).expect("couldn't connect to the given address");
        socket.set_nonblocking(true).unwrap();

        Self { rx: socket, data: None }
    }
}

impl<'a, Data: Send + Clone, const DATA_SIZE: usize> Publish<Data> for Publisher<'a, Data, DATA_SIZE> {
    fn send(&self, data: Data) {
        let buf: [u8; DATA_SIZE] = unsafe { std::mem::transmute_copy(&data) };
        for address in self.addresses.iter() {
            if let Err(err) = self.tx.send_to(&buf, address) {
                println!("Received Error on Send: {}", err)
            }
        }
    }
}

impl<'a, Data: Send + Clone, const DATA_SIZE: usize> SubscribeRemote<'a> for Publisher<'a, Data, DATA_SIZE> {
    fn add_subscriber(&mut self, address: &'a str) {
        self.addresses.push(address);
    }
}

impl<Data: Send + Clone, const DATA_SIZE: usize> Receive for Subscriber<Data, DATA_SIZE> {
    fn update_data(&mut self) {
        let mut data: Option<Data> = None;
        loop {
            let mut buf = [0u8; DATA_SIZE];
            match self.rx.recv(&mut buf) {
                Ok(_received) => unsafe { data = Some(std::mem::transmute_copy(&buf)); },
                Err(_) => break,
            }
        }
        self.data = data;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time};
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

    #[test]
    fn test_create_udp_publisher_and_subscriber() {
        let subscriber: Subscriber<u8, 1> = Subscriber::new("127.0.0.1:8001", "127.0.0.1:8000");
        let publisher: Publisher<u8, 1> = Publisher::new("127.0.0.1:8000", vec!["127.0.0.1:8001"]);

        assert_eq!(subscriber.rx.local_addr().unwrap(), SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8001)));
        assert_eq!(subscriber.data, None);

        assert_eq!(publisher.tx.local_addr().unwrap(), SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8000)));
        assert_eq!(String::from(publisher.addresses[0]), String::from("127.0.0.1:8001"));
    }

    #[test]
    fn test_send_data_udp_publisher_and_subscriber() {
        let mut subscriber: Subscriber<u8, 1> = Subscriber::new("127.0.0.1:8001", "127.0.0.1:8000");
        let publisher: Publisher<u8, 1> = Publisher::new("127.0.0.1:8000", vec!["127.0.0.1:8001"]);

        publisher.send(5u8);

        // It takes a bit of time to actually send data so we'll sleep for 10 milliseconds
        thread::sleep(time::Duration::from_millis(10));

        subscriber.update_data();

        assert_eq!(subscriber.data.unwrap(), 5);
    }

    #[test]
    fn test_send_many_data_udp_publisher_and_subscriber() {
        let mut subscriber: Subscriber<u8, 1> = Subscriber::new("127.0.0.1:7001", "127.0.0.1:7000");
        let publisher: Publisher<u8, 1> = Publisher::new("127.0.0.1:7000", vec!["127.0.0.1:7001"]);

        for i in 0..=8u8 {
            publisher.send(i);
        }

        thread::sleep(time::Duration::from_millis(10));

        subscriber.update_data();

        assert_eq!(subscriber.data.unwrap(), 8);
    }
}