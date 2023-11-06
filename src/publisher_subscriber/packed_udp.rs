//!
//! A Network Udp-Based Publisher + Subscriber that utilizes PackedStruct for Byte Packing
//! 
//! The PackedUdpPublisher sends data as a UDP Datagram to some other PackedUdpSubscriber
//! 

use std::net::UdpSocket;
use std::marker::PhantomData;

use packed_struct::{PackedStruct, PackedStructSlice};
use packed_struct::types::bits::ByteArray;

use crate::publisher_subscriber::{Publish, SubscribeRemote, Receive};

/// Packed Struct UDP Publisher Type
/// 
/// The Packed Udp Publisher has a group of subscribers that it will send a
/// specific data type packed into a packed udp datagram
pub struct PackedUdpPublisher<'a, Data: PackedStruct + Send + Clone> {
    tx: UdpSocket,
    addresses: Vec<&'a str>,
    phantom: PhantomData<Data>,
}

/// Packed Struct Udp Subscriber Type
/// 
/// The Packed Udp Subscriber subscribes to a publisher of some sort of packed data and
/// decodes the data into the packed struct type given by data.
pub struct PackedUdpSubscriber<Data: PackedStruct + Send + Clone, const DATA_SIZE: usize> {
    rx: UdpSocket,
    pub data: Option<Data>,
}

/// Buffered Packed Struct Udp Subscriber Type
/// 
/// The Buffered Packed Struct UDP Subscriber stores incoming data into a buffer that can be cleared or
/// read
pub struct BufferedPackedUdpSubscriber<Data: PackedStruct + Send + Clone, const DATA_SIZE: usize> {
    rx: UdpSocket,
    pub data: Vec<Data>,
}

impl<'a, Data: PackedStruct + Send + Clone> PackedUdpPublisher<'a, Data> {
    /// Create a new PackedUdpPublisher with a socket bound to the bind address that
    /// will send to the addresses listed in the addresses
    pub fn new(bind_address: &'a str, addresses: Vec<&'a str>) -> Self {
        let socket = UdpSocket::bind(bind_address).expect("couldn't bind to the given bind address");
        socket.set_nonblocking(true).unwrap();

        Self {
            tx: socket,
            addresses,
            phantom: PhantomData,
        }
    }
}

impl<'a, Data: PackedStruct + Send + Clone, const DATA_SIZE: usize> PackedUdpSubscriber<Data, DATA_SIZE> {
    /// Create a new PackedUdpSubscriber bound to the bind address.
    /// 
    /// The from_address field is optional and will make it so that the subscriber only listens
    /// to the given address.
    pub fn new(bind_address: &'a str, from_address: Option<&'a str>) -> Self {
        let socket = UdpSocket::bind(bind_address).expect("couldn't bind to the given address");
        if let Some(from_address) = from_address {
            socket.connect(from_address).expect("couldn't connect to the given address");
        }
        socket.set_nonblocking(true).unwrap();

        assert_eq!(<Data as PackedStruct>::ByteArray::len(), DATA_SIZE);

        Self {
            rx: socket,
            data: None,
        }
    }
}

impl<'a, Data: PackedStruct + Send + Clone, const DATA_SIZE: usize> BufferedPackedUdpSubscriber<Data, DATA_SIZE> {
    /// Create a new BufferedPackedSubscriber bound to the bind address
    /// 
    /// The from_address field is optional, but if given it will make this subscriber ignore all communcations
    /// except the one from the given address
    pub fn new(bind_address: &'a str, from_address: Option<&'a str>) -> Self {
        let socket = UdpSocket::bind(bind_address).expect("couldn't bind to the given address");
        if let Some(from_address) = from_address {
            socket.connect(from_address).expect("couldn't connect to the given address");
        }
        socket.set_nonblocking(true).unwrap();

        assert_eq!(<Data as PackedStruct>::ByteArray::len(), DATA_SIZE);

        Self {
            rx: socket,
            data: Vec::new(),
        }
    }
}

impl<'a, Data: PackedStruct + Send + Clone> Publish<Data> for PackedUdpPublisher<'a, Data> {
    fn send(&mut self, data: Data) {
        let packed_data = match data.pack() {
            Ok(bytes) => bytes,
            Err(err) => panic!("Unable to Pack Data to Send: {:?}", err),
        };

        let packed_data = packed_data.as_bytes_slice();

        for address in self.addresses.iter() {
            if let Err(err) = self.tx.send_to(&packed_data, address) {
                println!("Error on Sending: {}", err);
            }
        }
    }
}

impl<'a, Data: PackedStruct + Send + Clone> SubscribeRemote<'a> for PackedUdpPublisher<'a, Data> {
    fn add_subscriber(&mut self, address: &'a str) {
        self.addresses.push(address);
    }
}

impl<Data: PackedStruct + Send + Clone, const DATA_SIZE: usize> Receive for PackedUdpSubscriber<Data, DATA_SIZE> {
    fn update_data(&mut self) {
        let mut data: Option<Data> = None;

        loop {
            let mut buf = [0u8; DATA_SIZE];
            let temp = match self.rx.recv(&mut buf) {
                Ok(_received) => Data::unpack_from_slice(&buf[..]),
                Err(_) => break,
            };
            if let Ok(found_data) = temp {
                data = Some(found_data);
            }
        }

        if let Some(data) = data {
            self.data = Some(data);
        }
    }
}

impl<Data: PackedStruct + Send + Clone, const DATA_SIZE: usize> Receive for BufferedPackedUdpSubscriber<Data, DATA_SIZE> {
    fn update_data(&mut self) {
        loop {
            let mut buf = [0u8; DATA_SIZE];
            let temp = match self.rx.recv(&mut buf) {
                Ok(_received) => Data::unpack_from_slice(&buf[..]),
                Err(_) => break,
            };

            if let Ok(found_data) = temp {
                self.data.push(found_data);
            }
        }
    }
}